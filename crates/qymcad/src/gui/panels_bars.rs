//! THE BARS ACROSS THE TOP: the menu, the tool bars, the options of the active tool, the command bars.
//!
//! They used to sit inside the frame's body, where "what the frame does" and "what it draws" could not be told
//! apart and editing one command touched the whole life cycle of the frame.

pub(crate) use qymcad_part::*;
use super::*;

impl App {
    /// The command bar of the Part workbench, through its own doorway.
    pub(crate) fn feat_command_bar(&mut self, ui: &mut egui::Ui) {
        crate::gui::panels_bars::feat_command_bar(&mut self.part_ctx(), ui);
    }
}

/// THE MENU BAR OF THE WINDOW: the list said once in `menu_model`, drawn as egui menus. Whatever is chosen
/// goes through `apply_menu_action` after the drawing, the same door the menu bar of the system uses.
pub(crate) fn menu_bar(bc: &mut qymcad_ui_state::BarCtx, ui: &mut egui::Ui) {
    use crate::gui::bar_menu::BarMenu as _;
    // The panel lives inside a `Ui` now; the context is still wanted for windows, input and viewport
    // commands, and it comes from the same place.
    let ctx = &ui.ctx().clone();
    // THE MENU BAR OF THE SYSTEM catches up first, whichever menus it holds this frame.
    #[cfg(target_os = "macos")]
    crate::gui::native_menu::frame(bc, ctx);
    // WHERE THE SYSTEM BAR HOLDS THE MENUS, the window draws none.
    if crate::gui::menus_in_system_bar(bc.set) {
        return;
    }
    let model = crate::gui::menu_model::menu_model(&crate::gui::menu_model::menu_state(bc));
    let mut chosen = None;
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    egui::MenuBar::new().ui(ui, |ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        for menu in &model {
            ui.bar_menu_button(menu.caption.as_str(), |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                menu_nodes(ui, &menu.nodes, &mut chosen);
            });
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            if let Some(p) = &*bc.dxf_path {
                let fname = std::path::Path::new(p).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                ui.label(egui::RichText::new(format!("{} {fname}", ph::FILE)).weak());
            }
        });
    });
    if let Some(action) = chosen {
        crate::gui::menu_model::apply_menu_action(&action, bc, ctx);
    }
}

/// The lines of one menu, drawn; the item picked this frame lands in `chosen`.
fn menu_nodes(ui: &mut egui::Ui, nodes: &[crate::gui::menu_model::MenuNode], chosen: &mut Option<crate::gui::menu_model::MenuAction>) {
    use crate::gui::menu_model::{Checked, Enabled, MenuNode, NoteTone};
    for node in nodes {
        match node {
            MenuNode::Item(item) => {
                let mut button = egui::Button::new(with_glyph(action_glyph(&item.action), &item.caption));
                if let Some(key) = item.shortcut {
                    button = button.shortcut_text(key.label());
                }
                let mut response = ui.add_enabled(item.enabled == Enabled::Yes, button);
                if let Some(hint) = &item.hint {
                    response = response.on_hover_text(hint);
                }
                if response.clicked() {
                    *chosen = Some(item.action.clone());
                    ui.close();
                }
            }
            // a tick stays in the menu: choosing it does not close the menu
            MenuNode::Check { item, checked } => {
                let mut on = *checked == Checked::Yes;
                if ui.checkbox(&mut on, with_glyph(action_glyph(&item.action), &item.caption)).clicked() {
                    *chosen = Some(item.action.clone());
                }
            }
            MenuNode::Sub(sub) => {
                ui.menu_button(with_glyph(sub_glyph(sub.kind), &sub.caption), |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend); // a menu item is a command, not a paragraph
                    menu_nodes(ui, &sub.nodes, chosen);
                });
            }
            MenuNode::Separator => {
                ui.separator();
            }
            MenuNode::Note { text, tone: NoteTone::Weak } => {
                ui.label(egui::RichText::new(text).weak());
            }
            MenuNode::Note { text, tone: NoteTone::Plain } => {
                ui.label(text);
            }
        }
    }
}

/// An icon before the caption, when the item has one.
fn with_glyph(glyph: Option<&str>, caption: &str) -> String {
    match glyph {
        Some(g) => format!("{g}  {caption}"),
        None => caption.to_string(),
    }
}

/// The icon of a submenu of the menu bar.
pub(crate) fn sub_glyph(kind: crate::gui::menu_model::SubKind) -> Option<&'static str> {
    use crate::gui::menu_model::SubKind;
    Some(match kind {
        SubKind::Recent => ph::CLOCK_COUNTER_CLOCKWISE,
        SubKind::Export => ph::EXPORT,
    })
}

