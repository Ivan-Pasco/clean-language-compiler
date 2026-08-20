//! The /counter shape, end to end (backlog item; §14.14.5's stated
//! payoff): a Clean app whose handler calls a composed-bridge function
//! (`clean:fake-bridge/store.bump`, its own package inside the world's
//! multi-package WIT), compiled by the full `compile()` pipeline into a
//! component, then run against a **generated bridge stub** — recorded-call
//! semantics, canned responses — instead of a live bridge. The app's
//! import is served by the stub component's export; the counter values
//! the handler observes are the fixture's; the stub's log records every
//! bump. No server, no database, no bridge implementation — the library
//!-author test loop §14.14.5 promises.

use std::sync::{Arc, Mutex};

use clean_compiler::stub::{decode_log, generate_stub, StubFixture};
use wasmtime::component::{Component, Linker, Val};

mod common;

/// The bridge contract, standalone — what the stub is generated from.
const BRIDGE_WIT: &str = r#"package clean:fake-bridge@0.1.0;

interface store {
    bump: func() -> u32;
}
"#;

/// The app's target world: the same bridge interface as a braced package
/// of the multi-package host WIT, plus a probe for observations.
const WORLD_WIT: &str = r#"package clean:test@0.1.0;

package clean:fake-bridge@0.1.0 {
    interface store {
        bump: func() -> u32;
    }
}

package clean:probe@0.1.0 {
    interface out {
        emit-int: func(value: u64);
    }
}

world testworld {
    export clean:fake-bridge/store@0.1.0;
    export clean:probe/out@0.1.0;

    import init: func();
    import handle: func(handler-id: u32);
}
"#;

