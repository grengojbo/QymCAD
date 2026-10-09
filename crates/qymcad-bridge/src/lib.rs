//! THE CHANNEL BETWEEN THE OPEN WINDOW AND THE PROGRAM A LANGUAGE MODEL STARTS.
//!
//! The window opens one local socket ([`Listener`]); the program connects to it ([`Link`]) and passes each call of a
//! tool through, one JSON object a line: `{"id", "tool", "arguments"}` in, `{"id", "result"}` or `{"id", "error":
//! {"code", "message"}}` out. A thread of the channel accepts the clients and reads their lines; the window takes the
//! calls one by one from a queue on its own thread, does them, and answers.
//!
//! THE SOCKET IS ITS OWNER'S ALONE: it lies in the person's own folder and is made readable and writable by them only.
//! Nothing goes over a network.
//!
//! A CALL NOBODY TOOK IS WITHDRAWN, NOT LEFT TO HAPPEN LATER. A window busy with a person's own action takes nothing
//! from the queue; past [`Wait::answer_within`] the caller is told the window is busy, and the call is marked so the
//! window skips it when it comes free. A call the window has already taken is waited for however long it takes: an
//! answer of "busy" to an action that then happens would tell the caller the opposite of the truth.

use std::path::PathBuf;
use std::time::Duration;

/// The name of the socket in the program's own folder.
pub const SOCKET: &str = "mcp.sock";

/// WHERE THE WINDOW LISTENS: in the program's own folder, the one its settings are in - under the root a run of checks
/// gives, when one is given.
pub fn default_path() -> Option<PathBuf> {
    qymcad_paths::data_root().map(|d| d.join(SOCKET))
}

/// How long a call may wait for the window to take it.
#[derive(Clone, Copy, Debug)]
pub struct Wait {
    pub answer_within: Duration,
}

/// WHAT WAKES THE WINDOW when a call comes in. A window asks for calls once a frame, and a window nobody touches draws
/// no frames: without a wake a call would wait for the next movement of the mouse.
pub type Wake = std::sync::Arc<dyn Fn() + Send + Sync>;

/// WHY THE WINDOW'S END DID NOT OPEN.
#[derive(Debug)]
pub enum Opened {
    /// Another window listens on this socket.
    Taken,
    /// The path is longer than a socket path may be.
    TooLong(PathBuf),
    /// The system refused: the folder, the socket or its mode.
    Io(String),
    /// This system has no channel yet.
    Unsupported,
}

/// WHY A CALL OVER THE LINK CAME BACK WITHOUT AN ANSWER.
#[derive(Debug)]
pub enum LinkError {
    /// No window listens: none is open, or the window has the channel switched off.
    NoWindow,
    /// The window closed while the link was open.
    Gone,
    /// The window did not take the call in time; the call was withdrawn and will not happen.
    Busy,
    /// The window took the call and did not answer within the link's time.
    Late,
    /// The window answered with a refusal of the channel itself: its code and words.
    Refused { code: String, message: String },
    /// The answer could not be read.
    Broken(String),
    /// This system has no channel yet.
    Unsupported,
}

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{Call, Link, Listener};

#[cfg(not(unix))]
mod elsewhere;
#[cfg(not(unix))]
pub use elsewhere::{Call, Link, Listener};
