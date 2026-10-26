//! THE STORIES OF PRINTING, AS A PERSON ASKS THEM, through the program itself: a child process spoken to over its stdin
//! one line at a time, each call made from what the one before answered - the keys of bodies, the arguments of a way to
//! lay a body - as a client does.
//!
//! "This STL is too heavy and has holes - mend it and make it lighter"; "will this print without supports on my P1S?
//! If not, turn it"; "this part of 300 mm does not fit my printer - cut it in two". Each ends at a 3MF for the slicer.
mod check_folder;
use check_folder::CheckFolder;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

/// THE SERVER AS A CLIENT HOLDS IT: the process, the end its requests go in, the end its replies come out.
struct Server {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    next: u64,
}

impl Server {
    /// The server on a document of its own, the handshake done: left to choose, it would send the calls to a QymCAD
    /// window open on the machine running the checks.
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_qymcad-mcp")).arg("--headless").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().expect("the server starts");
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().expect("stdout is piped"));
        let mut s = Self { child, input, output, next: 0 };
        let hello = s.ask("initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "stories", "version": "0" } }));
        assert_eq!(hello["result"]["serverInfo"]["name"], "qymcad-mcp", "{hello}");
        s.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        s
    }

    fn send(&mut self, message: &Value) {
        let input = self.input.as_mut().expect("the server's input is open");
        writeln!(input, "{message}").expect("the server reads its stdin");
        input.flush().expect("the line goes out");
    }

    /// The reply to one request; every line on stdout is a JSON-RPC message.
    fn ask(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = self.next;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let mut line = String::new();
            let read = self.output.read_line(&mut line).expect("the server writes its stdout");
            assert!(read > 0, "the server ended before answering {method}");
            let message: Value = serde_json::from_str(&line).unwrap_or_else(|e| panic!("a stdout line is not JSON ({e}): {line}"));
            if message["id"] == json!(id) {
                return message;
            }
        }
    }

    /// What tool `name` answered, which must be a success.
    fn call(&mut self, name: &str, arguments: Value) -> Value {
        let reply = self.ask("tools/call", json!({ "name": name, "arguments": arguments }));
        let out = reply["result"]["structuredContent"].clone();
        assert_eq!(out["ok"], json!(true), "{name} refused: {reply}");
        out
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // its input closed, the server ends of itself
        self.input.take();
        let _ = self.child.wait();
    }
}

/// A CLOSED CYLINDER of radius 10 and height 20 round z, `around` sides, its ends fans to their middles, as ASCII STL;
/// the side triangles `left_out` are not written - each a hole of three edges.
fn cylinder_stl(around: u32, left_out: &[u32]) -> String {
    let at = |k: u32, z: f64| {
        let a = std::f64::consts::TAU * f64::from(k % around) / f64::from(around);
        [10.0 * a.cos(), 10.0 * a.sin(), z]
    };
    let mut tris: Vec<[[f64; 3]; 3]> = Vec::new();
    for k in 0..around {
        tris.push([at(k, 0.0), at(k + 1, 0.0), at(k + 1, 20.0)]);
        tris.push([at(k, 0.0), at(k + 1, 20.0), at(k, 20.0)]);
        tris.push([[0.0, 0.0, 0.0], at(k + 1, 0.0), at(k, 0.0)]);
        tris.push([[0.0, 0.0, 20.0], at(k, 20.0), at(k + 1, 20.0)]);
    }
    let mut out = String::from("solid can\n");
    for (k, t) in tris.iter().enumerate() {
        if left_out.iter().any(|&l| l as usize == k) {
            continue;
        }
        out.push_str("facet normal 0 0 0\nouter loop\n");
        for p in t {
            out.push_str(&format!("vertex {} {} {}\n", p[0], p[1], p[2]));
        }
        out.push_str("endloop\nendfacet\n");
    }
    out + "endsolid can\n"
}

fn number(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

/// US-4, A HEAVY MESH WITH HOLES: a can of 2 000 sides, 8 000 triangles, two of its side triangles missing. Read: two
/// holes, open. Mended and made lighter within 0.05 mm: closed, one piece, fewer triangles, its volume pi x 10^2 x 20
/// within 1 %; it prints, and goes out as a 3MF.
#[test]
fn a_heavy_mesh_with_holes_is_mended_made_lighter_and_written_for_the_slicer() {
    let dir = CheckFolder::new("story-heavy-mesh");
    let stl = dir.path().join("can.stl");
    // two side triangles far apart: the first of the side, and one half way round
    std::fs::write(&stl, cylinder_stl(2000, &[0, 4001])).expect("the can written");
    let mut s = Server::start();
    let mesh = s.call("import_mesh", json!({ "path": stl.to_string_lossy() }));
    let body = json!({ "body": mesh["bodies"][0]["id"] });

    let before = s.call("mesh_report", json!({ "body": body }));
    assert_eq!(before["holes"], json!(2), "{before}");
    assert_eq!(before["closed"], json!(false), "{before}");
    let triangles = number(&before["triangles"]);
    assert!(triangles > 7_990.0, "{before}");

    let mended = s.call("mesh_repair", json!({ "body": body, "fill_holes": 64, "simplify": 0.05 }));
    assert_eq!(mended["closed_holes"], json!(2), "{mended}");
    let after = s.call("mesh_report", json!({ "body": body }));
    assert_eq!(after["closed"], json!(true), "the mended can is open: {after}");
    assert_eq!(after["pieces"], json!(1), "{after}");
    assert!(number(&after["triangles"]) < triangles / 2.0, "the can is no lighter: {} of {triangles} triangles", after["triangles"]);
    let volume = std::f64::consts::PI * 100.0 * 20.0;
    assert!((number(&after["volume"]) - volume).abs() < 0.01 * volume, "the can holds {} against {volume}", after["volume"]);

    let check = s.call("check_printability", json!({ "body": body, "printer": "p1s" }));
    assert_eq!(check["verdict"], "pass", "{check}");
    let out = dir.path().join("can.3mf");
    let _ = s.call("export_mesh", json!({ "path": out.to_string_lossy(), "quality": "standard" }));
    assert!(std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0) > 0, "no 3MF at {}", out.display());
}

