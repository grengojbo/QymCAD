//! THE PROGRAM SENDING ITS CALLS TO THE WINDOW, against a stand-in window on a real socket that answers with the same
//! tools on a document of its own: a call in live mode lands in the window's document and not in the program's; with
//! no window a call is refused until one opens; a window lost is never replaced by the next one; a busy window and a
//! late one are each told, with what they left of the document; `--auto` takes the window when one listens and a
//! document of its own otherwise; and the command line is read as it is meant.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use qymcad_bridge::{Listener, Wait, Wake};
use qymcad_mcp::engine::{parse, Engine, Mode, Start, Window};
use qymcad_mcp::server::{answer, answer_window};
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

/// A socket of its own for each check, in the temporary folder (a checkout shared into a virtual machine refuses to
/// hold a socket).
fn place(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("qymcad-relay").join(format!("{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    dir.join("mcp.sock")
}

/// A wake nobody hears: the stand-in window asks for calls in a loop of its own.
fn quiet() -> Wake {
    std::sync::Arc::new(|| {})
}

/// THE STAND-IN WINDOW: a document of its own, answering every call the way the window does, until stopped; then it
/// hands its document back for reading. `hold` keeps the first call that many milliseconds before answering it.
struct StandIn {
    stop: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<Ctx>,
}

fn stand_in(path: &Path, hold: u64) -> StandIn {
    let listener = Listener::open(path, Wait { answer_within: Duration::from_secs(10) }, quiet()).unwrap_or_else(|e| panic!("the stand-in window did not open: {e:?}"));
    let stop = Arc::new(AtomicBool::new(false));
    let seen = stop.clone();
    let thread = std::thread::spawn(move || {
        let mut ctx = Ctx::blank();
        let mut first = true;
        while !seen.load(Ordering::Acquire) {
            match listener.next() {
                Some(call) => {
                    if first && hold > 0 {
                        std::thread::sleep(Duration::from_millis(hold));
                    }
                    first = false;
                    let reply = qymcad_tools::channel::answer(&mut ctx, &call.tool, call.arguments.clone());
                    call.answer(reply);
                }
                None => std::thread::sleep(Duration::from_millis(2)),
            }
        }
        drop(listener);
        ctx
    });
    StandIn { stop, thread }
}

impl StandIn {
    fn close(self) -> Ctx {
        self.stop.store(true, Ordering::Release);
        self.thread.join().expect("the stand-in window ends")
    }
}

fn start(mode: Mode, path: &Path, late: Duration) -> Engine {
    Engine::start(Start { mode, socket: Some(path.to_path_buf()), late })
}

fn live(path: &Path) -> Window {
    match start(Mode::Live, path, Duration::from_secs(30)) {
        Engine::Window(w) => w,
        Engine::Here(_) => panic!("--live kept a document of its own"),
    }
}

fn line(method: &str, params: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }).to_string()
}

fn call(w: &mut Window, name: &str, arguments: Value) -> Value {
    answer_window(w, &line("tools/call", json!({ "name": name, "arguments": arguments }))).expect("a request is answered")["result"]["structuredContent"].clone()
}

fn bodies(ctx: &Ctx) -> usize {
    ctx.doc.project().bodies.len()
}

#[test]
fn a_call_in_live_mode_lands_in_the_window() {
    let path = place("lands");
    let window = stand_in(&path, 0);
    let mut w = live(&path);
    let reply = call(&mut w, "box", json!({ "x": 10, "y": 10, "z": 10 }));
    assert_eq!(reply["ok"], json!(true), "{reply}");
    let read = answer_window(&mut w, &line("resources/read", json!({ "uri": "qymcad://document" }))).expect("a read is answered");
    let text = read["result"]["contents"][0]["text"].as_str().unwrap_or_else(|| panic!("no contents: {read}"));
    let doc: Value = serde_json::from_str(text).expect("the contents are JSON");
    assert_eq!(doc["bodies"].as_array().map(Vec::len), Some(1), "the window's document was not the one read: {doc}");
    let unknown = answer_window(&mut w, &line("tools/call", json!({ "name": "no_such_tool", "arguments": {} }))).expect("answered");
    assert_eq!(unknown["error"]["code"], -32602, "a tool the program does not have went to the window: {unknown}");
    let ctx = window.close();
    assert_eq!(bodies(&ctx), 1, "the box did not land in the window's document");
}

#[test]
fn with_no_window_a_call_is_refused_until_one_opens() {
    let path = place("none");
    let mut w = live(&path);
    let reply = call(&mut w, "box", json!({ "x": 10, "y": 10, "z": 10 }));
    assert_eq!(reply["error"]["code"], "no-window", "{reply}");
    assert_eq!(reply["error"]["stage"], "window", "{reply}");
    assert_eq!(reply["rolled_back"], json!(true), "nothing happened, and the answer does not say so: {reply}");
    let window = stand_in(&path, 0);
    let reply = call(&mut w, "box", json!({ "x": 10, "y": 10, "z": 10 }));
    assert_eq!(reply["ok"], json!(true), "the window that opened later was not reached: {reply}");
    assert_eq!(bodies(&window.close()), 1);
}

