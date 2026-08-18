//! The compile-time handler sandbox (ADR-0004, Platform 03 §3.8).
//!
//! Executes one framework-compiled handler wasm module per call, under:
//! - **epoch interruption** for the wall-clock budget (`handler_timeout_ms`,
//!   Platform 07 §7.8) — epoch over fuel per ADR-0004: near-zero cost while
//!   the handler stays within budget;
//! - **a memory cap** (`handler_memory_mb`) enforced through a
//!   `ResourceLimiter` that records the denial, so a budget breach is
//!   classified as `BLOCK005` and never mistaken for a handler crash;
//! - **no host imports at all**: every function import the module declares
//!   is stubbed to a trap that records the import's name (`BLOCK006`,
//!   chapter 21 §21.7 — "all host imports stubbed"). There is no WASI, no
//!   wall clock, and no randomness inside the sandbox; determinism is the
//!   absence of every such import, plus deterministic engine flags
//!   (NaN canonicalization, deterministic relaxed-simd).
//!
//! One engine, one epoch ticker: every store MUST be created through
//! [`run_handler`] so the deadline always applies. (The retired compiler
//! shipped a silent-hang bug because a store bypassed its ticker —
//! `clean-language-compiler-old/KNOWLEDGE.md` §12.)
//!
//! The wire ABI (how the `BlockAST` goes in and the `IR` envelope comes
//! out) is implementation-defined pending a foundation ruling; it is
//! recorded in docs/adr/0003-handler-wire-abi.md and DISCOVERIES-M5.

use std::sync::OnceLock;

use clean_compiler_types::request::CompileLimits;
use wasmtime::{
    Caller, Config, Engine, ExternType, Func, Instance, Module, ResourceLimiter, Store, Trap,
};

/// Epoch tick granularity. The deadline is expressed in ticks, so the
/// effective timeout resolution is one tick.
const EPOCH_TICK_MS: u64 = 10;

const MIB: usize = 1024 * 1024;

/// How one handler invocation ended.
#[derive(Debug)]
pub enum HandlerOutcome {
    /// The handler returned an envelope (UTF-8 text, expected JSON — the
    /// caller parses and validates it).
    Success(String),
    /// The wall-clock budget was exceeded (epoch interrupt) — `BLOCK005`.
    Timeout,
    /// The memory budget was exceeded (limiter denied a growth and the
    /// handler then failed) — `BLOCK005`.
    MemoryExceeded,
    /// The handler called a stubbed host import — `BLOCK006`. Carries
    /// `module::name` of the import.
    ForbiddenImport(String),
    /// The handler trapped or failed for any other reason — `BLOCK004`.
    Crash(String),
    /// The artifact does not satisfy the sandbox ABI (invalid wasm,
    /// non-function import, missing/mistyped export, out-of-bounds
    /// result) — `BLOCK004`. The message names the defect.
    Malformed(String),
}

/// One invocation's result plus the linear-memory footprint at the end of
/// the call, which feeds the per-library compile-time heap accounting
/// (`LIB014`, chapter 21 §21.7).
#[derive(Debug)]
pub struct HandlerRun {
    pub outcome: HandlerOutcome,
    pub memory_bytes: u64,
}

impl HandlerRun {
    fn failed(outcome: HandlerOutcome) -> Self {
        Self {
            outcome,
            memory_bytes: 0,
        }
    }
}

/// Per-store data: the memory cap (with its denial record) and the name of
/// the first forbidden import the handler called, if any.
struct SandboxData {
    cap: MemoryCap,
    forbidden_import: Option<String>,
}

struct MemoryCap {
    limit_bytes: usize,
    denied: bool,
}

impl ResourceLimiter for MemoryCap {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > self.limit_bytes {
            self.denied = true;
            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        // Tables hold references, not heap bytes; cap generously against
        // pathological growth only.
        Ok(desired <= 1_000_000)
    }
}

/// The single sandbox engine. Epoch interruption is enabled at engine
/// level and a ticker thread increments the epoch every `EPOCH_TICK_MS`;
/// deterministic-execution flags per ADR-0004.
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.epoch_interruption(true);
        config.cranelift_nan_canonicalization(true);
        config.relaxed_simd_deterministic(true);
        let engine = Engine::new(&config).expect("sandbox engine config is valid");
        let ticker = engine.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(EPOCH_TICK_MS));
            ticker.increment_epoch();
        });
        engine
    })
}

/// Classifies a failed wasm call. Order matters: a stub trap and a
/// memory-denial both surface as generic errors, so the store's records
/// are consulted before the trap code.
fn classify(data: &SandboxData, err: wasmtime::Error) -> HandlerOutcome {
    if let Some(name) = &data.forbidden_import {
        return HandlerOutcome::ForbiddenImport(name.clone());
    }
    if data.cap.denied {
        return HandlerOutcome::MemoryExceeded;
    }
    match err.downcast_ref::<Trap>() {
        Some(Trap::Interrupt) => HandlerOutcome::Timeout,
        Some(trap) => HandlerOutcome::Crash(trap.to_string()),
        None => HandlerOutcome::Crash(err.to_string()),
    }
}