/// US-6, WILL IT PRINT WITHOUT SUPPORTS: a plate of 60 x 40 x 5 with a countersunk hole, stood on its end. Standing,
/// the cone of the countersink hangs over the air past 45 deg - a warning, its places named. The way ranked first lays
/// it back on its bottom, the cone opening upward: nothing hangs, it passes.
#[test]
fn a_part_that_would_need_supports_is_turned_until_it_needs_none() {
    let dir = CheckFolder::new("story-supports");
    let mut s = Server::start();
    let _ = s.call("box", json!({ "x": 60, "y": 40, "z": 5 }));
    let _ = s.call("hole", json!({ "face": "top", "diameter": 3.4, "depth": 20, "kind": "countersink", "recess_diameter": 6.5, "recess_depth": 1.6 }));
    let _ = s.call("place_on_face", json!({ "face": { "extreme": { "axis": "x", "max": true } } }));

    let standing = s.call("check_printability", json!({ "printer": "p1s" }));
    assert_eq!(standing["verdict"], "warn", "{standing}");
    assert_eq!(standing["checks"]["overhang"]["verdict"], "warn", "{standing}");
    assert!(!standing["checks"]["overhang"]["places"].as_array().is_none_or(Vec::is_empty), "the places that hang are not named: {standing}");

    let ways = s.call("suggest_orientation", json!({}));
    let first = ways["ways"][0].clone();
    assert!(number(&first["overhang"]) < 1e-6, "the way ranked first still hangs: {ways}");
    let laid = s.call("place_on_face", first["lay"].clone());
    assert!(number(&laid["size"][2]) < 5.0 + 1e-6, "laid the best way the plate stands {} high: {laid}", laid["size"][2]);

    let lying = s.call("check_printability", json!({ "printer": "p1s" }));
    assert_eq!(lying["verdict"], "pass", "{lying}");
    let out = dir.path().join("plate.3mf");
    let _ = s.call("export_mesh", json!({ "path": out.to_string_lossy(), "quality": "standard" }));
    assert!(std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0) > 0, "no 3MF at {}", out.display());
}

/// US-7, TOO BIG FOR THE PRINTER: a bar of 300 x 20 x 20 does not fit the room of 256 even turned. Cut for it with
/// joints: two pieces, each fitting, one cut holding two round pegs; each piece one sound solid that prints; both go
/// out in one 3MF.
#[test]
fn a_part_too_big_for_the_printer_is_cut_in_two_that_fit_and_hold_together() {
    let dir = CheckFolder::new("story-too-big");
    let mut s = Server::start();
    let bar = s.call("box", json!({ "x": 300, "y": 20, "z": 20 }));
    let whole = s.call("check_printability", json!({ "printer": "p1s" }));
    assert_eq!(whole["checks"]["bed"]["verdict"], "fail", "a bar of 300 fits a room of 256: {whole}");

    let cut = s.call("split_for_bed", json!({ "body": { "body": bar["body"] }, "printer": "p1s", "joints": {} }));
    let pieces = cut["pieces"].as_array().cloned().unwrap_or_default();
    assert_eq!(pieces.len(), 2, "{cut}");
    assert_eq!(cut["joints"].as_array().map(Vec::len), Some(1), "{cut}");
    assert_eq!(cut["joints"][0]["spots"].as_array().map(Vec::len), Some(2), "{cut}");
    for p in &pieces {
        let body = json!({ "body": p["body"] });
        let health = s.call("inspect", json!({ "body": body }));
        assert!(health["valid"] == json!(true) && health["solids"] == json!(1), "a piece is not one sound solid: {health}");
        let check = s.call("check_printability", json!({ "body": body, "printer": "p1s" }));
        assert_eq!(check["checks"]["bed"]["verdict"], "pass", "a piece does not fit: {check}");
    }
    let out = dir.path().join("bar.3mf");
    let _ = s.call("export_mesh", json!({ "path": out.to_string_lossy(), "quality": "standard" }));
    assert!(std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0) > 0, "no 3MF at {}", out.display());
}
