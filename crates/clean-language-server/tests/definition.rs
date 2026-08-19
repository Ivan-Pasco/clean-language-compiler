//! `textDocument/definition`: jump to the declaration the resolver bound the
//! use to (Platform 04 §4.1), at the pipeline's byte-exact spans — user
//! functions, locals and parameters, and state variables.

mod common;

use common::{request_document, Client, ROOT_URI};

const MAIN: &str = "state:\n\
                    \tinteger counter = 0\n\
                    \n\
                    functions:\n\
                    \tvoid init()\n\
                    \t\tinteger n = 5\n\
                    \t\tinteger m = bump(n)\n\
                    \t\tcounter = counter + m\n\
                    \n\
                    \tinteger bump(integer seed)\n\
                    \t\treturn seed + 1\n";

fn definition_at(client: &mut Client, line: u32, character: u32) -> serde_json::Value {
    client
        .request(
            "textDocument/definition",
            serde_json::json!({
                "textDocument": { "uri": format!("{ROOT_URI}/app/main.cln") },
                "position": { "line": line, "character": character },
            }),
        )
        .expect("definition succeeds")
}

#[test]
fn definition_resolves_calls_locals_and_state() {
    let document = request_document(&[("app/main.cln", MAIN)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let _ = client.collect_publishes(2);

    // Line 6 is `\t\tinteger m = bump(n)`: the call at 14 jumps to the
    // `bump` declaration (line 9); its argument `n` at 19 jumps to the
    // binding on line 5.
    let result = definition_at(&mut client, 6, 14);
    assert_eq!(result["uri"], format!("{ROOT_URI}/app/main.cln"));
    assert_eq!(result["range"]["start"]["line"], 9);
    let result = definition_at(&mut client, 6, 19);
    assert_eq!(result["range"]["start"]["line"], 5);
    // The local's target is the name itself: `n` sits at characters 10..11.
    assert_eq!(result["range"]["start"]["character"], 10);
    assert_eq!(result["range"]["end"]["character"], 11);

    // Line 10 is `\t\treturn seed + 1`: the parameter read jumps to the
    // parameter declaration on line 9.
    let result = definition_at(&mut client, 10, 9);
    assert_eq!(result["range"]["start"]["line"], 9);

    // Line 7 is `\t\tcounter = counter + m`: the state read at 12 jumps to
    // the `state:` declaration on line 1.
    let result = definition_at(&mut client, 7, 12);
    assert_eq!(result["range"]["start"]["line"], 1);

    // The `functions:` header resolves to nothing.
    let result = definition_at(&mut client, 3, 0);
    assert!(result.is_null());

    client.stop();
}

#[test]
fn definition_jumps_across_files() {
    let utils = "functions:\n\
                 \tpublic:\n\
                 \t\tinteger add(integer a, integer b)\n\
                 \t\t\treturn a + b\n";
    let main = "import:\n\
                \tutils\n\
                \n\
                functions:\n\
                \tvoid init()\n\
                \t\tinteger s = add(2, 3)\n\
                \t\treturn\n";
    let document = request_document(&[("utils.cln", utils), ("main.cln", main)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let _ = client.collect_publishes(3);

    // Line 5 of main.cln is `\t\tinteger s = add(2, 3)`: the call at 14
    // jumps into utils.cln, to the `add` declaration on line 2.
    let result = client
        .request(
            "textDocument/definition",
            serde_json::json!({
                "textDocument": { "uri": format!("{ROOT_URI}/main.cln") },
                "position": { "line": 5, "character": 14 },
            }),
        )
        .expect("definition succeeds");
    assert_eq!(result["uri"], format!("{ROOT_URI}/utils.cln"));
    assert_eq!(result["range"]["start"]["line"], 2);
    client.stop();
}

#[test]
fn definition_follows_edits() {
    let document = request_document(&[("app/main.cln", MAIN)]);
    let uri = format!("{ROOT_URI}/app/main.cln");
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let _ = client.collect_publishes(2);

    // The edit inserts a line above `bump`, moving its declaration down by
    // one; the jump target must follow.
    let edited = MAIN.replace(
        "\tinteger bump(integer seed)",
        "\t// moved\n\tinteger bump(integer seed)",
    );
    client.notify(
        "textDocument/didChange",
        serde_json::json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{ "text": edited }]
        }),
    );
    let _ = client.collect_publishes(2);

    let result = definition_at(&mut client, 6, 14);
    assert_eq!(result["range"]["start"]["line"], 10);
    client.stop();
}
