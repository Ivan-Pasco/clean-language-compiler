//! Request replay (Platform 14 §14.14.6, the operation behind the replay
//! half of `cln repro`): read a request trace, instantiate the recorded
//! component, wire the runtime to a **replay host** that returns the
//! captured values instead of live ones, and re-run the request. The
//! captured response must match the original byte-for-byte — divergence is
//! a Clean runtime bug or a trace corruption, never a "close enough"
//! outcome.
//!
//! The trace format is normatively clean-server's to define and the
//! compiler only ships the schema for validation — but no such schema
//! exists anywhere in the spec today, so the shape here is a provisional
//! local pinning (DISCOVERIES-M8 §6): `spec_version` mirroring §14.1.1,
//! the component named by hash, the entry invocation, every host call
//! with its captured arguments and results in order, and the captured
//! response. Values use a typed JSON encoding covering the WIT value
//! kinds the vendored host contract uses; a kind outside it is a typed
//! `UnsupportedValue` failure, never a guess.
//!
//! The replay host is strict the way the §14.14.5 stubs are strict: every
//! host call the guest makes must be the next recorded call — same
//! function, same arguments — and leftover recorded calls at the end are
//! a divergence too.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasmtime::component::{Component, Linker, Val};

/// One request trace (provisional schema, DISCOVERIES-M8 §6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestTrace {
    /// Mirrors the request document's `spec_version` (§14.1.1).
    pub spec_version: String,
    /// Hex-lowercase SHA-256 of the `component.wasm` the trace was
    /// captured against; replay refuses any other bytes.
    pub component_sha256: String,
    /// The invocation that produced the response.
    pub entry: EntryRecord,
    /// Every host call the request made, in call order, with captured
    /// arguments (for verification) and results (to serve).
    #[serde(default)]
    pub host_calls: Vec<HostCallRecord>,
    /// The captured response: the entry function's results.
    #[serde(default)]
    pub response: Vec<TraceValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryRecord {
    /// A world-level export of the guest component, e.g. `init` or `handle`.
    pub function: String,
    #[serde(default)]
    pub arguments: Vec<TraceValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostCallRecord {
    /// Qualified as `<instance>#<function>`, e.g.
    /// `clean:host/websocket@0.7.0#accept`.
    pub function: String,
    #[serde(default)]
    pub arguments: Vec<TraceValue>,
    #[serde(default)]
    pub results: Vec<TraceValue>,
}

/// A WIT value in trace JSON. Self-describing, like `component::Val`;
/// integers outside `f64`'s exact range ride as JSON strings via serde's
/// `i64`/`u64` handling (serde_json keeps full 64-bit precision).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum TraceValue {
    Bool(bool),
    S8(i8),
    U8(u8),
    S16(i16),
    U16(u16),
    S32(i32),
    U32(u32),
    S64(i64),
    U64(u64),
    Float32(f32),
    Float64(f64),
    Char(char),
    String(String),
    List(Vec<TraceValue>),
    Record(Vec<(String, TraceValue)>),
    Tuple(Vec<TraceValue>),
    Enum(String),
    Option(Option<Box<TraceValue>>),
    Result(Result<Option<Box<TraceValue>>, Option<Box<TraceValue>>>),
}

impl TraceValue {
    fn to_val(&self) -> Val {
        match self {
            TraceValue::Bool(v) => Val::Bool(*v),
            TraceValue::S8(v) => Val::S8(*v),
            TraceValue::U8(v) => Val::U8(*v),
            TraceValue::S16(v) => Val::S16(*v),
            TraceValue::U16(v) => Val::U16(*v),
            TraceValue::S32(v) => Val::S32(*v),
            TraceValue::U32(v) => Val::U32(*v),
            TraceValue::S64(v) => Val::S64(*v),
            TraceValue::U64(v) => Val::U64(*v),
            TraceValue::Float32(v) => Val::Float32(*v),
            TraceValue::Float64(v) => Val::Float64(*v),
            TraceValue::Char(v) => Val::Char(*v),
            TraceValue::String(v) => Val::String(v.clone()),
            TraceValue::List(items) => Val::List(items.iter().map(Self::to_val).collect()),
            TraceValue::Record(fields) => Val::Record(
                fields
                    .iter()
                    .map(|(name, value)| (name.clone(), value.to_val()))
                    .collect(),
            ),
            TraceValue::Tuple(items) => Val::Tuple(items.iter().map(Self::to_val).collect()),
            TraceValue::Enum(case) => Val::Enum(case.clone()),
            TraceValue::Option(inner) => Val::Option(inner.as_ref().map(|v| Box::new(v.to_val()))),
            TraceValue::Result(result) => Val::Result(match result {
                Ok(inner) => Ok(inner.as_ref().map(|v| Box::new(v.to_val()))),
                Err(inner) => Err(inner.as_ref().map(|v| Box::new(v.to_val()))),
            }),
        }
    }

