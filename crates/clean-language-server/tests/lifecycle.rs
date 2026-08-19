//! Protocol lifecycle over the in-memory transport: the same `run` loop the
//! stdio binary drives, exercised end-to-end (initialize → shutdown → exit).

use lsp_server::{Connection, Message, Notification, Request};

/// Runs the server on a thread against an in-memory client, returning the
/// client half and the server join handle.
fn start() -> (
    Connection,
    std::thread::JoinHandle<Result<(), clean_language_server::ServerError>>,
) {
    let (server, client) = Connection::memory();
    let handle = std::thread::spawn(move || clean_language_server::run(server));
    (client, handle)
}

/// Next response, skipping server-initiated notifications (this suite runs
/// without a request document, so a `window/logMessage` saying exactly that
/// arrives after the handshake).
fn next_response(client: &Connection) -> lsp_server::Response {
    loop {
        match client.receiver.recv().unwrap() {
            Message::Response(response) => return response,
            Message::Notification(_) => continue,
            other => panic!("expected response, got {other:?}"),
        }
    }
}

#[test]
fn initialize_negotiates_and_shutdown_terminates() {
    let (client, handle) = start();

    client
        .sender
        .send(Message::Request(Request::new(
            1.into(),
            "initialize".to_string(),
            serde_json::json!({ "capabilities": {} }),
        )))
        .unwrap();
    let response = match client.receiver.recv().unwrap() {
        Message::Response(response) => response,
        other => panic!("expected initialize response, got {other:?}"),
    };
    let result = response.response_result.expect("initialize succeeds");
    assert_eq!(
        result["capabilities"]["textDocumentSync"],
        serde_json::json!(1),
        "full-document sync is declared"
    );
    assert_eq!(result["capabilities"]["positionEncoding"], "utf-16");
    assert_eq!(result["serverInfo"]["name"], "clean-language-server");
    // Capabilities are declared only once their handler exists; hover and
    // definition both are, as of their stages.
    assert_eq!(result["capabilities"]["hoverProvider"], true);
    assert_eq!(result["capabilities"]["definitionProvider"], true);

    client
        .sender
        .send(Message::Notification(Notification::new(
            "initialized".to_string(),
            serde_json::json!({}),
        )))
        .unwrap();

    // A request without a handler is answered with MethodNotFound, not
    // dropped and not faked.
    client
        .sender
        .send(Message::Request(Request::new(
            2.into(),
            "textDocument/references".to_string(),
            serde_json::json!({}),
        )))
        .unwrap();
    let response = next_response(&client);
    let error = response
        .response_result
        .expect_err("unhandled method is an error");
    assert_eq!(error.code, lsp_server::ErrorCode::MethodNotFound as i32);

    client
        .sender
        .send(Message::Request(Request::new(
            3.into(),
            "shutdown".to_string(),
            serde_json::Value::Null,
        )))
        .unwrap();
    let response = next_response(&client);
    assert!(response.response_result.is_ok());

    client
        .sender
        .send(Message::Notification(Notification::new(
            "exit".to_string(),
            serde_json::Value::Null,
        )))
        .unwrap();

    handle.join().unwrap().expect("server exits cleanly");
}
