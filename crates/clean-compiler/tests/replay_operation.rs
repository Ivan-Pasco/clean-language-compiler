//! The M8 stage-4 contract (Platform 14 §14.14.6, request replay): over the
//! same request document, a component compiled by `compile()` re-runs under
//! the replay host with every non-deterministic input frozen to the trace's
//! captured values, and the response must match byte-for-byte. Any host
//! call the trace did not record next — wrong function, wrong arguments,
//! extra or missing calls — is a divergence, never a "close enough".

mod common;

use clean_compiler::replay::{
    replay, trace_from_json, EntryRecord, HostCallRecord, ReplayFailure, RequestTrace, TraceValue,
};

/// The websocket surface of the vendored host contract, declared per
/// framework 09 §8 (ok types only; the world carries the `result<>`).
const WS_BRIDGE: &str = "\
host interface websocket version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function accept() returns integer:u64
\t\tdescription \"Accept the pending upgrade.\"

\thost function sendText(socket: integer:u64, message: string)
\t\tdescription \"Queue a text message.\"
";

const MAIN: &str = "\
functions:
\tvoid init()
\t\tinteger sock = accept() onError 0
\t\tsendText(sock, \"hi\")
";

fn compile_component() -> Vec<u8> {
    let mut request = common::minimal_valid_request();
    request.sources = [("app/host_bridge.cln", WS_BRIDGE), ("app/main.cln", MAIN)]
        .iter()
        .map(
            |(path, content)| clean_compiler_types::request::SourceFile {
                path: path.to_string(),
                sha256: common::sha256_hex(content.as_bytes()),
                content: content.to_string(),
            },
        )
        .collect();
    clean_compiler::compile(request)
        .expect("fixture compiles through the full pipeline")
        .wasm
}

fn ok_u64(value: u64) -> TraceValue {
    TraceValue::Result(Ok(Some(Box::new(TraceValue::U64(value)))))
}

fn ok_unit() -> TraceValue {
    TraceValue::Result(Ok(None))
}

/// The captured happy path: accept() returned 77, send-text(77, "hi")
/// succeeded, init returned nothing.
fn happy_trace(component: &[u8]) -> RequestTrace {
    RequestTrace {
        spec_version: "1".to_string(),
        component_sha256: common::sha256_hex(component),
        entry: EntryRecord {
            function: "init".to_string(),
            arguments: vec![],
        },
        host_calls: vec![
            HostCallRecord {
                function: "clean:host/websocket@0.1.0#accept".to_string(),
                arguments: vec![],
                results: vec![ok_u64(77)],
            },
            HostCallRecord {
                function: "clean:host/websocket@0.1.0#send-text".to_string(),
                arguments: vec![TraceValue::U64(77), TraceValue::String("hi".to_string())],
                results: vec![ok_unit()],
            },
        ],
        response: vec![],
    }
}

#[test]
fn replay_serves_every_recorded_call_and_matches_the_response() {
    let component = compile_component();
    let trace = happy_trace(&component);
    let report = replay(&trace, &component).expect("replay succeeds");
    assert_eq!(report.host_calls_replayed, 2, "both captured calls served");
}

/// The captured error path replays too: the recorded values steer the
/// guest down the same branch it took originally (`onError 0`).
#[test]
fn replay_reproduces_the_captured_error_path() {
    let component = compile_component();
    let mut trace = happy_trace(&component);
    trace.host_calls[0].results = vec![TraceValue::Result(Err(Some(Box::new(TraceValue::Enum(
        "not-an-upgrade".to_string(),
    )))))];
    // The fallback arm ran with sock = 0.
    trace.host_calls[1].arguments = vec![TraceValue::U64(0), TraceValue::String("hi".to_string())];
    let report = replay(&trace, &component).expect("error path replays");
    assert_eq!(report.host_calls_replayed, 2);
}

/// A recorded argument the re-run does not reproduce is a divergence
/// naming the call — never served, never guessed.
#[test]
fn diverging_arguments_fail_the_replay() {
    let component = compile_component();
    let mut trace = happy_trace(&component);
    trace.host_calls[1].arguments =
        vec![TraceValue::U64(77), TraceValue::String("bye".to_string())];

    match replay(&trace, &component) {
        Err(ReplayFailure::Diverged { detail, replayed }) => {
            assert!(detail.contains("send-text"), "names the call: {detail}");
            assert_eq!(replayed, 1, "accept was served before the divergence");
        }
        other => panic!("expected divergence, got {other:?}"),
    }
}

/// Recorded calls the re-run never makes are a divergence too.
#[test]
fn leftover_recorded_calls_fail_the_replay() {
    let component = compile_component();
    let mut trace = happy_trace(&component);
    trace.host_calls.push(HostCallRecord {
        function: "clean:host/websocket@0.1.0#accept".to_string(),
        arguments: vec![],
        results: vec![ok_u64(78)],
    });

    match replay(&trace, &component) {
        Err(ReplayFailure::Diverged { detail, replayed }) => {
            assert!(detail.contains("1 more host call"), "{detail}");
            assert_eq!(replayed, 2);
        }
        other => panic!("expected divergence, got {other:?}"),
    }
}

/// A response differing from the captured one fails, even when every host
/// call matched.
#[test]
fn response_mismatch_fails_the_replay() {
    let component = compile_component();
    let mut trace = happy_trace(&component);
    trace.response = vec![TraceValue::U32(1)];

    match replay(&trace, &component) {
        Err(ReplayFailure::ResponseMismatch { expected, actual }) => {
            assert_eq!(expected, vec![TraceValue::U32(1)]);
            assert!(actual.is_empty(), "init returns nothing");
        }
        other => panic!("expected response mismatch, got {other:?}"),
    }
}

/// Replay refuses bytes the trace was not captured against.
#[test]
fn component_hash_mismatch_is_refused_before_running() {
    let component = compile_component();
    let mut trace = happy_trace(&component);
    trace.component_sha256 = "0".repeat(64);

    match replay(&trace, &component) {
        Err(ReplayFailure::ComponentMismatch {
            expected_sha256, ..
        }) => {
            assert_eq!(expected_sha256, "0".repeat(64));
        }
        other => panic!("expected component mismatch, got {other:?}"),
    }
}

/// The schema-validation half the compiler ships (§14.14.6): version gate,
/// unknown-field refusal, and a pinned wire encoding.
#[test]
fn trace_schema_validation_is_strict() {
    assert!(
        trace_from_json(
            r#"{"spec_version":"2","component_sha256":"x","entry":{"function":"init"}}"#
        )
        .unwrap_err()
        .contains("spec_version"),
        "future versions are refused, not guessed"
    );
    assert!(
        trace_from_json(
            r#"{"spec_version":"1","component_sha256":"x","entry":{"function":"init"},"surprise":true}"#
        )
        .unwrap_err()
        .contains("surprise"),
        "unknown fields are refused, like the request document's intake"
    );

    // The pinned wire shape: this exact JSON is the provisional trace
    // format (DISCOVERIES-M8 §6).
    let json = r#"{
        "spec_version": "1",
        "component_sha256": "abc",
        "entry": { "function": "init", "arguments": [] },
        "host_calls": [
            {
                "function": "clean:host/websocket@0.1.0#accept",
                "arguments": [],
                "results": [ { "type": "result", "value": { "Ok": { "type": "u64", "value": 77 } } } ]
            }
        ],
        "response": []
    }"#;
    let trace = trace_from_json(json).expect("the pinned wire shape parses");
    assert_eq!(trace.entry.function, "init");
    assert_eq!(trace.host_calls[0].results, vec![ok_u64(77)]);

    // And it round-trips: serialize → parse → identical value.
    let reserialized = serde_json::to_string(&trace).unwrap();
    assert_eq!(trace_from_json(&reserialized).unwrap(), trace);
}
