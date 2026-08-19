//! Bridge stub components (Platform 14 §14.14.5): every `clean:bridge/*`
//! interface ships alongside a stub implementing it with recorded-call
//! semantics and canned responses from a JSON fixture; non-fixture calls
//! fail loudly (they trap).
//!
//! This module is the generator: WIT interface in, stub component out. The
//! normative bridge WIT sources Platform 02 §2.2 promises (`wit/` at the
//! foundation repository root) do not exist yet, so no `dist/bridges/stubs/`
//! artifacts can ship — the mechanism lands now, the catalog feeds it when
//! foundation lands the WITs (DISCOVERIES-M8 §7). Two provisional choices,
//! recorded there too: the fixture is baked at *generation* time (the spec
//! says "handed to the stub at instantiation", which would force a JSON
//! parser into every stub or a host-side fixture import), and the fixture
//! reuses the replay trace's typed value encoding
//! ([`crate::replay::TraceValue`]) so the two testing surfaces speak one
//! language.
//!
//! Stub anatomy: a hand-assembled core module componentized against a
//! synthesized world that exports the bridge interface plus `stub-log:
//! func() -> list<u8>`. Every interface function appends one record —
//! function name and every argument value — to an in-memory log and serves
//! the next baked response for that function; an exhausted (or absent)
//! fixture entry traps. [`decode_log`] types the raw log back into values
//! using the interface's own declared parameter types.

use indexmap::IndexMap;
use serde::Deserialize;
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, ExportKind, ExportSection, Function, FunctionSection,
    ImportSection, Instruction as I, MemArg, MemorySection, MemoryType, Module, TypeSection,
    ValType,
};
use wit_parser::{Resolve, Type, TypeDefKind};

use crate::replay::TraceValue;

/// The JSON fixture (provisional schema, DISCOVERIES-M8 §7): per function,
/// the canned responses in call order — call N of a function gets entry N.
/// A function with no entry, or called past its last entry, traps.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StubFixture {
    pub responses: IndexMap<String, Vec<Vec<TraceValue>>>,
}

#[derive(Debug)]
pub enum StubError {
    /// The WIT text does not parse or lacks the named interface.
    Wit(String),
    /// The fixture JSON is malformed or names a function the interface
    /// does not declare.
    Fixture(String),
    /// The interface (or fixture) uses a shape outside the v1 stub subset.
    Unsupported(String),
    /// The generated module failed componentization — a generator bug.
    Encode(String),
}

impl std::fmt::Display for StubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wit(e) => write!(f, "bridge WIT rejected: {e}"),
            Self::Fixture(e) => write!(f, "stub fixture rejected: {e}"),
            Self::Unsupported(e) => write!(f, "outside the v1 stub subset: {e}"),
            Self::Encode(e) => write!(f, "stub encoding failed: {e}"),
        }
    }
}

/// The dist-layout artifact name for one interface (§14.14.5):
/// `clean-bridge-<interface>-stub.wasm` under `dist/bridges/stubs/`.
pub fn stub_artifact_name(interface: &str) -> String {
    format!("clean-bridge-{interface}-stub.wasm")
}

/// One recorded call, decoded and typed against the interface declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedCall {
    pub function: String,
    pub arguments: Vec<TraceValue>,
}

// ---- memory layout ----------------------------------------------------
const CURSOR_ADDR: u32 = 0; // u32: next free log byte
const BUMP_ADDR: u32 = 4; // u32: cabi_realloc bump pointer
const SCRATCH_ADDR: u32 = 8; // 8 bytes: {ptr,len} pair for stub-log
const COUNTERS_ADDR: u32 = 16; // u32 per interface function

const ALLOC_SIZE: u32 = 1 << 20; // caller-lowered arguments
const LOG_SIZE: u32 = 1 << 20; // recorded calls

// log record encoding (decoded by `decode_log`):
//   u32 name_len | name bytes | u32 nargs | args*
//   arg := u8 kind | payload
const KIND_SCALAR: u8 = 0; // 8-byte LE payload
const KIND_STRING: u8 = 4; // u32 len | bytes

/// A supported WIT value shape, resolved once up front.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Bool,
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    S64,
    U64,
    F32,
    F64,
    Char,
    String,
    /// `enum` or payload-less `variant`: a bare discriminant.
    Enum,
}

