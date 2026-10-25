//! THE PICTURES OF THE MANUAL CHECKS of the work on STL and STEP files: the window driven by the hand as a person drives
//! it, in Ukrainian, each step taken as a picture of the whole window with a red ring where the pointer is. The words
//! the window said at each step are printed beside the name of the picture, so the text of a check quotes the window
//! rather than a memory of it.
//!
//! To draw: `cargo test -p qymcad --lib -- --ignored --nocapture tutorial_shots`. The pictures go to
//! `target/tutorial/<check>/`.
use crate::gui::hand::Hand;
use crate::gui::App;
use qymcad_kernel::specimen::{self, Specimen};
use qymcad_kernel::Shape;

mod face_gap;
mod fully_define;
mod recognise;
mod recognise_all;
mod section;
mod sketch_from_face;

/// A file written for the pictures, and the folder holding it, removed when the pictures are taken.
pub(super) struct Written {
    _folder: crate::gui::check_folder::tests::CheckFolder,
    pub(super) path: String,
}

/// `s` written to a STEP file named `name`.
pub(super) fn step_of(s: &Shape, name: &str) -> Written {
    let folder = crate::gui::check_folder::tests::CheckFolder::new(name);
    let path = folder.file(&format!("{name}.step")).to_string_lossy().into_owned();
    qymcad_kernel::write_step_of(s, &path).expect("the STEP is written");
    Written { _folder: folder, path }
}

/// `s` written to an STL file named `name`, at the standard quality of the export.
pub(super) fn stl_of(s: &Shape, name: &str) -> Written {
    let folder = crate::gui::check_folder::tests::CheckFolder::new(name);
    let path = folder.file(&format!("{name}.stl")).to_string_lossy().into_owned();
    let mesh = specimen::mesh_merged(s, 0.05).expect("a mesh").mesh;
    qymcad_io::export_stl(&[mesh], &path).expect("the STL is written");
    Written { _folder: folder, path }
}

/// A block of `size` from the origin.
pub(super) fn block(size: [f64; 3]) -> Shape {
    specimen::extrude(&[0.0, 0.0, size[0], 0.0, size[0], size[1], 0.0, size[1]], size[2]).expect("a block")
}

/// `s` carried by `by`.
pub(super) fn placed(s: &Shape, by: [f64; 3]) -> Shape {
    s.moved(&[1.0, 0.0, 0.0, by[0], 0.0, 1.0, 0.0, by[1], 0.0, 0.0, 1.0, by[2]]).expect("placed")
}

/// The upright edge of `s` standing at `at`.
pub(super) fn upright(s: &Shape, at: [f64; 2]) -> u32 {
    s.edges_info().into_iter().find(|e| e.poly.iter().all(|p| (p[0] as f64 - at[0]).abs() < 1e-6 && (p[1] as f64 - at[1]).abs() < 1e-6)).map(|e| e.id).expect("the upright edge")
}

/// A drill of radius `r` from z = `from` up `h`, standing at `at`.
pub(super) struct Drill {
    pub(super) at: [f64; 2],
    pub(super) r: f64,
    pub(super) from: f64,
    pub(super) h: f64,
}

/// `s` drilled by `d`.
pub(super) fn drilled(s: Shape, d: Drill) -> Shape {
    s.combined(&placed(&specimen::cylinder(d.r, d.h).expect("a drill"), [d.at[0], d.at[1], d.from]), 0).expect("drilled")
}

/// The window in Ukrainian, the view drawn by the processor as the pictures are.
pub(super) fn window() -> App {
    crate::i18n::set_language("uk");
    let mut app = App::default();
    app.set.gpu_viewport = false;
    app
}

