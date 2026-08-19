//! Core-module emission (the first half of pass [10], Platform 14
//! §14.4.2): MIR → core WebAssembly. Every host import carries its
//! interface-qualified module name (`clean:host/routing@0.1.0` — Platform
//! 15 P2); the component wrap and WIT embedding are `component.rs`'s job
//! (step 8).
//!
//! Memory follows MMD-01: static data at `DATA_SECTION_START` (the shared
//! empty-string constant first), a fixed Canonical ABI return area after
//! it, and the heap at `HEAP_START` behind the `__heap_start`/`__heap_ptr`
//! globals. The function index space is: imports, user functions, runtime
//! helpers (ADR 0004), then the `handle` shim and `cabi_realloc`.
//!
//! Determinism: sections are emitted in index order derived from MIR, which
//! is itself in declaration order — no maps are iterated (§14.5).

use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, DataSection, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, Instruction, MemArg, MemorySection,
    MemoryType, Module, TypeSection, ValType,
};

use crate::layout::{
    DATA_SECTION_START, HEAP_PTR_GLOBAL, HEAP_START, HEAP_START_GLOBAL, WASM_PAGE_SIZE,
};
use crate::mir::runtime::RuntimeFn;
use crate::mir::{CmpOp, F64Op, F64Un, I32Op, I64Op, Inst, MirFunction, MirProgram, Val};

/// Return-area address and size: after the static data, 8-aligned, below
/// `HEAP_START` (MMD-01 leaves the region between data and heap to the
/// compiler).
const RET_AREA_SIZE: u32 = 64;

fn memory_map(program: &MirProgram) -> Result<(u32, u32), String> {
    let data_end = DATA_SECTION_START + program.data.len() as u32;
    let ret_area = (data_end + 7) & !7;
    let static_end = ret_area + RET_AREA_SIZE;
    // HEAP_START is fixed (MMD-01); static data cannot spill into the heap.
    // What a conforming compiler does with >1 MiB of static data is a spec
    // gap (DISCOVERIES-M6); until resolved this is an internal capacity
    // invariant surfaced as COM013 by the driver.
    if static_end > HEAP_START {
        return Err(format!(
            "static data ({} bytes) exceeds the fixed heap start boundary at {HEAP_START}",
            program.data.len()
        ));
    }
    Ok((ret_area, static_end))
}

fn val(v: Val) -> ValType {
    match v {
        Val::I32 => ValType::I32,
        Val::I64 => ValType::I64,
        Val::F64 => ValType::F64,
    }
}

/// Function-index geometry shared by every instruction-emission site.
struct EmitCtx {
    import_count: u32,
    user_count: u32,
    ret_area: u32,
}

impl EmitCtx {
    fn runtime_index(&self, f: RuntimeFn) -> u32 {
        self.import_count + self.user_count + f as u32
    }
}

