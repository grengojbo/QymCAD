//! THE PROGRAM: the server's loop over stdin and stdout.
//!
//! ONE THREAD. The interface language is bound to the thread that set it, and every message the server writes is
//! English - the reader is a model, and one language keeps its replies alike from one machine to the next whatever
//! the system locale says.

fn main() -> std::process::ExitCode {
    qymcad_i18n::set_language("en");
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match qymcad_mcp::server::serve(stdin.lock(), stdout.lock()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qymcad-mcp: the wire broke: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
