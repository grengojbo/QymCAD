//! EVERY REQUEST IS ANSWERED IN TIME. A client gives up on a request it waited 60 s for, and the calls that reach a
//! document are done one at a time: a `mesh_report` of 72.5 s once kept `get_document`, asked meanwhile, unanswered as
//! well, and to the client the server hung.
//!
//! So the lines are read on a thread of their own, and a request that reaches no document - the handshake, a ping, the
//! lists - is answered there at once. The calls that reach a document wait in line for the one thread that holds the
//! document, in the order they came. A call not done within `answer_within` of its coming is answered there and then:
//! one still being done is told `still-running` and goes on, one not yet begun is withdrawn, told `not-started`. The
//! answer of a call told `still-running` is kept, and the same call asked again gets it instead of being done twice;
//! any other call drops it.

use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::engine::{Cut, Engine};
use crate::rpc::{self, Incoming};
use crate::server::{answer_alone, answer_engine, answer_refused};
use qymcad_tools::tool::{After, Refusal, Stage};

/// HOW LONG A REQUEST WAITS FOR ITS ANSWER at most: 15 s short of the 60 s a client waits, so that the answer is
/// read and not cut off.
pub const ANSWER_WITHIN: Duration = Duration::from_secs(45);

/// A call that reaches a document, waiting in line.
struct Job {
    id: Value,
    line: String,
    asked: Value,
    came: Instant,
}

/// Whether the client has been answered for the call being done.
#[derive(Clone, Copy, PartialEq)]
enum Told {
    Not,
    StillRunning,
}

/// The call being done.
struct Running {
    job: Job,
    told: Told,
}

/// How far the run has got.
#[derive(Clone, Copy, PartialEq)]
enum End {
    /// Lines may still come.
    Open,
    /// The input has ended: the calls in line are still done.
    Closed,
    /// Every call is done and answered.
    Done,
}

/// The answer of a call the client was told is still running, and what was asked of it.
struct Kept {
    asked: Value,
    reply: Value,
}

/// The call taken out of line: its line, its id, and the answer kept for it when it is a call asked again.
struct Turn {
    line: String,
    id: Value,
    again: Option<Value>,
}

/// What the three threads share.
struct State {
    waiting: VecDeque<Job>,
    running: Option<Running>,
    end: End,
    broken: Option<std::io::Error>,
}

struct Shared<W> {
    state: Mutex<State>,
    moved: Condvar,
    out: Mutex<W>,
}

impl<W: Write> Shared<W> {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Writes one reply; the first fault of the wire is kept for the end and closes the run.
    fn send(&self, reply: &Value) {
        let wrote = {
            let mut out = self.out.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            serde_json::to_writer(&mut *out, reply).map_err(std::io::Error::from).and_then(|()| out.write_all(b"\n")).and_then(|()| out.flush())
        };
        if let Err(e) = wrote {
            let mut state = self.state();
            state.broken.get_or_insert(e);
            if state.end == End::Open {
                state.end = End::Closed;
            }
            self.moved.notify_all();
        }
    }
}

/// Answers every line of `input` on `output` until the input ends and every call is answered, the calls going where
/// `engine` sends them, each answered within `answer_within` of its coming.
pub fn serve(engine: Engine, input: impl BufRead + Send, output: impl Write + Send, answer_within: Duration) -> std::io::Result<()> {
    let shared = Shared { state: Mutex::new(State { waiting: VecDeque::new(), running: None, end: End::Open, broken: None }), moved: Condvar::new(), out: Mutex::new(output) };
    std::thread::scope(|scope| {
        scope.spawn(|| read(&shared, input));
        scope.spawn(|| watch(&shared, answer_within));
        work(&shared, engine);
    });
    shared.state.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner).broken.map_or(Ok(()), Err)
}

/// THE READER: answers what reaches no document, puts the rest in line, and closes the line when the input ends.
fn read<W: Write>(shared: &Shared<W>, mut input: impl BufRead) {
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match input.read_until(b'\n', &mut bytes) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                shared.state().broken.get_or_insert(e);
                break;
            }
        }
        if shared.state().end != End::Open {
            break;
        }
        // bytes that are not UTF-8 are still a line, and the parse answers it as one that is not JSON
        let line = String::from_utf8_lossy(&bytes).into_owned();
        if line.trim().is_empty() {
            continue;
        }
        match rpc::parse(&line) {
            Incoming::Request { id, method, params } if matches!(method.as_str(), "tools/call" | "resources/read") => {
                let asked = json!({ "method": method, "params": without_meta(params) });
                shared.state().waiting.push_back(Job { id, line, asked, came: Instant::now() });
                shared.moved.notify_all();
            }
            _ => {
                if let Some(reply) = answer_alone(&line) {
                    shared.send(&reply);
                }
            }
        }
    }
    let mut state = shared.state();
    if state.end == End::Open {
        state.end = End::Closed;
    }
    shared.moved.notify_all();
}