#[test]
fn a_window_lost_is_not_replaced() {
    let path = place("lost");
    let first = stand_in(&path, 0);
    let mut w = live(&path);
    assert_eq!(call(&mut w, "box", json!({ "x": 10, "y": 10, "z": 10 }))["ok"], json!(true));
    let _ = first.close();
    let reply = call(&mut w, "box", json!({ "x": 5, "y": 5, "z": 5 }));
    assert_eq!(reply["error"]["code"], "window-gone", "{reply}");
    let second = stand_in(&path, 0);
    let reply = call(&mut w, "box", json!({ "x": 5, "y": 5, "z": 5 }));
    assert_eq!(reply["error"]["code"], "window-gone", "a window opened later took the lost one's place: {reply}");
    assert_eq!(bodies(&second.close()), 0, "a call reached the second window");
}

#[test]
fn a_busy_window_is_told_and_nothing_happens() {
    let path = place("busy");
    let listener = Listener::open(&path, Wait { answer_within: Duration::from_millis(200) }, quiet()).unwrap_or_else(|e| panic!("{e:?}"));
    let mut w = live(&path);
    let reply = call(&mut w, "box", json!({ "x": 10, "y": 10, "z": 10 }));
    assert_eq!(reply["error"]["code"], "window-busy", "{reply}");
    assert_eq!(reply["rolled_back"], json!(true), "a withdrawn call is not told as having left the document alone: {reply}");
    assert!(listener.next().is_none(), "the withdrawn call reached the window");
}

#[test]
fn a_late_window_is_told_and_the_next_call_reaches_it_again() {
    let path = place("late");
    let window = stand_in(&path, 1500);
    let mut w = match start(Mode::Live, &path, Duration::from_millis(300)) {
        Engine::Window(w) => w,
        Engine::Here(_) => panic!("--live kept a document of its own"),
    };
    let reply = call(&mut w, "box", json!({ "x": 10, "y": 10, "z": 10 }));
    assert_eq!(reply["error"]["code"], "window-late", "{reply}");
    assert!(reply["rolled_back"].is_null(), "a late call is told as settled: {reply}");
    std::thread::sleep(Duration::from_millis(1500));
    let reply = call(&mut w, "cylinder", json!({ "radius": 5, "height": 5 }));
    assert_eq!(reply["ok"], json!(true), "the same window was not reached after a late answer: {reply}");
    assert_eq!(reply["op"], "cylinder", "the late call's answer was read as the next call's: {reply}");
    let ctx = window.close();
    let kinds: Vec<String> = ctx.doc.project().timeline.iter().map(|n| qymcad_doc::report::kind_of(&n.kind)).collect();
    assert!(kinds.iter().any(|k| k.starts_with("Box")) && kinds.iter().any(|k| k == "Cylinder"), "the late box and the cylinder are not both in the window: {kinds:?}");
}

#[test]
fn auto_takes_the_window_when_one_listens_and_its_own_otherwise() {
    let path = place("auto");
    match start(Mode::Auto, &path, Duration::from_secs(30)) {
        Engine::Here(mut ctx) => {
            let reply = answer(&mut ctx, &line("tools/call", json!({ "name": "box", "arguments": { "x": 1, "y": 1, "z": 1 } }))).expect("answered");
            assert_eq!(reply["result"]["structuredContent"]["ok"], json!(true), "{reply}");
        }
        Engine::Window(_) => panic!("--auto took a window where none listens"),
    }
    let window = stand_in(&path, 0);
    match start(Mode::Auto, &path, Duration::from_secs(30)) {
        Engine::Window(mut w) => assert_eq!(call(&mut w, "box", json!({ "x": 1, "y": 1, "z": 1 }))["ok"], json!(true)),
        Engine::Here(_) => panic!("--auto kept a document of its own where a window listens"),
    }
    assert_eq!(bodies(&window.close()), 1);
}

#[test]
fn the_command_line_is_read_as_it_is_meant() {
    struct Case {
        args: &'static [&'static str],
        mode: Option<Mode>,
        socket: Option<&'static str>,
    }
    let cases = [
        Case { args: &[], mode: Some(Mode::Auto), socket: None },
        Case { args: &["--live"], mode: Some(Mode::Live), socket: None },
        Case { args: &["--headless", "--socket", "/tmp/q.sock"], mode: Some(Mode::Headless), socket: Some("/tmp/q.sock") },
        Case { args: &["--auto", "--auto"], mode: Some(Mode::Auto), socket: None },
        Case { args: &["--live", "--headless"], mode: None, socket: None },
        Case { args: &["--socket"], mode: None, socket: None },
        Case { args: &["--verbose"], mode: None, socket: None },
    ];
    for c in cases {
        let args: Vec<String> = c.args.iter().map(|s| s.to_string()).collect();
        match (parse(&args), c.mode) {
            (Ok(s), Some(m)) => {
                assert_eq!(s.mode, m, "{:?}", c.args);
                assert_eq!(s.socket.as_deref(), c.socket.map(Path::new), "{:?}", c.args);
            }
            (Err(_), None) => {}
            (got, want) => panic!("{:?} read as {got:?}, wanted {want:?}", c.args),
        }
    }
}
