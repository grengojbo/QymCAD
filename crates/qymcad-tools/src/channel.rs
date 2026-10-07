//! WHAT A DOCUMENT HELD ELSEWHERE ANSWERS A CALL WITH. The program passes the model's calls over the window's channel
//! by name; the holder of the document answers each the way the program would answer it itself: a tool as the
//! protocol answers a tool, a read of an address as its contents.

use serde_json::Value;

use crate::tool::{self, After, Ctx, Refusal, Stage};

/// The name a read of an address travels under: no tool has a slash in its name.
pub const READ: &str = "resources/read";

/// ANSWER ONE CALL that came over the channel, on `ctx`.
pub fn answer(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    if name == READ {
        let uri = arguments.get("uri").and_then(Value::as_str).unwrap_or_default();
        return crate::reading::read_as_answer(ctx, uri);
    }
    match tool::find(name) {
        Some(found) => tool::call(ctx, found, arguments),
        None => tool::refused_reply(name, &Refusal::new("no-tool", &format!("There is no tool {name}."), Stage::Validate), After::Untouched),
    }
}
