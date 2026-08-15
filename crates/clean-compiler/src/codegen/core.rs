//! Core-module emission (the first half of pass [10], Platform 14
//! §14.4.2): MIR → core WebAssembly. Every host import carries its
//! interface-qualified module name (`clean:host/routing@0.1.0` — Platform
//! 15 P2); the component wrap and WIT embedding are `component.rs`'s job
//! (step 8).
//!
//! Determinism: sections are emitted in index order derived from MIR, which
//! is itself in declaration order — no maps are iterated (§14.5).

use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, DataSection, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, Instruction, MemArg, MemorySection,
    MemoryType, Module, TypeSection, ValType,
};

use crate::mir::{CmpOp, I64Op, Inst, MirFunction, MirProgram, Val, DATA_OFFSET};

/// Memory map after the static data blob: a fixed 64-byte return area for
/// Canonical ABI retptr results, then the bump-allocator heap base, both
/// 8-aligned.
fn memory_map(program: &MirProgram) -> (u32, u32) {
    let data_end = DATA_OFFSET + program.data.len() as u32;
    let ret_area = (data_end + 7) & !7;
    let heap_base = ret_area + 64;
    (ret_area, heap_base)
}

fn val(v: Val) -> ValType {
    match v {
        Val::I32 => ValType::I32,
        Val::I64 => ValType::I64,
    }
}

/// Emits the core module: one type per import and per function, imports in
/// MIR order, `init`/`handle` exported, plus an exported linear memory (the
/// Canonical ABI realloc export joins in step 6 with the first pointer
/// type).
pub fn emit_core(program: &MirProgram) -> Vec<u8> {
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

    let (ret_area, heap_base) = memory_map(program);

    let import_count = program.imports.len() as u32;
    // The `handle` entry point needs a boundary shim: the world declares
    // `handle: func(handler-id: u32)` (core i32) while the Clean function
    // takes a surface `integer` (i64). The shim widens and forwards.
    let mut handle_shim: Option<u32> = None;
    for (offset, function) in program.functions.iter().enumerate() {
        let type_index = import_count + offset as u32;
        types.ty().function(
            function.params.iter().copied().map(val).collect::<Vec<_>>(),
            function
                .results
                .iter()
                .copied()
                .map(val)
                .collect::<Vec<_>>(),
        );
        functions.function(type_index);
        if function.export {
            if function.name == "handle" && function.params == [Val::I64] {
                handle_shim = Some(import_count + offset as u32);
            } else {
                exports.export(
                    &function.name,
                    ExportKind::Func,
                    import_count + offset as u32,
                );
            }
        }
        code.function(&emit_function(function, import_count, ret_area));
    }

    let mut extra_functions = 0u32;
    if let Some(target) = handle_shim {
        let shim_type = import_count + program.functions.len() as u32 + extra_functions;
        types
            .ty()
            .function(vec![ValType::I32], Vec::<ValType>::new());
        functions.function(shim_type);
        exports.export("handle", ExportKind::Func, shim_type);
        let mut shim = Function::new([]);
        shim.instruction(&Instruction::LocalGet(0));
        shim.instruction(&Instruction::I64ExtendI32U);
        shim.instruction(&Instruction::Call(target));
        shim.instruction(&Instruction::End);
        code.function(&shim);
        extra_functions += 1;
    }

    // `cabi_realloc` — the Canonical ABI allocator hosts call to lift
    // values into the guest: a deterministic bump allocator over global 0.
    // Growth checks are a known gap until M6 (TESTING.md §7).
    let realloc_type = import_count + program.functions.len() as u32 + extra_functions;
    types.ty().function(
        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        vec![ValType::I32],
    );
    functions.function(realloc_type);
    exports.export("cabi_realloc", ExportKind::Func, realloc_type);
    code.function(&emit_realloc());

    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(heap_base as i32),
    );

    memories.memory(MemoryType {
        minimum: 1,
        maximum: None,
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
            &ConstExpr::i32_const(DATA_OFFSET as i32),
            program.data.iter().copied(),
        );
        module.section(&data);
    }
    module.finish()
}