/// THE ONE THAT HOLDS THE DOCUMENT: does the calls in line one by one, until the line is closed and empty.
fn work<W: Write>(shared: &Shared<W>, mut engine: Engine) {
    let mut kept: Option<Kept> = None;
    loop {
        let Turn { line, id, again } = {
            let mut state = shared.state();
            let job = loop {
                if let Some(job) = state.waiting.pop_front() {
                    break job;
                }
                if state.end != End::Open {
                    state.end = End::Done;
                    shared.moved.notify_all();
                    return;
                }
                state = shared.moved.wait(state).unwrap_or_else(std::sync::PoisonError::into_inner);
            };
            let again = kept.take().filter(|k| k.asked == job.asked).map(|k| k.reply);
            let turn = Turn { line: job.line.clone(), id: job.id.clone(), again };
            state.running = Some(Running { job, told: Told::Not });
            turn
        };
        shared.moved.notify_all();
        let reply = match again {
            Some(mut reply) => {
                reply["id"] = id;
                Some(reply)
            }
            None => answer_engine(&mut engine, &line),
        };
        let Some(done) = shared.state().running.take() else { continue };
        shared.moved.notify_all();
        let Some(reply) = reply else { continue };
        match done.told {
            Told::Not => shared.send(&reply),
            Told::StillRunning => kept = Some(Kept { asked: done.job.asked, reply }),
        }
    }
}

/// THE WATCH: answers every call not done within `answer_within` of its coming, until every call is answered.
fn watch<W: Write>(shared: &Shared<W>, answer_within: Duration) {
    let mut state = shared.state();
    while state.end != End::Done {
        let now = Instant::now();
        let mut owed = Vec::new();
        if let Some(running) = state.running.as_mut().filter(|r| r.told == Told::Not && now >= r.job.came + answer_within) {
            running.told = Told::StillRunning;
            owed.push(answer_refused(&running.job.line, still_running(answer_within)));
        }
        while state.waiting.front().is_some_and(|j| now >= j.came + answer_within) {
            let Some(job) = state.waiting.pop_front() else { break };
            // the same call asked again while it is still being done is told so, not told it waited behind another
            let cut = if state.running.as_ref().is_some_and(|r| r.job.asked == job.asked) { still_running(answer_within) } else { not_started(answer_within) };
            owed.push(answer_refused(&job.line, cut));
        }
        let next = state.running.iter().filter(|r| r.told == Told::Not).map(|r| r.job.came).chain(state.waiting.front().map(|j| j.came)).min().map(|came| came + answer_within);
        drop(state);
        for reply in owed.into_iter().flatten() {
            shared.send(&reply);
        }
        state = shared.state();
        if state.end == End::Done {
            break;
        }
        state = match next {
            Some(at) => shared.moved.wait_timeout(state, at.saturating_duration_since(Instant::now())).unwrap_or_else(std::sync::PoisonError::into_inner).0,
            None => shared.moved.wait(state).unwrap_or_else(std::sync::PoisonError::into_inner),
        };
    }
}

/// What a call asked, without what the client says about the request itself: a call asked again carries a progress
/// token of its own.
fn without_meta(mut params: Value) -> Value {
    if let Some(p) = params.as_object_mut() {
        p.remove("_meta");
    }
    params
}

fn still_running(within: Duration) -> Cut {
    let refusal = Refusal::new("still-running", &format!("The call is still being done after {} s, and goes on; the calls that reach a document wait for it.", within.as_secs()), Stage::Time)
        .with_hint("Make the same call again, with the same arguments, to get its answer once it is done: it is not done twice. Any other call drops that answer.");
    Cut { refusal, after: After::Unknown }
}

fn not_started(within: Duration) -> Cut {
    let refusal = Refusal::new("not-started", &format!("The call waited {} s behind another one still being done, and was withdrawn without being done.", within.as_secs()), Stage::Time)
        .with_hint("Call again in a moment.");
    Cut { refusal, after: After::Untouched }
}