/// Runs one handler invocation: validates and instantiates `wasm` with
/// every function import stubbed, writes `input` into the guest via its
/// `alloc` export, calls `expand(ptr, len)`, and reads the result envelope
/// through the returned 8-byte header (`u32` LE out-pointer, `u32` LE
/// out-length).
pub fn run_handler(wasm: &[u8], input: &str, limits: &CompileLimits) -> HandlerRun {
    let engine = engine();
    let module = match Module::new(engine, wasm) {
        Ok(module) => module,
        Err(err) => {
            return HandlerRun::failed(HandlerOutcome::Malformed(format!(
                "handler wasm does not validate: {err}"
            )))
        }
    };

    let mut store = Store::new(
        engine,
        SandboxData {
            cap: MemoryCap {
                limit_bytes: (limits.handler_memory_mb as usize).saturating_mul(MIB),
                denied: false,
            },
            forbidden_import: None,
        },
    );
    store.limiter(|data| &mut data.cap);
    store.set_epoch_deadline(limits.handler_timeout_ms.div_ceil(EPOCH_TICK_MS).max(1));

    // Chapter 21 §21.7: the sandbox provides nothing — every function
    // import is a stub that records its name and traps (`BLOCK006`).
    // Non-function imports cannot be stubbed and the framework never
    // produces them, so they are an artifact defect.
    let mut imports = Vec::new();
    for import in module.imports() {
        let name = format!("{}::{}", import.module(), import.name());
        match import.ty() {
            ExternType::Func(func_ty) => {
                let stub = Func::new(
                    &mut store,
                    func_ty.clone(),
                    move |mut caller: Caller<'_, SandboxData>, _params, _results| {
                        caller
                            .data_mut()
                            .forbidden_import
                            .get_or_insert_with(|| name.clone());
                        Err(wasmtime::Error::msg(
                            "forbidden host import called in compiletime sandbox",
                        ))
                    },
                );
                imports.push(stub.into());
            }
            _ => {
                return HandlerRun::failed(HandlerOutcome::Malformed(format!(
                    "handler wasm imports non-function '{name}'"
                )))
            }
        }
    }

    let instance = match Instance::new(&mut store, &module, &imports) {
        Ok(instance) => instance,
        Err(err) => {
            let outcome = classify(store.data(), err);
            return HandlerRun {
                memory_bytes: 0,
                outcome,
            };
        }
    };

    let Some(memory) = instance.get_memory(&mut store, "memory") else {
        return HandlerRun::failed(HandlerOutcome::Malformed(
            "handler wasm does not export 'memory'".to_string(),
        ));
    };
    let alloc = match instance.get_typed_func::<i32, i32>(&mut store, "alloc") {
        Ok(func) => func,
        Err(_) => {
            return HandlerRun::failed(HandlerOutcome::Malformed(
                "handler wasm does not export 'alloc: func(i32) -> i32'".to_string(),
            ))
        }
    };
    let expand = match instance.get_typed_func::<(i32, i32), i32>(&mut store, "expand") {
        Ok(func) => func,
        Err(_) => {
            return HandlerRun::failed(HandlerOutcome::Malformed(
                "handler wasm does not export 'expand: func(i32, i32) -> i32'".to_string(),
            ))
        }
    };

    let input_bytes = input.as_bytes();
    let Ok(input_len) = i32::try_from(input_bytes.len()) else {
        return HandlerRun::failed(HandlerOutcome::Malformed(
            "handler input exceeds the 2 GiB ABI addressing range".to_string(),
        ));
    };
    let input_ptr = match alloc.call(&mut store, input_len) {
        Ok(ptr) => ptr,
        Err(err) => {
            let outcome = classify(store.data(), err);
            return HandlerRun {
                memory_bytes: memory.data_size(&store) as u64,
                outcome,
            };
        }
    };
    if input_ptr < 0
        || memory
            .write(&mut store, input_ptr as usize, input_bytes)
            .is_err()
    {
        return HandlerRun {
            memory_bytes: memory.data_size(&store) as u64,
            outcome: HandlerOutcome::Malformed(format!(
                "handler 'alloc' returned {input_ptr}, out of bounds for {} byte(s)",
                input_bytes.len()
            )),
        };
    }

    let header_ptr = match expand.call(&mut store, (input_ptr, input_len)) {
        Ok(ptr) => ptr,
        Err(err) => {
            let outcome = classify(store.data(), err);
            return HandlerRun {
                memory_bytes: memory.data_size(&store) as u64,
                outcome,
            };
        }
    };

    let memory_bytes = memory.data_size(&store) as u64;
    let mut header = [0u8; 8];
    if header_ptr < 0
        || memory
            .read(&store, header_ptr as usize, &mut header)
            .is_err()
    {
        return HandlerRun {
            memory_bytes,
            outcome: HandlerOutcome::Malformed(format!(
                "handler 'expand' returned header pointer {header_ptr}, out of bounds"
            )),
        };
    }
    let out_ptr = u32::from_le_bytes(header[0..4].try_into().unwrap()) as usize;
    let out_len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    let mut out = vec![0u8; out_len];
    if memory.read(&store, out_ptr, &mut out).is_err() {
        return HandlerRun {
            memory_bytes,
            outcome: HandlerOutcome::Malformed(format!(
                "handler result ({out_ptr}, {out_len}) is out of bounds"
            )),
        };
    }
    match String::from_utf8(out) {
        Ok(text) => HandlerRun {
            memory_bytes,
            outcome: HandlerOutcome::Success(text),
        },
        Err(_) => HandlerRun {
            memory_bytes,
            outcome: HandlerOutcome::Malformed("handler result envelope is not UTF-8".to_string()),
        },
    }
}