    fn from_val(val: &Val) -> Result<TraceValue, String> {
        Ok(match val {
            Val::Bool(v) => TraceValue::Bool(*v),
            Val::S8(v) => TraceValue::S8(*v),
            Val::U8(v) => TraceValue::U8(*v),
            Val::S16(v) => TraceValue::S16(*v),
            Val::U16(v) => TraceValue::U16(*v),
            Val::S32(v) => TraceValue::S32(*v),
            Val::U32(v) => TraceValue::U32(*v),
            Val::S64(v) => TraceValue::S64(*v),
            Val::U64(v) => TraceValue::U64(*v),
            Val::Float32(v) => TraceValue::Float32(*v),
            Val::Float64(v) => TraceValue::Float64(*v),
            Val::Char(v) => TraceValue::Char(*v),
            Val::String(v) => TraceValue::String(v.clone()),
            Val::List(items) => {
                TraceValue::List(items.iter().map(Self::from_val).collect::<Result<_, _>>()?)
            }
            Val::Record(fields) => TraceValue::Record(
                fields
                    .iter()
                    .map(|(name, value)| Ok((name.clone(), Self::from_val(value)?)))
                    .collect::<Result<_, String>>()?,
            ),
            Val::Tuple(items) => {
                TraceValue::Tuple(items.iter().map(Self::from_val).collect::<Result<_, _>>()?)
            }
            Val::Enum(case) => TraceValue::Enum(case.clone()),
            Val::Option(inner) => TraceValue::Option(match inner {
                Some(v) => Some(Box::new(Self::from_val(v)?)),
                None => None,
            }),
            Val::Result(result) => TraceValue::Result(match result {
                Ok(Some(v)) => Ok(Some(Box::new(Self::from_val(v)?))),
                Ok(None) => Ok(None),
                Err(Some(v)) => Err(Some(Box::new(Self::from_val(v)?))),
                Err(None) => Err(None),
            }),
            other => {
                return Err(format!(
                    "value kind not covered by the trace schema: {other:?}"
                ))
            }
        })
    }
}

/// Why a replay did not reproduce the captured request.
#[derive(Debug)]
pub enum ReplayFailure {
    /// The trace's `spec_version` is not one this build validates.
    UnsupportedTraceVersion(String),
    /// The provided component does not hash to `component_sha256`.
    ComponentMismatch {
        expected_sha256: String,
        actual_sha256: String,
    },
    /// The bytes are not a loadable component, or its imports could not
    /// be wired.
    Setup(String),
    /// The trace names an entry the component does not export.
    MissingEntry(String),
    /// A value in the trace or produced by the guest is outside the
    /// encodable subset (DISCOVERIES-M8 §6).
    UnsupportedValue(String),
    /// The re-run made a host call the trace did not record next — or
    /// finished with recorded calls left unserved. A Clean runtime bug or
    /// a trace corruption (§14.14.6).
    Diverged {
        detail: String,
        /// Host calls served before the divergence.
        replayed: usize,
    },
    /// The guest trapped without a host-call divergence.
    Trap(String),
    /// The re-run completed but its response differs from the captured
    /// one.
    ResponseMismatch {
        expected: Vec<TraceValue>,
        actual: Vec<TraceValue>,
    },
}

