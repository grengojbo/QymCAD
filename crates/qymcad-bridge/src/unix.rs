//! The channel over a Unix domain socket: macOS and Linux.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{LinkError, Opened, Wait};

/// The longest socket path the systems take: `sun_path` holds 104 bytes on macOS and 108 on Linux, the closing zero
/// among them.
const PATH_LIMIT: usize = 103;

/// The code of a call the window did not take in time.
const BUSY: &str = "window-busy";
/// The code of a line that is no call.
const BAD_CALL: &str = "bad-call";
/// The code of a call dropped without an answer: the window closed with it in the queue or in hand.
const NO_ANSWER: &str = "no-answer";

/// One call as it travels in.
#[derive(Serialize, Deserialize)]
struct Request {
    id: u64,
    tool: String,
    #[serde(default)]
    arguments: Value,
}

/// A refusal of the channel, as it travels out.
#[derive(Serialize, Deserialize)]
struct Fault {
    code: String,
    message: String,
}

/// One answer as it travels out.
#[derive(Serialize, Deserialize)]
struct Reply {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<Fault>,
}

impl Reply {
    fn fault(id: Option<u64>, code: &str, message: &str) -> Self {
        Reply { id, result: None, error: Some(Fault { code: code.into(), message: message.into() }) }
    }
}

/// Where a call stands between the channel and the window.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Stand {
    Waiting = 0,
    Taken = 1,
    Withdrawn = 2,
}

fn shift(stand: &AtomicU8, from: Stand, to: Stand) -> bool {
    stand.compare_exchange(from as u8, to as u8, Ordering::AcqRel, Ordering::Acquire).is_ok()
}

/// ONE CALL FOR THE WINDOW: the tool and its arguments, and the way back to the caller.
pub struct Call {
    pub tool: String,
    pub arguments: Value,
    back: Sender<Value>,
    stand: Arc<AtomicU8>,
}

impl Call {
    /// ANSWER THE CALL. A caller that has gone in the meantime is not a fault: there is nobody left to tell.
    pub fn answer(self, result: Value) {
        let _ = self.back.send(result);
    }
}

/// THE WINDOW'S END: the socket, and the queue of calls the window takes from.
pub struct Listener {
    path: PathBuf,
    queue: Receiver<Call>,
    stop: Arc<AtomicBool>,
}

impl Listener {
    /// OPEN THE SOCKET AT `path`. A socket another window listens on is refused; a file nobody listens on - what a
    /// window that died without closing leaves - is cleared first. The socket is made its owner's alone.
    pub fn open(path: &Path, wait: Wait) -> Result<Listener, Opened> {
        if path.as_os_str().len() > PATH_LIMIT {
            return Err(Opened::TooLong(path.to_path_buf()));
        }
        let io = |e: std::io::Error| Opened::Io(format!("{}: {e}", path.display()));
        if std::fs::symlink_metadata(path).is_ok() {
            if UnixStream::connect(path).is_ok() {
                return Err(Opened::Taken);
            }
            std::fs::remove_file(path).map_err(io)?;
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let socket = UnixListener::bind(path).map_err(io)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(io)?;

        let (queue_in, queue) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_seen = stop.clone();
        std::thread::spawn(move || accept(socket, queue_in, wait, &stop_seen));
        Ok(Listener { path: path.to_path_buf(), queue, stop })
    }

    /// THE NEXT CALL TO DO, or nothing when none waits. Never blocks: the window asks once a frame. A call the channel
    /// has withdrawn meanwhile is passed over.
    pub fn next(&self) -> Option<Call> {
        loop {
            let call = self.queue.try_recv().ok()?;
            if shift(&call.stand, Stand::Waiting, Stand::Taken) {
                return Some(call);
            }
        }
    }
}

impl Drop for Listener {
    /// CLOSE THE WINDOW'S END: the thread accepting clients is woken and stops, and the socket file goes. A client
    /// connected now learns it at its next call: the queue is gone, its thread ends and closes the connection, and
    /// a call waiting in the queue is dropped unanswered - each read on the program's end as the window gone.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = UnixStream::connect(&self.path);
        let _ = std::fs::remove_file(&self.path);
    }
}

/// ACCEPT CLIENTS until the window's end closes, each answered on a thread of its own.
fn accept(socket: UnixListener, queue: Sender<Call>, wait: Wait, stop: &AtomicBool) {
    for stream in socket.incoming() {
        if stop.load(Ordering::Acquire) {
            return;
        }
        let Ok(stream) = stream else { continue };
        let queue = queue.clone();
        std::thread::spawn(move || serve(stream, &queue, wait));
    }
}

