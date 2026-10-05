//! THE WIRE, THROUGH THE PROGRAM ITSELF: a child process fed lines on stdin, its stdout read whole. A call into
//! the handler would not see a stray line on stdout, and a stray line is what breaks a client.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

/// What the program wrote back: every stdout line parsed, and stderr for the message of a failed check.
struct Run {
    replies: Vec<Value>,
    stderr: String,
}

/// Sends the lines, closes stdin, and reads every reply.
fn run(lines: &[String]) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_qymcad-mcp")).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("the server starts");
    {
        let mut stdin = child.stdin.take().expect("stdin is piped");
        for l in lines {
            writeln!(stdin, "{l}").expect("the server reads its stdin");
        }
    }
    let out = child.wait_with_output().expect("the server ends when its input does");
    assert!(out.status.success(), "the server ended with {}", out.status);
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let replies = stdout.lines().map(|l| serde_json::from_str::<Value>(l).unwrap_or_else(|e| panic!("a stdout line is not JSON ({e}): {l}"))).collect();
    Run { replies, stderr: String::from_utf8_lossy(&out.stderr).into_owned() }
}

fn request(id: u64, method: &str, params: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
}

fn notification(method: &str) -> String {
    json!({ "jsonrpc": "2.0", "method": method }).to_string()
}

fn reply_to(replies: &[Value], id: u64) -> &Value {
    replies.iter().find(|r| r["id"] == json!(id)).unwrap_or_else(|| panic!("no reply to request {id}: {replies:?}"))
}

/// A CLIENT'S FIRST MINUTE: the handshake, the notification that ends it, a ping and the three lists. Every
/// request is answered once, the notification not at all, and every line on stdout is a JSON-RPC message.
#[test]
fn the_handshake_and_the_lists() {
    let lines = [
        request(1, "initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "probe", "version": "0" } })),
        notification("notifications/initialized"),
        request(2, "ping", json!({})),
        request(3, "tools/list", json!({})),
        request(4, "resources/list", json!({})),
        request(5, "prompts/list", json!({})),
    ];
    let run = run(&lines);
    assert_eq!(run.replies.len(), 5, "five requests, one notification - five replies: {:?}\n{}", run.replies, run.stderr);
    for r in &run.replies {
        assert_eq!(r["jsonrpc"], "2.0", "a reply without jsonrpc 2.0: {r}");
        assert!(r.get("result").is_some(), "a request the server has went unanswered: {r}");
    }
    let init = &reply_to(&run.replies, 1)["result"];
    assert_eq!(init["protocolVersion"], "2025-06-18", "a revision the server speaks is not the one it answers with: {init}");
    assert_eq!(init["serverInfo"]["name"], "qymcad-mcp");
    assert!(init["capabilities"]["tools"].is_object(), "the server does not offer tools: {init}");
    assert_eq!(reply_to(&run.replies, 2)["result"], json!({}));
    assert!(reply_to(&run.replies, 3)["result"]["tools"].is_array());
    assert!(reply_to(&run.replies, 4)["result"]["resources"].is_array());
    assert!(reply_to(&run.replies, 5)["result"]["prompts"].is_array());
}

/// A REVISION THE SERVER DOES NOT SPEAK is answered with the newest it does; the client decides whether to go on.
#[test]
fn an_unknown_revision_gets_the_newest() {
    let run = run(&[request(1, "initialize", json!({ "protocolVersion": "1999-01-01" }))]);
    assert_eq!(reply_to(&run.replies, 1)["result"]["protocolVersion"], "2025-11-25");
}

/// WHAT THE SERVER CANNOT TAKE is refused with the code JSON-RPC reserves for it, and the server goes on.
#[test]
fn what_is_not_a_request_is_refused_and_the_server_goes_on() {
    let lines = [
        request(1, "no/such/method", json!({})),
        "this is not json".to_string(),
        json!([{ "jsonrpc": "2.0", "id": 2, "method": "ping" }]).to_string(),
        json!({ "id": 3, "method": "ping" }).to_string(),
        json!({ "jsonrpc": "2.0", "id": 4 }).to_string(),
        // a reply from the client to nothing the server asked: dropped, not answered
        json!({ "jsonrpc": "2.0", "id": 9, "result": {} }).to_string(),
        request(5, "ping", json!({})),
    ];
    let run = run(&lines);
    let codes: Vec<i64> = run.replies.iter().filter_map(|r| r["error"]["code"].as_i64()).collect();
    assert_eq!(codes, [-32601, -32700, -32600, -32600, -32600], "the refusals: {:?}", run.replies);
    assert_eq!(reply_to(&run.replies, 1)["error"]["code"], -32601, "an unknown method is refused under its own id");
    assert_eq!(run.replies[1]["id"], Value::Null, "a line that is not JSON has no id to answer under");
    assert_eq!(reply_to(&run.replies, 5)["result"], json!({}), "after the refusals the server does not answer");
    assert_eq!(run.replies.len(), 6, "the stray reply was answered: {:?}", run.replies);
}
