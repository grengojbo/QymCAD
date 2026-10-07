//! THE DOCUMENT OVER THE MODEL CONTEXT PROTOCOL. A client starts the `qymcad-mcp` program and talks to it on stdin
//! and stdout; the program keeps one document and changes it the way the window does - one action, one undo step.
//!
//! The library holds the server so the checks can reach the list of tools and call one without a process; the
//! program is the loop over stdin around it.
//!
//! The tools themselves live in `qymcad-tools`, a module any caller with a document can take; they are named here as
//! well, so the program's checks reach them where they always have.

pub use qymcad_tools::{args, picture, tool};
pub mod engine;
mod prompts;
mod resources;
pub mod rpc;
pub mod server;