impl std::fmt::Display for ReplayFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedTraceVersion(v) => {
                write!(f, "trace spec_version `{v}` is not supported (expected `1`)")
            }
            Self::ComponentMismatch {
                expected_sha256,
                actual_sha256,
            } => write!(
                f,
                "component hashes to {actual_sha256} but the trace was captured against {expected_sha256}"
            ),
            Self::Setup(err) => write!(f, "cannot set up the replay host: {err}"),
            Self::MissingEntry(name) => {
                write!(f, "the component exports no function named `{name}`")
            }
            Self::UnsupportedValue(err) => write!(f, "{err}"),
            Self::Diverged { detail, replayed } => write!(
                f,
                "replay diverged after {replayed} host call(s): {detail} — a Clean runtime bug or a trace corruption"
            ),
            Self::Trap(err) => write!(f, "guest trapped during replay: {err}"),
            Self::ResponseMismatch { .. } => write!(
                f,
                "replayed response differs from the captured response — a Clean runtime bug or a trace corruption"
            ),
        }
    }
}

/// A successful replay: the response matched byte-for-byte and every
/// recorded host call was served in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplayReport {
    pub host_calls_replayed: usize,
}

/// Parses and validates a trace document (the schema-validation half the
/// compiler ships per §14.14.6). Strict: unknown fields are refused, like
/// the request document's own intake.
pub fn trace_from_json(text: &str) -> Result<RequestTrace, String> {
    let trace: RequestTrace =
        serde_json::from_str(text).map_err(|e| format!("not a request trace: {e}"))?;
    if trace.spec_version != "1" {
        return Err(format!(
            "trace spec_version `{}` is not supported (expected `1`)",
            trace.spec_version
        ));
    }
    Ok(trace)
}

struct ReplayState {
    pending: VecDeque<HostCallRecord>,
    replayed: usize,
    divergence: Option<String>,
}

/// The replay operation: re-run `trace.entry` against `component_wasm`
/// with every host import served from the trace.
pub fn replay(trace: &RequestTrace, component_wasm: &[u8]) -> Result<ReplayReport, ReplayFailure> {
    if trace.spec_version != "1" {
        return Err(ReplayFailure::UnsupportedTraceVersion(
            trace.spec_version.clone(),
        ));
    }
    let mut hasher = Sha256::new();
    hasher.update(component_wasm);
    let actual_sha256 = format!("{:x}", hasher.finalize());
    if actual_sha256 != trace.component_sha256 {
        return Err(ReplayFailure::ComponentMismatch {
            expected_sha256: trace.component_sha256.clone(),
            actual_sha256,
        });
    }

    let engine = wasmtime::Engine::default();
    let component = Component::new(&engine, component_wasm)
        .map_err(|e| ReplayFailure::Setup(format!("component does not load: {e}")))?;

    let mut linker: Linker<ReplayState> = Linker::new(&engine);
    wire_replay_host(&engine, &component, &mut linker)?;

    let mut store = wasmtime::Store::new(
        &engine,
        ReplayState {
            pending: trace.host_calls.iter().cloned().collect(),
            replayed: 0,
            divergence: None,
        },
    );
    let instance = linker
        .instantiate(&mut store, &component)
        .map_err(|e| ReplayFailure::Setup(format!("component does not instantiate: {e}")))?;

    let result_count = component
        .component_type()
        .exports(&engine)
        .find(|(name, _)| *name == trace.entry.function)
        .and_then(|(_, item)| match item.ty {
            wasmtime::component::types::ComponentItem::ComponentFunc(func) => {
                Some(func.results().len())
            }
            _ => None,
        })
        .ok_or_else(|| ReplayFailure::MissingEntry(trace.entry.function.clone()))?;
    let index = component
        .get_export_index(None, &trace.entry.function)
        .ok_or_else(|| ReplayFailure::MissingEntry(trace.entry.function.clone()))?;
    let func = instance
        .get_func(&mut store, index)
        .ok_or_else(|| ReplayFailure::MissingEntry(trace.entry.function.clone()))?;

    let params: Vec<Val> = trace
        .entry
        .arguments
        .iter()
        .map(TraceValue::to_val)
        .collect();
    let mut results = vec![Val::Bool(false); result_count];
    let call = func.call(&mut store, &params, &mut results);

    if let Some(detail) = store.data_mut().divergence.take() {
        return Err(ReplayFailure::Diverged {
            detail,
            replayed: store.data().replayed,
        });
    }
    if let Err(err) = call {
        return Err(ReplayFailure::Trap(format!("{err:#}")));
    }

    if !store.data().pending.is_empty() {
        let next = &store.data().pending[0];
        return Err(ReplayFailure::Diverged {
            detail: format!(
                "the request finished but the trace records {} more host call(s), next `{}`",
                store.data().pending.len(),
                next.function
            ),
            replayed: store.data().replayed,
        });
    }

    let actual: Vec<TraceValue> = results
        .iter()
        .map(TraceValue::from_val)
        .collect::<Result<_, _>>()
        .map_err(ReplayFailure::UnsupportedValue)?;
    if actual != trace.response {
        return Err(ReplayFailure::ResponseMismatch {
            expected: trace.response.clone(),
            actual,
        });
    }

    Ok(ReplayReport {
        host_calls_replayed: store.data().replayed,
    })
}

