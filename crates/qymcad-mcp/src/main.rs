//! THE DOCUMENT OVER THE MODEL CONTEXT PROTOCOL. A client starts this program and talks to it on stdin and
//! stdout; the program keeps one document and changes it the way the window does.
//!
//! ONE THREAD. The interface language is bound to the thread that set it, and every message the server writes
//! is taken from the catalogue in English - the reader is a model, and one language keeps its replies alike
//! from one machine to the next whatever the system locale says.

mod rpc;
mod server;

fn main() -> std::process::ExitCode {
    qymcad_i18n::set_language("en");
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match server::serve(stdin.lock(), stdout.lock()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qymcad-mcp: the wire broke: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