impl Shape {
    fn flat(&self) -> &'static [ValType] {
        match self {
            Shape::S64 | Shape::U64 => &[ValType::I64],
            Shape::F32 => &[ValType::F32],
            Shape::F64 => &[ValType::F64],
            Shape::String => &[ValType::I32, ValType::I32],
            _ => &[ValType::I32],
        }
    }
}

fn resolve_shape(resolve: &Resolve, ty: &Type) -> Result<Shape, StubError> {
    Ok(match ty {
        Type::Bool => Shape::Bool,
        Type::S8 => Shape::S8,
        Type::U8 => Shape::U8,
        Type::S16 => Shape::S16,
        Type::U16 => Shape::U16,
        Type::S32 => Shape::S32,
        Type::U32 => Shape::U32,
        Type::S64 => Shape::S64,
        Type::U64 => Shape::U64,
        Type::F32 => Shape::F32,
        Type::F64 => Shape::F64,
        Type::Char => Shape::Char,
        Type::String => Shape::String,
        Type::Id(id) => {
            let def = &resolve.types[*id];
            match &def.kind {
                TypeDefKind::Enum(_) => Shape::Enum,
                TypeDefKind::Variant(v) if v.cases.iter().all(|c| c.ty.is_none()) => Shape::Enum,
                TypeDefKind::Type(inner) => resolve_shape(resolve, inner)?,
                other => {
                    return Err(StubError::Unsupported(format!(
                        "type kind {other:?} (v1 stubs cover scalars, strings, and payload-less variants)"
                    )))
                }
            }
        }
        other => {
            return Err(StubError::Unsupported(format!(
                "type {other:?} (v1 stubs cover scalars, strings, and payload-less variants)"
            )))
        }
    })
}

struct FuncPlan {
    name: String,
    params: Vec<(String, Shape)>,
    result: Option<Shape>,
    responses: Vec<Vec<TraceValue>>,
}

/// A baked scalar response as its 8-byte table entry.
fn scalar_entry(shape: Shape, value: &TraceValue) -> Result<[u8; 8], StubError> {
    let bits: u64 = match (shape, value) {
        (Shape::Bool, TraceValue::Bool(v)) => *v as u64,
        (Shape::S8, TraceValue::S8(v)) => *v as u32 as u64,
        (Shape::U8, TraceValue::U8(v)) => *v as u64,
        (Shape::S16, TraceValue::S16(v)) => *v as u32 as u64,
        (Shape::U16, TraceValue::U16(v)) => *v as u64,
        (Shape::S32, TraceValue::S32(v)) => *v as u32 as u64,
        (Shape::U32, TraceValue::U32(v)) => *v as u64,
        (Shape::S64, TraceValue::S64(v)) => *v as u64,
        (Shape::U64, TraceValue::U64(v)) => *v,
        (Shape::F32, TraceValue::Float32(v)) => v.to_bits() as u64,
        (Shape::F64, TraceValue::Float64(v)) => v.to_bits(),
        (Shape::Char, TraceValue::Char(v)) => *v as u64,
        (Shape::Enum, TraceValue::Enum(_)) => {
            return Err(StubError::Fixture(
                "enum responses must be resolved to a discriminant by the generator".to_string(),
            ))
        }
        (shape, value) => {
            return Err(StubError::Fixture(format!(
                "response value {value:?} does not fit the declared result shape {shape:?}"
            )))
        }
    };
    Ok(bits.to_le_bytes())
}