/// `cabi_realloc(old_ptr, old_size, align, new_size) -> ptr`: aligned bump
/// allocation from global 0; the old block is never reclaimed (arena reset
/// discipline arrives with the memory model in M6).
fn emit_realloc() -> Function {
    use Instruction as I;
    let mut f = Function::new([(1, ValType::I32)]);
    // aligned = (heap + align - 1) & !(align - 1)
    f.instruction(&I::GlobalGet(0));
    f.instruction(&I::LocalGet(2));
    f.instruction(&I::I32Add);
    f.instruction(&I::I32Const(1));
    f.instruction(&I::I32Sub);
    f.instruction(&I::LocalGet(2));
    f.instruction(&I::I32Const(1));
    f.instruction(&I::I32Sub);
    f.instruction(&I::I32Const(-1));
    f.instruction(&I::I32Xor);
    f.instruction(&I::I32And);
    f.instruction(&I::LocalTee(4));
    // heap = aligned + new_size
    f.instruction(&I::LocalGet(3));
    f.instruction(&I::I32Add);
    f.instruction(&I::GlobalSet(0));
    f.instruction(&I::LocalGet(4));
    f.instruction(&I::End);
    f
}

fn emit_function(function: &MirFunction, import_count: u32, ret_area: u32) -> Function {
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
        emit_inst(inst, import_count, ret_area, &mut f);
    }
    f.instruction(&Instruction::End);
    f
}

fn emit_inst(inst: &Inst, import_count: u32, ret_area: u32, f: &mut Function) {
    use Instruction as I;
    match inst {
        Inst::I32Const(v) => f.instruction(&I::I32Const(*v)),
        Inst::I64Const(v) => f.instruction(&I::I64Const(*v)),
        Inst::LocalGet(slot) => f.instruction(&I::LocalGet(*slot)),
        Inst::LocalSet(slot) => f.instruction(&I::LocalSet(*slot)),
        Inst::CallImport(index) => f.instruction(&I::Call(*index)),
        Inst::Call(index) => f.instruction(&I::Call(import_count + *index)),
        Inst::I64Bin(op) => f.instruction(&match op {
            I64Op::Add => I::I64Add,
            I64Op::Sub => I::I64Sub,
            I64Op::Mul => I::I64Mul,
            I64Op::DivS => I::I64DivS,
            I64Op::RemS => I::I64RemS,
        }),
        Inst::I64Cmp(op) => f.instruction(&match op {
            CmpOp::Eq => I::I64Eq,
            CmpOp::Ne => I::I64Ne,
            CmpOp::LtS => I::I64LtS,
            CmpOp::LeS => I::I64LeS,
            CmpOp::GtS => I::I64GtS,
            CmpOp::GeS => I::I64GeS,
        }),
        Inst::I32Cmp(op) => f.instruction(&match op {
            CmpOp::Eq => I::I32Eq,
            CmpOp::Ne => I::I32Ne,
            CmpOp::LtS => I::I32LtS,
            CmpOp::LeS => I::I32LeS,
            CmpOp::GtS => I::I32GtS,
            CmpOp::GeS => I::I32GeS,
        }),
        Inst::I32Eqz => f.instruction(&I::I32Eqz),
        Inst::I32WrapI64 => f.instruction(&I::I32WrapI64),
        Inst::I64ExtendI32U => f.instruction(&I::I64ExtendI32U),
        Inst::RetAreaPtr => f.instruction(&I::I32Const(ret_area as i32)),
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
        Inst::If { result, then, els } => {
            let block_type = match result {
                Some(v) => BlockType::Result(val(*v)),
                None => BlockType::Empty,
            };
            f.instruction(&I::If(block_type));
            for inst in then {
                emit_inst(inst, import_count, ret_area, f);
            }
            if !els.is_empty() {
                f.instruction(&I::Else);
                for inst in els {
                    emit_inst(inst, import_count, ret_area, f);
                }
            }
            f.instruction(&I::End)
        }
        Inst::Return => f.instruction(&I::Return),
        Inst::Drop => f.instruction(&I::Drop),
    };
}
