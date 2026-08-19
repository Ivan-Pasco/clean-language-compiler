//! Process adapter for the language server: stdio transport over
//! [`clean_language_server::run`]. Like `clean-compiler`, this binary is not
//! a user-facing command (CCMP-04): Clean Manager resolves and launches it at
//! the project's pinned compiler version (CCMP-26, LSP-05), and editors speak
//! LSP to it — no flags, no configuration files.

use std::process::ExitCode;

fn main() -> ExitCode {
    let (connection, io_threads) = lsp_server::Connection::stdio();
    let served = clean_language_server::run(connection);
    let joined = io_threads.join();
    match (served, joined) {
        (Ok(()), Ok(())) => ExitCode::SUCCESS,
        (Err(err), _) => {
            eprintln!("clean-language-server: {err}");
            ExitCode::FAILURE
        }
        (_, Err(err)) => {
            eprintln!("clean-language-server: io threads: {err}");
            ExitCode::FAILURE
        }
    }
}