/// Emits the core module.
pub fn emit_core(program: &MirProgram) -> Result<Vec<u8>, String> {
    let mut types = TypeSection::new();
    let mut imports = ImportSection::new();
    let mut functions = FunctionSection::new();
    let mut exports = ExportSection::new();
    let mut code = CodeSection::new();
    let mut memories = MemorySection::new();

    for (index, import) in program.imports.iter().enumerate() {
        types.ty().function(
            import.params.iter().copied().map(val).collect::<Vec<_>>(),
            import.results.iter().copied().map(val).collect::<Vec<_>>(),
        );
        imports.import(
            &import.module,
            &import.name,
            wasm_encoder::EntityType::Function(index as u32),
        );
    }

    let (ret_area, static_end) = memory_map(program)?;
    let ctx = EmitCtx {
        import_count: program.imports.len() as u32,
        user_count: program.functions.len() as u32,
        ret_area,
    };

    // The `handle` entry point needs a boundary shim: the world declares
    // `handle: func(handler-id: u32)` (core i32) while the Clean function
    // takes a surface `integer` (i64). The shim widens, forwards, and
    // wraps the call in the per-request arena scope (TIER-04: reset after
    // each handler returns).
    let mut handle_shim: Option<u32> = None;
    let mut next_type = ctx.import_count;
    let mut next_func = ctx.import_count;
    for function in program.functions.iter().chain(&program.runtime) {
        types.ty().function(
            function.params.iter().copied().map(val).collect::<Vec<_>>(),
            function
                .results
                .iter()
                .copied()
                .map(val)
                .collect::<Vec<_>>(),
        );
        functions.function(next_type);
        if function.export {
            if function.name == "handle" && function.params == [Val::I64] {
                handle_shim = Some(next_func);
            } else {
                exports.export(&function.name, ExportKind::Func, next_func);
            }
        }
        code.function(&emit_function(function, &ctx));
        next_type += 1;
        next_func += 1;
    }

    if let Some(target) = handle_shim {
        types
            .ty()
            .function(vec![ValType::I32], Vec::<ValType>::new());
        functions.function(next_type);
        exports.export("handle", ExportKind::Func, next_func);
        let mut shim = Function::new([(1, ValType::I32)]);
        // save = __heap_ptr (arena push)
        shim.instruction(&Instruction::GlobalGet(HEAP_PTR_GLOBAL));
        shim.instruction(&Instruction::LocalSet(1));
        shim.instruction(&Instruction::LocalGet(0));
        shim.instruction(&Instruction::I64ExtendI32U);
        shim.instruction(&Instruction::Call(target));
        // __heap_ptr = save (arena pop, O(1), no destructors — MMD-03)
        shim.instruction(&Instruction::LocalGet(1));
        shim.instruction(&Instruction::GlobalSet(HEAP_PTR_GLOBAL));
        shim.instruction(&Instruction::End);
        code.function(&shim);
        next_type += 1;
        next_func += 1;
    }

    // `cabi_realloc` — the Canonical ABI allocator hosts call to lift
    // values into the guest; delegates to the MMD-02 allocator (the old
    // block is never reclaimed — arena discipline reclaims wholesale).
    types.ty().function(
        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        vec![ValType::I32],
    );
    functions.function(next_type);
    exports.export("cabi_realloc", ExportKind::Func, next_func);
    let mut realloc = Function::new([]);
    realloc.instruction(&Instruction::LocalGet(3));
    realloc.instruction(&Instruction::LocalGet(2));
    realloc.instruction(&Instruction::Call(ctx.runtime_index(RuntimeFn::Alloc)));
    realloc.instruction(&Instruction::End);
    code.function(&realloc);

    // MMD-01 guest-visible globals.
    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: false,
            shared: false,
        },
        &ConstExpr::i32_const(HEAP_START as i32),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(HEAP_START as i32),
    );
    debug_assert_eq!(HEAP_START_GLOBAL, 0);
    debug_assert_eq!(HEAP_PTR_GLOBAL, 1);
    exports.export("__heap_start", ExportKind::Global, HEAP_START_GLOBAL);
    exports.export("__heap_ptr", ExportKind::Global, HEAP_PTR_GLOBAL);

    // State variables (SMG-01): one mutable global per internal slot, in
    // declaration order, right after the heap globals.
    for (val_ty, init) in &program.state_globals {
        let (val_type, init) = match (val_ty, init) {
            (Val::I32, crate::mir::StateInit::I32(v)) => (ValType::I32, ConstExpr::i32_const(*v)),
            (Val::I64, crate::mir::StateInit::I64(v)) => (ValType::I64, ConstExpr::i64_const(*v)),
            (Val::F64, crate::mir::StateInit::F64(v)) => {
                (ValType::F64, ConstExpr::f64_const((*v).into()))
            }
            mismatch => unreachable!("state global slot/init mismatch: {mismatch:?}"),
        };
        globals.global(
            GlobalType {
                val_type,
                mutable: true,
                shared: false,
            },
            &init,
        );
    }

    // TIER-01: the tier fixes initial/maximum; the active data segment must
    // fit inside the initial commitment. Never `shared` (MMD-05: a shared
    // guest memory is a build error, so one is never emitted).
    let static_pages = static_end.div_ceil(WASM_PAGE_SIZE) as u64;
    memories.memory(MemoryType {
        minimum: program.tier.initial_pages().max(static_pages),
        maximum: Some(program.tier.max_pages()),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    exports.export("memory", ExportKind::Memory, 0);

    let mut module = Module::new();
    module.section(&types);
    module.section(&imports);
    module.section(&functions);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    module.section(&code);
    if !program.data.is_empty() {
        let mut data = DataSection::new();
        data.active(
            0,
            &ConstExpr::i32_const(DATA_SECTION_START as i32),
            program.data.iter().copied(),
        );
        module.section(&data);
    }
    Ok(module.finish())
}

fn emit_function(function: &MirFunction, ctx: &EmitCtx) -> Function {
    // Locals after params, run-length encoded in order.
    let mut runs: Vec<(u32, ValType)> = Vec::new();
    for local in function.locals.iter().copied().map(val) {
        match runs.last_mut() {
            Some((count, ty)) if *ty == local => *count += 1,
            _ => runs.push((1, local)),
        }
    }
    let mut f = Function::new(runs);
    for inst in &function.body {
        emit_inst(inst, ctx, &mut f);
    }
    f.instruction(&Instruction::End);
    f
}

fn emit_inst(inst: &Inst, ctx: &EmitCtx, f: &mut Function) {
    use Instruction as I;
    match inst {
        Inst::I32Const(v) => f.instruction(&I::I32Const(*v)),
        Inst::I64Const(v) => f.instruction(&I::I64Const(*v)),
        Inst::LocalGet(slot) => f.instruction(&I::LocalGet(*slot)),
        Inst::LocalSet(slot) => f.instruction(&I::LocalSet(*slot)),
        Inst::LocalTee(slot) => f.instruction(&I::LocalTee(*slot)),
        Inst::CallImport(index) => f.instruction(&I::Call(*index)),
        Inst::Call(index) => f.instruction(&I::Call(ctx.import_count + *index)),
        Inst::CallRuntime(rt) => f.instruction(&I::Call(ctx.runtime_index(*rt))),
        Inst::I64Bin(op) => f.instruction(&match op {
            I64Op::Add => I::I64Add,
            I64Op::Sub => I::I64Sub,
            I64Op::Mul => I::I64Mul,
            I64Op::DivS => I::I64DivS,
            I64Op::RemS => I::I64RemS,
            I64Op::DivU => I::I64DivU,
            I64Op::RemU => I::I64RemU,
        }),
        Inst::I32Bin(op) => f.instruction(&match op {
            I32Op::Add => I::I32Add,
            I32Op::Sub => I::I32Sub,
            I32Op::Mul => I::I32Mul,
            I32Op::DivU => I::I32DivU,
            I32Op::And => I::I32And,
            I32Op::Or => I::I32Or,
            I32Op::Xor => I::I32Xor,
            I32Op::Shl => I::I32Shl,
            I32Op::ShrU => I::I32ShrU,
        }),
        Inst::I64Cmp(op) => f.instruction(&match op {
            CmpOp::Eq => I::I64Eq,
            CmpOp::Ne => I::I64Ne,
            CmpOp::LtS => I::I64LtS,
            CmpOp::LeS => I::I64LeS,
            CmpOp::GtS => I::I64GtS,
            CmpOp::GeS => I::I64GeS,
            CmpOp::LtU => I::I64LtU,
            CmpOp::GtU => I::I64GtU,
        }),
        Inst::I32Cmp(op) => f.instruction(&match op {
            CmpOp::Eq => I::I32Eq,
            CmpOp::Ne => I::I32Ne,
            CmpOp::LtS => I::I32LtS,
            CmpOp::LeS => I::I32LeS,
            CmpOp::GtS => I::I32GtS,
            CmpOp::GeS => I::I32GeS,
            CmpOp::LtU => I::I32LtU,
            CmpOp::GtU => I::I32GtU,
        }),
        Inst::F64Const(v) => f.instruction(&I::F64Const((*v).into())),
        Inst::F64Bin(op) => f.instruction(&match op {
            F64Op::Add => I::F64Add,
            F64Op::Sub => I::F64Sub,
            F64Op::Mul => I::F64Mul,
            F64Op::Div => I::F64Div,
            F64Op::Min => I::F64Min,
            F64Op::Max => I::F64Max,
        }),
        Inst::F64Un(op) => f.instruction(&match op {
            F64Un::Neg => I::F64Neg,
            F64Un::Abs => I::F64Abs,
            F64Un::Ceil => I::F64Ceil,
            F64Un::Floor => I::F64Floor,
            F64Un::Trunc => I::F64Trunc,
            F64Un::Nearest => I::F64Nearest,
            F64Un::Sqrt => I::F64Sqrt,
        }),
        Inst::F64Cmp(op) => f.instruction(&match op {
            CmpOp::Eq => I::F64Eq,
            CmpOp::Ne => I::F64Ne,
            CmpOp::LtS | CmpOp::LtU => I::F64Lt,
            CmpOp::LeS => I::F64Le,
            CmpOp::GtS | CmpOp::GtU => I::F64Gt,
            CmpOp::GeS => I::F64Ge,
        }),
        Inst::I32Eqz => f.instruction(&I::I32Eqz),
        Inst::I32WrapI64 => f.instruction(&I::I32WrapI64),
        Inst::I64ExtendI32U => f.instruction(&I::I64ExtendI32U),
        Inst::F64ConvertI64S => f.instruction(&I::F64ConvertI64S),
        Inst::F64ConvertI32S => f.instruction(&I::F64ConvertI32S),
        Inst::I64ExtendI32S => f.instruction(&I::I64ExtendI32S),
        Inst::I64TruncF64S => f.instruction(&I::I64TruncF64S),
        Inst::Select => f.instruction(&I::Select),
        Inst::RetAreaPtr => f.instruction(&I::I32Const(ctx.ret_area as i32)),
        Inst::I32Load(offset) => f.instruction(&I::I32Load(MemArg {
            offset: *offset as u64,
            align: 2,
            memory_index: 0,
        })),
        Inst::I32Load8U(offset) => f.instruction(&I::I32Load8U(MemArg {
            offset: *offset as u64,
            align: 0,
            memory_index: 0,
        })),
        Inst::I64Load(offset) => f.instruction(&I::I64Load(MemArg {
            offset: *offset as u64,
            align: 3,
            memory_index: 0,
        })),
        Inst::F64Load(offset) => f.instruction(&I::F64Load(MemArg {
            offset: *offset as u64,
            align: 3,
            memory_index: 0,
        })),
        Inst::I32Store(offset) => f.instruction(&I::I32Store(MemArg {
            offset: *offset as u64,
            align: 2,
            memory_index: 0,
        })),
        Inst::I32Store8(offset) => f.instruction(&I::I32Store8(MemArg {
            offset: *offset as u64,
            align: 0,
            memory_index: 0,
        })),
        Inst::I64Store(offset) => f.instruction(&I::I64Store(MemArg {
            offset: *offset as u64,
            align: 3,
            memory_index: 0,
        })),
        Inst::F64Store(offset) => f.instruction(&I::F64Store(MemArg {
            offset: *offset as u64,
            align: 3,
            memory_index: 0,
        })),
        Inst::MemorySize => f.instruction(&I::MemorySize(0)),
        Inst::MemoryGrow => f.instruction(&I::MemoryGrow(0)),
        Inst::MemoryCopy => f.instruction(&I::MemoryCopy {
            src_mem: 0,
            dst_mem: 0,
        }),
        Inst::GlobalGet(index) => f.instruction(&I::GlobalGet(*index)),
        Inst::GlobalSet(index) => f.instruction(&I::GlobalSet(*index)),
        Inst::Unreachable => f.instruction(&I::Unreachable),
        Inst::If { result, then, els } => {
            let block_type = match result {
                Some(v) => BlockType::Result(val(*v)),
                None => BlockType::Empty,
            };
            f.instruction(&I::If(block_type));
            for inst in then {
                emit_inst(inst, ctx, f);
            }
            if !els.is_empty() {
                f.instruction(&I::Else);
                for inst in els {
                    emit_inst(inst, ctx, f);
                }
            }
            f.instruction(&I::End)
        }
        Inst::Block { body } => {
            f.instruction(&I::Block(BlockType::Empty));
            for inst in body {
                emit_inst(inst, ctx, f);
            }
            f.instruction(&I::End)
        }
        Inst::Loop { body } => {
            f.instruction(&I::Loop(BlockType::Empty));
            for inst in body {
                emit_inst(inst, ctx, f);
            }
            f.instruction(&I::End)
        }
        Inst::Br(depth) => f.instruction(&I::Br(*depth)),
        Inst::BrIf(depth) => f.instruction(&I::BrIf(*depth)),
        Inst::Return => f.instruction(&I::Return),
        Inst::Drop => f.instruction(&I::Drop),
    };
}
