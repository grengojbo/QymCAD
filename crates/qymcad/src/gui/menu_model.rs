//! THE MENU BAR AS ONE LIST: what each menu holds, in which order, under which words, with which key and
//! whether it can be chosen now.
//!
//! The bar used to be written as drawing code, item after item, each with its own `if ui.button(..)` and
//! its own request. A second place that shows the same menu (the menu bar of the system) would have been a
//! second copy of that code, and two copies drift: an item added to one is missing from the other, an item
//! disabled in one is live in the other. Here the menu is said once, as data; whoever shows it reads this
//! list, and whatever is chosen goes through `apply_menu_action`, the one place that knows what an item does.

use crate::gui::export_menu::{ExportChoice, ExportFrom};
use qymcad_ui_state::{BarAsk, BarCtx, ExportTarget, Nav, Want, WinKind};

/// What an item of the menu does when it is chosen.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MenuAction {
    New,
    NewFromTemplate,
    SaveAsTemplate,
    Open,
    /// Open the file at this path, one of the recent ones.
    OpenRecent(String),
    ClearRecent,
    Save,
    DocProps,
    SaveAs,
    Import,
    Export(ExportChoice),
    Quit,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    Rebuild,
    Orbit3d,
    FitView,
    Scheme(SchemeChoice),
    Settings,
    PartsLibrary,
    Start,
    Help,
    Hotkeys,
    CheckUpdates,
    /// The window that connects the program to Claude through MCP.
    ConnectClaude,
    Report,
    About,
}

/// A colour scheme the View menu offers: its stable identifier and how it is marked.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SchemeChoice {
    pub id: String,
    pub look: SchemeLook,
}

/// How a scheme is marked in the menu: a built-in light one, a built-in dark one, or one of the person's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SchemeLook {
    Light,
    Dark,
    Own,
}

/// Whether an item can be chosen now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Enabled {
    Yes,
    No,
}

impl Enabled {
    fn when(on: bool) -> Self {
        if on {
            Enabled::Yes
        } else {
            Enabled::No
        }
    }
}

/// Whether a check item stands ticked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Checked {
    Yes,
    No,
}

/// Whether an item that is not always there is offered at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Offered {
    Yes,
    No,
}

/// The modifiers of a menu key. `Command` is Ctrl on Windows and Linux and Cmd on macOS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Chord {
    Command,
    CommandShift,
}

/// The key an item answers to, shown beside its caption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Shortcut {
    pub chord: Chord,
    pub key: egui::Key,
}

impl Shortcut {
    const fn command(key: egui::Key) -> Self {
        Self { chord: Chord::Command, key }
    }

    const fn command_shift(key: egui::Key) -> Self {
        Self { chord: Chord::CommandShift, key }
    }

    /// The key as the menu of the window writes it.
    pub(crate) fn label(self) -> String {
        let held = match self.chord {
            Chord::Command => "Ctrl+",
            Chord::CommandShift => "Ctrl+Shift+",
        };
        format!("{held}{}", self.key.name())
    }
}

/// One item: what it does, the words it is shown with, its key, whether it can be chosen, its hint.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MenuItem {
    pub action: MenuAction,
    pub caption: String,
    pub shortcut: Option<Shortcut>,
    pub enabled: Enabled,
    pub hint: Option<String>,
}

impl MenuItem {
    fn new(action: MenuAction, caption: String) -> Self {
        Self { action, caption, shortcut: None, enabled: Enabled::Yes, hint: None }
    }

    fn key(mut self, shortcut: Shortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    fn enabled(mut self, enabled: Enabled) -> Self {
        self.enabled = enabled;
        self
    }

    fn hint(mut self, hint: String) -> Self {
        self.hint = Some(hint);
        self
    }
}

/// How a line of words in a menu is set: dimmed, or as plain text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoteTone {
    Weak,
    Plain,
}

/// Which submenu this is: each has an icon of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SubKind {
    Recent,
    Export,
}