/// Registers one replay-host function for every function every imported
/// instance of the component declares. Each serves the next recorded call
/// or records the divergence and traps.
fn wire_replay_host(
    engine: &wasmtime::Engine,
    component: &Component,
    linker: &mut Linker<ReplayState>,
) -> Result<(), ReplayFailure> {
    let component_type = component.component_type();
    for (instance_name, item) in component_type.imports(engine) {
        let wasmtime::component::types::ComponentItem::ComponentInstance(instance) = item.ty else {
            continue;
        };
        let mut linker_instance = linker
            .instance(instance_name)
            .map_err(|e| ReplayFailure::Setup(format!("cannot wire `{instance_name}`: {e}")))?;
        for (func_name, func_item) in instance.exports(engine) {
            let wasmtime::component::types::ComponentItem::ComponentFunc(_) = func_item.ty else {
                continue;
            };
            let qualified = format!("{instance_name}#{func_name}");
            linker_instance
                .func_new(func_name, move |mut store, _ty, params, results| {
                    serve_recorded_call(&qualified, store.data_mut(), params, results)
                })
                .map_err(|e| {
                    ReplayFailure::Setup(format!("cannot wire `{instance_name}#{func_name}`: {e}"))
                })?;
        }
    }
    Ok(())
}

/// The replay host's one behavior: the call must be the next recorded one
/// (same function, same arguments), and its recorded results are served
/// back. Anything else is a divergence, recorded and trapped.
fn serve_recorded_call(
    qualified: &str,
    state: &mut ReplayState,
    params: &[Val],
    results: &mut [Val],
) -> wasmtime::Result<()> {
    let diverge = |state: &mut ReplayState, detail: String| {
        state.divergence = Some(detail.clone());
        Err(wasmtime::Error::msg(detail))
    };

    let Some(expected) = state.pending.front() else {
        return diverge(
            state,
            format!("the request called `{qualified}` but the trace records no more host calls"),
        );
    };
    if expected.function != qualified {
        let expected_name = expected.function.clone();
        return diverge(
            state,
            format!(
                "the request called `{qualified}` but the trace records `{expected_name}` next"
            ),
        );
    }
    let actual_args: Vec<TraceValue> = match params.iter().map(TraceValue::from_val).collect() {
        Ok(args) => args,
        Err(err) => return diverge(state, format!("`{qualified}` arguments: {err}")),
    };
    if actual_args != expected.arguments {
        return diverge(
            state,
            format!(
                "`{qualified}` was called with arguments that differ from the captured ones: got {actual_args:?}, trace records {:?}",
                expected.arguments
            ),
        );
    }
    if expected.results.len() != results.len() {
        let (got, recorded) = (results.len(), expected.results.len());
        return diverge(
            state,
            format!("`{qualified}` expects {got} result value(s) but the trace records {recorded}"),
        );
    }
    let record = state.pending.pop_front().expect("front exists");
    for (slot, value) in results.iter_mut().zip(record.results.iter()) {
        *slot = value.to_val();
    }
    state.replayed += 1;
    Ok(())
}
