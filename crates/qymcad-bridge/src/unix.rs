//! The channel over a Unix domain socket: macOS and Linux.

use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use crate::wire::{self, Answering, Answers, Call, Heard, Link};
use crate::{LinkError, Opened, Wait, Wake};

/// The longest socket path the systems take: `sun_path` holds 104 bytes on macOS and 108 on Linux, the closing zero
/// among them.
const PATH_LIMIT: usize = 103;

/// THE WINDOW'S END: the socket, and the queue of calls the window takes from.
pub struct Listener {
    path: PathBuf,
    queue: Receiver<Call>,
    stop: Arc<AtomicBool>,
}

impl Listener {
    /// OPEN THE SOCKET AT `path`. A socket another window listens on is refused; a file nobody listens on - what a
    /// window that died without closing leaves - is cleared first. The socket is made its owner's alone. `wake` is
    /// called each time a call is queued.
    pub fn open(path: &Path, wait: Wait, wake: Wake) -> Result<Listener, Opened> {
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
        std::thread::spawn(move || accept(socket, queue_in, Answering { wait, wake }, &stop_seen));
        Ok(Listener { path: path.to_path_buf(), queue, stop })
    }

    /// THE NEXT CALL TO DO, or nothing when none waits. Never blocks: the window asks once a frame.
    pub fn next(&self) -> Option<Call> {
        wire::next(&self.queue)
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
fn accept(socket: UnixListener, queue: Sender<Call>, answering: Answering, stop: &AtomicBool) {
    for stream in socket.incoming() {
        if stop.load(Ordering::Acquire) {
            return;
        }
        let Ok(stream) = stream else { continue };
        let Ok(read) = stream.try_clone() else { continue };
        let (queue, answering) = (queue.clone(), answering.clone());
        std::thread::spawn(move || wire::serve(read, stream, &queue, &answering));
    }
}

/// The answers over a socket: a read with the socket's own timeout.
struct SocketAnswers {
    reader: BufReader<UnixStream>,
}

impl Answers for SocketAnswers {
    fn line_within(&mut self, late: Duration) -> Heard {
        if self.reader.get_ref().set_read_timeout(Some(late)).is_err() {
            return Heard::End;
        }
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => Heard::End,
            Ok(_) => Heard::Line(line),
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => Heard::Late,
            Err(_) => Heard::End,
        }
    }
}

/// CONNECT TO THE WINDOW AT `path`; an answer is waited for at most `late`.
pub fn connect(path: &Path, late: Duration) -> Result<Link, LinkError> {
    let stream = UnixStream::connect(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused => LinkError::NoWindow,
        _ => LinkError::Broken(e.to_string()),
    })?;
    let read = stream.try_clone().map_err(|e| LinkError::Broken(e.to_string()))?;
    Ok(Link::over(SocketAnswers { reader: BufReader::new(read) }, stream, late))
}
