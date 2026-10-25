//! THE EDGES OF A BODY ARE DRAWN, with or without a tool in hand: a thin dark line where two faces meet at an angle, so
//! the top of a plate and its sides read apart. Reported behaviour: the top of a plate, its walls and a boss on it came
//! out one colour and nearly one brightness, no edge seen, and a person could not tell where to click.
use crate::gui::App;

fn rect() -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 900.0))
}

/// A plate of 60 x 40 x 5 in a fresh document, the camera fitted to it, seen from the default corner.
fn a_plate() -> App {
    let mut app = App { project: qymcad_core::model::Project::default(), ..Default::default() };
    app.project.new_document();
    let body = app.project.add_box(60.0, 40.0, 5.0);
    app.project.finish_base_body(body, 1);
    app.viewing.mode_3d = true;
    qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
    let Bounds { lo, hi } = bounds(&app);
    app.viewing.cam.target = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
    app.viewing.cam.scale = 12.0;
    app.viewing.cam.init = true;
    app
}

/// The box of the first body: its lowest and its highest corner.
struct Bounds {
    lo: [f64; 3],
    hi: [f64; 3],
}

fn bounds(app: &App) -> Bounds {
    let b = app.project.bodies.first().expect("a body");
    let mut out = Bounds { lo: [f64::MAX; 3], hi: [f64::MIN; 3] };
    for v in &b.mesh.verts {
        for (k, c) in [v.x, v.y, v.z].into_iter().enumerate() {
            out.lo[k] = out.lo[k].min(c);
            out.hi[k] = out.hi[k].max(c);
        }
    }
    out
}

/// How bright the pixels within `reach` of `at` are, each the sum of its three channels.
fn brightness(img: &egui::ColorImage, at: egui::Pos2, reach: i32) -> Vec<u32> {
    let mut out = Vec::new();
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let (x, y) = (at.x as i32 + dx, at.y as i32 + dy);
            if x < 0 || y < 0 || x as usize >= img.size[0] || y as usize >= img.size[1] {
                continue;
            }
            let p = img.pixels[y as usize * img.size[0] + x as usize].to_array();
            out.push(p[0] as u32 + p[1] as u32 + p[2] as u32);
        }
    }
    out
}

#[test]
fn the_edge_between_the_top_and_a_side_is_a_dark_line() {
    let app = a_plate();
    let basis = app.viewing.cam.basis();
    let img = crate::gui::render::rasterize_3d(&app.painting(), rect(), &basis, 1.0, 1.0).expect("the frame was not drawn");
    let Bounds { lo, hi } = bounds(&app);
    let mid_x = (lo[0] + hi[0]) / 2.0;
    let scr = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect: rect(), basis: &basis };
    // the edge where the top meets the front, and a point on each of the two faces a little way off it
    let edge = scr.at([mid_x, lo[1], hi[2]]).0;
    let top = scr.at([mid_x, lo[1] + 4.0, hi[2]]).0;
    let front = scr.at([mid_x, lo[1], hi[2] - 2.5]).0;
    let on_edge = brightness(&img, edge, 1).into_iter().min().unwrap_or(u32::MAX);
    let average = |v: Vec<u32>| v.iter().sum::<u32>() / v.len().max(1) as u32;
    let (on_top, on_front) = (average(brightness(&img, top, 1)), average(brightness(&img, front, 1)));
    assert!(on_edge + 60 < on_top.min(on_front), "no dark line where the top meets the front: {on_edge} on the edge, {on_top} on the top, {on_front} on the front");
    // AN EDGE BEHIND THE BODY IS NOT SEEN THROUGH IT: the back of the bottom lies under the top
    let hidden = scr.at([mid_x, hi[1], lo[2]]).0;
    let behind = brightness(&img, hidden, 1).into_iter().min().unwrap_or(0);
    assert!(behind + 30 > average(brightness(&img, hidden, 4)), "the edge behind the body shows through it: {behind} against {}", average(brightness(&img, hidden, 4)));
    if let Some(png) = crate::gui::color_image_to_png(&img) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/edges-look");
        if std::fs::create_dir_all(&dir).is_ok() {
            let _ = std::fs::write(dir.join("plate.png"), png);
        }
    }
}

/// THE FACES OF A PLATE READ APART BY THEIR LIGHT ALONE: the top, the front and the side, each a tone of its own well
/// apart from the others, the edges hidden. Reported behaviour: the top and the walls came out nearly one brightness.
#[test]
fn the_top_the_front_and_the_side_take_tones_well_apart() {
    let mut app = a_plate();
    app.set.body_edges = qymcad_ui_state::BodyEdges::Hidden;
    let basis = app.viewing.cam.basis();
    let img = crate::gui::render::rasterize_3d(&app.painting(), rect(), &basis, 1.0, 1.0).expect("the frame was not drawn");
    let Bounds { lo, hi } = bounds(&app);
    let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
    let scr = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect: rect(), basis: &basis };
    let tone = |p: [f64; 3]| {
        let v = brightness(&img, scr.at(p).0, 2);
        v.iter().sum::<u32>() / v.len().max(1) as u32
    };
    let faces = [tone([mid[0], mid[1], hi[2]]), tone([mid[0], lo[1], mid[2]]), tone([hi[0], mid[1], mid[2]])];
    eprintln!("top, front, side: {faces:?}");
    for i in 0..3 {
        for j in i + 1..3 {
            assert!(faces[i].abs_diff(faces[j]) >= 40, "two faces of the plate take near one tone - top, front, side: {faces:?}");
        }
    }
}

/// AN EDGE BEHIND A BODY STAYS HIDDEN IN A PARALLEL VIEW OF A WIDE SCENE: the plate of 5 seen orthographically while a
/// block of another part stands 100 m off, so the depth of the view spans the 100 m. Reported behaviour, by the check
/// of parts far apart on a Linux machine: a face of the near block drawn in two tones, an edge of its back seen through
/// it - the line was led towards the eye by a share of the depth span, and that share grew past the plate.
#[test]
fn an_edge_behind_the_body_stays_hidden_in_a_parallel_view_of_a_wide_scene() {
    let mut app = a_plate();
    let root = app.project.root;
    app.project.set_active_component(Some(root));
    let part = app.project.add_part("Far");
    app.project.set_active_component(Some(part));
    let far = app.project.add_box(10.0, 10.0, 10.0);
    app.project.finish_base_body(far, 1);
    if let Some(c) = app.project.components.iter_mut().find(|c| c.id == part) {
        c.transform[3] = 100_000.0;
    }
    qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
    assert_eq!(app.project.bodies.len(), 2, "setup: the plate and the far block");
    app.set.projection = qymcad_ui_state::Projection::Ortho;
    let basis = app.viewing.cam.basis();
    let img = crate::gui::render::rasterize_3d(&app.painting(), rect(), &basis, 1.0, 1.0).expect("the frame was not drawn");
    let Bounds { lo, hi } = bounds(&app);
    let scr = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect: rect(), basis: &basis };
    let average = |v: Vec<u32>| v.iter().sum::<u32>() / v.len().max(1) as u32;
    let hidden = scr.at([(lo[0] + hi[0]) / 2.0, hi[1], lo[2]]).0;
    let behind = brightness(&img, hidden, 1).into_iter().min().unwrap_or(0);
    assert!(behind + 30 > average(brightness(&img, hidden, 4)), "the edge behind the plate shows through it: {behind} against {}", average(brightness(&img, hidden, 4)));
}