/// Generates the stub component for `interface_name` as declared in
/// `bridge_wit`, with `fixture` baked in. Deterministic: the same three
/// inputs produce byte-identical output (CMP-02 applies to every emitting
/// path).
pub fn generate_stub(
    bridge_wit: &str,
    interface_name: &str,
    fixture: &StubFixture,
) -> Result<Vec<u8>, StubError> {
    // ---- resolve the interface ----------------------------------------
    let mut resolve = Resolve::new();
    let package = resolve
        .push_str("bridge.wit", bridge_wit)
        .map_err(|e| StubError::Wit(format!("{e:#}")))?;
    let package_name = resolve.packages[package].name.clone();
    let (_, iface_id) = resolve.packages[package]
        .interfaces
        .iter()
        .find(|(name, _)| name.as_str() == interface_name)
        .ok_or_else(|| {
            StubError::Wit(format!(
                "package `{package_name}` declares no interface `{interface_name}`"
            ))
        })?;
    let iface = &resolve.interfaces[*iface_id];

    let version = package_name
        .version
        .as_ref()
        .map(|v| format!("@{v}"))
        .unwrap_or_default();
    let qualified_interface = format!(
        "{}:{}/{}{version}",
        package_name.namespace, package_name.name, interface_name
    );

    let mut plans = Vec::new();
    for (func_name, func) in &iface.functions {
        let mut params = Vec::new();
        for param in func.params.iter() {
            params.push((param.name.clone(), resolve_shape(&resolve, &param.ty)?));
        }
        let result = match &func.result {
            Some(ty) => Some(resolve_shape(&resolve, ty)?),
            None => None,
        };
        let flat: usize = params.iter().map(|(_, s)| s.flat().len()).sum();
        if flat > 16 {
            return Err(StubError::Unsupported(format!(
                "`{func_name}` has more than 16 flat parameters"
            )));
        }
        let responses = fixture
            .responses
            .get(func_name.as_str())
            .cloned()
            .unwrap_or_default();
        for response in &responses {
            match (&result, response.len()) {
                (None, 0) | (Some(_), 1) => {}
                (None, n) => {
                    return Err(StubError::Fixture(format!(
                        "`{func_name}` returns nothing but a fixture entry carries {n} value(s)"
                    )))
                }
                (Some(_), n) => {
                    return Err(StubError::Fixture(format!(
                        "`{func_name}` returns one value but a fixture entry carries {n}"
                    )))
                }
            }
        }
        plans.push(FuncPlan {
            name: func_name.clone(),
            params,
            result,
            responses,
        });
    }
    for name in fixture.responses.keys() {
        if !plans.iter().any(|p| &p.name == name) {
            return Err(StubError::Fixture(format!(
                "fixture names `{name}`, which `{qualified_interface}` does not declare"
            )));
        }
    }

    // ---- bake the data segment ----------------------------------------
    // Layout after the per-func counters: for each func, its name bytes,
    // then its response table (8-byte entries; strings live as extra bytes
    // and their entries hold {ptr,len}).
    let counters_end = COUNTERS_ADDR + 4 * plans.len() as u32;
    let data_start = (counters_end + 7) & !7;
    let mut data: Vec<u8> = Vec::new();
    let at = |data: &Vec<u8>| data_start + data.len() as u32;

    let mut name_offsets = Vec::new();
    let mut table_offsets = Vec::new();
    // First pass: names and string payloads need addresses before the
    // tables that point at them, so bake strings first.
    let mut string_addrs: Vec<Vec<(u32, u32)>> = Vec::new(); // per func, per response
    for plan in &plans {
        name_offsets.push(at(&data));
        data.extend_from_slice(plan.name.as_bytes());
        let mut addrs = Vec::new();
        if plan.result == Some(Shape::String) {
            for response in &plan.responses {
                if let Some(TraceValue::String(s)) = response.first() {
                    addrs.push((at(&data), s.len() as u32));
                    data.extend_from_slice(s.as_bytes());
                } else {
                    return Err(StubError::Fixture(format!(
                        "`{}` returns a string but a fixture entry holds {:?}",
                        plan.name,
                        response.first()
                    )));
                }
            }
        }
        string_addrs.push(addrs);
    }
    // Second pass: response tables (aligned).
    for (f, plan) in plans.iter().enumerate() {
        while !data.len().is_multiple_of(8) {
            data.push(0);
        }
        table_offsets.push(at(&data));
        match plan.result {
            None => {}
            Some(Shape::String) => {
                for (ptr, len) in &string_addrs[f] {
                    data.extend_from_slice(&ptr.to_le_bytes());
                    data.extend_from_slice(&len.to_le_bytes());
                }
            }
            Some(Shape::Enum) => {
                // Discriminants resolved against the declared enum cases.
                let cases = enum_cases(&resolve, iface, &plan.name)?;
                for response in &plan.responses {
                    let TraceValue::Enum(case) = &response[0] else {
                        return Err(StubError::Fixture(format!(
                            "`{}` returns an enum but a fixture entry holds {:?}",
                            plan.name, response[0]
                        )));
                    };
                    let disc = cases.iter().position(|c| c == case).ok_or_else(|| {
                        StubError::Fixture(format!("`{}` has no case named `{case}`", plan.name))
                    })? as u64;
                    data.extend_from_slice(&disc.to_le_bytes());
                }
            }
            Some(shape) => {
                for response in &plan.responses {
                    data.extend_from_slice(&scalar_entry(shape, &response[0])?);
                }
            }
        }
    }

    let data_end = data_start + data.len() as u32;
    let alloc_start = (data_end + 15) & !15;
    let alloc_end = alloc_start + ALLOC_SIZE;
    let log_start = alloc_end;
    let log_end = log_start + LOG_SIZE;
    let pages = log_end.div_ceil(65536) as u64;

    // ---- assemble the core module --------------------------------------
    let mut types = TypeSection::new();
    let mut functions = FunctionSection::new();
    let mut exports = ExportSection::new();
    let mut code = CodeSection::new();

    // type 0: cabi_realloc (i32,i32,i32,i32)->i32
    types
        .ty()
        .function(vec![ValType::I32; 4], vec![ValType::I32]);
    functions.function(0);
    exports.export("cabi_realloc", ExportKind::Func, 0);
    code.function(&realloc_body(alloc_start, alloc_end));

    // type 1: stub-log () -> i32 (retptr to {ptr,len})
    types.ty().function(vec![], vec![ValType::I32]);
    functions.function(1);
    exports.export("stub-log", ExportKind::Func, 1);
    code.function(&stub_log_body(log_start));

    // interface functions
    for (f, plan) in plans.iter().enumerate() {
        let mut params: Vec<ValType> = Vec::new();
        for (_, shape) in &plan.params {
            params.extend_from_slice(shape.flat());
        }
        let results: Vec<ValType> = match plan.result {
            None => vec![],
            Some(Shape::String) => vec![ValType::I32], // retptr
            Some(shape) => shape.flat().to_vec(),
        };
        let type_index = 2 + f as u32;
        types.ty().function(params.clone(), results);
        functions.function(type_index);
        let func_index = 2 + f as u32;
        exports.export(
            &format!("{qualified_interface}#{}", plan.name),
            ExportKind::Func,
            func_index,
        );
        code.function(&interface_func_body(
            plan,
            f as u32,
            name_offsets[f],
            table_offsets[f],
            log_end,
        ));
    }

    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: pages,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    exports.export("memory", ExportKind::Memory, 0);

    let mut data_section = DataSection::new();
    // Initialize cursor and bump pointer, then the baked tables.
    let mut header = Vec::new();
    header.extend_from_slice(&log_start.to_le_bytes()); // cursor
    header.extend_from_slice(&alloc_start.to_le_bytes()); // bump
    data_section.active(0, &ConstExpr::i32_const(CURSOR_ADDR as i32), header);
    if !data.is_empty() {
        data_section.active(0, &ConstExpr::i32_const(data_start as i32), data);
    }

    let mut module = Module::new();
    module.section(&types);
    module.section(&ImportSection::new());
    module.section(&functions);
    module.section(&memories);
    module.section(&exports);
    module.section(&code);
    module.section(&data_section);
    let mut core = module.finish();

    // ---- componentize ---------------------------------------------------
    let world_wit = format!(
        "package clean:stub@0.1.0;\n\nworld stub {{\n    export {qualified_interface};\n    export stub-log: func() -> list<u8>;\n}}\n"
    );
    let stub_package = resolve
        .push_str("stub.wit", &world_wit)
        .map_err(|e| StubError::Encode(format!("stub world does not parse: {e:#}\n{world_wit}")))?;
    let world = resolve
        .select_world(&[stub_package], Some("stub"))
        .map_err(|e| StubError::Encode(format!("stub world selection failed: {e:#}")))?;
    wit_component::embed_component_metadata(
        &mut core,
        &resolve,
        world,
        wit_component::StringEncoding::UTF8,
    )
    .map_err(|e| StubError::Encode(format!("embedding stub metadata failed: {e:#}")))?;
    wit_component::ComponentEncoder::default()
        .validate(true)
        .module(&core)
        .map_err(|e| StubError::Encode(format!("core module rejected: {e:#}")))?
        .encode()
        .map_err(|e| StubError::Encode(format!("component encoding failed: {e:#}")))
}

