//! THE PROGRAM: the server's loop over stdin and stdout, on one thread - requests are answered one at a time. The
//! command line says where the calls go (see `engine::parse`).

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let start = match qymcad_mcp::engine::parse(&args) {
        Ok(s) => s,
        Err(why) => {
            eprintln!("qymcad-mcp: {why}");
            return std::process::ExitCode::from(2);
        }
    };
    let engine = qymcad_mcp::engine::Engine::start(start);
    match engine {
        qymcad_mcp::engine::Engine::Window(_) => eprintln!("qymcad-mcp: the calls go to the open window"),
        qymcad_mcp::engine::Engine::Undecided(_) => eprintln!("qymcad-mcp: the first call goes to the open window if one listens then"),
        qymcad_mcp::engine::Engine::Here(_) => {}
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match qymcad_mcp::server::serve_engine(engine, stdin.lock(), stdout.lock()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qymcad-mcp: the wire broke: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
