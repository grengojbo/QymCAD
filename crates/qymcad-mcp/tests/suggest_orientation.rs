//! THE BEST WAY TO LAY A BODY, THROUGH THE SERVER: a T standing on its stem hangs the two arms of its bar, 200 mm^2, as
//! it lies. Laid on its side, the T's own outline of 350 mm^2 on the bed, it hangs nothing and stands 10 tall: that is
//! told first, before the top of its bar (300 mm^2, 25 tall). Laid so by place_on_face with the arguments the way holds,
//! it prints with nothing hanging.

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone()
}

fn ok(reply: Value) -> Value {
    assert_eq!(reply["ok"], json!(true), "{reply}");
    reply
}

fn near(v: &Value, want: f64) -> bool {
    v.as_f64().is_some_and(|x| (x - want).abs() < 1e-3)
}

#[test]
fn a_t_on_its_stem_is_best_laid_on_its_side() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 10, "y": 10, "z": 20 })));
    let _ = ok(call(&mut ctx, "box", json!({ "x": 30, "y": 10, "z": 5, "at": [0, 0, 20] })));
    let r = ok(call(&mut ctx, "suggest_orientation", json!({})));
    assert!(near(&r["as_it_lies"]["overhang"], 200.0), "{r}");
    let best = &r["ways"][0];
    assert!(near(&best["overhang"], 0.0) && near(&best["contact"], 350.0) && near(&best["height"], 10.0), "the first way is not the T on its side: {r}");
    assert!(near(&best["down"][1], 1.0) || near(&best["down"][1], -1.0), "{r}");
    let bar = r["ways"].as_array().expect("the ways").iter().find(|w| near(&w["down"][2], 1.0)).cloned().expect("the top of the bar is one of the ways");
    assert!(near(&bar["contact"], 300.0) && near(&bar["overhang"], 0.0), "{r}");
    let _ = ok(call(&mut ctx, "place_on_face", best["lay"].clone()));
    let check = ok(call(&mut ctx, "check_printability", json!({ "printer": "p1s" })));
    assert_eq!(check["checks"]["overhang"]["verdict"], "pass", "laid the way told first it still hangs: {check}");
}
