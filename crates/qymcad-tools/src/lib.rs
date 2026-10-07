//! THE TOOLS A MODEL DRIVES THE DOCUMENT WITH: each reads its arguments into a typed record, does one action on the
//! document the way the window does - one action, one undo step - and answers in JSON.
//!
//! A module of its own rather than a part of the `qymcad-mcp` program: the program keeps a document of its own and
//! answers a client on stdin and stdout, while a caller holding a document elsewhere - a window lends one through
//! `qymcad_doc::Lent` - runs the same list of tools on it, so the two cannot drift apart.

pub mod args;
mod paths;
pub mod picture;
pub mod tool;
pub mod tools;
