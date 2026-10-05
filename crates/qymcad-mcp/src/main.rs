//! THE PROGRAM: the server's loop over stdin and stdout, on one thread - requests are answered one at a time.

fn main() -> std::process::ExitCode {
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