fn enum_cases(
    resolve: &Resolve,
    iface: &wit_parser::Interface,
    func_name: &str,
) -> Result<Vec<String>, StubError> {
    let func = &iface.functions[func_name];
    let Some(Type::Id(id)) = func.result else {
        return Err(StubError::Encode(format!(
            "`{func_name}` result is not a named type"
        )));
    };
    match &resolve.types[id].kind {
        TypeDefKind::Enum(e) => Ok(e.cases.iter().map(|c| c.name.clone()).collect()),
        TypeDefKind::Variant(v) => Ok(v.cases.iter().map(|c| c.name.clone()).collect()),
        other => Err(StubError::Encode(format!(
            "`{func_name}` result kind {other:?} has no cases"
        ))),
    }
}

/// `cabi_realloc`: a bump allocator over the arena the caller lowers
/// arguments into. Old allocations are never resized in practice (the
/// canonical ABI passes `old_ptr = 0` for fresh strings/lists); running
/// past the arena traps.
fn realloc_body(alloc_start: u32, alloc_end: u32) -> Function {
    let _ = alloc_start;
    // locals: 4 params (old_ptr, old_size, align, new_size) + ptr(4)
    let mut f = Function::new(vec![(1, ValType::I32)]);
    let mem = MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    };
    // ptr = (bump + align - 1) & !(align - 1)
    f.instruction(&I::I32Const(BUMP_ADDR as i32));
    f.instruction(&I::I32Load(mem));
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
    f.instruction(&I::LocalSet(4));
    // if ptr + new_size > alloc_end → unreachable
    f.instruction(&I::LocalGet(4));
    f.instruction(&I::LocalGet(3));
    f.instruction(&I::I32Add);
    f.instruction(&I::I32Const(alloc_end as i32));
    f.instruction(&I::I32GtU);
    f.instruction(&I::If(wasm_encoder::BlockType::Empty));
    f.instruction(&I::Unreachable);
    f.instruction(&I::End);
    // bump = ptr + new_size
    f.instruction(&I::I32Const(BUMP_ADDR as i32));
    f.instruction(&I::LocalGet(4));
    f.instruction(&I::LocalGet(3));
    f.instruction(&I::I32Add);
    f.instruction(&I::I32Store(mem));
    f.instruction(&I::LocalGet(4));
    f.instruction(&I::End);
    f
}