/// The icon of an item of the menu bar. A recent file and an export format are rows of a list and go without.
pub(crate) fn action_glyph(action: &crate::gui::menu_model::MenuAction) -> Option<&'static str> {
    use crate::gui::menu_model::{MenuAction as A, SchemeLook};
    match action {
        A::New => Some(ph::FILE),
        A::NewFromTemplate | A::DocProps => Some(ph::FILE_TEXT),
        A::SaveAsTemplate | A::PartsLibrary => Some(ph::PACKAGE),
        A::Open => Some(ph::FOLDER_OPEN),
        A::OpenRecent(_) | A::Export(_) => None,
        A::ClearRecent => Some(ph::TRASH),
        A::Save | A::SaveAs => Some(ph::FLOPPY_DISK),
        A::Import => Some(ph::FILE_ARROW_DOWN),
        A::Quit => Some(ph::SIGN_OUT),
        A::Undo => Some(ph::ARROW_COUNTER_CLOCKWISE),
        A::Redo | A::CheckUpdates => Some(ph::ARROW_CLOCKWISE),
        A::Copy => Some(ph::COPY),
        A::Cut => Some(ph::SCISSORS),
        A::Paste => Some(ph::CLIPBOARD),
        A::Rebuild => Some(ph::ARROWS_CLOCKWISE),
        A::Orbit3d => Some(ph::CUBE),
        A::FitView => Some(ph::CORNERS_OUT),
        A::Scheme(s) => Some(match s.look {
            SchemeLook::Light => ph::SUN,
            SchemeLook::Dark => ph::MOON,
            SchemeLook::Own => ph::PENCIL_SIMPLE,
        }),
        A::Settings => Some(ph::GEAR),
        A::Start => Some(ph::HOUSE),
        A::Help => Some(ph::BOOK_OPEN),
        A::Hotkeys => Some(ph::KEYBOARD),
        A::ConnectClaude => Some(ph::PLUGS_CONNECTED),
        A::Report => Some(ph::BUG),
        A::About => Some(ph::INFO),
    }
}

/// WHAT THE CHECK FOR A NEWER VERSION HAS TO SAY, in the status line.
///
/// Silent while nothing has been asked and while nothing was found: a line that says "no updates" for
/// ever is a line people stop reading, and the one time it matters they would not read it either.
///
/// A FAILED CHECK IS SILENT TOO, unless a person asked for it themselves. Somebody who pressed the menu
/// item is owed the answer, even the disappointing one; somebody who did not press anything has no
/// business being told that a request they never made did not go through.
fn update_note(scheme: &qymcad_ui_state::SchemeUi, win: &mut qymcad_ui_state::Windows, ui: &mut egui::Ui) {
    use qymcad_update::Outcome;
    let asked_by_hand = win.is(qymcad_ui_state::WinKind::Updates);
    match crate::gui::update_ui::outcome() {
        Outcome::Idle => {}
        Outcome::Asking => {
            ui.separator();
            ui.label(egui::RichText::new(crate::i18n::tr("update-asking")).weak());
        }
        Outcome::UpToDate if asked_by_hand => {
            ui.separator();
            ui.label(egui::RichText::new(crate::i18n::tr("update-none")).weak());
        }
        Outcome::Unreachable if asked_by_hand => {
            ui.separator();
            ui.label(egui::RichText::new(crate::i18n::tr("update-unreachable")).weak());
        }
        Outcome::UpToDate | Outcome::Unreachable => {}
        Outcome::Found(latest) => {
            ui.separator();
            // The badge leads where the menu item leads: seeing the news and being unable to act on it
            // from the same place is what makes people hunt through menus.
            let text = format!("{} {} {}", ph::ARROW_CIRCLE_UP, crate::i18n::tr("update-found"), latest.latest);
            if ui.button(egui::RichText::new(text).color(scheme.pal.hint_action())).clicked() {
                win.open(qymcad_ui_state::WinKind::Updates);
            }
        }
    }
}

/// THE STATUS LINE: what was just said, how defined the sketch is, the units and the cursor.
///
/// Takes what it reads and nothing else - six borrows rather than the whole application. THE SKETCH'S
/// DEFINEDNESS is the sketcher's main state, and in CAD it belongs in the status line: one looks at the
/// drawing rather than hunting for it in a panel off to the side.
pub(crate) fn status_bar(sc: &mut qymcad_ui_state::StatusCtx, ui: &mut egui::Ui) {
    let qymcad_ui_state::StatusCtx { cache, cursor, project, scheme, set, sketch_ses, status, win } = sc;
    let (cursor, status) = (*cursor, *status);
    // THE CHECK FOR A NEWER VERSION IS STARTED HERE, and shown here, so that the two cannot drift apart.
    //
    // A frame is where it belongs: nothing else in the program knows that a start happened, and this
    // costs one comparison of two numbers when there is nothing to do. It never waits for the network.
    crate::gui::update_ui::ask_if_due(set);
    ui.horizontal(|ui| {
        ui.label(status);
        update_note(scheme, win, ui);
        if let Some(sid) = sketch_ses.editing {
            if let Some(si) = project.sketches.iter().position(|s| s.id == sid) {
                let (line, col) = crate::gui::sketching::sketch_dof_line(cache, project, scheme, si);
                ui.separator();
                ui.label(egui::RichText::new(line).color(col));
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(crate::i18n::tr("bar-units-mm"));
            ui.separator();
            match cursor {
                Some(c) => ui.monospace(format!("X {:.2}  Y {:.2}", c.x, c.y)),
                None => ui.monospace("X —  Y —"),
            };
        });
    });
}
