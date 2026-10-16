use super::*;
use crate::gui::hand::Hand;

/// CLAUDE IN THE WINDOW: the housing made 5 mm taller through the window, and the person taking it back.
pub(super) fn claude_makes_the_housing_taller(app: &mut App, problems: &mut Vec<String>) {
    {
        use crate::gui::bridge_ui::tests::{claude_says, place, reply_of, served, SOCKET};
        app.set.claude_link = qymcad_ui_state::ClaudeLink::On;
        let folder = place("user-case");
        let path = folder.file(SOCKET);
        crate::gui::bridge_ui::listen_at(path.clone());
        let extrude = app.project.timeline.iter().rev().find(|n| qymcad_doc::report::kind_of(&n.kind).starts_with("Extrude")).map(|n| n.id).expect("the housing's extrusion");
        let mut hand = Hand::new(app);
        hand.frame(Vec::new());
        let did = served(&mut hand, claude_says(&path, vec![("edit_feature", serde_json::json!({ "feature": extrude, "values": { "height": 30 } }))]));
        if reply_of(&did[0])["ok"] != serde_json::json!(true) {
            problems.push(format!("[Claude makes the housing taller] the window refused: {}", did[0]));
        }
        hand.close_window(); // the frames marked the window live: what they started is waited for, and the mark taken away
        drop(hand);
        check_all(app, "Claude made the housing taller through the window", problems);
        Hand::new(app).undo().close_window();
        check_all(app, "the person took Claude's change back", problems);
        let body = app.project.timeline.iter().rev().find_map(|n| n.kind.body()).expect("the body");
        let top = app.project.bodies.iter().find(|b| b.id == body).map_or(f64::NAN, |b| b.faces.iter().map(|f| f.centroid.z).fold(f64::NEG_INFINITY, f64::max));
        if (top - 25.0).abs() > 1e-6 {
            problems.push(format!("[the person took Claude's change back] the housing stands {top} high, not 25"));
        }
        app.set.claude_link = qymcad_ui_state::ClaudeLink::Off;
        Hand::new(app).frame(Vec::new()).close_window(); // the window closes its end of the channel
    }
}
