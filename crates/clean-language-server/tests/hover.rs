//! `textDocument/hover`: the type checker's answer at the pipeline's spans
//! (Platform 04 §4.1) — type of the expression under the cursor, callee
//! signatures on calls, nothing when the program has no authoritative types.

mod common;

use common::{request_document, Client, ROOT_URI};

const SOURCE: &str = "functions:\n\
                      \tvoid init()\n\
                      \t\tstring s = \"a\"\n\
                      \t\tinteger n = count(5)\n\
                      \n\
                      \tinteger count(integer seed)\n\
                      \t\treturn seed + 1\n";

fn hover_at(client: &mut Client, line: u32, character: u32) -> serde_json::Value {
    client
        .request(
            "textDocument/hover",
            serde_json::json!({
                "textDocument": { "uri": format!("{ROOT_URI}/app/main.cln") },
                "position": { "line": line, "character": character },
            }),
        )
        .expect("hover succeeds")
}

fn hover_text(result: &serde_json::Value) -> &str {
    result["contents"]["value"]
        .as_str()
        .expect("hover carries markup content")
}

#[test]
fn hover_reports_expression_types_and_signatures() {
    let document = request_document(&[("app/main.cln", SOURCE)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let _ = client.collect_publishes(2);

    // Line 2 is `\t\tstring s = "a"`: the literal starts at character 13.
    let result = hover_at(&mut client, 2, 14);
    assert_eq!(hover_text(&result), "```clean\nstring\n```");
    // The reported range is the literal itself, byte-exact.
    assert_eq!(
        result["range"]["start"],
        serde_json::json!({ "line": 2, "character": 13 })
    );
    assert_eq!(
        result["range"]["end"],
        serde_json::json!({ "line": 2, "character": 16 })
    );

    // Line 3 is `\t\tinteger n = count(5)`: hovering the call shows the
    // callee's signature; hovering its argument shows the argument's type.
    let result = hover_at(&mut client, 3, 15);
    assert_eq!(
        hover_text(&result),
        "```clean\ninteger count(integer seed)\n```"
    );
    let result = hover_at(&mut client, 3, 20);
    assert_eq!(hover_text(&result), "```clean\ninteger\n```");

    client.stop();
}

#[test]
fn hover_follows_edits() {
    let document = request_document(&[("app/main.cln", SOURCE)]);
    let uri = format!("{ROOT_URI}/app/main.cln");
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let _ = client.collect_publishes(2);

    // The edit turns `s` into a number binding; hover must answer from the
    // re-checked overlay, not the base document.
    let edited = "functions:\n\tvoid init()\n\t\tnumber s = 1.5\n";
    client.notify(
        "textDocument/didChange",
        serde_json::json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{ "text": edited }]
        }),
    );
    let _ = client.collect_publishes(2);

    // Line 2 is now `\t\tnumber s = 1.5`: the literal starts at character 13.
    let result = hover_at(&mut client, 2, 13);
    assert_eq!(hover_text(&result), "```clean\nnumber\n```");
    client.stop();
}

#[test]
fn hover_is_empty_without_authoritative_types() {
    // The program fails pass [5]; there is no typed program to answer from,
    // and a stale or guessed answer would be worse than none (LSP-04).
    let broken = "functions:\n\tvoid init()\n\t\tinteger n = \"text\"\n";
    let document = request_document(&[("app/main.cln", broken)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let _ = client.collect_publishes(2);

    let result = hover_at(&mut client, 2, 14);
    assert!(result.is_null(), "no hover on an ill-typed program");
    client.stop();
}

#[test]
fn hover_outside_any_span_is_null() {
    let document = request_document(&[("app/main.cln", SOURCE)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let _ = client.collect_publishes(2);

    // Line 0 is the `functions:` block header — outside every function and
    // expression span.
    let result = hover_at(&mut client, 0, 0);
    assert!(result.is_null());
    client.stop();
}