/// Frames run, as they do by themselves, until nothing is being built.
pub(super) fn at_rest(h: &mut Hand) {
    for _ in 0..600 {
        h.frame(Vec::new());
        h.app.drain_bg_for_test();
        if h.app.regen.busy.is_none() && h.app.regen.bg.is_empty() && !h.app.regen.wanted {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// Enter once the trial of the value is back - the button waits for it, as a person waits for it to light.
pub(super) fn enter(h: &mut Hand) {
    at_rest(h);
    let _ = h.trial_says();
    h.key(egui::Key::Enter);
    at_rest(h);
}

/// `value` typed into the field after the label `key`, all it held replaced; typed where the keyboard is when that
/// label is not on screen.
pub(super) fn type_in(h: &mut Hand, key: &str, value: f64) {
    let text = value.to_string();
    if !h.type_after_word(&crate::i18n::tr(key), &text) {
        println!("  no field after {key}: typed where the keyboard is");
        h.type_text(&text);
    }
}

/// The row of the tree whose words start with `head`, opened again by a double click; where it stands.
pub(super) fn reopen_row(h: &mut Hand, head: &str) -> Option<egui::Pos2> {
    h.frame(Vec::new());
    let at = h.words_drawn().iter().find(|(t, r)| t.starts_with(head) && r.left() < 340.0).map(|(_, r)| r.center());
    match at {
        Some(at) => {
            h.double_click_screen(at);
            at_rest(h);
        }
        None => println!("  no row starting {head:?}"),
    }
    at
}

/// The tool with the hint `hint` taken by its button, the picture `name` taken of the button with its hint up.
pub(super) fn take_tool(h: &mut Hand, shots: &mut Shots, hint: &str, name: Option<&str>) {
    let at = h.button_of(&crate::i18n::tr(hint));
    if let Some(name) = name {
        shots.take(h, name, at);
    }
    if let Some(at) = at {
        h.click_screen(at);
    }
    at_rest(h);
}

/// A click on a place of the scene, the pointer brought to rest over it first; where it landed on screen.
pub(super) fn click(h: &mut Hand, world: [f64; 3]) -> egui::Pos2 {
    let on = point_at(h, world);
    h.press3d(world, egui::Modifiers::NONE);
    on
}

/// `file` brought in by File -> Import as millimetres, its part entered.
pub(super) fn brought_in(h: &mut Hand, file: &Written) {
    h.import(&file.path);
    h.press_word(&crate::i18n::tr("import-scale-import"), egui::pos2(700.0, 450.0));
    at_rest(h);
    let body = h.app.project.bodies.last().map(|b| b.id).expect("the file came in");
    if let Some(part) = h.app.project.body_owner(body) {
        h.app.enter_component(part);
    }
    at_rest(h);
}

/// The pointer brought to rest over a place of the scene: the face under it lights up as it does before a click.
pub(super) fn point_at(h: &mut Hand, world: [f64; 3]) -> egui::Pos2 {
    let at = h.on_screen3d(world);
    h.frame(vec![egui::Event::PointerMoved(at)]);
    at
}

/// A point on the wall of a hole of radius `r` whose axis passes through `centre`, at the height of `centre`, on the
/// side the eye looks at; a negative `r` is the outside of a round boss, the side facing the eye.
pub(super) fn wall_seen(h: &Hand, centre: [f64; 3], r: f64) -> [f64; 3] {
    let yaw = h.app.viewing.cam.yaw;
    [centre[0] - r * yaw.cos(), centre[1] - r * yaw.sin(), centre[2]]
}

/// THE PICTURES OF ONE CHECK, numbered in the order they are taken.
pub(super) struct Shots {
    dir: std::path::PathBuf,
    n: usize,
}

impl Shots {
    pub(super) fn new(check: &str) -> Self {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tutorial").join(check);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the folder of the pictures");
        Shots { dir, n: 0 }
    }

    /// The whole window as it stands, a red ring where the pointer is when `ring` is given; the words the window says
    /// printed beside the name.
    pub(super) fn take(&mut self, h: &mut Hand, name: &str, ring: Option<egui::Pos2>) {
        // the trial of the value comes back, and a box of a tool fades in over a few frames
        at_rest(h);
        let _ = h.trial_says();
        for _ in 0..30 {
            match ring {
                Some(at) => h.frame(vec![egui::Event::PointerMoved(at)]),
                None => h.frame(Vec::new()),
            };
        }
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, h.screen());
        let mut img = h.picture(screen);
        if let Some(at) = ring {
            ringed(&mut img, at);
        }
        let file = format!("{:02}-{name}.png", self.n);
        self.n += 1;
        let png = crate::gui::color_image_to_png(&img).expect("encoding the PNG");
        std::fs::write(self.dir.join(&file), png).expect("writing the picture");
        let says: Vec<String> = h.tool_says().into_iter().map(|n| n.words).collect();
        println!("SHOT {file}\n  status: {}\n  says: {says:?}", h.app.status);
    }
}

/// A red ring of 18 px round `at`, 3 px wide.
fn ringed(img: &mut egui::ColorImage, at: egui::Pos2) {
    let [w, h] = img.size;
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 - at.x).powi(2) + (y as f32 - at.y).powi(2)).sqrt();
            if (16.5..=19.5).contains(&d) {
                img.pixels[y * w + x] = egui::Color32::from_rgb(230, 30, 30);
            }
        }
    }
}
