//! THE TIME OF A FRAME, ON REQUEST: in the corner of the viewport, how long the processor worked on the last frame,
//! how many frames a second the window draws, and how much of a frame the 3D view took. Reported behaviour: on Windows
//! the program was "very slow", and nothing told the slow 3D view from a slow window, nor a driver from a setting -
//! every guess cost a round trip. With the numbers in the corner a person compares "GPU" and "CPU", antialiasing on
//! and off, by turning the model and reading.
//!
//! The processor's time comes from the framework (`eframe::Frame::info().cpu_usage`), which only the frame of the live
//! window has; the 3D view's is measured around its drawing. Both are kept per thread: the window draws on one.

use std::cell::Cell;
use std::time::Duration;

use qymcad_ui_state::FrameTime;

thread_local! {
    /// The processor's time on the last whole frame, seconds; none before the live window has drawn one.
    static FRAME: Cell<Option<f32>> = const { Cell::new(None) };
    /// The time the last drawing of the 3D view took.
    static VIEW: Cell<Option<Duration>> = const { Cell::new(None) };
}

/// The processor's time on the frame the framework last finished.
pub(crate) fn note_frame(cpu_seconds: Option<f32>) {
    FRAME.with(|f| f.set(cpu_seconds));
}

/// The time the 3D view just took to draw.
pub(crate) fn note_view(took: Duration) {
    VIEW.with(|v| v.set(Some(took)));
}

/// THE LINE IN THE CORNER of `rect` in `colour`, when the setting asks for it: the frame's time, the frames a second,
/// the view's time. A number not measured yet is a dash, not a zero.
pub(crate) fn paint(painter: &egui::Painter, rect: egui::Rect, shown: FrameTime, colour: egui::Color32) {
    if shown == FrameTime::Hidden {
        return;
    }
    let ms = |s: Option<f32>| s.map_or_else(|| "-".to_string(), |s| qymcad_i18n::num(f64::from(s) * 1000.0, 1));
    let dt = painter.ctx().input(|i| i.stable_dt);
    let fps = if dt > 0.0 { qymcad_i18n::num(f64::from(1.0 / dt), 0) } else { "-".to_string() };
    let frame = ms(FRAME.with(Cell::get));
    let view = ms(VIEW.with(Cell::get).map(|d| d.as_secs_f32()));
    let line = crate::i18n::trn("frame-time", &[("frame", frame.as_str()), ("fps", fps.as_str()), ("view", view.as_str())]);
    painter.text(rect.left_bottom() + egui::vec2(8.0, -8.0), egui::Align2::LEFT_BOTTOM, line, egui::FontId::monospace(12.0), colour);
}

#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use qymcad_ui_state::FrameTime;

    /// THE TIME OF A FRAME IS IN THE CORNER WHEN ASKED FOR, AND ONLY THEN: whole frames of the window over a plate,
    /// with the setting shown and hidden. Shown, the line carries the processor's time the framework gave the frame,
    /// and a measured time of the 3D view rather than a dash.
    #[test]
    fn the_time_of_a_frame_is_shown_when_asked() {
        let prev = crate::i18n::language();
        crate::i18n::set_language("en");
        let head = crate::i18n::tr_prefix("frame-time", "frame");
        let mut app = crate::gui::screen_keys::tests::populated();

        app.set.frame_time = FrameTime::Hidden;
        let mut hand = Hand::new(&mut app);
        hand.frame(Vec::new());
        let hidden = hand.texts();
        drop(hand);

        app.set.frame_time = FrameTime::Shown;
        let mut hand = Hand::new(&mut app);
        super::note_frame(Some(0.0123));
        hand.frame(Vec::new());
        let shown = hand.texts();
        drop(hand);
        crate::i18n::set_language(&prev);

        let line = shown.iter().find(|t| t.starts_with(&head)).cloned();
        assert!(hidden.iter().all(|t| !t.starts_with(&head)), "the time of a frame is drawn with the setting hidden: {hidden:?}");
        let line = line.unwrap_or_else(|| panic!("the time of a frame is not in the view with the setting shown: {shown:?}"));
        assert!(line.contains("12.3"), "the processor's time of the frame is not the one the framework gave: {line}");
        assert!(!line.contains("3D - ms"), "the time of the 3D view was not measured: {line}");
    }
}