/// `stub-log`: returns a retptr to `{ptr = log_start, len = cursor -
/// log_start}` written into scratch space.
fn stub_log_body(log_start: u32) -> Function {
    let mut f = Function::new(vec![]);
    let mem = MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    };
    f.instruction(&I::I32Const(SCRATCH_ADDR as i32));
    f.instruction(&I::I32Const(log_start as i32));
    f.instruction(&I::I32Store(mem));
    f.instruction(&I::I32Const(SCRATCH_ADDR as i32 + 4));
    f.instruction(&I::I32Const(CURSOR_ADDR as i32));
    f.instruction(&I::I32Load(mem));
    f.instruction(&I::I32Const(log_start as i32));
    f.instruction(&I::I32Sub);
    f.instruction(&I::I32Store(mem));
    f.instruction(&I::I32Const(SCRATCH_ADDR as i32));
    f.instruction(&I::End);
    f
}

/// One interface function: record the call, serve the next baked response
/// or trap.
fn interface_func_body(
    plan: &FuncPlan,
    func_index: u32,
    name_offset: u32,
    table_offset: u32,
    log_end: u32,
) -> Function {
    let mem = MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    };
    let mem8 = MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    };
    let mem64 = MemArg {
        offset: 0,
        align: 3,
        memory_index: 0,
    };

    let flat_params: u32 = plan.params.iter().map(|(_, s)| s.flat().len() as u32).sum();
    // locals: flat params, then cursor(i32), needed(i32)
    let cursor = flat_params;
    let needed = flat_params + 1;
    let mut f = Function::new(vec![(2, ValType::I32)]);

    // needed = fixed bytes + every string length
    let name_len = plan.name.len() as u32;
    let mut fixed: u32 = 4 + name_len + 4; // name_len + name + nargs
    for (_, shape) in &plan.params {
        fixed += 1; // kind byte
        fixed += match shape {
            Shape::String => 4, // len prefix; bytes added dynamically
            _ => 8,
        };
    }
    f.instruction(&I::I32Const(fixed as i32));
    {
        let mut flat_at = 0u32;
        for (_, shape) in &plan.params {
            if *shape == Shape::String {
                f.instruction(&I::LocalGet(flat_at + 1)); // len
                f.instruction(&I::I32Add);
            }
            flat_at += shape.flat().len() as u32;
        }
    }
    f.instruction(&I::LocalSet(needed));

    // cursor = load; if cursor + needed > log_end → unreachable
    f.instruction(&I::I32Const(CURSOR_ADDR as i32));
    f.instruction(&I::I32Load(mem));
    f.instruction(&I::LocalSet(cursor));
    f.instruction(&I::LocalGet(cursor));
    f.instruction(&I::LocalGet(needed));
    f.instruction(&I::I32Add);
    f.instruction(&I::I32Const(log_end as i32));
    f.instruction(&I::I32GtU);
    f.instruction(&I::If(wasm_encoder::BlockType::Empty));
    f.instruction(&I::Unreachable);
    f.instruction(&I::End);

    // name_len | name bytes
    f.instruction(&I::LocalGet(cursor));
    f.instruction(&I::I32Const(name_len as i32));
    f.instruction(&I::I32Store(mem));
    f.instruction(&I::LocalGet(cursor));
    f.instruction(&I::I32Const(4));
    f.instruction(&I::I32Add);
    f.instruction(&I::I32Const(name_offset as i32));
    f.instruction(&I::I32Const(name_len as i32));
    f.instruction(&I::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&I::LocalGet(cursor));
    f.instruction(&I::I32Const(4 + name_len as i32));
    f.instruction(&I::I32Add);
    f.instruction(&I::LocalSet(cursor));

    // nargs
    f.instruction(&I::LocalGet(cursor));
    f.instruction(&I::I32Const(plan.params.len() as i32));
    f.instruction(&I::I32Store(mem));
    f.instruction(&I::LocalGet(cursor));
    f.instruction(&I::I32Const(4));
    f.instruction(&I::I32Add);
    f.instruction(&I::LocalSet(cursor));

    // args
    let mut flat_at = 0u32;
    for (_, shape) in &plan.params {
        match shape {
            Shape::String => {
                // kind
                f.instruction(&I::LocalGet(cursor));
                f.instruction(&I::I32Const(KIND_STRING as i32));
                f.instruction(&I::I32Store8(mem8));
                // len
                f.instruction(&I::LocalGet(cursor));
                f.instruction(&I::I32Const(1));
                f.instruction(&I::I32Add);
                f.instruction(&I::LocalGet(flat_at + 1));
                f.instruction(&I::I32Store(mem));
                // bytes
                f.instruction(&I::LocalGet(cursor));
                f.instruction(&I::I32Const(5));
                f.instruction(&I::I32Add);
                f.instruction(&I::LocalGet(flat_at)); // ptr
                f.instruction(&I::LocalGet(flat_at + 1)); // len
                f.instruction(&I::MemoryCopy {
                    src_mem: 0,
                    dst_mem: 0,
                });
                // cursor += 5 + len
                f.instruction(&I::LocalGet(cursor));
                f.instruction(&I::I32Const(5));
                f.instruction(&I::I32Add);
                f.instruction(&I::LocalGet(flat_at + 1));
                f.instruction(&I::I32Add);
                f.instruction(&I::LocalSet(cursor));
            }
            shape => {
                // kind
                f.instruction(&I::LocalGet(cursor));
                f.instruction(&I::I32Const(KIND_SCALAR as i32));
                f.instruction(&I::I32Store8(mem8));
                // 8-byte payload
                f.instruction(&I::LocalGet(cursor));
                f.instruction(&I::I32Const(1));
                f.instruction(&I::I32Add);
                f.instruction(&I::LocalGet(flat_at));
                match shape {
                    Shape::S64 | Shape::U64 => {}
                    Shape::F32 => {
                        f.instruction(&I::I32ReinterpretF32);
                        f.instruction(&I::I64ExtendI32U);
                    }
                    Shape::F64 => {
                        f.instruction(&I::I64ReinterpretF64);
                    }
                    Shape::S8 | Shape::S16 | Shape::S32 => {
                        f.instruction(&I::I64ExtendI32S);
                    }
                    _ => {
                        f.instruction(&I::I64ExtendI32U);
                    }
                }
                f.instruction(&I::I64Store(mem64));
                // cursor += 9
                f.instruction(&I::LocalGet(cursor));
                f.instruction(&I::I32Const(9));
                f.instruction(&I::I32Add);
                f.instruction(&I::LocalSet(cursor));
            }
        }
        flat_at += shape.flat().len() as u32;
    }

    // store cursor back
    f.instruction(&I::I32Const(CURSOR_ADDR as i32));
    f.instruction(&I::LocalGet(cursor));
    f.instruction(&I::I32Store(mem));

    // ---- serve the next baked response (or trap) -----------------------
    let counter_addr = COUNTERS_ADDR + 4 * func_index;
    // counter >= responses.len() → unreachable (non-fixture call fails
    // loudly, §14.14.5)
    f.instruction(&I::I32Const(counter_addr as i32));
    f.instruction(&I::I32Load(mem));
    f.instruction(&I::I32Const(plan.responses.len() as i32));
    f.instruction(&I::I32GeU);
    f.instruction(&I::If(wasm_encoder::BlockType::Empty));
    f.instruction(&I::Unreachable);
    f.instruction(&I::End);

    // entry_addr = table + counter*8 (reuse `needed` local)
    f.instruction(&I::I32Const(counter_addr as i32));
    f.instruction(&I::I32Load(mem));
    f.instruction(&I::I32Const(8));
    f.instruction(&I::I32Mul);
    f.instruction(&I::I32Const(table_offset as i32));
    f.instruction(&I::I32Add);
    f.instruction(&I::LocalSet(needed));

    // counter += 1
    f.instruction(&I::I32Const(counter_addr as i32));
    f.instruction(&I::I32Const(counter_addr as i32));
    f.instruction(&I::I32Load(mem));
    f.instruction(&I::I32Const(1));
    f.instruction(&I::I32Add);
    f.instruction(&I::I32Store(mem));

    // result
    match plan.result {
        None => {}
        Some(Shape::String) => {
            // The table entry *is* the {ptr,len} pair — return its address.
            f.instruction(&I::LocalGet(needed));
        }
        Some(Shape::S64) | Some(Shape::U64) => {
            f.instruction(&I::LocalGet(needed));
            f.instruction(&I::I64Load(mem64));
        }
        Some(Shape::F32) => {
            f.instruction(&I::LocalGet(needed));
            f.instruction(&I::F32Load(MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }));
        }
        Some(Shape::F64) => {
            f.instruction(&I::LocalGet(needed));
            f.instruction(&I::F64Load(mem64));
        }
        Some(_) => {
            f.instruction(&I::LocalGet(needed));
            f.instruction(&I::I32Load(mem));
        }
    }
    f.instruction(&I::End);
    f
}