/// ANSWER ONE CLIENT, a line at a time, until it goes or the window closes.
fn serve(stream: UnixStream, queue: &Sender<Call>, wait: Wait) {
    let Ok(read) = stream.try_clone() else { return };
    let mut out = stream;
    for line in BufReader::new(read).lines() {
        let Ok(line) = line else { return };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Request>(&line) {
            Err(e) => Reply::fault(None, BAD_CALL, &format!("not a call: {e}")),
            Ok(r) => {
                let (back, answer) = mpsc::channel();
                let stand = Arc::new(AtomicU8::new(Stand::Waiting as u8));
                if queue.send(Call { tool: r.tool, arguments: r.arguments, back, stand: stand.clone() }).is_err() {
                    return; // the window has closed
                }
                match wait_for(&answer, &stand, wait.answer_within) {
                    Answer::Given(result) => Reply { id: Some(r.id), result: Some(result), error: None },
                    Answer::NotTaken => Reply::fault(Some(r.id), BUSY, "The window did not take the call in time; it will not happen."),
                    Answer::Dropped => Reply::fault(Some(r.id), NO_ANSWER, "The window closed before it answered."),
                }
            }
        };
        let Ok(mut text) = serde_json::to_string(&reply) else { return };
        text.push('\n');
        if out.write_all(text.as_bytes()).and_then(|()| out.flush()).is_err() {
            return;
        }
    }
}

/// How a call came back from the window.
enum Answer {
    Given(Value),
    /// Nobody took the call in time, and it was withdrawn.
    NotTaken,
    /// The call was dropped without an answer.
    Dropped,
}

/// WAIT FOR THE WINDOW'S ANSWER: `within` for the window to take the call, then as long as it takes to do it.
fn wait_for(answer: &Receiver<Value>, stand: &AtomicU8, within: Duration) -> Answer {
    match answer.recv_timeout(within) {
        Ok(v) => return Answer::Given(v),
        Err(RecvTimeoutError::Disconnected) => return Answer::Dropped,
        Err(RecvTimeoutError::Timeout) => {}
    }
    if shift(stand, Stand::Waiting, Stand::Withdrawn) {
        return Answer::NotTaken;
    }
    match answer.recv() {
        Ok(v) => Answer::Given(v),
        Err(_) => Answer::Dropped,
    }
}

/// THE PROGRAM'S END: one connection to the window, carrying calls one at a time.
pub struct Link {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
    next_id: u64,
}

impl Link {
    /// CONNECT TO THE WINDOW AT `path`; an answer is waited for at most `late`.
    pub fn connect(path: &Path, late: Duration) -> Result<Link, LinkError> {
        let stream = UnixStream::connect(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused => LinkError::NoWindow,
            _ => LinkError::Broken(e.to_string()),
        })?;
        stream.set_read_timeout(Some(late)).map_err(|e| LinkError::Broken(e.to_string()))?;
        let reader = BufReader::new(stream.try_clone().map_err(|e| LinkError::Broken(e.to_string()))?);
        Ok(Link { stream, reader, next_id: 0 })
    }

    /// CALL `tool` IN THE WINDOW and wait for its answer.
    pub fn call(&mut self, tool: &str, arguments: Value) -> Result<Value, LinkError> {
        self.next_id += 1;
        let request = Request { id: self.next_id, tool: tool.to_string(), arguments };
        let mut text = serde_json::to_string(&request).map_err(|e| LinkError::Broken(e.to_string()))?;
        text.push('\n');
        self.stream.write_all(text.as_bytes()).and_then(|()| self.stream.flush()).map_err(|_| LinkError::Gone)?;
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => return Err(LinkError::Gone),
            Ok(_) => {}
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => return Err(LinkError::Late),
            Err(_) => return Err(LinkError::Gone),
        }
        let reply: Reply = serde_json::from_str(&line).map_err(|e| LinkError::Broken(format!("{e}: {line}")))?;
        if let Some(f) = reply.error {
            return Err(match f.code.as_str() {
                BUSY => LinkError::Busy,
                NO_ANSWER => LinkError::Gone,
                _ => LinkError::Refused { code: f.code, message: f.message },
            });
        }
        reply.result.ok_or_else(|| LinkError::Broken(format!("an answer with neither a result nor an error: {line}")))
    }
}
