//! A SIZE IS SET BETWEEN TWO FACES BY HAND: "Size on body" taken, the top of a block clicked, the view turned, its
//! bottom clicked - the field stands at the distance between them - a size typed and Enter: one node, the block that
//! tall, the face the bar names moved.
#[cfg(test)]
mod tests {
    use crate::gui::hand::Hand;
    use crate::gui::App;

    /// The block of 20 x 20 x 10 in a part of its own, the part entered.
    fn block() -> App {
        let mut app = App::default();
        super::super::joint_flow::tests::add_part_at(&mut app, 0.0);
        qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
        let body = app.project.mesh_id(0).expect("the block");
        if let Some(owner) = app.project.body_owner(body) {
            app.enter_component(owner);
        }
        app
    }

    #[test]
    fn the_top_and_the_bottom_of_a_block_are_made_fifteen_apart() {
        let mut app = block();
        let mut h = Hand::new(&mut app);
        h.look_at([10.0, 10.0, 5.0], 6.0).frame(Vec::new());
        h.tool(qymcad_ui_state::PartTool::FaceGap).click([10.0, 10.0, 10.0]);
        h.frame(Vec::new());
        assert_eq!(h.app.params.recognise.gap.len(), 1, "the top was not taken: {}", h.app.status);
        h.look_from_below();
        h.click([10.0, 10.0, 0.0]).frame(Vec::new());
        let field = h.app.tools.cmd.params.iter().find(|p| p.key == "gap").map(|p| p.val);
        assert_eq!(field, Some(10.0), "the field does not stand at the distance between the faces: {}", h.app.status);
        let nodes = h.app.project.timeline.len();
        h.set("gap", 15.0);
        h.enter();
        // the window rebuilds in the background: frames run, as they do by themselves, until nothing is being built
        for _ in 0..300 {
            h.frame(Vec::new());
            h.app.drain_bg_for_test();
            if h.app.regen.busy.is_none() && h.app.regen.bg.is_empty() && !h.app.regen.wanted {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(h.app.project.timeline.len(), nodes + 1, "Enter laid no node: {}", h.app.status);
        let node = h.app.project.timeline.last().expect("the node");
        let built = node.kind.body().and_then(|b| h.app.live.shapes.get(&b)).map(|s| s.volume());
        let said = h.app.project.regen_errors.get(&node.id).map(|e| format!("{e:?}"));
        assert!(built.is_some_and(|v| (v - 20.0 * 20.0 * 15.0).abs() < 1e-6), "the block made 15 tall holds {built:?} mm^3, not 6000; the node says {said:?}");
        let row = super::super::panels_tree::feature_row_label(&h.app.project, h.app.project.timeline.len() - 1);
        assert!(row.contains("15"), "the row of the size does not name it: {row:?}");
    }
}
