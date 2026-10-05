//! THE DOCUMENT OVER THE MODEL CONTEXT PROTOCOL. A client starts the `qymcad-mcp` program and talks to it on stdin
//! and stdout; the program keeps one document and changes it the way the window does - one action, one undo step.
//!
//! The library holds the server so the checks can reach the list of tools and call one without a process; the
//! program is the loop over stdin around it.

pub mod args;
mod paths;
pub mod rpc;
pub mod server;
pub mod tool;
mod tools;
