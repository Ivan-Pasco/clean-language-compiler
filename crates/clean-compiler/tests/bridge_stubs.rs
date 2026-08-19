//! The M8 stage-5 contract (Platform 14 §14.14.5, bridge stubs): a
//! generated stub component implements its interface with recorded-call
//! semantics and canned responses from a JSON fixture, and non-fixture
//! calls fail loudly. Exercised end to end: generate → validate →
//! instantiate under the component runtime → call → read the log back,
//! typed against the interface's own declaration.

use clean_compiler::replay::TraceValue;
use clean_compiler::stub::{
    decode_log, generate_stub, stub_artifact_name, RecordedCall, StubError, StubFixture,
};
use wasmtime::component::{Component, Linker, Val};

/// The `clean:bridge/console` shape from Platform 02 §2.1's informative
/// example — typed print and leveled log over a payload-less variant.
const CONSOLE_WIT: &str = r#"
package clean:bridge@1.0.0;

interface console {
    variant log-level { info, warn, error, debug }
    print: func(text: string);
    log: func(level: log-level, message: string);
}
"#;

/// A self-authored probe interface exercising responses: scalar, string,
/// and float shapes.
const PROBE_WIT: &str = r#"
package clean:bridge@1.0.0;

interface probe {
    next-id: func() -> u64;
    greet: func(name: string) -> string;
    scale: func(factor: f64) -> f64;
}
"#;

fn fixture(json: &str) -> StubFixture {
    serde_json::from_str(json).expect("fixture JSON parses")
}

struct StubInstance {
    store: wasmtime::Store<()>,
    instance: wasmtime::component::Instance,
    component: Component,
}

impl StubInstance {
    fn new(wasm: &[u8]) -> Self {
        let engine = wasmtime::Engine::default();
        let component = Component::new(&engine, wasm).expect("stub component loads");
        let linker: Linker<()> = Linker::new(&engine);
        let mut store = wasmtime::Store::new(&engine, ());
        let instance = linker
            .instantiate(&mut store, &component)
            .expect("stub instantiates with no imports");
        Self {
            store,
            instance,
            component,
        }
    }

    fn call(
        &mut self,
        interface: &str,
        function: &str,
        params: &[Val],
        result_count: usize,
    ) -> Result<Vec<Val>, wasmtime::Error> {
        let iface_index = self
            .component
            .get_export_index(None, interface)
            .expect("interface exported");
        let index = self
            .component
            .get_export_index(Some(&iface_index), function)
            .expect("function exported");
        let func = self
            .instance
            .get_func(&mut self.store, index)
            .expect("function instance");
        let mut results = vec![Val::Bool(false); result_count];
        func.call(&mut self.store, params, &mut results)?;
        Ok(results)
    }

    fn log(&mut self) -> Vec<u8> {
        let index = self
            .component
            .get_export_index(None, "stub-log")
            .expect("stub-log exported");
        let func = self
            .instance
            .get_func(&mut self.store, index)
            .expect("stub-log instance");
        let mut results = vec![Val::Bool(false)];
        func.call(&mut self.store, &[], &mut results)
            .expect("stub-log call");
        let Val::List(items) = &results[0] else {
            panic!("stub-log returns list<u8>, got {:?}", results[0]);
        };
        items
            .iter()
            .map(|v| match v {
                Val::U8(b) => *b,
                other => panic!("list<u8> element, got {other:?}"),
            })
            .collect()
    }
}

const CONSOLE_IFACE: &str = "clean:bridge/console@1.0.0";
const PROBE_IFACE: &str = "clean:bridge/probe@1.0.0";

/// Recorded-call semantics: every call lands in the log in order, with
/// arguments typed by the interface's declaration.
#[test]
fn stub_records_every_call_with_typed_arguments() {
    let fixture = fixture(r#"{ "responses": { "print": [[], []], "log": [[]] } }"#);
    let wasm = generate_stub(CONSOLE_WIT, "console", &fixture).expect("stub generates");
    wasmparser::Validator::new()
        .validate_all(&wasm)
        .expect("stub passes component-model validation");

    let mut stub = StubInstance::new(&wasm);
    stub.call(CONSOLE_IFACE, "print", &[Val::String("hola".into())], 0)
        .expect("print serves");
    stub.call(
        CONSOLE_IFACE,
        "log",
        &[
            Val::Variant("warn".into(), None),
            Val::String("cuidado".into()),
        ],
        0,
    )
    .expect("log serves");
    stub.call(CONSOLE_IFACE, "print", &[Val::String("adiós".into())], 0)
        .expect("second print serves");

    let calls = decode_log(CONSOLE_WIT, "console", &stub.log()).expect("log decodes");
    assert_eq!(
        calls,
        vec![
            RecordedCall {
                function: "print".to_string(),
                arguments: vec![TraceValue::String("hola".to_string())],
            },
            RecordedCall {
                function: "log".to_string(),
                arguments: vec![
                    TraceValue::Enum("warn".to_string()),
                    TraceValue::String("cuidado".to_string()),
                ],
            },
            RecordedCall {
                function: "print".to_string(),
                arguments: vec![TraceValue::String("adiós".to_string())],
            },
        ]
    );
}

/// Canned responses: call N of a function gets fixture entry N, across
/// scalar, string, and float result shapes.
#[test]
fn stub_serves_fixture_responses_in_order() {
    let fixture = fixture(
        r#"{ "responses": {
            "next-id": [
                [ { "type": "u64", "value": 7 } ],
                [ { "type": "u64", "value": 9 } ]
            ],
            "greet": [ [ { "type": "string", "value": "hola, ana" } ] ],
            "scale": [ [ { "type": "float64", "value": 2.5 } ] ]
        } }"#,
    );
    let wasm = generate_stub(PROBE_WIT, "probe", &fixture).expect("stub generates");

    let mut stub = StubInstance::new(&wasm);
    assert_eq!(
        stub.call(PROBE_IFACE, "next-id", &[], 1).expect("first"),
        vec![Val::U64(7)]
    );
    assert_eq!(
        stub.call(PROBE_IFACE, "next-id", &[], 1).expect("second"),
        vec![Val::U64(9)]
    );
    assert_eq!(
        stub.call(PROBE_IFACE, "greet", &[Val::String("ana".into())], 1)
            .expect("greet"),
        vec![Val::String("hola, ana".into())]
    );
    assert_eq!(
        stub.call(PROBE_IFACE, "scale", &[Val::Float64(4.0)], 1)
            .expect("scale"),
        vec![Val::Float64(2.5)]
    );
}

