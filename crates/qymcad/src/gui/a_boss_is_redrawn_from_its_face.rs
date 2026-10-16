//! A BOSS IS REDRAWN FROM ITS FACE BY HAND: a plate with a boss comes in from a STEP file, "Sketch from face" is taken,
//! the top of the boss clicked - the boss lit, its height in the field - and Enter: a sketch on the top, fully defined,
//! and one node redrawing the boss by it, the body holding what it held. The tool stays in hand.
#[cfg(test)]
mod tests {
    use crate::gui::hand::Hand;
    use crate::gui::App;
    use qymcad_kernel::specimen::Specimen;

    /// A file written for the check, and the folder holding it, removed when the check is done with it.
    struct Written {
        _folder: crate::gui::check_folder::tests::CheckFolder,
        path: String,
    }

    /// A plate of 60 x 40 x 5 with a boss of 30 x 20 x 8 from (10, 10), written to a STEP file.
    fn bossed_file() -> Written {
        let block = |x: f64, y: f64, w: f64, d: f64, z: f64, h: f64| {
            let at = [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z];
            qymcad_kernel::specimen::extrude(&[0.0, 0.0, w, 0.0, w, d, 0.0, d], h).expect("a block").moved(&at).expect("placed")
        };
        let s = block(0.0, 0.0, 60.0, 40.0, 0.0, 5.0).combined(&block(10.0, 10.0, 30.0, 20.0, 5.0, 8.0), 1).expect("fused");
        let folder = crate::gui::check_folder::tests::CheckFolder::new("boss-redrawn");
        let file = folder.file("bossed.step").to_string_lossy().into_owned();
        qymcad_kernel::write_step(&[(&s, qymcad_core::feature::PLACE_IDENTITY)], &file).expect("the STEP is written");
        Written { _folder: folder, path: file }
    }

    /// Frames run, as they do by themselves, until nothing is being built.
    fn at_rest(h: &mut Hand) {
        for _ in 0..600 {
            h.frame(Vec::new());
            h.app.drain_bg_for_test();
            if h.app.regen.busy.is_none() && h.app.regen.bg.is_empty() && !h.app.regen.wanted {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn the_top_of_a_boss_clicked_and_enter_redraw_it_by_a_sketch() {
        let mut app = App::default();
        let file = bossed_file();
        let mut h = Hand::new(&mut app);
        h.import(&file.path);
        // the file is asked about its units, as every file brought in is: as it says, millimetres
        h.press_word(&crate::i18n::tr("import-scale-import"), egui::pos2(700.0, 450.0));
        at_rest(&mut h);
        let body = h.app.project.bodies.last().map(|b| b.id).expect("the plate came in");
        if let Some(part) = h.app.project.body_owner(body) {
            h.app.enter_component(part);
        }
        let before = h.app.live.shapes.get(&body).map(|s| s.volume()).expect("the plate is live");
        h.look_at([25.0, 20.0, 10.0], 6.0).frame(Vec::new());
        h.tool(qymcad_part::SKETCH_FROM_FACE).click([25.0, 20.0, 13.0]).frame(Vec::new());
        assert!(h.app.params.recognise.redraw.as_ref().is_some_and(|r| r.prism.is_some()), "the top of the boss clicked found no boss: {}", h.app.status);
        // what Enter takes off is seen before it: the faces of the boss filled
        let lit = h.app.scheme.pal.selected();
        h.frame(Vec::new());
        assert!(h.mesh_corners_in(lit) > 0, "the boss to be redrawn is not lit before Enter");
        let field = h.app.tools.cmd.params.iter().find(|p| p.key == "height").map(|p| p.val);
        assert_eq!(field, Some(8.0), "the height of the boss does not stand in the field: {}", h.app.status);
        let nodes = h.app.project.timeline.len();
        let sketches = h.app.project.sketches.len();
        h.enter();
        at_rest(&mut h);
        assert_eq!(h.app.project.sketches.len(), sketches + 1, "Enter laid no sketch: {}", h.app.status);
        assert_eq!(h.app.project.timeline.len(), nodes + 2, "Enter laid not a sketch and one node: {}", h.app.status);
        let node = h.app.project.timeline.last().expect("the node");
        assert!(matches!(node.kind, qymcad_core::feature::FeatureKind::Redrawn { .. }), "the last node is not the boss redrawn");
        let built = node.kind.body().and_then(|b| h.app.live.shapes.get(&b)).map(|s| s.volume());
        let said = h.app.project.regen_errors.get(&node.id).map(|e| format!("{e:?}"));
        assert!(built.is_some_and(|v| (v - before).abs() < 1e-3 * before), "redrawn the plate holds {built:?} mm^3, not {before}; the node says {said:?}");
        let si = h.app.project.sketches.len() - 1;
        assert_eq!(h.app.project.sketch_dof(si), (0, 0), "the sketch is not fully defined");
        assert!(h.app.status.words.contains(&crate::i18n::tr("dof-fully-defined")), "the status does not say the sketch is fully defined: {}", h.app.status);
        assert_eq!(h.app.tools.armed.cmd_kind(), qymcad_part::SKETCH_FROM_FACE, "the tool did not stay in hand");
    }
}
