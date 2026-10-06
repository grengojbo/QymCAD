//! THE VIEW OUT OF A WINDOW: the camera aimed at the model, the angles of a direction looked along, and
//! a picture written as PNG. The window calls them, and so does a program that draws the document with no
//! window at all: the same fit and the same angles give the same picture.

use egui::Rect;
use qymcad_core::model::Project;
use qymcad_ui_state::Cam3;

/// The angles of the camera looking along a direction: the turn about the vertical and the tilt above the
/// horizon, in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Angles {
    pub yaw: f64,
    pub pitch: f64,
}

/// A direction becomes the angles of the camera. One function for the view cube, the animation of the
/// turn and the checks: two copies of the formula would diverge in a sign and turn the view the wrong way.
pub fn dir_to_angles(d: [f64; 3]) -> Angles {
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-12);
    let n = [d[0] / l, d[1] / l, d[2] / l];
    Angles { yaw: n[1].atan2(n[0]), pitch: n[2].clamp(-1.0, 1.0).asin() }
}

/// A PNG out of a `ColorImage` (RGBA8): the `thumb.png` inside a `.qpart`, a picture a check looks at, a picture handed out of the program. Uses `image` with its png feature.
pub fn color_image_to_png(img: &egui::ColorImage) -> Option<Vec<u8>> {
    use image::ImageEncoder;
    let (w, h) = (img.size[0] as u32, img.size[1] as u32);
    let mut rgba: Vec<u8> = Vec::with_capacity(img.pixels.len() * 4);
    for p in &img.pixels {
        rgba.extend_from_slice(&p.to_array());
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png).write_image(&rgba, w, h, image::ExtendedColorType::Rgba8).ok()?;
    Some(png)
}

/// THE CAMERA AIMED AT THE WHOLE MODEL: the target at the middle of what is drawn, the scale that fits it into `rect`.
pub fn fit3d(cam: &mut Cam3, project: &Project, rect: Rect) {
    let mut mn = [f64::INFINITY; 3];
    let mut mx = [f64::NEG_INFINITY; 3];
    // A POINT THAT IS NOT A NUMBER DOES NOT MOVE THE BOUNDS, and that is why the bounds are widened by
    // COMPARISON rather than by `min`/`max`: every comparison against a NaN is false, so one bad vertex - an
    // import gone wrong, a degenerate face - is simply not measured. Held by a check of its own, because
    // written the obvious way it would poison both bounds, the finite test below would refuse the whole
    // fit, `cam.init` would stay false, and the viewport would open on nothing every frame for ever.
    let mut acc = |p: [f64; 3]| {
        for a in 0..3 {
            if p[a] < mn[a] {
                mn[a] = p[a];
            }
            if p[a] > mx[a] {
                mx[a] = p[a];
            }
        }
    };
    // THE BODIES IN WORLD SPACE, and only the ones that are actually drawn.
    //
    // Reported behaviour: "sometimes on starting the program, if there is a finished project already, the
    // camera flies terribly far away, or an empty 3D viewport opens."
    //
    // A body's mesh lives in the coordinates of the component that owns it; where that component stands
    // comes from `body_world_transform`, and this measurement never asked for it. In an assembly whose
    // parts are placed apart the camera was aimed at the local zero of the meshes while the parts were
    // drawn elsewhere: measured on two parts 200 mm apart, none of the far part's bounding box was on
    // screen at all. Consumed and hidden bodies are left out for the same reason - they are not on screen,
    // and framing the view around them aims it at nothing.
    let consumed = project.consumed_bodies();
    for b in project.bodies.iter().filter(|b| b.visible && !consumed.contains(&b.id)) {
        let wt = project.body_world_transform(b.id);
        for v in &b.mesh.verts {
            acc(qymcad_core::feature::apply12(&wt, [v.x, v.y, v.z]));
        }
    }
    // THE SKETCHES, LIFTED ONTO THEIR OWN PLANES. `project.contours` holds flat 2D coordinates of a sketch
    // on its own plane, and they were fed in as world X and Y with z = 0 - so a sketch on the front plane,
    // or one belonging to a part standing away from the origin, dragged the measurement to a place where
    // nothing is drawn.
    for (si, s) in project.sketches.iter().enumerate() {
        let Some(fr) = project.sketch_frame(si) else { continue };
        let wt = project.sketch_owner(s.id).map(|c| project.world_transform(c)).unwrap_or(qymcad_core::feature::PLACE_IDENTITY);
        for cid in &s.contour_ids {
            let Some(ci) = project.contour_index(*cid) else { continue };
            for p in &project.contours[ci].points {
                let w = fr.lift(*p);
                acc(qymcad_core::feature::apply12(&wt, [w.x, w.y, w.z]));
            }
        }
    }
    if !mn[0].is_finite() {
        return;
    }
    cam.target = [(mn[0] + mx[0]) / 2.0, (mn[1] + mx[1]) / 2.0, (mn[2] + mx[2]) / 2.0];
    // a model under 1 mm is framed at its own size; only a scene with no extent at all, a single point, takes 1 mm
    let raw = (mx[0] - mn[0]).max(mx[1] - mn[1]).max(mx[2] - mn[2]);
    let ext = (if raw > 1e-9 { raw } else { 1.0 }) as f32;
    cam.scale = (rect.width().min(rect.height()) / ext) * 0.55;
    cam.fit = cam.scale;
    cam.init = true;
}
