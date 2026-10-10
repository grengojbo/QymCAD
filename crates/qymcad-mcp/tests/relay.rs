//! THE PROGRAM SENDING ITS CALLS TO THE WINDOW, against a stand-in window on a real socket that answers with the same
//! tools on a document of its own: a call in live mode lands in the window's document and not in the program's; with
//! no window a call is refused until one opens; a window opened again is reached after a warning; a busy window and a
//! late one are each told, with what they left of the document; `--auto` takes the window when one listens and a
//! document of its own otherwise; and the command line is read as it is meant.
#![cfg(unix)]
mod check_folder;
use check_folder::CheckFolder;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use qymcad_bridge::{Listener, Wait, Wake};
use qymcad_mcp::engine::{parse, Engine, Mode, Start, Window};
use qymcad_mcp::server::{answer_engine, answer_window};
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

/// The folder of a socket of its own for each check, in the temporary folder (a checkout shared into a virtual machine
/// refuses to hold a socket). The socket is `SOCKET` in it.
fn place(name: &str) -> CheckFolder {
    CheckFolder::new(&format!("relay-{name}"))
}

/// The name of the socket in its folder.
const SOCKET: &str = "mcp.sock";

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
        _ => panic!("--live did not go to the window"),
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
    let folder = place("lands");
    let path = folder.path().join(SOCKET);
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
    let folder = place("none");
    let path = folder.path().join(SOCKET);
    let mut w = live(&path);
    let reply = call(&mut w, "box", json!({ "x": 10, "y": 10, "z": 10 }));
    assert_eq!(reply["error"]["code"], "no-window", "{reply}");
    assert_eq!(reply["error"]["stage"], "window", "{reply}");
    assert_eq!(reply["rolled_back"], json!(true), "nothing happened, and the answer does not say so: {reply}");
    let switch = qymcad_i18n::tr_in(&qymcad_tools::person::language(), "settings-claude").expect("the switch in the person's language");
    let hint = reply["error"]["hint"].as_str().unwrap_or_default();
    assert!(hint.contains(&switch) && !hint.contains("Help ->"), "the hint does not lead to the switch {switch:?}: {hint}");
    let window = stand_in(&path, 0);
    let reply = call(&mut w, "box", json!({ "x": 10, "y": 10, "z": 10 }));
    assert_eq!(reply["ok"], json!(true), "the window that opened later was not reached: {reply}");
    assert_eq!(bodies(&window.close()), 1);
}

/// A WINDOW OPENED AGAIN IS REACHED, AFTER A WARNING. The window this session worked with closed; one opened later may
/// hold another document. Its first call is refused untouched with `window-new`, so the model reads the document
/// before changing it; the call after goes through - no restart of Claude.
#[test]
fn a_window_opened_again_is_reached_after_a_warning() {
    let folder = place("lost");
    let path = folder.path().join(SOCKET);
    let first = stand_in(&path, 0);
    let mut w = live(&path);
    assert_eq!(call(&mut w, "box", json!({ "x": 10, "y": 10, "z": 10 }))["ok"], json!(true));
    let _ = first.close();
    let reply = call(&mut w, "box", json!({ "x": 5, "y": 5, "z": 5 }));
    assert_eq!(reply["error"]["code"], "window-gone", "{reply}");
    let second = stand_in(&path, 0);
    let reply = call(&mut w, "box", json!({ "x": 5, "y": 5, "z": 5 }));
    assert_eq!(reply["error"]["code"], "window-new", "a window opened later took the lost one's place unannounced: {reply}");
    assert_eq!(reply["rolled_back"], json!(true), "the warning does not say nothing happened: {reply}");
    let reply = call(&mut w, "cylinder", json!({ "radius": 2, "height": 2 }));
    assert_eq!(reply["ok"], json!(true), "the window opened again is not reached after the warning: {reply}");
    let ctx = second.close();
    let kinds: Vec<String> = ctx.doc.project().timeline.iter().map(|n| qymcad_doc::report::kind_of(&n.kind)).collect();
    assert_eq!(kinds, vec!["Cylinder".to_string()], "the second window holds {kinds:?}: the warned call went through, or the next one did not");
}

