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

    /// A plate of 60 x 40 x 5 with a boss of 30 x 20 x 8 from (10, 10), its top rounded `r` when one is given.
    fn bossed_solid(r: Option<f64>) -> qymcad_kernel::Shape {
        let block = |x: f64, y: f64, w: f64, d: f64, z: f64, h: f64| {
            let at = [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z];
            qymcad_kernel::specimen::extrude(&[0.0, 0.0, w, 0.0, w, d, 0.0, d], h).expect("a block").moved(&at).expect("placed")
        };
        let s = block(0.0, 0.0, 60.0, 40.0, 0.0, 5.0).combined(&block(10.0, 10.0, 30.0, 20.0, 5.0, 8.0), 1).expect("fused");
        let Some(r) = r else { return s };
        let top: Vec<u32> = s.edges_info().into_iter().filter(|e| e.poly.iter().all(|p| (p[2] as f64 - 13.0).abs() < 1e-6)).map(|e| e.id).collect();
        s.filleted(r, &top).expect("rounded")
    }

    /// A plate of 60 x 40 x 5 with a boss of 30 x 20 x 8 from (10, 10), written to a STEP file.
    fn bossed_file() -> Written {
        let block = |x: f64, y: f64, w: f64, d: f64, z: f64, h: f64| {
            let at = [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z];
            qymcad_kernel::specimen::extrude(&[0.0, 0.0, w, 0.0, w, d, 0.0, d], h).expect("a block").moved(&at).expect("placed")
        };
        let s = block(0.0, 0.0, 60.0, 40.0, 0.0, 5.0).combined(&block(10.0, 10.0, 30.0, 20.0, 5.0, 8.0), 1).expect("fused");
        written(&s, "bossed")
    }

    /// `s` written to a STEP file named `name`.
    fn written(s: &qymcad_kernel::Shape, name: &str) -> Written {
        let folder = crate::gui::check_folder::tests::CheckFolder::new(name);
        let file = folder.file(&format!("{name}.step")).to_string_lossy().into_owned();
        qymcad_kernel::write_step(&[(s, qymcad_core::feature::PLACE_IDENTITY)], &file).expect("the STEP is written");
        Written { _folder: folder, path: file }
    }

    /// A plate of 60 x 40 x 5 with a round boss of 12 across and 8 high at (30, 20), written to a STEP file.
    fn round_boss_file() -> Written {
        let plate = qymcad_kernel::specimen::extrude(&[0.0, 0.0, 60.0, 0.0, 60.0, 40.0, 0.0, 40.0], 5.0).expect("a plate");
        let boss = qymcad_kernel::specimen::cylinder(6.0, 8.0).expect("a boss").moved(&[1.0, 0.0, 0.0, 30.0, 0.0, 1.0, 0.0, 20.0, 0.0, 0.0, 1.0, 5.0]).expect("placed");
        written(&plate.combined(&boss, 1).expect("fused"), "round-boss")
    }

    /// The window with `file` brought in, as millimetres, and its part entered.
    fn brought_in(app: &mut App, file: &Written) {
        let mut h = Hand::new(app);
        h.import(&file.path);
        h.press_word(&crate::i18n::tr("import-scale-import"), egui::pos2(700.0, 450.0));
        at_rest(&mut h);
        let body = h.app.project.bodies.last().map(|b| b.id).expect("the plate came in");
        if let Some(part) = h.app.project.body_owner(body) {
            h.app.enter_component(part);
        }
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
        h.tool(qymcad_ui_state::PartTool::SketchFromFace).click([25.0, 20.0, 13.0]).frame(Vec::new());
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
        assert_eq!(h.app.tools.armed.part_tool(), Some(qymcad_ui_state::PartTool::SketchFromFace), "the tool did not stay in hand");
    }

    /// THE ROUND WALL OF A BOSS CLICKED is refused where the tool speaks - on its plate in the corner of the view, in the
    /// tone of a refusal - and not only in the status line at the foot of the window. Reported behaviour: the click held
    /// no face, the tool said nothing, "nothing happened".
    #[test]
    fn a_round_wall_clicked_is_refused_by_the_tool() {
        let mut app = App::default();
        let file = round_boss_file();
        brought_in(&mut app, &file);
        let mut h = Hand::new(&mut app);
        h.look_at([30.0, 20.0, 9.0], 6.0).frame(Vec::new());
        h.tool(qymcad_ui_state::PartTool::SketchFromFace).click([30.0, 14.0, 9.0]).frame(Vec::new());
        let refusal = crate::i18n::tr("error-prism-face-not-flat");
        assert!(h.app.status.words.contains(&refusal), "the click on the round wall was not refused: {}", h.app.status);
        let said = h.tool_says();
        assert!(said.iter().any(|n| n.words == refusal && n.tone == qymcad_ui_state::Tone::Refusal), "the tool does not say the refusal: {said:?}");
    }

    /// THE FACE A TOOL TOOK IS NAMED IN THE PROPERTIES BY THAT TOOL. Reported behaviour: under the top of a boss Sketch
    /// from face had taken stood "picked for a command (Shell or Hole)".
    #[test]
    fn the_face_taken_is_named_by_the_tool_that_took_it() {
        let mut app = App::default();
        let file = bossed_file();
        brought_in(&mut app, &file);
        let mut h = Hand::new(&mut app);
        h.look_at([25.0, 20.0, 10.0], 6.0).frame(Vec::new());
        h.tool(qymcad_ui_state::PartTool::SketchFromFace).click([25.0, 20.0, 13.0]).frame(Vec::new());
        h.frame(Vec::new());
        let tool = crate::i18n::tr("f-sketch-from-face");
        let named = crate::i18n::tr1("face-picked-for-cmd", "tool", &tool);
        assert!(named.contains(&tool), "the line under a face taken does not name the tool: {named}");
        let texts = h.texts();
        assert!(
            texts.iter().any(|t| t.contains(&named)),
            "the properties do not say the face is held by Sketch from face; {:?} is under the face",
            texts.iter().filter(|t| t.contains("Shell") || t.contains(&named)).collect::<Vec<_>>()
        );
    }

    /// SKETCH ONLY ON A BOSS SAYS THE SKETCH IS A COPY, and how to have it drive the boss. Reported behaviour: the sizes
    /// of a sketch taken with Sketch only were changed, and the person waited for the block to follow.
    #[test]
    fn sketch_only_says_it_is_a_copy() {
        let mut app = App::default();
        let file = bossed_file();
        brought_in(&mut app, &file);
        let mut h = Hand::new(&mut app);
        h.look_at([25.0, 20.0, 10.0], 6.0).frame(Vec::new());
        h.tool(qymcad_ui_state::PartTool::SketchFromFace).frame(Vec::new());
        assert!(h.press_word(&crate::i18n::tr("cmd-sketch-only"), egui::pos2(400.0, 40.0)), "no Sketch only in the bar");
        h.click([25.0, 20.0, 13.0]).frame(Vec::new());
        let told = crate::i18n::tr("msg-sketch-from-face-copy-replace");
        assert!(h.app.status.words.contains(&told), "Sketch only on a boss does not say how to have the sketch drive it: {}", h.app.status);
    }

    /// A RIM ROUNDED R2 GOES WITH THE BOSS REDRAWN and is named and changed with it: the row in the tree names R2.0, the
    /// node reopened shows the rounding beside the height, and 1.5 typed there lays the rim again 1.5. Reported
    /// behaviour: the tree said "Boss by sketch h=8.0" alone, and R2 could not be changed.
    #[test]
    fn a_rounded_rim_is_named_and_changed_with_the_boss() {
        let mut app = App::default();
        let file = written(&bossed_solid(Some(2.0)), "rounded-boss");
        brought_in(&mut app, &file);
        let mut h = Hand::new(&mut app);
        h.look_at([25.0, 20.0, 10.0], 6.0).frame(Vec::new());
        h.tool(qymcad_ui_state::PartTool::SketchFromFace).click([25.0, 20.0, 13.0]).frame(Vec::new());
        let rim = h.app.tools.cmd.params.iter().find(|p| p.key == "rim").map(|p| p.val);
        assert_eq!(rim, Some(2.0), "the rounding of the rim is not a field beside the height: {}", h.app.status);
        h.enter();
        at_rest(&mut h);
        let node = h.app.project.timeline.last().map(|n| n.id).expect("the node");
        let rounded = crate::i18n::tr1("msg-rim-round", "d", &crate::i18n::num(2.0, 1));
        let row = h.texts().into_iter().find(|t| t.contains(&crate::i18n::tr1("feat-redrawn-boss", "h", &crate::i18n::num(8.0, 1))));
        assert!(row.as_deref().is_some_and(|r| r.contains(&rounded)), "the row of the boss redrawn does not name {rounded}: {row:?}");
        h.key(egui::Key::Escape).key(egui::Key::Escape);
        assert!(h.double_click_word(row.as_deref().unwrap_or_default()), "the row of the boss redrawn is not on screen");
        let rim = h.app.tools.cmd.params.iter().find(|p| p.key == "rim").map(|p| p.val);
        assert_eq!(rim, Some(2.0), "the node reopened does not show its rounding: {}", h.app.status);
        h.set("rim", 1.5).enter();
        at_rest(&mut h);
        let built = h.app.project.timeline.iter().find(|n| n.id == node).and_then(|n| n.kind.body()).and_then(|b| h.app.live.shapes.get(&b)).map(|s| s.volume());
        let want = bossed_solid(Some(1.5)).volume();
        assert!(built.is_some_and(|v| (v - want).abs() < 1e-3 * want), "the rim made 1.5 holds {built:?} mm^3, not {want}");
    }
}
