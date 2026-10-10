//! WHAT TRAVELS OVER THE CHANNEL, whatever carries it: one JSON object a line each way, the queue the window takes
//! calls from, the withdrawal of a call nobody took in time, and the program's end that sends a call and waits for its
//! answer. A system's own end - a Unix socket, a named pipe - only opens, accepts and connects, and hands its streams
//! here.

use std::io::{BufRead, BufReader, Read, Write};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{LinkError, Wait, Wake};

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

/// THE NEXT CALL TO DO from `queue`, or nothing when none waits. Never blocks: the window asks once a frame. A call the
/// channel has withdrawn meanwhile is passed over.
pub(crate) fn next(queue: &Receiver<Call>) -> Option<Call> {
    loop {
        let call = queue.try_recv().ok()?;
        if shift(&call.stand, Stand::Waiting, Stand::Taken) {
            return Some(call);
        }
    }
}

/// How a client is answered: how long a call may wait to be taken, and how the window is woken for it.
#[derive(Clone)]
pub(crate) struct Answering {
    pub wait: Wait,
    pub wake: Wake,
}

/// ANSWER ONE CLIENT, a line at a time, until it goes or the window closes: `read` and `out` are the two directions of
/// one connection.
pub(crate) fn serve(read: impl Read, mut out: impl Write, queue: &Sender<Call>, answering: &Answering) {
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
                (answering.wake)();
                match wait_for(&answer, &stand, answering.wait.answer_within) {
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

/// WHAT THE PROGRAM'S END HEARD from the window while it waited.
pub(crate) enum Heard {
    Line(String),
    /// The window closed its end.
    End,
    /// Nothing came within the time.
    Late,
}

/// HOW A SYSTEM'S END READS THE WINDOW'S ANSWER: one line, waited for at most `late`, on the thread that sent the call.
///
/// ON THAT THREAD, NOT ON ONE OF ITS OWN: on Windows the reads and writes of one pipe opened without overlapping are
/// done one after another, and a read left waiting on another thread held the write of the next call for ever -
/// the first call over the pipe never left.
pub(crate) trait Answers: Send {
    fn line_within(&mut self, late: Duration) -> Heard;
}

/// THE PROGRAM'S END: one connection to the window, carrying calls one at a time.
pub struct Link {
    out: Box<dyn Write + Send>,
    answers: Box<dyn Answers>,
    late: Duration,
    next_id: u64,
}

impl Link {
    /// A LINK OVER ONE CONNECTION: `answers` reads it, `out` writes it; an answer is waited for at most `late`.
    pub(crate) fn over(answers: impl Answers + 'static, out: impl Write + Send + 'static, late: Duration) -> Link {
        Link { out: Box::new(out), answers: Box::new(answers), late, next_id: 0 }
    }

    /// CALL `tool` IN THE WINDOW and wait for its answer.
    pub fn call(&mut self, tool: &str, arguments: Value) -> Result<Value, LinkError> {
        self.next_id += 1;
        let request = Request { id: self.next_id, tool: tool.to_string(), arguments };
        let mut text = serde_json::to_string(&request).map_err(|e| LinkError::Broken(e.to_string()))?;
        text.push('\n');
        self.out.write_all(text.as_bytes()).and_then(|()| self.out.flush()).map_err(|_| LinkError::Gone)?;
        let line = match self.answers.line_within(self.late) {
            Heard::Line(line) => line,
            Heard::End => return Err(LinkError::Gone),
            Heard::Late => return Err(LinkError::Late),
        };
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