/// Non-fixture calls fail loudly (§14.14.5): a function past its last
/// fixture entry traps, and a function with no fixture entry traps on its
/// first call — but everything up to that point is already in the log.
#[test]
fn non_fixture_calls_trap() {
    let fixture =
        fixture(r#"{ "responses": { "next-id": [ [ { "type": "u64", "value": 1 } ] ] } }"#);
    let wasm = generate_stub(PROBE_WIT, "probe", &fixture).expect("stub generates");

    let mut stub = StubInstance::new(&wasm);
    assert_eq!(
        stub.call(PROBE_IFACE, "next-id", &[], 1).expect("fixtured"),
        vec![Val::U64(1)]
    );
    let err = stub
        .call(PROBE_IFACE, "next-id", &[], 1)
        .expect_err("exhausted fixture must trap");
    assert!(
        err.downcast_ref::<wasmtime::Trap>().is_some(),
        "expected a trap: {err:?}"
    );

    // A fresh instance: a function absent from the fixture traps at once.
    // (The trap poisons the component instance, so the log is unreadable
    // afterwards — the trap itself is the loud failure.)
    let mut stub = StubInstance::new(&wasm);
    let err = stub
        .call(PROBE_IFACE, "greet", &[Val::String("eva".into())], 1)
        .expect_err("non-fixture function must trap");
    assert!(err.downcast_ref::<wasmtime::Trap>().is_some());
}

/// Same WIT, same fixture ⇒ byte-identical stub (CMP-02 holds on every
/// emitting path).
#[test]
fn generation_is_deterministic() {
    let fixture = fixture(r#"{ "responses": { "print": [[]] } }"#);
    let first = generate_stub(CONSOLE_WIT, "console", &fixture).expect("generates");
    let second = generate_stub(CONSOLE_WIT, "console", &fixture).expect("generates");
    assert_eq!(first, second);
}

/// The guardrails: unknown fixture functions, wrong response arity, and
/// shapes outside the v1 subset are typed refusals, not guesses.
#[test]
fn generator_refuses_bad_inputs() {
    match generate_stub(
        CONSOLE_WIT,
        "console",
        &fixture(r#"{ "responses": { "missing-func": [[]] } }"#),
    ) {
        Err(StubError::Fixture(msg)) => assert!(msg.contains("missing-func"), "{msg}"),
        other => panic!("expected fixture refusal, got {other:?}"),
    }

    match generate_stub(
        CONSOLE_WIT,
        "console",
        &fixture(r#"{ "responses": { "print": [ [ { "type": "u64", "value": 1 } ] ] } }"#),
    ) {
        Err(StubError::Fixture(msg)) => assert!(msg.contains("returns nothing"), "{msg}"),
        other => panic!("expected arity refusal, got {other:?}"),
    }

    let record_wit = r#"
package clean:bridge@1.0.0;

interface db {
    record row { id: u64, name: string }
    first-row: func() -> row;
}
"#;
    match generate_stub(record_wit, "db", &fixture(r#"{ "responses": {} }"#)) {
        Err(StubError::Unsupported(msg)) => assert!(msg.contains("Record"), "{msg}"),
        other => panic!("expected unsupported-shape refusal, got {other:?}"),
    }

    match generate_stub(
        CONSOLE_WIT,
        "nonexistent",
        &fixture(r#"{ "responses": {} }"#),
    ) {
        Err(StubError::Wit(msg)) => assert!(msg.contains("nonexistent"), "{msg}"),
        other => panic!("expected WIT refusal, got {other:?}"),
    }
}

/// The dist layout name (§14.14.5).
#[test]
fn artifact_names_follow_the_dist_layout() {
    assert_eq!(
        stub_artifact_name("console"),
        "clean-bridge-console-stub.wasm"
    );
}