/// A menu inside a menu.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SubMenu {
    pub kind: SubKind,
    pub caption: String,
    pub nodes: Vec<MenuNode>,
}

/// One line of a menu.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MenuNode {
    Item(MenuItem),
    /// An item with a tick that stays in the menu: choosing it switches the tick.
    Check {
        item: MenuItem,
        checked: Checked,
    },
    Sub(SubMenu),
    Separator,
    /// Words that are not a command: a heading, or "nothing here".
    Note {
        text: String,
        tone: NoteTone,
    },
}

/// Which menu of the bar this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuRole {
    File,
    Edit,
    View,
    Windows,
    Help,
}

impl MenuRole {
    /// The catalogue key of the menu's title.
    pub(crate) fn key(self) -> &'static str {
        match self {
            MenuRole::File => "menu-file",
            MenuRole::Edit => "menu-edit",
            MenuRole::View => "menu-view",
            MenuRole::Windows => "menu-windows",
            MenuRole::Help => "menu-help",
        }
    }
}

/// One menu of the bar.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TopMenu {
    pub role: MenuRole,
    pub caption: String,
    pub nodes: Vec<MenuNode>,
}

/// The name of the step an undo or a redo would take, if there is one.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Step {
    None,
    Named(String),
}

/// WHAT THE MENU DEPENDS ON, read out of the application once per frame. Owned rather than borrowed: the
/// menu is built from it, and two of these compare to tell whether the menu changed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MenuState {
    pub undo: Step,
    pub redo: Step,
    pub copy: Enabled,
    pub paste: Enabled,
    pub rebuild: Enabled,
    pub recent: Vec<String>,
    pub schemes: Vec<SchemeRow>,
    pub orbit: Checked,
    pub updates: Offered,
}

/// A scheme as the View menu lists it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SchemeRow {
    pub choice: SchemeChoice,
    pub title: String,
}

/// The state the menu is built from, read out of the bar's context.
pub(crate) fn menu_state(bc: &BarCtx) -> MenuState {
    let step = |s: Option<&qymcad_ui_state::Step>| s.map_or(Step::None, |s| Step::Named(s.name.clone()));
    // THE SCHEMES COME FROM THE LIVE LIST (the built-in ones and the user's), and the label from `title()`:
    // a built-in scheme has NO name of its own, it comes from the language catalogue.
    let schemes = bc.scheme.all.iter().map(|p| SchemeRow { choice: SchemeChoice { id: p.id.clone(), look: scheme_look(&p.id, p.light) }, title: p.title() }).collect();
    MenuState {
        undo: step(bc.edits.undo.last()),
        redo: step(bc.edits.redo.last()),
        copy: Enabled::when(qymcad_ui_state::clipboard_can_copy(&*bc.project, *bc.sel, &*bc.sel_sk, *bc.sketch_ses)),
        paste: Enabled::when(bc.clip.tree.is_some() || bc.clip.geom.is_some()),
        rebuild: Enabled::when(!bc.project.timeline.is_empty()),
        recent: bc.set.recent.clone(),
        schemes,
        orbit: if *bc.mode_3d { Checked::Yes } else { Checked::No },
        // CHECK FOR UPDATES is absent where it cannot work: inside Flatpak there is no network, and a build
        // with no release tag has nothing to compare against.
        updates: if crate::gui::update_ui::available() { Offered::Yes } else { Offered::No },
    }
}

fn scheme_look(id: &str, light: bool) -> SchemeLook {
    if !qymcad_scheme::store::is_builtin(id) {
        SchemeLook::Own
    } else if light {
        SchemeLook::Light
    } else {
        SchemeLook::Dark
    }
}

fn item(action: MenuAction, key: &str) -> MenuItem {
    MenuItem::new(action, qymcad_i18n::tr(key))
}