const BRIDGE_DECL: &str = "\
host interface store version \"0.1.0\":
\trequires host worlds [\"testworld\"]

\thost function bump() returns integer:u32
\t\tdescription \"Increment and return the counter.\"

host interface out version \"0.1.0\":
\trequires host worlds [\"testworld\"]

\thost function emitInt(value: integer:u64)
\t\tdescription \"Record one integer.\"
";

/// The /counter handler: every request bumps the store and reports the
/// count.
const MAIN: &str = "\
functions:
\tvoid init()
\t\treturn
\tvoid handle(integer id)
\t\tinteger n = bump()
\t\temitInt(n)
";

fn compile_counter() -> Vec<u8> {
    let mut request = common::minimal_valid_request();
    request.target_world.wit = WORLD_WIT.to_string();
    request.target_world.sha256 = common::sha256_hex(WORLD_WIT.as_bytes());
    request.target_world.world = "testworld".to_string();
    request.sources = [("app/host_bridge.cln", BRIDGE_DECL), ("app/main.cln", MAIN)]
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
        .expect("the /counter app compiles through the full pipeline")
        .wasm
}

#[test]
fn counter_runs_against_the_composed_stub_bridge() {
    // The stub: bump answers 1, 2, 3 — and nothing after that.
    let fixture: StubFixture = serde_json::from_str(
        r#"{ "responses": { "bump": [
            [ { "type": "u32", "value": 1 } ],
            [ { "type": "u32", "value": 2 } ],
            [ { "type": "u32", "value": 3 } ]
        ] } }"#,
    )
    .expect("fixture parses");
    let stub_wasm = generate_stub(BRIDGE_WIT, "store", &fixture).expect("stub generates");
    let app_wasm = compile_counter();

    let engine = wasmtime::Engine::default();

    // Instantiate the stub component on its own store.
    let stub_component = Component::new(&engine, &stub_wasm).expect("stub loads");
    let stub_linker: Linker<()> = Linker::new(&engine);
    let mut stub_store = wasmtime::Store::new(&engine, ());
    let stub_instance = stub_linker
        .instantiate(&mut stub_store, &stub_component)
        .expect("stub instantiates");
    let bump_index = {
        let iface = stub_component
            .get_export_index(None, "clean:fake-bridge/store@0.1.0")
            .expect("stub exports the interface");
        stub_component
            .get_export_index(Some(&iface), "bump")
            .expect("stub exports bump")
    };
    let bump_func = stub_instance
        .get_func(&mut stub_store, bump_index)
        .expect("bump instance");
    let stub = Arc::new(Mutex::new((stub_store, bump_func)));

    // The app component: its bridge import is served by the stub's
    // export; the probe collects what the handler reports.
    let app_component = Component::new(&engine, &app_wasm).expect("app loads");
    let mut app_linker: Linker<Vec<u64>> = Linker::new(&engine);
    {
        let stub = Arc::clone(&stub);
        app_linker
            .instance("clean:fake-bridge/store@0.1.0")
            .expect("bridge instance")
            .func_new("bump", move |_store, _ty, params, results| {
                assert!(params.is_empty(), "bump takes no arguments");
                let (stub_store, bump) = &mut *stub.lock().unwrap();
                bump.call(&mut *stub_store, &[], results)
            })
            .expect("bump wired to the stub");
    }
    app_linker
        .instance("clean:probe/out@0.1.0")
        .expect("probe instance")
        .func_new("emit-int", |mut store, _ty, params, _results| {
            let Val::U64(value) = params[0] else {
                panic!("emit-int takes a u64, got {:?}", params[0]);
            };
            store.data_mut().push(value);
            Ok(())
        })
        .expect("probe wired");

    let mut app_store = wasmtime::Store::new(&engine, Vec::new());
    let app = app_linker
        .instantiate(&mut app_store, &app_component)
        .expect("app instantiates against the stub");

    let handle_index = app_component
        .get_export_index(None, "handle")
        .expect("handle exported");
    let handle = app
        .get_func(&mut app_store, handle_index)
        .expect("handle instance");

    // Three /counter requests.
    for _ in 0..3 {
        handle
            .call(&mut app_store, &[Val::U32(0)], &mut [])
            .expect("handler runs");
    }

    // The handler observed the fixture's counter values, in order.
    assert_eq!(app_store.data().as_slice(), [1, 2, 3]);

    // The stub recorded every bump, each with no arguments. (Scoped: the
    // lock must release before the next request needs the stub again.)
    {
        let log_index = stub_component
            .get_export_index(None, "stub-log")
            .expect("stub-log exported");
        let (stub_store, _) = &mut *stub.lock().unwrap();
        let log_func = stub_instance
            .get_func(&mut *stub_store, log_index)
            .expect("stub-log instance");
        let mut results = vec![Val::Bool(false)];
        log_func
            .call(&mut *stub_store, &[], &mut results)
            .expect("stub-log call");
        let Val::List(items) = &results[0] else {
            panic!("stub-log returns list<u8>");
        };
        let raw: Vec<u8> = items
            .iter()
            .map(|v| match v {
                Val::U8(b) => *b,
                other => panic!("list<u8> element, got {other:?}"),
            })
            .collect();
        let calls = decode_log(BRIDGE_WIT, "store", &raw).expect("log decodes");
        assert_eq!(calls.len(), 3);
        for call in &calls {
            assert_eq!(call.function, "bump");
            assert!(call.arguments.is_empty());
        }
    }

    // A fourth request exhausts the fixture: the stub fails loudly
    // (§14.14.5), and the failure surfaces through the app as a trap.
    handle
        .call(&mut app_store, &[Val::U32(0)], &mut [])
        .expect_err("a non-fixture call must fail loudly");
}

/// Determinism (CMP-02 applies to every emitting path): same app, same
/// fixture, byte-identical stub and component.
#[test]
fn composition_inputs_are_deterministic() {
    let fixture: StubFixture = serde_json::from_str(
        r#"{ "responses": { "bump": [ [ { "type": "u32", "value": 1 } ] ] } }"#,
    )
    .expect("fixture parses");
    assert_eq!(
        generate_stub(BRIDGE_WIT, "store", &fixture).unwrap(),
        generate_stub(BRIDGE_WIT, "store", &fixture).unwrap()
    );
    assert_eq!(compile_counter(), compile_counter());
}