#[test]
fn a_busy_window_is_told_and_nothing_happens() {
    let folder = place("busy");
    let path = folder.path().join(SOCKET);
    let listener = Listener::open(&path, Wait { answer_within: Duration::from_millis(200) }, quiet()).unwrap_or_else(|e| panic!("{e:?}"));
    let mut w = live(&path);
    let reply = call(&mut w, "box", json!({ "x": 10, "y": 10, "z": 10 }));
    assert_eq!(reply["error"]["code"], "window-busy", "{reply}");
    assert_eq!(reply["rolled_back"], json!(true), "a withdrawn call is not told as having left the document alone: {reply}");
    assert!(listener.next().is_none(), "the withdrawn call reached the window");
}

#[test]
fn a_late_window_is_told_and_the_next_call_reaches_it_again() {
    let folder = place("late");
    let path = folder.path().join(SOCKET);
    let window = stand_in(&path, 1500);
    let mut w = match start(Mode::Live, &path, Duration::from_millis(300)) {
        Engine::Window(w) => w,
        _ => panic!("--live did not go to the window"),
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

fn box_through(engine: &mut Engine) -> Value {
    answer_engine(engine, &line("tools/call", json!({ "name": "box", "arguments": { "x": 1, "y": 1, "z": 1 } }))).expect("answered")["result"]["structuredContent"].clone()
}

/// `--auto` TAKES THE WINDOW OPENED AFTER THE PROGRAM STARTED. Claude Desktop starts the program when it starts itself,
/// mostly before QymCAD is opened: the handshake and the lists come first and decide nothing, and the first call that
/// reaches a document goes to the window open by then.
#[test]
fn auto_takes_a_window_opened_after_the_start() {
    let folder = place("auto-late");
    let path = folder.path().join(SOCKET);
    let mut engine = start(Mode::Auto, &path, Duration::from_secs(30));
    let hello = answer_engine(&mut engine, &line("initialize", json!({ "protocolVersion": "2025-06-18" }))).expect("answered");
    assert!(hello["result"]["serverInfo"].is_object(), "{hello}");
    let _ = answer_engine(&mut engine, &line("tools/list", json!({}))).expect("answered");
    let window = stand_in(&path, 0);
    assert_eq!(box_through(&mut engine)["ok"], json!(true));
    assert!(matches!(engine, Engine::Window(_)), "the first call did not take the window that was open by then");
    assert_eq!(bodies(&window.close()), 1, "the box did not land in the window");
}

/// `--auto` WAITS FOR THE WINDOW WHILE ITS OWN DOCUMENT IS UNTOUCHED. Reported behaviour: the program started with the
/// switch off, the model asked what was selected, was told there was no window, the person switched it on - and every
/// call after went to the program's own empty document until Claude was restarted. A reading changes nothing, so the
/// next call still looks for the window and finds it.
#[test]
fn auto_reaches_a_window_switched_on_after_a_reading() {
    let folder = place("auto-read-first");
    let path = folder.path().join(SOCKET);
    let mut engine = start(Mode::Auto, &path, Duration::from_secs(30));
    let read = answer_engine(&mut engine, &line("tools/call", json!({ "name": "get_selection", "arguments": {} }))).expect("answered")["result"]["structuredContent"].clone();
    assert_eq!(read["error"]["code"], "no-window", "{read}");
    let doc = answer_engine(&mut engine, &line("tools/call", json!({ "name": "get_document", "arguments": {} }))).expect("answered")["result"]["structuredContent"].clone();
    assert_eq!(doc["ok"], json!(true), "the own document was not read: {doc}");
    let window = stand_in(&path, 0);
    assert_eq!(box_through(&mut engine)["ok"], json!(true));
    assert!(matches!(engine, Engine::Window(_)), "the window switched on after two readings was not taken");
    assert_eq!(bodies(&window.close()), 1, "the box did not land in the window");
}

/// `--auto` WITH NO WINDOW AT THE FIRST CHANGE keeps a document of its own for the run: the model's work is there, a
/// window opened afterwards may hold another document, and the session does not move to it unawares.
#[test]
fn auto_with_no_window_at_the_first_change_keeps_its_own() {
    let folder = place("auto-own");
    let path = folder.path().join(SOCKET);
    let mut engine = start(Mode::Auto, &path, Duration::from_secs(30));
    assert_eq!(box_through(&mut engine)["ok"], json!(true));
    assert!(matches!(engine, Engine::Here(_)), "--auto with no window did not keep a document of its own");
    let window = stand_in(&path, 0);
    assert_eq!(box_through(&mut engine)["ok"], json!(true));
    assert_eq!(bodies(&window.close()), 0, "the session moved to a window opened after it had decided");
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