/// THE MENU BAR, said once.
pub(crate) fn menu_model(state: &MenuState) -> Vec<TopMenu> {
    vec![file_menu(state), edit_menu(state), view_menu(state), windows_menu(), help_menu(state)]
}

fn top(role: MenuRole, nodes: Vec<MenuNode>) -> TopMenu {
    TopMenu { role, caption: qymcad_i18n::tr(role.key()), nodes }
}

fn file_menu(state: &MenuState) -> TopMenu {
    use MenuNode::{Item, Separator, Sub};
    // ONE DOOR FOR EVERY FORMAT: the file's extension decides what it becomes (see `import_door`)
    let formats = qymcad_io::Format::names_of(&qymcad_io::Format::ALL);
    top(
        MenuRole::File,
        vec![
            // A NEW PROJECT IS AN EMPTY ASSEMBLY: a part is made by "New part" of the start screen or of the
            // assembly, by the person's own intent.
            Item(item(MenuAction::New, "file-new")),
            // TEMPLATES ARE CHOSEN IN THE CHOOSER, opened in the folder of templates.
            Item(item(MenuAction::NewFromTemplate, "file-new-from-template")),
            Item(item(MenuAction::SaveAsTemplate, "file-save-as-template").hint(qymcad_i18n::tr("file-save-as-template-hint"))),
            Separator,
            Item(item(MenuAction::Open, "file-open")),
            Sub(recent_menu(state)),
            Item(item(MenuAction::Save, "file-save").key(Shortcut::command(egui::Key::S))),
            Item(item(MenuAction::DocProps, "file-doc-props")),
            Item(item(MenuAction::SaveAs, "file-save-as").key(Shortcut::command_shift(egui::Key::S))),
            Separator,
            Item(item(MenuAction::Import, "file-import").hint(qymcad_i18n::tr1("file-import-hint", "formats", &formats))),
            Separator,
            Sub(export_menu()),
            Separator,
            Item(item(MenuAction::Quit, "file-quit")),
        ],
    )
}

/// RECENT FILES: the submenu always opens. Empty, it says so in words, and "Clear the list" stands in it
/// disabled. A disabled item said nothing at all on a clean start.
fn recent_menu(state: &MenuState) -> SubMenu {
    let mut nodes = Vec::new();
    if state.recent.is_empty() {
        nodes.push(MenuNode::Note { text: qymcad_i18n::tr("file-recent-empty"), tone: NoteTone::Weak });
    }
    for path in &state.recent {
        // the row shows THE FILE NAME with the full path in the hint: paths are longer than the menu
        let name = std::path::Path::new(path).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.clone());
        nodes.push(MenuNode::Item(MenuItem::new(MenuAction::OpenRecent(path.clone()), name).hint(path.clone())));
    }
    nodes.push(MenuNode::Separator);
    nodes.push(MenuNode::Item(item(MenuAction::ClearRecent, "file-recent-clear").enabled(Enabled::when(!state.recent.is_empty()))));
    SubMenu { kind: SubKind::Recent, caption: qymcad_i18n::tr("file-recent"), nodes }
}

/// ONE ITEM, THE FORMATS INSIDE IT: the exact formats first, the meshes below a line.
fn export_menu() -> SubMenu {
    let mut nodes = Vec::new();
    for choice in crate::gui::export_menu::choices() {
        if matches!(choice, ExportChoice::Mesh(qymcad_ui_state::MeshFormat::Stl)) {
            nodes.push(MenuNode::Separator);
        }
        let caption = qymcad_i18n::tr1("file-export-as", "format", crate::gui::export_menu::name_of(choice));
        let hint = qymcad_i18n::tr(crate::gui::export_menu::hint_of(choice, ExportFrom::Project));
        nodes.push(MenuNode::Item(MenuItem::new(MenuAction::Export(choice), caption).hint(hint)));
    }
    SubMenu { kind: SubKind::Export, caption: qymcad_i18n::tr("file-export"), nodes }
}

