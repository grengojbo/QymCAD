//! PAGEUP AND PAGEDOWN IN A FIELD ARE THE FIELD'S OWN: the keys a CAD mouse's driver sends for its side buttons zoom
//! the view, but not while a field takes the keyboard - a value typed at the geometry stands over the view, and the
//! hand is over the view while the keys are pressed in it.
//!
//! Driven through egui's own frames: a field given the focus, a canvas under the pointer, PageUp pressed.

/// What the frame of the view is asked to zoom when PageUp is pressed over the canvas, with a field beside it focused
/// (`Focus::Field`) or not.
#[derive(Clone, Copy)]
enum Focus {
    Field,
    Nothing,
}

fn zoom_asked(focus: Focus) -> f32 {
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
    let over = egui::pos2(400.0, 400.0);
    let mut text = String::from("12.5");
    let mut asked = 0.0;
    for frame in 0..3 {
        let events = match frame {
            2 => vec![
                egui::Event::PointerMoved(over),
                egui::Event::Key { key: egui::Key::PageUp, physical_key: Some(egui::Key::PageUp), pressed: true, repeat: false, modifiers: egui::Modifiers::NONE },
            ],
            _ => vec![egui::Event::PointerMoved(over)],
        };
        let raw = egui::RawInput { screen_rect: Some(screen), events, time: Some(frame as f64 * 0.016), ..Default::default() };
        let _ = ctx.run_ui(raw, |ui| {
            let field = ui.add(egui::TextEdit::singleline(&mut text));
            if frame == 0 && matches!(focus, Focus::Field) {
                field.request_focus();
            }
            let (resp, _) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
            asked = qymcad_ui_state::view_scroll(ui.ctx(), &resp, qymcad_ui_state::MouseNav::QymCad, false);
        });
    }
    asked
}

#[test]
fn page_up_zooms_the_view_only_while_no_field_takes_the_keyboard() {
    let free = zoom_asked(Focus::Nothing);
    assert!(free > 0.0, "GUARD: PageUp over the view with no field focused must zoom in; it asked {free}");
    let in_a_field = zoom_asked(Focus::Field);
    assert_eq!(in_a_field, 0.0, "PageUp pressed in a focused field zoomed the view by {in_a_field}");
}
