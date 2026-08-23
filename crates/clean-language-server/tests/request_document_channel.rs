//! LSP-06 mid-session updates (Platform 04 §4.10): the
//! `clean/requestDocument` notification carries a complete replacement
//! document through the same intake as the bootstrap, followed by a full
//! re-check and republication; buckets the replacement no longer owns are
//! cleared, and a defective notification is said once and ignored.

mod common;

use common::{request_document, Client};

const CLEAN_SRC: &str = "functions:\n\tvoid init()\n\t\treturn\n";
const BROKEN_SRC: &str = "functions:\n\tvoid init()\n\t\tinteger x = missing\n\t\treturn\n";

#[test]
fn replacement_document_rechecks_and_clears_stale_buckets() {
    let before = request_document(&[("app/a.cln", BROKEN_SRC)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": before }));
    let initial = client.collect_publishes(2);
    assert_eq!(initial[1].uri, "file:///workspace/app/a.cln");
    assert!(
        !initial[1].diagnostics.is_empty(),
        "the broken source reports before the replacement"
    );

    let after = request_document(&[("app/b.cln", CLEAN_SRC)]);
    client.notify(
        "clean/requestDocument",
        serde_json::json!({ "requestDocument": after }),
    );
    // One clear for the bucket the replacement does not own, then the full
    // republication round (request bucket first, then sources in order).
    let publishes = client.collect_publishes(3);
    assert_eq!(publishes[0].uri, "file:///workspace/app/a.cln");
    assert!(
        publishes[0].diagnostics.is_empty(),
        "the old source's bucket clears"
    );
    assert_eq!(publishes[1].uri, "clean:request");
    assert_eq!(publishes[2].uri, "file:///workspace/app/b.cln");
    assert!(
        publishes[2].diagnostics.is_empty(),
        "the clean replacement checks clean"
    );
    client.stop();
}

#[test]
fn malformed_replacement_is_rejected_with_rqd_diagnostics() {
    let before = request_document(&[("app/a.cln", CLEAN_SRC)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": before }));
    client.collect_publishes(2);

    client.notify(
        "clean/requestDocument",
        serde_json::json!({ "requestDocument": { "spec_version": "1" } }),
    );
    // The old source bucket clears; the RQD diagnostics publish once under
    // the request-document URI, exactly like a rejected bootstrap.
    let publishes = client.collect_publishes(2);
    assert_eq!(publishes[0].uri, "file:///workspace/app/a.cln");
    assert!(publishes[0].diagnostics.is_empty());
    assert_eq!(publishes[1].uri, "clean:request");
    assert!(
        publishes[1]
            .diagnostics
            .iter()
            .any(|d| matches!(&d.code, Some(lsp_types::NumberOrString::String(c)) if c.starts_with("RQD"))),
        "expected RQD diagnostics, got {:?}",
        publishes[1].diagnostics
    );
    client.stop();
}

#[test]
fn notification_without_a_document_is_logged_and_the_session_survives() {
    let before = request_document(&[("app/a.cln", CLEAN_SRC)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": before }));
    client.collect_publishes(2);

    client.notify("clean/requestDocument", serde_json::json!({}));
    let log = client.wait_for_log();
    assert!(
        log.contains("no `requestDocument` in clean/requestDocument params"),
        "unexpected log: {log}"
    );

    // The live session still serves: an overlay edit re-checks as usual.
    client.notify(
        "textDocument/didChange",
        serde_json::json!({
            "textDocument": { "uri": "file:///workspace/app/a.cln", "version": 2 },
            "contentChanges": [{ "text": BROKEN_SRC }],
        }),
    );
    let publishes = client.collect_publishes(2);
    assert_eq!(publishes[1].uri, "file:///workspace/app/a.cln");
    assert!(
        !publishes[1].diagnostics.is_empty(),
        "the session kept serving after the defective notification"
    );
    client.stop();
}
