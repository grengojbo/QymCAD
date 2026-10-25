//! THE SIDE BUTTONS OF A MOUSE ZOOM THE VIEW, as the QuickZoom buttons of a CAD mouse are meant to: the front one in,
//! the back one out, about the point under the cursor; held, the view goes on zooming.
//!
//! A CAD mouse sends them two ways, measured on a 3Dconnexion CadMouse Compact Wireless under macOS. With the maker's
//! driver running and the program unknown to it, the front button is the key PageUp and the back one PageDown, a held
//! button repeating the key about twelve times a second. Without the driver - and on Linux, where there is none - they
//! are the buttons Back and Forward, egui's `Extra1` and `Extra2`, held as one press. Both ways zoom the same, and the
//! repeats of a held key do not zoom faster than a held button.
use crate::gui::App;

const SCREEN: egui::Vec2 = egui::vec2(1400.0, 900.0);

/// THE APP AND THE TIME OF ITS FRAMES: the clock is the test's, so a hold lasts as long as it says whatever the
/// number of frames it is cut into.
struct Running {
    app: App,
    ctx: egui::Context,
    now: f64,
}

impl Running {
    /// One frame `dt` seconds after the last, with `events`.
    fn frame(&mut self, dt: f64, events: Vec<egui::Event>) {
        self.now += dt;
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), SCREEN)), time: Some(self.now), events, ..Default::default() };
        let app = &mut self.app;
        let _ = self.ctx.run_ui(raw, |ui| app.draw_frame(ui));
    }
}

/// The app running, and the place of the view the hand is over.
struct InView {
    r: Running,
    at: egui::Pos2,
}

/// A part with a body seen in 3D, the panels settled and the hand over a place of the view off its middle.
fn a_part_in_view() -> InView {
    let mut app = App::default();
    app.set.gpu_viewport = false;
    let cid = app.project.add_part("P");
    app.enter_component(cid);
    let c = app.project.add_cylinder(10.0, 20.0);
    app.project.finish_base_body(c, 1);
    app.exit_context();
    qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
    app.viewing.mode_3d = true;
    let ctx = egui::Context::default();
    crate::gui::install_fonts(&ctx);
    app.waiting.splash_until = None;
    let mut r = Running { app, ctx, now: 0.0 };
    for _ in 0..3 {
        r.frame(0.016, Vec::new()); // the panels settle and the viewport learns its rectangle
    }
    let view = r.app.viewing.view_rect;
    crate::gui::fit3d(&mut r.app.viewing.cam, &r.app.project, view);
    let at = view.center() + egui::vec2(view.width() * 0.2, -view.height() * 0.2);
    r.frame(0.016, vec![egui::Event::PointerMoved(at)]);
    InView { r, at }
}

/// The two ends of a press.
#[derive(Clone, Copy)]
enum Edge {
    Down,
    Up,
}

/// What holds a side button down: the button itself, or the key the maker's driver sends for it.
#[derive(Clone, Copy)]
enum By {
    Button,
    Key,
}

fn button(at: egui::Pos2, button: egui::PointerButton, edge: Edge) -> egui::Event {
    egui::Event::PointerButton { pos: at, button, pressed: matches!(edge, Edge::Down), modifiers: egui::Modifiers::NONE }
}

fn key(key: egui::Key, edge: Edge) -> egui::Event {
    egui::Event::Key { key, physical_key: Some(key), pressed: matches!(edge, Edge::Down), repeat: false, modifiers: egui::Modifiers::NONE }
}

/// The press a held key sends again and again.
fn repeated(key: egui::Key) -> egui::Event {
    egui::Event::Key { key, physical_key: Some(key), pressed: true, repeat: true, modifiers: egui::Modifiers::NONE }
}

/// A way of zooming one step: its name, the press and release that make it, and the scale it is to give.
struct Way {
    name: &'static str,
    make: fn(egui::Pos2, Edge) -> egui::Event,
    times: f32,
}

/// Where the point of the model under `at` is seen after the press of `way` and its release, one frame apart.
fn press_and_let_go(r: &mut Running, at: egui::Pos2, way: &Way) -> egui::Pos2 {
    let view = r.app.viewing.view_rect;
    let held = crate::gui::look_at_point::point_under(&r.app.painting(), view, at);
    r.frame(0.016, vec![egui::Event::PointerMoved(at), (way.make)(at, Edge::Down)]);
    r.frame(0.016, vec![egui::Event::PointerMoved(at), (way.make)(at, Edge::Up)]);
    let basis = r.app.viewing.cam.basis();
    qymcad_ui_state::Screen { cam: &r.app.viewing.cam, set: &r.app.set, rect: view, basis: &basis }.at(held).0
}

