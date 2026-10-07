//! THE CHANNEL TO THE WINDOW, over a real socket in a folder of the check's own: a call reaches the window and its
//! answer comes back; the socket is its owner's alone; a second window is refused while the first lives, and a file a
//! closed window left behind is cleared; a window that does not answer in time and a window that has closed are each
//! told as such rather than waited on for ever; two clients at once are both answered; a line that is no call is
//! answered as one and the link goes on.
#![cfg(unix)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use qymcad_bridge::{Link, LinkError, Listener, Opened, Wait};
use serde_json::{json, Value};

/// A socket path of its own for each check and each run, short enough for the system's limit - in the system's
/// temporary folder, not under `target/`: a checkout shared into a virtual machine (virtiofs) refuses to hold a socket
/// at all, measured as "Invalid argument" on every bind.
fn place(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("qymcad-bridge").join(format!("{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    dir.join("mcp.sock")
}

const SOON: Wait = Wait { answer_within: Duration::from_secs(10) };

fn open(path: &std::path::Path, wait: Wait) -> Listener {
    match Listener::open(path, wait) {
        Ok(l) => l,
        Err(e) => panic!("the window's end did not open at {}: {e:?}", path.display()),
    }
}

/// THE WINDOW in a thread of its own: it answers every call with the tool's name and arguments, until told to stop.
fn answering(listener: Listener) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(20);
        while Instant::now() < until {
            match listener.next() {
                Some(call) => {
                    if call.tool == "stop" {
                        call.answer(json!({ "stopped": true }));
                        return;
                    }
                    let echo = json!({ "tool": call.tool, "arguments": call.arguments });
                    call.answer(echo);
                }
                None => std::thread::sleep(Duration::from_millis(2)),
            }
        }
    })
}

fn link(path: &std::path::Path) -> Link {
    match Link::connect(path, Duration::from_secs(10)) {
        Ok(l) => l,
        Err(e) => panic!("the link did not reach the window: {e:?}"),
    }
}

#[test]
fn a_call_reaches_the_window_and_its_answer_comes_back() {
    let path = place("call");
    let window = answering(open(&path, SOON));
    let mut l = link(&path);
    let a = l.call("fillet", json!({ "radius": 2 })).expect("the window answers");
    assert_eq!(a, json!({ "tool": "fillet", "arguments": { "radius": 2 } }));
    let b = l.call("undo", json!({})).expect("the same link carries a second call");
    assert_eq!(b["tool"], "undo");
    let _ = l.call("stop", Value::Null);
    window.join().expect("the window's thread ends");
}

#[test]
fn the_socket_is_its_owners_alone() {
    use std::os::unix::fs::PermissionsExt;
    let path = place("mode");
    let _l = open(&path, SOON);
    let mode = std::fs::metadata(&path).expect("the socket is there").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the socket is open to others: {mode:o}");
}

#[test]
fn a_second_window_is_refused_and_a_left_file_is_cleared() {
    let path = place("twice");
    let first = open(&path, SOON);
    match Listener::open(&path, SOON) {
        Err(Opened::Taken) => {}
        Err(e) => panic!("a second window was refused for the wrong reason: {e:?}"),
        Ok(_) => panic!("a second window took the socket of a living one"),
    }
    drop(first);
    assert!(!path.exists(), "a closed window left its socket behind");

    // what a window that died without closing leaves: a file nobody listens on
    std::fs::write(&path, b"").expect("a left file");
    let _again = open(&path, SOON);
}

#[test]
fn a_window_that_does_not_answer_in_time_is_told_busy() {
    let path = place("busy");
    let listener = open(&path, Wait { answer_within: Duration::from_millis(200) });
    let mut l = link(&path);
    let start = Instant::now();
    match l.call("fillet", json!({})) {
        Err(LinkError::Busy) => {}
        other => panic!("a window that never answered was not told busy: {other:?}"),
    }
    assert!(start.elapsed() < Duration::from_secs(5), "busy came after {:?}", start.elapsed());
    // the window comes free: the call it was told busy about must not happen now
    assert!(listener.next().is_none(), "a call withdrawn as busy reached the window afterwards");
}

#[test]
fn a_closed_window_is_told_gone() {
    let path = place("gone");
    let listener = open(&path, SOON);
    let mut l = link(&path);
    drop(listener);
    match l.call("fillet", json!({})) {
        Err(LinkError::Gone) => {}
        other => panic!("a closed window was not told gone: {other:?}"),
    }
    match Link::connect(&path, Duration::from_secs(1)) {
        Err(LinkError::NoWindow) => {}
        other => panic!("a link to no window was not refused as such: {:?}", other.map(|_| ())),
    }
}

/// A WINDOW THAT CLOSES WITH A CALL IN ITS QUEUE: the call is dropped unanswered, and the caller reads the window
/// gone rather than waiting out its time.
#[test]
fn a_window_closing_with_a_call_waiting_is_told_gone() {
    let path = place("closing");
    let listener = open(&path, SOON);
    let p2 = path.clone();
    let caller = std::thread::spawn(move || {
        let start = Instant::now();
        (link(&p2).call("fillet", json!({})), start.elapsed())
    });
    std::thread::sleep(Duration::from_millis(300)); // the call is in the queue by now
    drop(listener);
    let (got, took) = caller.join().expect("the caller ends");
    match got {
        Err(LinkError::Gone) => {}
        other => panic!("a call the closing window dropped was not told gone: {other:?}"),
    }
    assert!(took < Duration::from_secs(5), "the caller waited {took:?} for a closed window");
}

#[test]
fn two_clients_at_once_are_both_answered() {
    let path = place("two");
    let window = answering(open(&path, SOON));
    let p2 = path.clone();
    let other = std::thread::spawn(move || link(&p2).call("box", json!({ "x": 1 })).expect("the second client is answered"));
    let mine = link(&path).call("cylinder", json!({ "r": 1 })).expect("the first client is answered");
    assert_eq!(mine["tool"], "cylinder");
    assert_eq!(other.join().expect("the second client ends")["tool"], "box");
    let _ = link(&path).call("stop", Value::Null);
    window.join().expect("the window's thread ends");
}

#[test]
fn a_line_that_is_no_call_is_answered_and_the_link_goes_on() {
    use std::io::{BufRead, Write};
    let path = place("junk");
    let window = answering(open(&path, SOON));
    let mut s = std::os::unix::net::UnixStream::connect(&path).expect("the socket takes a client");
    s.write_all(b"this is not json\n").expect("a line goes out");
    let mut reader = std::io::BufReader::new(s.try_clone().expect("the stream clones"));
    let mut line = String::new();
    reader.read_line(&mut line).expect("an answer comes");
    let reply: Value = serde_json::from_str(&line).expect("the answer is JSON");
    assert_eq!(reply["error"]["code"], "bad-call", "{reply}");
    s.write_all(format!("{}\n", json!({ "id": 7, "tool": "box", "arguments": {} })).as_bytes()).expect("a call goes out");
    line.clear();
    reader.read_line(&mut line).expect("the call after it is answered");
    let reply: Value = serde_json::from_str(&line).expect("the answer is JSON");
    assert_eq!(reply["id"], 7, "{reply}");
    assert_eq!(reply["result"]["tool"], "box", "{reply}");
    let _ = link(&path).call("stop", Value::Null);
    window.join().expect("the window's thread ends");
}