/// The caption of an undo or a redo: the name of the step it would take, when there is one.
fn step_caption(step: &Step, plain: &str, with_name: &str) -> String {
    match step {
        Step::Named(what) => qymcad_i18n::tr1(with_name, "what", what),
        Step::None => qymcad_i18n::tr(plain),
    }
}

fn edit_menu(state: &MenuState) -> TopMenu {
    use MenuNode::{Item, Separator};
    // THE OPERATION'S NAME IN THE MENU: what exactly will be undone is visible - a step knows its own name,
    // because it was created by a command rather than by the frame.
    let able = |step: &Step| Enabled::when(matches!(step, Step::Named(_)));
    let undo = MenuItem::new(MenuAction::Undo, step_caption(&state.undo, "menu-undo", "menu-undo-named"));
    let redo = MenuItem::new(MenuAction::Redo, step_caption(&state.redo, "menu-redo", "menu-redo-named"));
    top(
        MenuRole::Edit,
        vec![
            Item(undo.key(Shortcut::command(egui::Key::Z)).enabled(able(&state.undo))),
            Item(redo.key(Shortcut::command_shift(egui::Key::Z)).enabled(able(&state.redo))),
            Separator,
            // The clipboard: sketches, parts and subassemblies in the tree, or geometry in the sketch editor.
            Item(item(MenuAction::Copy, "menu-copy").key(Shortcut::command(egui::Key::C)).enabled(state.copy)),
            Item(item(MenuAction::Cut, "menu-cut").key(Shortcut::command(egui::Key::X)).enabled(state.copy)),
            Item(item(MenuAction::Paste, "win-insert").key(Shortcut::command(egui::Key::V)).enabled(state.paste)),
            Separator,
            // REBUILD EVERYTHING. The file stores finished meshes and computes nothing anew on opening, so a
            // part built by an older version of the kernel stays as it was until this is chosen.
            Item(item(MenuAction::Rebuild, "menu-rebuild").enabled(state.rebuild).hint(qymcad_i18n::tr("menu-rebuild-hint"))),
        ],
    )
}

fn view_menu(state: &MenuState) -> TopMenu {
    let mut nodes = vec![
        MenuNode::Check { item: item(MenuAction::Orbit3d, "menu-orbit3d"), checked: state.orbit },
        MenuNode::Item(item(MenuAction::FitView, "menu-fit-view")),
        MenuNode::Separator,
        MenuNode::Note { text: qymcad_i18n::tr("settings-scheme"), tone: NoteTone::Plain },
    ];
    for row in &state.schemes {
        nodes.push(MenuNode::Item(MenuItem::new(MenuAction::Scheme(row.choice.clone()), row.title.clone())));
    }
    top(MenuRole::View, nodes)
}

fn windows_menu() -> TopMenu {
    top(
        MenuRole::Windows,
        vec![
            MenuNode::Item(item(MenuAction::Settings, "win-settings")),
            MenuNode::Item(item(MenuAction::PartsLibrary, "win-parts-library")),
            MenuNode::Item(item(MenuAction::Start, "win-start")),
        ],
    )
}

fn help_menu(state: &MenuState) -> TopMenu {
    let mut nodes = vec![MenuNode::Item(item(MenuAction::Help, "help-title")), MenuNode::Separator, MenuNode::Item(item(MenuAction::Hotkeys, "help-hotkeys"))];
    // CHECK FOR UPDATES. Chosen by hand it asks ALWAYS - even with the automatic check switched off, because
    // choosing it IS the asking.
    if state.updates == Offered::Yes {
        nodes.push(MenuNode::Item(item(MenuAction::CheckUpdates, "help-check-updates")));
    }
    nodes.push(MenuNode::Item(item(MenuAction::ConnectClaude, "help-connect-claude")));
    nodes.push(MenuNode::Item(item(MenuAction::Report, "help-report")));
    nodes.push(MenuNode::Item(item(MenuAction::About, "help-about")));
    top(MenuRole::Help, nodes)
}