/// ONE PRESS IS ONE STEP of 1.25 times, in with the front button or PageUp and out with the back one or PageDown,
/// and the point under the cursor stays under it.
#[test]
fn a_press_of_a_side_button_or_of_its_key_zooms_one_step_about_the_cursor() {
    let mut wrong = Vec::new();
    let ways = [
        Way { name: "the front button", make: |at, e| button(at, egui::PointerButton::Extra1, e), times: 1.25 },
        Way { name: "the back button", make: |at, e| button(at, egui::PointerButton::Extra2, e), times: 0.8 },
        Way { name: "PageUp", make: |_, e| key(egui::Key::PageUp, e), times: 1.25 },
        Way { name: "PageDown", make: |_, e| key(egui::Key::PageDown, e), times: 0.8 },
    ];
    for way in ways {
        let InView { mut r, at } = a_part_in_view();
        let was = r.app.viewing.cam.scale;
        let landed = press_and_let_go(&mut r, at, &way);
        let got = r.app.viewing.cam.scale / was;
        if (got - way.times).abs() > 0.01 {
            wrong.push(format!("{}: the view was scaled {got:.3} times, not {}", way.name, way.times));
        }
        if landed.distance(at) > 0.5 {
            wrong.push(format!("{}: the point under the cursor went from {at:?} to {landed:?}", way.name));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// HOW MUCH A HOLD OF `secs` SCALES THE VIEW, cut into `frames`, the front button held - or PageUp held with the
/// repeats a held key sends, one every 83 ms as measured.
fn held(secs: f64, frames: usize, by: By) -> f32 {
    let InView { mut r, at } = a_part_in_view();
    let was = r.app.viewing.cam.scale;
    let press = |edge| match by {
        By::Key => key(egui::Key::PageUp, edge),
        By::Button => button(at, egui::PointerButton::Extra1, edge),
    };
    let down = press(Edge::Down);
    r.frame(0.016, vec![egui::Event::PointerMoved(at), down]);
    let dt = secs / frames as f64;
    let mut next_repeat = 0.5;
    for i in 1..=frames {
        let t = dt * i as f64;
        let mut events = vec![egui::Event::PointerMoved(at)];
        while matches!(by, By::Key) && next_repeat <= t {
            events.push(repeated(egui::Key::PageUp));
            next_repeat += 0.083;
        }
        r.frame(dt, events);
    }
    r.frame(0.016, vec![egui::Event::PointerMoved(at), press(Edge::Up)]);
    r.app.viewing.cam.scale / was
}

/// HELD, THE VIEW GOES ON ZOOMING by the time it is held, not by the frames or the repeats of a key: the step, then
/// after 0.35 s twice a second. Held 1.35 s: 1.25 x 2 = 2.5 times, in 10 frames or in 100, by the button or by its
/// repeated key.
#[test]
fn a_held_side_button_zooms_by_the_time_it_is_held() {
    /// A hold made one way, and the scale it gave.
    struct Run {
        how: &'static str,
        times: f32,
    }
    let runs = [
        Run { how: "10 frames", times: held(1.35, 10, By::Button) },
        Run { how: "100 frames", times: held(1.35, 100, By::Button) },
        Run { how: "the key with its repeats", times: held(1.35, 100, By::Key) },
    ];
    let wrong: Vec<String> = runs.iter().filter(|r| (r.times - 2.5).abs() > 0.05).map(|r| format!("{}: scaled {:.3} times", r.how, r.times)).collect();
    assert!(wrong.is_empty(), "held 1.35 s the view was to scale 2.5 times:\n{}", wrong.join("\n"));
}

/// AND THE FLAT SHEET OF A SKETCH zooms the same step.
#[test]
fn a_side_button_zooms_the_sheet_of_a_sketch() {
    use crate::gui::import_door::tests::{frame, running};
    let (mut app, ctx) = running();
    let si = app.create_sketch_on(qymcad_core::feature::SketchPlane::default());
    app.project.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, qymcad_core::feature::Purpose::Real);
    app.project.regen_sketch(si);
    app.finish_sketch_edit();
    app.enter_sketch_edit(si);
    for _ in 0..3 {
        let _ = frame(&mut app, &ctx, Vec::new());
    }
    let at = app.viewing.view_rect.center();
    let _ = frame(&mut app, &ctx, vec![egui::Event::PointerMoved(at)]);
    let was = app.viewing.view.scale;
    let _ = frame(&mut app, &ctx, vec![egui::Event::PointerMoved(at), button(at, egui::PointerButton::Extra1, Edge::Down)]);
    let _ = frame(&mut app, &ctx, vec![egui::Event::PointerMoved(at), button(at, egui::PointerButton::Extra1, Edge::Up)]);
    let got = app.viewing.view.scale / was;
    assert!((got - 1.25).abs() < 0.01, "the front button scaled the sheet {got:.3} times, not 1.25");
}
