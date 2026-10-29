//! THE PROGRAM: the server's loop over stdin and stdout - the calls that reach a document are done one at a time, and
//! each request is answered in time (see `queue`). The command line says where the calls go (see `engine::parse`).

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let start = match qymcad_mcp::engine::parse(&args) {
        Ok(s) => s,
        Err(why) => {
            eprintln!("qymcad-mcp: {why}");
            return std::process::ExitCode::from(2);
        }
    };
    let answer_within = start.answer_within;
    let engine = qymcad_mcp::engine::Engine::start(start);
    match engine {
        qymcad_mcp::engine::Engine::Window(_) => eprintln!("qymcad-mcp: the calls go to the open window"),
        qymcad_mcp::engine::Engine::Undecided { .. } => eprintln!("qymcad-mcp: the calls go to the open window once one listens"),
        qymcad_mcp::engine::Engine::Here(_) => {}
    }
    let stdin = std::io::BufReader::new(std::io::stdin());
    match qymcad_mcp::server::serve_engine(engine, stdin, std::io::stdout(), answer_within) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qymcad-mcp: the wire broke: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