/// Decodes a stub's raw log against the interface declaration, typing every
/// argument by its declared parameter type.
pub fn decode_log(
    bridge_wit: &str,
    interface_name: &str,
    log: &[u8],
) -> Result<Vec<RecordedCall>, StubError> {
    let mut resolve = Resolve::new();
    let package = resolve
        .push_str("bridge.wit", bridge_wit)
        .map_err(|e| StubError::Wit(format!("{e:#}")))?;
    let (_, iface_id) = resolve.packages[package]
        .interfaces
        .iter()
        .find(|(name, _)| name.as_str() == interface_name)
        .ok_or_else(|| StubError::Wit(format!("no interface `{interface_name}`")))?;
    let iface = &resolve.interfaces[*iface_id];

    let mut calls = Vec::new();
    let mut at = 0usize;
    let take = |at: &mut usize, n: usize| -> Result<&[u8], StubError> {
        if *at + n > log.len() {
            return Err(StubError::Encode("log truncated".to_string()));
        }
        let slice = &log[*at..*at + n];
        *at += n;
        Ok(slice)
    };
    while at < log.len() {
        let name_len = u32::from_le_bytes(take(&mut at, 4)?.try_into().unwrap()) as usize;
        let name = String::from_utf8(take(&mut at, name_len)?.to_vec())
            .map_err(|_| StubError::Encode("log name is not UTF-8".to_string()))?;
        let nargs = u32::from_le_bytes(take(&mut at, 4)?.try_into().unwrap()) as usize;
        let func = iface
            .functions
            .get(&name)
            .ok_or_else(|| StubError::Encode(format!("log records unknown `{name}`")))?;
        if func.params.len() != nargs {
            return Err(StubError::Encode(format!(
                "log records {nargs} argument(s) for `{name}`, which declares {}",
                func.params.len()
            )));
        }
        let mut arguments = Vec::new();
        for param in func.params.iter() {
            let ty = &param.ty;
            let shape = resolve_shape(&resolve, ty)?;
            let kind = take(&mut at, 1)?[0];
            let value = match shape {
                Shape::String => {
                    if kind != KIND_STRING {
                        return Err(StubError::Encode("kind mismatch in log".to_string()));
                    }
                    let len = u32::from_le_bytes(take(&mut at, 4)?.try_into().unwrap()) as usize;
                    TraceValue::String(
                        String::from_utf8(take(&mut at, len)?.to_vec()).map_err(|_| {
                            StubError::Encode("log string is not UTF-8".to_string())
                        })?,
                    )
                }
                shape => {
                    if kind != KIND_SCALAR {
                        return Err(StubError::Encode("kind mismatch in log".to_string()));
                    }
                    let bits = u64::from_le_bytes(take(&mut at, 8)?.try_into().unwrap());
                    scalar_from_bits(&resolve, ty, shape, bits)?
                }
            };
            arguments.push(value);
        }
        calls.push(RecordedCall {
            function: name,
            arguments,
        });
    }
    Ok(calls)
}

