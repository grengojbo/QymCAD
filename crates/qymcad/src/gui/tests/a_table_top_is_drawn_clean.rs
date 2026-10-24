//! THE SLATS OF A TABLE TOP ARE DRAWN IN ONE TONE: `examples/Table_CNC_WORK.stp` brought in by File -> Import, drawn by
//! the software raster from the usual three-quarter view. The top face of every slat comes out in
//! one tone. Reported behaviour: four of the six slats speckled with light noise, as if two surfaces fought for the same
//! pixels. The file marks its helper bodies (a white plate and its pockets lying in the plane of the slats) invisible;
//! they came in shown.
#[cfg(test)]
mod tests {
    use crate::gui::hand::Hand;
    use crate::gui::App;

    const SIZE: egui::Vec2 = egui::vec2(1200.0, 900.0);

    fn rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), SIZE)
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

    /// The top face of one slat on screen: whose it is and its triangles, each drawn in towards its middle so that no
    /// pixel of it touches a hole or the rim, where a neighbouring face is drawn.
    struct TopSeen {
        label: String,
        triangles: Vec<[egui::Pos2; 3]>,
    }

    /// The faces turned up of every shown body named as a slat of the top.
    fn slat_tops(app: &App) -> Vec<TopSeen> {
        let basis = app.viewing.cam.basis();
        let scr = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect: rect(), basis: &basis };
        let shown = qymcad_ui_state::shown_bodies(&app.painting());
        let mut out = Vec::new();
        for (mi, body) in app.project.bodies.iter().enumerate() {
            let Some(id) = app.project.mesh_id(mi) else { continue };
            let name = app.project.body_owner(id).and_then(|c| app.project.components.iter().find(|k| k.id == c)).map(|c| c.name.clone()).unwrap_or_default();
            if !name.starts_with("Table_Top") || !shown.iter().any(|s| s.1 == id) {
                continue;
            }
            let wt = app.project.body_world_transform(id);
            let on_screen = |p: qymcad_core::geom::Point3| scr.at(qymcad_core::feature::apply12(&wt, [p.x, p.y, p.z])).0;
            for (fi, f) in body.faces.iter().enumerate() {
                if qymcad_core::feature::apply12_dir(&wt, f.normal)[2] < 0.99 {
                    continue;
                }
                let triangles = f
                    .triangles
                    .iter()
                    .map(|&t| {
                        let c = body.mesh.triangle(t as usize).map(on_screen);
                        let mid = egui::pos2((c[0].x + c[1].x + c[2].x) / 3.0, (c[0].y + c[1].y + c[2].y) / 3.0);
                        c.map(|p| mid + (p - mid) * 0.7)
                    })
                    .collect();
                out.push(TopSeen { label: format!("{name} (body {mi}, face {fi})"), triangles });
            }
        }
        out
    }

    /// The pixels whose middles fall inside the triangles, each once.
    fn pixels_in(triangles: &[[egui::Pos2; 3]], size: [usize; 2]) -> std::collections::BTreeSet<usize> {
        let side = |a: egui::Pos2, b: egui::Pos2, p: egui::Pos2| (b - a).x * (p - a).y - (b - a).y * (p - a).x;
        let mut out = std::collections::BTreeSet::new();
        for t in triangles {
            let r = egui::Rect::from_points(t);
            for y in r.top().max(0.0) as usize..(r.bottom().ceil() as usize).min(size[1]) {
                for x in r.left().max(0.0) as usize..(r.right().ceil() as usize).min(size[0]) {
                    let p = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
                    let s = [side(t[0], t[1], p), side(t[1], t[2], p), side(t[2], t[0], p)];
                    if s.iter().all(|v| *v >= 0.0) || s.iter().all(|v| *v <= 0.0) {
                        out.insert(y * size[0] + x);
                    }
                }
            }
        }
        out
    }

    /// How far apart two colours are, in levels of the channel that differs most.
    fn apart(pair: [egui::Color32; 2]) -> i32 {
        let (a, b) = (pair[0].to_array(), pair[1].to_array());
        (0..3).map(|k| (a[k] as i32 - b[k] as i32).abs()).max().unwrap_or(0)
    }

    #[test]
    fn every_slat_of_the_table_top_is_drawn_in_one_tone() {
        let mut app = App::default();
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/Table_CNC_WORK.stp");
        {
            let mut h = Hand::new(&mut app);
            h.import(path);
            // the file is asked about its units, as every file brought in is: as it says, millimetres
            h.press_word(&crate::i18n::tr("import-scale-import"), egui::pos2(700.0, 450.0));
            at_rest(&mut h);
        }
        app.viewing.mode_3d = true;
        crate::gui::fit3d(&mut app.viewing.cam, &app.project, rect());
        let img = qymcad_render::rasterize_3d(&app.painting(), rect(), &app.viewing.cam.basis(), 1.0, 1.0).expect("the raster drew nothing");
        let tops = slat_tops(&app);
        assert_eq!(tops.len(), 6, "not the six slats of the top on screen: {:?}", tops.iter().map(|t| &t.label).collect::<Vec<_>>());
        // a flat face under one light is one colour; the noise stood 40 and more levels off it
        let mut wrong = Vec::new();
        for top in &tops {
            let seen: Vec<egui::Color32> = pixels_in(&top.triangles, img.size).into_iter().map(|i| img.pixels[i]).collect();
            let mut count: std::collections::HashMap<egui::Color32, usize> = std::collections::HashMap::new();
            for c in &seen {
                *count.entry(*c).or_insert(0) += 1;
            }
            let tone = count.into_iter().max_by_key(|e| e.1).map(|e| e.0).unwrap_or_default();
            let off = seen.iter().filter(|c| apart([**c, tone]) > 12).count();
            if seen.len() < 1000 || off > 0 {
                wrong.push(format!("{}: {off} of {} pixels off its tone {tone:?}", top.label, seen.len()));
            }
        }
        if !wrong.is_empty() {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/look");
            std::fs::create_dir_all(&dir).ok();
            if let Some(png) = crate::gui::color_image_to_png(&img) {
                std::fs::write(dir.join("table-top-speckled.png"), png).ok();
            }
        }
        assert!(wrong.is_empty(), "the slats of the top are not drawn in one tone (target/look/table-top-speckled.png):\n{}", wrong.join("\n"));
    }
}