/// WHAT AN ITEM DOES, in one place for every menu that shows it.
pub(crate) fn apply_menu_action(action: &MenuAction, bc: &mut BarCtx, ctx: &egui::Context) {
    match action {
        MenuAction::New => bc.ask.push(BarAsk::Nav(Nav::NewAssembly)),
        MenuAction::NewFromTemplate => bc.ask.push(BarAsk::Nav(Nav::NewFromTemplate)),
        MenuAction::SaveAsTemplate => {
            bc.win.tpl_name = bc.project.meta.title.clone();
            bc.win.open(WinKind::SaveTemplate);
        }
        MenuAction::Open => bc.ask.push(BarAsk::Nav(Nav::OpenDialog)),
        MenuAction::OpenRecent(path) => bc.ask.push(BarAsk::Nav(Nav::OpenPath(path.clone()))),
        MenuAction::ClearRecent => bc.set.recent.clear(),
        MenuAction::Save => bc.ask.push(BarAsk::Save),
        MenuAction::DocProps => bc.win.open(WinKind::DocProps),
        MenuAction::SaveAs => bc.ask.push(BarAsk::SaveAs),
        MenuAction::Import => bc.ask.push(BarAsk::Import(Want::Anything)),
        MenuAction::Export(ExportChoice::Exact(f)) => bc.ask.push(BarAsk::ExportExact(*f, ExportTarget::Project)),
        MenuAction::Export(ExportChoice::Mesh(f)) => *bc.mesh_export = Some((*f, ExportTarget::Project)),
        MenuAction::Quit => bc.ask.push(BarAsk::Nav(Nav::Exit)),
        MenuAction::Undo => bc.ask.push(BarAsk::Undo),
        MenuAction::Redo => bc.ask.push(BarAsk::Redo),
        MenuAction::Copy => bc.ask.push(BarAsk::Clipboard { cut: false }),
        MenuAction::Cut => bc.ask.push(BarAsk::Clipboard { cut: true }),
        MenuAction::Paste => bc.ask.push(BarAsk::Paste),
        MenuAction::Rebuild => bc.ask.push(BarAsk::RebuildEverything),
        MenuAction::Orbit3d => *bc.mode_3d = !*bc.mode_3d,
        MenuAction::FitView => {
            bc.view.initialized = false;
            bc.cam.init = false;
        }
        MenuAction::Scheme(choice) => {
            bc.set.scheme = choice.id.clone();
            qymcad_ui_state::apply_theme(&mut *bc.scheme, &*bc.set, ctx);
        }
        MenuAction::Settings => {
            bc.win.toggle(WinKind::Settings);
        }
        MenuAction::PartsLibrary => bc.ask.push(BarAsk::ToggleLibrary),
        // it was ASKED for rather than raising itself - see `start_screen_visible`
        MenuAction::Start => bc.win.start_asked = true,
        MenuAction::Help => bc.ask.push(BarAsk::Help("index".to_string())),
        MenuAction::Hotkeys => bc.win.open(WinKind::Hotkeys),
        MenuAction::CheckUpdates => {
            crate::gui::update_ui::ask(bc.set);
            bc.win.open(WinKind::Updates);
        }
        MenuAction::ConnectClaude => bc.win.open(WinKind::Claude),
        MenuAction::Report => bc.win.open(WinKind::Report),
        MenuAction::About => bc.win.open(WinKind::About),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::hand::Hand;
    use crate::gui::App;

    /// A fresh document: nothing to undo, nothing recent, nothing on the clipboard.
    fn quiet() -> MenuState {
        MenuState { undo: Step::None, redo: Step::None, copy: Enabled::No, paste: Enabled::No, rebuild: Enabled::No, recent: Vec::new(), schemes: Vec::new(), orbit: Checked::No, updates: Offered::No }
    }

    /// Every item of the bar, submenus walked into.
    fn items(model: &[TopMenu]) -> Vec<MenuItem> {
        fn walk(nodes: &[MenuNode], out: &mut Vec<MenuItem>) {
            for node in nodes {
                match node {
                    MenuNode::Item(item) | MenuNode::Check { item, .. } => out.push(item.clone()),
                    MenuNode::Sub(sub) => walk(&sub.nodes, out),
                    MenuNode::Separator | MenuNode::Note { .. } => {}
                }
            }
        }
        let mut out = Vec::new();
        model.iter().for_each(|m| walk(&m.nodes, &mut out));
        out
    }

    fn item_of(state: &MenuState, action: &MenuAction) -> Option<MenuItem> {
        items(&menu_model(state)).into_iter().find(|i| i.action == *action)
    }

    fn notes(nodes: &[MenuNode]) -> Vec<String> {
        nodes.iter().filter_map(|n| if let MenuNode::Note { text, .. } = n { Some(text.clone()) } else { None }).collect()
    }

    fn sub(state: &MenuState, kind: SubKind) -> SubMenu {
        let model = menu_model(state);
        let file = model.iter().find(|m| m.role == MenuRole::File).expect("the File menu");
        file.nodes.iter().find_map(|n| if let MenuNode::Sub(s) = n { (s.kind == kind).then(|| s.clone()) } else { None }).expect("the submenu is in the File menu")
    }

    /// THE FIVE MENUS, in their order, titled from the catalogue.
    #[test]
    fn the_bar_holds_five_menus_titled_from_the_catalogue() {
        let model = menu_model(&quiet());
        let roles: Vec<MenuRole> = model.iter().map(|m| m.role).collect();
        assert_eq!(roles, [MenuRole::File, MenuRole::Edit, MenuRole::View, MenuRole::Windows, MenuRole::Help]);
        for menu in &model {
            assert_eq!(menu.caption, qymcad_i18n::tr(menu.role.key()), "the {:?} menu must take its title from the catalogue", menu.role);
        }
    }

    /// UNDO SAYS WHAT IT WILL UNDO, and with nothing to undo it stands disabled under its plain name.
    #[test]
    fn undo_names_its_step_and_waits_without_one() {
        let empty = item_of(&quiet(), &MenuAction::Undo).expect("Undo is in the bar");
        assert_eq!(empty.caption, qymcad_i18n::tr("menu-undo"));
        assert_eq!(empty.enabled, Enabled::No, "nothing to undo: the item must not be choosable");
        let state = MenuState { undo: Step::Named("Extrude".into()), redo: Step::Named("Fillet".into()), ..quiet() };
        let undo = item_of(&state, &MenuAction::Undo).expect("Undo is in the bar");
        assert_eq!(undo.caption, qymcad_i18n::tr1("menu-undo-named", "what", "Extrude"), "the item must name the step it undoes");
        assert_eq!(undo.enabled, Enabled::Yes);
        let redo = item_of(&state, &MenuAction::Redo).expect("Redo is in the bar");
        assert_eq!(redo.caption, qymcad_i18n::tr1("menu-redo-named", "what", "Fillet"));
        assert_eq!(redo.enabled, Enabled::Yes);
        assert_eq!(undo.shortcut.map(Shortcut::label).as_deref(), Some("Ctrl+Z"));
        assert_eq!(redo.shortcut.map(Shortcut::label).as_deref(), Some("Ctrl+Shift+Z"));
    }

    /// AN EMPTY LIST OF RECENT FILES SAYS SO IN WORDS, and its "clear" cannot be chosen; a full one lists the
    /// files by name with the whole path as the hint.
    #[test]
    fn the_recent_files_say_what_they_hold() {
        let empty = sub(&quiet(), SubKind::Recent);
        assert_eq!(notes(&empty.nodes), [qymcad_i18n::tr("file-recent-empty")], "an empty list must say it is empty");
        let clear = item_of(&quiet(), &MenuAction::ClearRecent).expect("the clear item is always there");
        assert_eq!(clear.enabled, Enabled::No, "an empty list cannot be cleared");

        let path = "/tmp/a/box.qcad".to_string();
        let state = MenuState { recent: vec![path.clone()], ..quiet() };
        let full = sub(&state, SubKind::Recent);
        assert!(notes(&full.nodes).is_empty(), "a list with a file must not say it is empty");
        let row = item_of(&state, &MenuAction::OpenRecent(path.clone())).expect("the recent file is listed");
        assert_eq!(row.caption, "box.qcad", "the row shows the file name");
        assert_eq!(row.hint, Some(path), "the whole path is the hint");
        assert_eq!(item_of(&state, &MenuAction::ClearRecent).map(|i| i.enabled), Some(Enabled::Yes));
    }

    /// THE CLIPBOARD ITEMS FOLLOW WHAT CAN BE COPIED AND WHAT IS HELD; Rebuild follows the timeline.
    #[test]
    fn the_edit_items_follow_the_document() {
        for able in [Enabled::No, Enabled::Yes] {
            let state = MenuState { copy: able, paste: able, rebuild: able, ..quiet() };
            for action in [MenuAction::Copy, MenuAction::Cut, MenuAction::Paste, MenuAction::Rebuild] {
                assert_eq!(item_of(&state, &action).map(|i| i.enabled), Some(able), "{action:?} must follow the document");
            }
        }
        let copy_only = MenuState { copy: Enabled::Yes, ..quiet() };
        assert_eq!(item_of(&copy_only, &MenuAction::Paste).map(|i| i.enabled), Some(Enabled::No), "Paste follows what is held, not what can be copied");
        assert_eq!(item_of(&copy_only, &MenuAction::Cut).map(|i| i.enabled), Some(Enabled::Yes), "Cut follows what can be copied");
    }

    /// CHECK FOR UPDATES is in the Help menu only where it can work.
    #[test]
    fn the_update_item_is_there_only_when_offered() {
        assert!(item_of(&quiet(), &MenuAction::CheckUpdates).is_none(), "where a check cannot work, the item must be absent");
        assert!(item_of(&MenuState { updates: Offered::Yes, ..quiet() }, &MenuAction::CheckUpdates).is_some());
    }

    /// THE SCHEMES ARE LISTED BY THEIR WORDS under a heading, the 3D orbit is a tick.
    #[test]
    fn the_view_menu_lists_the_schemes_by_name() {
        let dark = SchemeRow { choice: SchemeChoice { id: "dark".into(), look: SchemeLook::Dark }, title: "Dark".into() };
        let mine = SchemeRow { choice: SchemeChoice { id: "mine".into(), look: SchemeLook::Own }, title: "Mine".into() };
        let state = MenuState { schemes: vec![dark.clone(), mine.clone()], orbit: Checked::Yes, ..quiet() };
        let model = menu_model(&state);
        let view = model.iter().find(|m| m.role == MenuRole::View).expect("the View menu");
        assert_eq!(notes(&view.nodes), [qymcad_i18n::tr("settings-scheme")]);
        for row in [dark, mine] {
            let item = item_of(&state, &MenuAction::Scheme(row.choice.clone())).expect("every scheme is listed");
            assert_eq!(item.caption, row.title, "a scheme is listed by its words, not by an icon alone");
        }
        assert!(view.nodes.iter().any(|n| matches!(n, MenuNode::Check { item, checked: Checked::Yes } if item.action == MenuAction::Orbit3d)), "the 3D orbit stands ticked");
    }

    /// THE EXPORT SUBMENU: every format once, the exact ones above a line, the meshes below.
    #[test]
    fn the_export_submenu_lists_every_format_once() {
        let export = sub(&quiet(), SubKind::Export);
        let listed: Vec<ExportChoice> = export.nodes.iter().filter_map(|n| if let MenuNode::Item(MenuItem { action: MenuAction::Export(c), .. }) = n { Some(*c) } else { None }).collect();
        assert_eq!(listed, crate::gui::export_menu::choices().collect::<Vec<_>>());
        let line = export.nodes.iter().position(|n| *n == MenuNode::Separator).expect("a line between the exact formats and the meshes");
        assert!(matches!(&export.nodes[line + 1], MenuNode::Item(MenuItem { action: MenuAction::Export(ExportChoice::Mesh(qymcad_ui_state::MeshFormat::Stl)), .. })));
    }

    /// Press a menu of the bar, then an item in it, as a person does.
    fn choose(hand: &mut Hand, menu: &str, path: &[&str]) {
        assert!(hand.press_word(menu, egui::pos2(0.0, 0.0)), "the bar shows no \"{menu}\"");
        for word in path {
            assert!(hand.press_word(word, egui::pos2(0.0, 0.0)), "the open menu shows no \"{word}\"");
        }
    }

    /// WINDOWS -> SETTINGS, CLICKED IN A REAL FRAME, opens the settings.
    #[test]
    fn the_menu_opens_the_settings() {
        let mut app = App::default();
        let mut hand = Hand::new(&mut app);
        choose(&mut hand, &qymcad_i18n::tr("menu-windows"), &[&qymcad_i18n::tr("win-settings")]);
        assert!(app.win.is(WinKind::Settings), "the settings window did not open from the menu");
    }

    /// HELP -> CONNECT TO CLAUDE, CLICKED IN A REAL FRAME, opens the window that connects the program to Claude.
    #[test]
    fn the_menu_opens_the_claude_window() {
        let mut app = App::default();
        let mut hand = Hand::new(&mut app);
        choose(&mut hand, &qymcad_i18n::tr("menu-help"), &[&qymcad_i18n::tr("help-connect-claude")]);
        assert!(app.win.is(WinKind::Claude), "the Claude window did not open from the menu");
    }

    /// EDIT -> UNDO, CLICKED IN A REAL FRAME, takes back the step it names.
    #[test]
    fn the_menu_undoes_the_step_it_names() {
        let mut app = App::default();
        app.create_sketch_on(qymcad_core::feature::SketchPlane::default());
        let steps = app.disk.edits.undo.len();
        let name = app.disk.edits.undo.last().expect("setup: making a sketch is a step").name.clone();
        let mut hand = Hand::new(&mut app);
        choose(&mut hand, &qymcad_i18n::tr("menu-edit"), &[&qymcad_i18n::tr1("menu-undo-named", "what", &name)]);
        assert_eq!(app.disk.edits.undo.len(), steps - 1, "the step named in the menu was not undone");
    }

    /// FILE -> RECENT -> CLEAR, CLICKED IN A REAL FRAME, empties the list.
    #[test]
    fn the_menu_clears_the_recent_files() {
        let mut app = App::default();
        app.set.recent = vec!["/tmp/menu-check/box.qcad".to_string()];
        let mut hand = Hand::new(&mut app);
        choose(&mut hand, &qymcad_i18n::tr("menu-file"), &[&qymcad_i18n::tr("file-recent"), &qymcad_i18n::tr("file-recent-clear")]);
        assert!(app.set.recent.is_empty(), "the list of recent files was not cleared from the menu");
    }

    /// VIEW -> A SCHEME, CLICKED IN A REAL FRAME, becomes the scheme of the window.
    #[test]
    fn the_menu_switches_the_scheme() {
        let mut app = App::default();
        let other = app.scheme.all.iter().find(|p| p.id != app.set.scheme).cloned().expect("setup: a second scheme to switch to");
        let mut hand = Hand::new(&mut app);
        choose(&mut hand, &qymcad_i18n::tr("menu-view"), &[&other.title()]);
        assert_eq!(app.set.scheme, other.id, "the scheme chosen in the menu was not taken");
    }
}
