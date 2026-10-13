//! A FACE BEING PUSHED IS PREVIEWED BY ITS RIM: the dashed outline where the face will stand runs along its edges and the
//! rims of its holes, never across it. A face is drawn as triangles, and the diagonals they are cut into are no edge of
//! the body.
//!
//! Reported behaviour: the top of a drilled plate, a fan of 102 triangles, pushed with the preview drawn triangle by
//! triangle, read as a star left over from a recognition.
#[cfg(test)]
mod tests {
    use crate::gui::hand::Hand;
    use crate::gui::App;

    fn on_screen(app: &App, p: [f64; 3]) -> egui::Pos2 {
        let basis = app.viewing.cam.basis();
        qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect: app.viewing.view_rect, basis: &basis }.at(p).0
    }

    /// How far point `p` is from the piece of line `a`-`b`, on the screen.
    fn off_line(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
        let ab = b - a;
        let t = if ab.length_sq() < 1e-6 { 0.0 } else { ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0) };
        (p - (a + ab * t)).length()
    }

    #[test]
    fn the_top_of_a_block_pushed_is_previewed_by_its_four_edges() {
        let mut app = App::default();
        super::super::joint_flow::tests::add_part_at(&mut app, 0.0);
        qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
        let body = app.project.mesh_id(0).expect("the block");
        if let Some(owner) = app.project.body_owner(body) {
            app.enter_component(owner);
        }
        let mut h = Hand::new(&mut app);
        h.look_at([10.0, 10.0, 5.0], 6.0).frame(Vec::new());
        h.tool(25).click([10.0, 10.0, 10.0]);
        h.frame(Vec::new());
        assert!(!h.app.tools.gsel.faces.is_empty(), "setup: the top face was not taken: {}", h.app.status);
        h.set("dist", 5.0).frame(Vec::new());
        let ok = h.app.scheme.pal.ok();
        let dashes = h.segments_in(ok);
        assert!(!dashes.is_empty(), "the push of 5 draws no preview");
        // the rim the top will stand on, 5 up, and the arrow from the face to it
        let rim: Vec<[egui::Pos2; 2]> = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]]
            .iter()
            .zip([[20.0, 0.0], [20.0, 20.0], [0.0, 20.0], [0.0, 0.0]].iter())
            .map(|(a, b)| [on_screen(h.app, [a[0], a[1], 15.0]), on_screen(h.app, [b[0], b[1], 15.0])])
            .chain(std::iter::once([on_screen(h.app, [10.0, 10.0, 10.0]), on_screen(h.app, [10.0, 10.0, 15.0])]))
            .collect();
        let across: Vec<egui::Pos2> = dashes.iter().map(|d| d[0] + (d[1] - d[0]) * 0.5).filter(|m| rim.iter().map(|r| off_line(*m, r[0], r[1])).fold(f32::MAX, f32::min) > 2.0).collect();
        assert!(across.is_empty(), "{} of {} dashes of the preview run across the face, off its edges: {:?}", across.len(), dashes.len(), &across[..across.len().min(5)]);
    }
}
