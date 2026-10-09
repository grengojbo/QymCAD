//! CLAUDE BUILDS IN THE OPEN WINDOW - A CHAIN, NOT ONE CALL.
//!
//! One call through the window can be right while a chain of them goes wrong: a step leaves the window's caches behind,
//! the next one reads them. So Claude builds a plate here the way it builds one for a person - a parameter, a sketch,
//! an extrusion that follows the parameter, a rounding, a hole - the person clicks the rounding and Claude makes it
//! 0.2 mm smaller, the parameter is changed in the middle, and the person takes the last step back. After EVERY step the
//! end-to-end check of the window runs (`user_case::check_all`) and the window's own document is measured.
#[cfg(test)]
mod tests {
    use super::super::bridge_ui::tests::{aim_and_click, claude_says, on_the_rounding, place, reply_of, served};
    use super::super::hand::Hand;
    use super::super::user_case::tests::check_all;
    use super::super::App;
    use qymcad_core::model::Id;
    use serde_json::{json, Value};

    /// One call of Claude through the window, whole frames turning until it is answered; the window closed after, so
    /// the check that follows works on a settled document.
    fn claude(app: &mut App, path: &std::path::Path, tool: &'static str, arguments: Value, problems: &mut Vec<String>) -> Value {
        let mut hand = Hand::new(app);
        let answer = served(&mut hand, claude_says(path, vec![(tool, arguments)])).remove(0);
        hand.close_window();
        let reply = reply_of(&answer).clone();
        if reply["ok"] != json!(true) {
            problems.push(format!("[{tool}] the window refused: {reply}"));
        }
        reply
    }

    /// The highest point of `body` in the window's document.
    fn top(app: &App, body: Id) -> f64 {
        app.project.bodies.iter().find(|b| b.id == body).map_or(f64::NAN, |b| b.faces.iter().map(|f| f.centroid.z).fold(f64::NEG_INFINITY, f64::max))
    }

    #[test]
    fn claude_builds_a_plate_in_the_window_and_the_person_changes_it() {
        let mut problems: Vec<String> = Vec::new();
        let mut app = App::default();
        app.set.claude_link = qymcad_ui_state::ClaudeLink::On;
        let path = place("live-chain");
        super::super::bridge_ui::listen_at(path.clone());
        Hand::new(&mut app).frame(Vec::new()).close_window(); // the window opens its end of the channel
        let steps = app.disk.edits.undo.len();

        let _ = claude(&mut app, &path, "set_parameter", json!({ "name": "t", "expr": "4" }), &mut problems);
        check_all(&mut app, "Claude set the thickness t = 4", &mut problems);

        let sketch = claude(&mut app, &path, "create_sketch", json!({ "plane": "xy" }), &mut problems)["sketch"].clone();
        check_all(&mut app, "Claude started a sketch", &mut problems);
        let _ = claude(&mut app, &path, "sketch_add", json!({ "sketch": sketch, "entities": [{ "rect": { "from": [0, 0], "to": [60, 40] } }] }), &mut problems);
        check_all(&mut app, "Claude drew the rectangle", &mut problems);

        let laid = claude(&mut app, &path, "extrude", json!({ "sketch": sketch, "distance": "t" }), &mut problems);
        let body = laid["body"].as_u64().unwrap_or_else(|| panic!("the extrusion names no body: {laid}"));
        check_all(&mut app, "Claude extruded the plate t high", &mut problems);
        assert!((top(&app, body) - 4.0).abs() < 1e-6, "the plate in the window stands {} high, not t = 4", top(&app, body));

        let _ = claude(&mut app, &path, "fillet", json!({ "edges": { "adjacent": "top" }, "radius": 1 }), &mut problems);
        check_all(&mut app, "Claude rounded the top edges", &mut problems);
        let rounded = app.project.timeline.iter().rev().find_map(|n| n.kind.body()).expect("the rounded body");
        let _ = claude(&mut app, &path, "hole", json!({ "body": { "body": rounded }, "face": "top", "diameter": 5, "depth": 10 }), &mut problems);
        check_all(&mut app, "Claude drilled a hole in the top", &mut problems);

        // THE PERSON POINTS AT THE ROUNDING, Claude reads what that is and makes it 0.2 mm smaller
        let fillet = app
            .project
            .timeline
            .iter()
            .find(|n| qymcad_doc::report::kind_of(&n.kind) == "Fillet")
            .map(|n| n.id)
            .unwrap_or_else(|| panic!("no rounding in the window; the chain so far: {problems:#?}"));
        let shown = app.project.timeline.iter().rev().find_map(|n| n.kind.body()).expect("the body shown");
        let at = on_the_rounding(&app, fillet, shown);
        let mut hand = Hand::new(&mut app);
        aim_and_click(&mut hand, shown, at, 60.0);
        hand.close_window();
        let seen = claude(&mut app, &path, "get_selection", json!({}), &mut problems);
        let by = &seen["selected"][0]["made_by"];
        assert_eq!(by["feature"], json!(fillet), "the click on the rounding is not read as the rounding: {seen}");
        let radius = by["sizes"].as_array().and_then(|s| s.iter().find(|s| s["key"] == "radius")).and_then(|s| s["value"].as_f64()).expect("the rounding's radius");
        let _ = claude(&mut app, &path, "edit_feature", json!({ "feature": fillet, "values": { "radius": radius - 0.2 } }), &mut problems);
        check_all(&mut app, "Claude made the selected rounding 0.2 mm smaller", &mut problems);

        // THE PARAMETER CHANGED IN THE MIDDLE: everything after it is built again
        let _ = claude(&mut app, &path, "set_parameter", json!({ "name": "t", "expr": "6" }), &mut problems);
        check_all(&mut app, "Claude made the plate t = 6", &mut problems);
        let shown = app.project.timeline.iter().rev().find_map(|n| n.kind.body()).expect("the body shown");
        assert!((top(&app, shown) - 6.0).abs() < 1e-6, "after t = 6 the plate stands {} high", top(&app, shown));

        // every change of Claude's is one step of its own, named for it; the reading of the selection is none
        let claude_steps = app.disk.edits.undo.len() - steps;
        assert_eq!(claude_steps, 8, "eight changes made {claude_steps} steps of undo");

        // THE PERSON TAKES THE LAST STEP BACK
        Hand::new(&mut app).undo().close_window();
        check_all(&mut app, "the person took the last step back", &mut problems);
        let shown = app.project.timeline.iter().rev().find_map(|n| n.kind.body()).expect("the body shown");
        assert!((top(&app, shown) - 4.0).abs() < 1e-6, "Ctrl+Z did not take the plate back to t = 4: it stands {} high", top(&app, shown));

        assert!(problems.is_empty(), "THE LIVE CHAIN FOUND {} problems:\n  {}", problems.len(), problems.join("\n  "));
    }
}