fn scalar_from_bits(
    resolve: &Resolve,
    ty: &Type,
    shape: Shape,
    bits: u64,
) -> Result<TraceValue, StubError> {
    Ok(match shape {
        Shape::Bool => TraceValue::Bool(bits != 0),
        Shape::S8 => TraceValue::S8(bits as i64 as i8),
        Shape::U8 => TraceValue::U8(bits as u8),
        Shape::S16 => TraceValue::S16(bits as i64 as i16),
        Shape::U16 => TraceValue::U16(bits as u16),
        Shape::S32 => TraceValue::S32(bits as i64 as i32),
        Shape::U32 => TraceValue::U32(bits as u32),
        Shape::S64 => TraceValue::S64(bits as i64),
        Shape::U64 => TraceValue::U64(bits),
        Shape::F32 => TraceValue::Float32(f32::from_bits(bits as u32)),
        Shape::F64 => TraceValue::Float64(f64::from_bits(bits)),
        Shape::Char => TraceValue::Char(
            char::from_u32(bits as u32)
                .ok_or_else(|| StubError::Encode("log char out of range".to_string()))?,
        ),
        Shape::Enum => {
            let Type::Id(id) = resolved_named(resolve, ty) else {
                return Err(StubError::Encode("enum without a named type".to_string()));
            };
            let cases: Vec<String> = match &resolve.types[id].kind {
                TypeDefKind::Enum(e) => e.cases.iter().map(|c| c.name.clone()).collect(),
                TypeDefKind::Variant(v) => v.cases.iter().map(|c| c.name.clone()).collect(),
                _ => return Err(StubError::Encode("enum kind mismatch".to_string())),
            };
            let case = cases
                .get(bits as usize)
                .ok_or_else(|| StubError::Encode("log discriminant out of range".to_string()))?;
            TraceValue::Enum(case.clone())
        }
        Shape::String => unreachable!("handled by the caller"),
    })
}

/// Follows `type` aliases to the named type an enum shape resolved from.
fn resolved_named(resolve: &Resolve, ty: &Type) -> Type {
    match ty {
        Type::Id(id) => match &resolve.types[*id].kind {
            TypeDefKind::Type(inner) => resolved_named(resolve, inner),
            _ => *ty,
        },
        _ => *ty,
    }
}
