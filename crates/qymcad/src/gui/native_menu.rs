//! THE MENU BAR OF THE SYSTEM ON MACOS: the same list as the menu bar of the window, shown at the top of the
//! screen where a Mac program keeps its menus.
//!
//! The list comes from `menu_model`, and a chosen item goes through `apply_menu_action`, so the two bars cannot
//! drift apart. What differs is only the shape a Mac expects: an application menu first (About, Settings,
//! Services, Hide, Quit), a Window menu with Minimize and Zoom, and nothing repeated elsewhere.
//!
//! THE BAR IS NOT REBUILT EVERY FRAME. Each frame the list is built afresh (it is a few dozen strings) and
//! compared with the one on screen; only the items that changed are touched. Replacing the menu of the system
//! while a person has it open would close it under the pointer.
//!
//! The part that decides the shape and the changes is plain data and is checked on every system; the part that
//! talks to the system exists only on macOS.

use crate::gui::menu_model::{Checked, Enabled, MenuAction, MenuItem, MenuNode, MenuRole, Shortcut, SubKind, TopMenu};

/// An item the system itself carries out: the application and its windows, not the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SystemItem {
    Services,
    Hide,
    HideOthers,
    ShowAll,
    Minimize,
    Zoom,
}

/// One line of a menu of the system bar.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum NativeNode {
    Item(MenuItem),
    Check { item: MenuItem, checked: Checked },
    Sub { kind: SubKind, caption: String, nodes: Vec<NativeNode> },
    Separator,
    Note(String),
    System { item: SystemItem, caption: String },
}

/// Which menu of the system bar this is: the application's own, or one of the bar of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeRole {
    App,
    Bar(MenuRole),
}

/// One menu of the system bar.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeTop {
    pub role: NativeRole,
    pub caption: String,
    pub nodes: Vec<NativeNode>,
}

fn native(nodes: &[MenuNode]) -> Vec<NativeNode> {
    nodes
        .iter()
        .map(|node| match node {
            MenuNode::Item(item) => NativeNode::Item(item.clone()),
            MenuNode::Check { item, checked } => NativeNode::Check { item: item.clone(), checked: *checked },
            MenuNode::Sub(sub) => NativeNode::Sub { kind: sub.kind, caption: sub.caption.clone(), nodes: native(&sub.nodes) },
            MenuNode::Separator => NativeNode::Separator,
            MenuNode::Note { text, .. } => NativeNode::Note(text.clone()),
        })
        .collect()
}

/// The item of `action` in a menu of the window, if it stands there.
fn take(nodes: &mut Vec<NativeNode>, action: &MenuAction) -> Option<MenuItem> {
    let at = nodes.iter().position(|n| matches!(n, NativeNode::Item(item) if item.action == *action))?;
    match nodes.remove(at) {
        NativeNode::Item(item) => Some(item),
        _ => None,
    }
}

/// A line left with nothing below it, or two lines in a row, once items have moved out.
fn tidy(nodes: &mut Vec<NativeNode>) {
    while nodes.last() == Some(&NativeNode::Separator) {
        nodes.pop();
    }
    nodes.dedup_by(|a, b| *a == NativeNode::Separator && *b == NativeNode::Separator);
}

fn system(item: SystemItem, key: &str) -> NativeNode {
    NativeNode::System { item, caption: qymcad_i18n::tr1(key, "app", crate::gui::APP_NAME) }
}

/// THE MENU BAR IN THE SHAPE A MAC EXPECTS, made from the menu bar of the window.
///
/// About, Settings and Quit move into the application menu and are not repeated where the window keeps them;
/// the Window menu starts with what the system does to a window.
pub(crate) fn system_bar(model: &[TopMenu]) -> Vec<NativeTop> {
    let mut bar: Vec<NativeTop> = model.iter().map(|m| NativeTop { role: NativeRole::Bar(m.role), caption: m.caption.clone(), nodes: native(&m.nodes) }).collect();
    let mut moved = |role: MenuRole, action: MenuAction| bar.iter_mut().find(|t| t.role == NativeRole::Bar(role)).and_then(|t| take(&mut t.nodes, &action));
    let about = moved(MenuRole::Help, MenuAction::About);
    let settings = moved(MenuRole::Windows, MenuAction::Settings);
    let quit = moved(MenuRole::File, MenuAction::Quit);
    let app = crate::gui::APP_NAME;
    let mut nodes = Vec::new();
    if let Some(about) = about {
        nodes.push(NativeNode::Item(MenuItem { caption: qymcad_i18n::tr1("menu-app-about", "app", app), ..about }));
        nodes.push(NativeNode::Separator);
    }
    if let Some(settings) = settings {
        nodes.push(NativeNode::Item(MenuItem { caption: qymcad_i18n::tr("menu-app-settings"), shortcut: Some(Shortcut::command(egui::Key::Comma)), ..settings }));
        nodes.push(NativeNode::Separator);
    }
    nodes.extend([
        system(SystemItem::Services, "menu-app-services"),
        NativeNode::Separator,
        system(SystemItem::Hide, "menu-app-hide"),
        system(SystemItem::HideOthers, "menu-app-hide-others"),
        system(SystemItem::ShowAll, "menu-app-show-all"),
    ]);
    if let Some(quit) = quit {
        nodes.push(NativeNode::Separator);
        nodes.push(NativeNode::Item(MenuItem { caption: qymcad_i18n::tr1("menu-app-quit", "app", app), shortcut: Some(Shortcut::command(egui::Key::Q)), ..quit }));
    }
    for top in &mut bar {
        tidy(&mut top.nodes);
        if top.role == NativeRole::Bar(MenuRole::Windows) {
            top.caption = qymcad_i18n::tr("menu-window");
            let mut window = vec![system(SystemItem::Minimize, "menu-window-minimize"), system(SystemItem::Zoom, "menu-window-zoom")];
            if !top.nodes.is_empty() {
                window.push(NativeNode::Separator);
            }
            window.append(&mut top.nodes);
            top.nodes = window;
        }
    }
    bar.insert(0, NativeTop { role: NativeRole::App, caption: app.to_string(), nodes });
    bar
}

/// THE NAME AN ITEM GOES BY IN THE SYSTEM BAR: the event of a chosen item carries it back. Unique within the
/// bar, since two items never do the same thing.
pub(crate) fn id_of(action: &MenuAction) -> String {
    format!("{action:?}")
}

/// What the item named `id` does, read from the bar a person saw. An id no item has any more (the bar
/// changed between the click and this frame) does nothing.
pub(crate) fn resolve(bar: &[NativeTop], id: &str) -> Option<MenuAction> {
    fn walk(nodes: &[NativeNode], id: &str) -> Option<MenuAction> {
        nodes.iter().find_map(|node| match node {
            NativeNode::Item(item) | NativeNode::Check { item, .. } => (id_of(&item.action) == id).then(|| item.action.clone()),
            NativeNode::Sub { nodes, .. } => walk(nodes, id),
            NativeNode::Separator | NativeNode::Note(_) | NativeNode::System { .. } => None,
        })
    }
    bar.iter().find_map(|top| walk(&top.nodes, id))
}

/// What changes about one item on screen.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Edit {
    Text(String),
    Enabled(Enabled),
    Checked(Checked),
}

/// Where a change lands: an item, a submenu, or the title of a menu of the bar.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Target {
    Item(String),
    Sub(SubKind),
    Top(NativeRole),
}

/// One change to the bar on screen.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Change {
    pub target: Target,
    pub edit: Edit,
}

/// HOW THE BAR ON SCREEN CATCHES UP WITH THE LIST.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Refresh {
    /// Nothing changed.
    Same,
    /// The menus themselves are different: the bar is built anew.
    Whole,
    /// The menus named are built anew (their lines are different); the rest only change words, states and ticks.
    Parts { rebuild: Vec<NativeRole>, changes: Vec<Change> },
}

/// The lines of a menu with words, states and ticks left out: what a rebuild is needed for.
#[derive(PartialEq)]
enum Shape {
    Item(String),
    Check(String),
    Sub(SubKind, Vec<Shape>),
    Separator,
    Note(String),
    System(SystemItem),
}

fn shape(nodes: &[NativeNode]) -> Vec<Shape> {
    nodes
        .iter()
        .map(|node| match node {
            NativeNode::Item(item) => Shape::Item(id_of(&item.action)),
            NativeNode::Check { item, .. } => Shape::Check(id_of(&item.action)),
            NativeNode::Sub { kind, nodes, .. } => Shape::Sub(*kind, shape(nodes)),
            NativeNode::Separator => Shape::Separator,
            NativeNode::Note(text) => Shape::Note(text.clone()),
            NativeNode::System { item, .. } => Shape::System(*item),
        })
        .collect()
}

fn item_changes(old: &MenuItem, new: &MenuItem, out: &mut Vec<Change>) {
    let target = || Target::Item(id_of(&new.action));
    if old.caption != new.caption {
        out.push(Change { target: target(), edit: Edit::Text(new.caption.clone()) });
    }
    if old.enabled != new.enabled {
        out.push(Change { target: target(), edit: Edit::Enabled(new.enabled) });
    }
}

/// The changes between two menus of the same shape: the lines stand in the same order, so each old line is
/// compared with the new line in its place.
fn node_changes(old: &[NativeNode], new: &[NativeNode], out: &mut Vec<Change>) {
    for (was, now) in old.iter().zip(new) {
        match now {
            NativeNode::Item(n) => {
                if let NativeNode::Item(o) = was {
                    item_changes(o, n, out);
                }
            }
            NativeNode::Check { item: n, checked } => {
                if let NativeNode::Check { item: o, checked: before } = was {
                    item_changes(o, n, out);
                    if before != checked {
                        out.push(Change { target: Target::Item(id_of(&n.action)), edit: Edit::Checked(*checked) });
                    }
                }
            }
            NativeNode::Sub { kind, caption, nodes } => {
                if let NativeNode::Sub { caption: before, nodes: o, .. } = was {
                    if before != caption {
                        out.push(Change { target: Target::Sub(*kind), edit: Edit::Text(caption.clone()) });
                    }
                    node_changes(o, nodes, out);
                }
            }
            // a system item keeps the words it was built with; a separator and a note have none that change
            NativeNode::Separator | NativeNode::Note(_) | NativeNode::System { .. } => {}
        }
    }
}

/// WHAT HAS TO HAPPEN TO THE BAR ON SCREEN (`old`) FOR IT TO SHOW `new`.
pub(crate) fn plan_refresh(old: &[NativeTop], new: &[NativeTop]) -> Refresh {
    if old.len() != new.len() || old.iter().zip(new).any(|(o, n)| o.role != n.role) {
        return Refresh::Whole;
    }
    let mut rebuild = Vec::new();
    let mut changes = Vec::new();
    for (o, n) in old.iter().zip(new) {
        if shape(&o.nodes) != shape(&n.nodes) {
            rebuild.push(n.role);
            continue;
        }
        if o.caption != n.caption {
            changes.push(Change { target: Target::Top(n.role), edit: Edit::Text(n.caption.clone()) });
        }
        node_changes(&o.nodes, &n.nodes, &mut changes);
    }
    if rebuild.is_empty() && changes.is_empty() {
        Refresh::Same
    } else {
        Refresh::Parts { rebuild, changes }
    }
}

/// Whether the menu bar of this window is the one of the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Where {
    Window,
    SystemBar,
}

thread_local! {
    /// Set once the menu bar of the system has been taken over, on the thread that draws the window.
    static WHERE: std::cell::Cell<Where> = const { std::cell::Cell::new(Where::Window) };
}

/// Whether the menus live in the menu bar of the system rather than in the window.
pub(crate) fn in_system_bar() -> bool {
    WHERE.with(|w| w.get()) == Where::SystemBar
}

/// For the checks: the window behaves as if the system bar had been taken over, without touching the system.
#[cfg(test)]
pub(crate) fn pretend_system_bar() {
    WHERE.with(|w| w.set(Where::SystemBar));
}

#[cfg(target_os = "macos")]
pub(crate) use mac::{frame, install};

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{CheckMenuItem, PredefinedMenuItem, Submenu};
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// The items chosen in the system bar since the last frame, by name. Filled on whatever thread the system
    /// reports a choice on, emptied by the frame.
    static CHOSEN: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

    /// An item on screen that can change after it is built.
    enum Live {
        Plain(muda::MenuItem),
        Check(CheckMenuItem),
        Sub(Submenu),
    }

    /// The bar on screen: the menu of the system, its menus in order, its items by name, and the list it shows.
    struct Bar {
        menu: muda::Menu,
        tops: Vec<Submenu>,
        items: HashMap<String, Live>,
        sub_ids: HashMap<String, SubKind>,
        shown: Vec<NativeTop>,
    }

    thread_local! {
        static BAR: RefCell<Option<Bar>> = const { RefCell::new(None) };
    }

    /// TAKE OVER THE MENU BAR OF THE SYSTEM. Called once, on the main thread, after the window exists; the bar
    /// itself is built by the first frame, from the document as it is then.
    pub(crate) fn install(ctx: &egui::Context) {
        let ctx = ctx.clone();
        // a chosen item wakes the window: an idle window draws no frame, and the choice would wait for the mouse
        muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
            if let Ok(mut chosen) = CHOSEN.lock() {
                chosen.push(event.id.0);
            }
            ctx.request_repaint();
        }));
        BAR.with(|b| *b.borrow_mut() = Some(Bar { menu: muda::Menu::new(), tops: Vec::new(), items: HashMap::new(), sub_ids: HashMap::new(), shown: Vec::new() }));
        WHERE.with(|w| w.set(Where::SystemBar));
    }

    /// THE SYSTEM BAR'S PART OF A FRAME: what was chosen in it is done, then the bar catches up with the document.
    pub(crate) fn frame(bc: &mut qymcad_ui_state::BarCtx, ctx: &egui::Context) {
        let chosen: Vec<String> = CHOSEN.lock().map(|mut c| std::mem::take(&mut *c)).unwrap_or_default();
        let actions: Vec<MenuAction> = BAR.with(|b| b.borrow().as_ref().map(|bar| chosen.iter().filter_map(|id| resolve(&bar.shown, id)).collect()).unwrap_or_default());
        for action in &actions {
            crate::gui::menu_model::apply_menu_action(action, bc, ctx);
        }
        let new = system_bar(&crate::gui::menu_model::menu_model(&crate::gui::menu_model::menu_state(bc)));
        let failed = BAR.with(|b| {
            let mut b = b.borrow_mut();
            let Some(bar) = b.as_mut() else { return false };
            let built = catch_up(bar, new);
            built.is_err()
        });
        // A BAR THE SYSTEM REFUSED leaves the menus where they always were: in the window.
        if failed {
            BAR.with(|b| *b.borrow_mut() = None);
            WHERE.with(|w| w.set(Where::Window));
        }
    }

    fn catch_up(bar: &mut Bar, new: Vec<NativeTop>) -> muda::Result<()> {
        let plan = if bar.tops.is_empty() { Refresh::Whole } else { plan_refresh(&bar.shown, &new) };
        match plan {
            Refresh::Same => {}
            Refresh::Whole => {
                let menu = muda::Menu::new();
                let mut tops = Vec::new();
                bar.items.clear();
                bar.sub_ids.clear();
                for top in &new {
                    let sub = build_top(top, &mut bar.items, &mut bar.sub_ids)?;
                    menu.append(&sub)?;
                    tops.push(sub);
                }
                menu.init_for_nsapp();
                bar.menu = menu;
                bar.tops = tops;
                mark_roles(bar, &new);
            }
            Refresh::Parts { rebuild, changes } => {
                for role in &rebuild {
                    let Some(at) = new.iter().position(|t| t.role == *role) else { continue };
                    let sub = build_top(&new[at], &mut bar.items, &mut bar.sub_ids)?;
                    bar.menu.remove(&bar.tops[at])?;
                    bar.menu.insert(&sub, at)?;
                    bar.tops[at] = sub;
                }
                if !rebuild.is_empty() {
                    mark_roles(bar, &new);
                }
                for change in changes {
                    apply(bar, &new, change);
                }
            }
        }
        bar.shown = new;
        Ok(())
    }

    /// The Window menu and the Help menu are the system's own kinds: it lists the open windows in the first and
    /// puts a search over the menus into the second.
    fn mark_roles(bar: &Bar, tops: &[NativeTop]) {
        for (top, sub) in tops.iter().zip(&bar.tops) {
            match top.role {
                NativeRole::Bar(MenuRole::Windows) => sub.set_as_windows_menu_for_nsapp(),
                NativeRole::Bar(MenuRole::Help) => sub.set_as_help_menu_for_nsapp(),
                _ => {}
            }
        }
    }

    fn apply(bar: &Bar, tops: &[NativeTop], change: Change) {
        let live = match &change.target {
            Target::Item(id) => bar.items.get(id),
            Target::Sub(kind) => bar.sub_ids.iter().find(|(_, k)| *k == kind).and_then(|(id, _)| bar.items.get(id)),
            Target::Top(role) => {
                if let Edit::Text(text) = &change.edit {
                    if let Some(at) = tops.iter().position(|t| t.role == *role) {
                        bar.tops[at].set_text(text);
                    }
                }
                return;
            }
        };
        let Some(live) = live else { return };
        match change.edit {
            Edit::Text(text) => match live {
                Live::Plain(m) => m.set_text(text),
                Live::Check(m) => m.set_text(text),
                Live::Sub(s) => s.set_text(text),
            },
            Edit::Enabled(enabled) => match live {
                Live::Plain(m) => m.set_enabled(enabled == Enabled::Yes),
                Live::Check(m) => m.set_enabled(enabled == Enabled::Yes),
                Live::Sub(s) => s.set_enabled(enabled == Enabled::Yes),
            },
            Edit::Checked(checked) => {
                if let Live::Check(m) = live {
                    m.set_checked(checked == Checked::Yes);
                }
            }
        }
    }

    fn build_top(top: &NativeTop, items: &mut HashMap<String, Live>, sub_ids: &mut HashMap<String, SubKind>) -> muda::Result<Submenu> {
        let sub = Submenu::new(&top.caption, true);
        build_into(&sub, &top.nodes, items, sub_ids)?;
        Ok(sub)
    }

    fn build_into(sub: &Submenu, nodes: &[NativeNode], items: &mut HashMap<String, Live>, sub_ids: &mut HashMap<String, SubKind>) -> muda::Result<()> {
        for node in nodes {
            match node {
                NativeNode::Item(item) => {
                    let id = id_of(&item.action);
                    let m = muda::MenuItem::with_id(id.clone(), &item.caption, item.enabled == Enabled::Yes, accelerator(item));
                    sub.append(&m)?;
                    items.insert(id, Live::Plain(m));
                }
                NativeNode::Check { item, checked } => {
                    let id = id_of(&item.action);
                    let m = CheckMenuItem::with_id(id.clone(), &item.caption, item.enabled == Enabled::Yes, *checked == Checked::Yes, accelerator(item));
                    sub.append(&m)?;
                    items.insert(id, Live::Check(m));
                }
                NativeNode::Sub { kind, caption, nodes } => {
                    let inner = Submenu::new(caption, true);
                    build_into(&inner, nodes, items, sub_ids)?;
                    sub.append(&inner)?;
                    let id = inner.id().0.clone();
                    sub_ids.insert(id.clone(), *kind);
                    items.insert(id, Live::Sub(inner));
                }
                NativeNode::Separator => sub.append(&PredefinedMenuItem::separator())?,
                NativeNode::Note(text) => sub.append(&muda::MenuItem::new(text, false, None))?,
                NativeNode::System { item, caption } => {
                    let text = Some(caption.as_str());
                    let m = match item {
                        SystemItem::Services => PredefinedMenuItem::services(text),
                        SystemItem::Hide => PredefinedMenuItem::hide(text),
                        SystemItem::HideOthers => PredefinedMenuItem::hide_others(text),
                        SystemItem::ShowAll => PredefinedMenuItem::show_all(text),
                        SystemItem::Minimize => PredefinedMenuItem::minimize(text),
                        SystemItem::Zoom => PredefinedMenuItem::maximize(text),
                    };
                    sub.append(&m)?;
                }
            }
        }
        Ok(())
    }

    /// THE KEY AN ITEM ANSWERS TO IN THE SYSTEM BAR. Only the application's own keys: a key the window reads
    /// itself (Cmd+Z, Cmd+C, Cmd+S and the rest) would be taken by the menu before the window sees it, and a
    /// text field would lose its undo and its clipboard.
    fn accelerator(item: &MenuItem) -> Option<Accelerator> {
        if !matches!(item.action, MenuAction::Quit | MenuAction::Settings) {
            return None;
        }
        key_of(item.shortcut?)
    }

    /// A key of the menu as the system writes it, Cmd for Command.
    pub(super) fn key_of(shortcut: Shortcut) -> Option<Accelerator> {
        let code = match shortcut.key {
            egui::Key::Q => Code::KeyQ,
            egui::Key::Comma => Code::Comma,
            egui::Key::S => Code::KeyS,
            egui::Key::Z => Code::KeyZ,
            egui::Key::C => Code::KeyC,
            egui::Key::X => Code::KeyX,
            egui::Key::V => Code::KeyV,
            _ => return None,
        };
        let held = match shortcut.chord {
            crate::gui::menu_model::Chord::Command => Modifiers::SUPER,
            crate::gui::menu_model::Chord::CommandShift => Modifiers::SUPER | Modifiers::SHIFT,
        };
        Some(Accelerator::new(Some(held), code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::menu_model::{menu_model, MenuState, Offered, SchemeChoice, SchemeLook, SchemeRow, Step};

    fn quiet() -> MenuState {
        MenuState::quiet()
    }

    /// Everything the menus offer to choose, submenus walked into.
    fn actions(bar: &[NativeTop]) -> Vec<MenuAction> {
        fn walk(nodes: &[NativeNode], out: &mut Vec<MenuAction>) {
            for node in nodes {
                match node {
                    NativeNode::Item(item) | NativeNode::Check { item, .. } => out.push(item.action.clone()),
                    NativeNode::Sub { nodes, .. } => walk(nodes, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        bar.iter().for_each(|t| walk(&t.nodes, &mut out));
        out
    }

    fn top(bar: &[NativeTop], role: NativeRole) -> &NativeTop {
        bar.iter().find(|t| t.role == role).expect("the menu is in the bar")
    }

    fn systems(nodes: &[NativeNode]) -> Vec<SystemItem> {
        nodes.iter().filter_map(|n| if let NativeNode::System { item, .. } = n { Some(*item) } else { None }).collect()
    }

    fn busy() -> MenuState {
        let scheme = SchemeRow { choice: SchemeChoice { id: "dark".into(), look: SchemeLook::Dark }, title: "Dark".into() };
        MenuState { undo: Step::Named("Extrude".into()), recent: vec!["/tmp/a/box.qcad".into()], schemes: vec![scheme], updates: Offered::Yes, ..quiet() }
    }

    /// THE APPLICATION MENU COMES FIRST and holds what a Mac keeps there, in the order a Mac keeps it.
    #[test]
    fn the_application_menu_leads_the_bar() {
        let bar = system_bar(&menu_model(&busy()));
        assert_eq!(bar[0].role, NativeRole::App, "the first menu of a Mac bar is the application's own");
        let own: Vec<MenuAction> = actions(&bar[..1]);
        assert_eq!(own, [MenuAction::About, MenuAction::Settings, MenuAction::Quit]);
        assert_eq!(systems(&bar[0].nodes), [SystemItem::Services, SystemItem::Hide, SystemItem::HideOthers, SystemItem::ShowAll]);
        let quit = bar[0].nodes.last();
        assert!(matches!(quit, Some(NativeNode::Item(MenuItem { action: MenuAction::Quit, .. }))), "Quit closes the application menu");
        let roles: Vec<NativeRole> = bar.iter().map(|t| t.role).collect();
        let expected = [
            NativeRole::App,
            NativeRole::Bar(MenuRole::File),
            NativeRole::Bar(MenuRole::Edit),
            NativeRole::Bar(MenuRole::View),
            NativeRole::Bar(MenuRole::Windows),
            NativeRole::Bar(MenuRole::Help),
        ];
        assert_eq!(roles, expected);
    }

    /// NOTHING IS LOST AND NOTHING IS SAID TWICE: every item of the window's bar is in the system bar exactly once.
    #[test]
    fn every_item_of_the_window_is_in_the_system_bar_once() {
        let state = busy();
        let window: Vec<MenuAction> = actions(&menu_model(&state).iter().map(|m| NativeTop { role: NativeRole::Bar(m.role), caption: m.caption.clone(), nodes: native(&m.nodes) }).collect::<Vec<_>>());
        let system = actions(&system_bar(&menu_model(&state)));
        for action in &window {
            assert_eq!(system.iter().filter(|a| *a == action).count(), 1, "{action:?} must stand once in the system bar");
        }
        assert_eq!(system.len(), window.len(), "the system bar must not invent items of its own beyond what the system does");
    }

    /// THE WINDOW MENU OF A MAC starts with what the system does to a window; the File menu ends without Quit,
    /// and no menu ends on a line.
    #[test]
    fn the_window_menu_starts_with_the_system_and_no_menu_ends_on_a_line() {
        let bar = system_bar(&menu_model(&quiet()));
        let window = top(&bar, NativeRole::Bar(MenuRole::Windows));
        assert_eq!(window.caption, qymcad_i18n::tr("menu-window"));
        assert_eq!(systems(&window.nodes), [SystemItem::Minimize, SystemItem::Zoom]);
        assert!(matches!(window.nodes[..2], [NativeNode::System { .. }, NativeNode::System { .. }]), "Minimize and Zoom come first");
        for t in &bar {
            assert_ne!(t.nodes.last(), Some(&NativeNode::Separator), "the {:?} menu ends on a line", t.role);
        }
    }

    /// THE KEYS OF THE APPLICATION MENU: Cmd+, for the settings and Cmd+Q to quit.
    #[test]
    fn settings_and_quit_carry_the_keys_of_a_mac() {
        let bar = system_bar(&menu_model(&quiet()));
        let key = |action: MenuAction| bar[0].nodes.iter().find_map(|n| if let NativeNode::Item(i) = n { (i.action == action).then_some(i.shortcut) } else { None }).flatten();
        assert_eq!(key(MenuAction::Settings), Some(Shortcut::command(egui::Key::Comma)));
        assert_eq!(key(MenuAction::Quit), Some(Shortcut::command(egui::Key::Q)));
    }

    /// AN ITEM CHOSEN IN THE SYSTEM BAR IS FOUND BY ITS NAME; a name no item has any more does nothing.
    #[test]
    fn a_chosen_name_leads_back_to_its_item() {
        let bar = system_bar(&menu_model(&busy()));
        let all = actions(&bar);
        let mut names: Vec<String> = all.iter().map(id_of).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), all.len(), "two items of the bar share a name, and a choice could not tell them apart");
        for action in &all {
            assert_eq!(resolve(&bar, &id_of(action)).as_ref(), Some(action), "{action:?} cannot be found by its name");
        }
        assert_eq!(resolve(&bar, "no such item"), None);
        let gone = id_of(&MenuAction::OpenRecent("/tmp/gone.qcad".into()));
        assert_eq!(resolve(&bar, &gone), None, "a file no longer in the list must not be opened");
    }

    /// THE BAR ON SCREEN IS TOUCHED ONLY WHERE THE LIST CHANGED.
    #[test]
    fn the_bar_catches_up_by_what_changed() {
        let before = system_bar(&menu_model(&quiet()));
        assert_eq!(plan_refresh(&before, &before), Refresh::Same, "an unchanged list must not touch the bar");

        // a new step to undo: the words and the state of one item
        let after = system_bar(&menu_model(&MenuState { undo: Step::Named("Extrude".into()), ..quiet() }));
        let undo = Target::Item(id_of(&MenuAction::Undo));
        let expected = Refresh::Parts {
            rebuild: Vec::new(),
            changes: vec![
                Change { target: undo.clone(), edit: Edit::Text(qymcad_i18n::tr1("menu-undo-named", "what", "Extrude")) },
                Change { target: undo, edit: Edit::Enabled(Enabled::Yes) },
            ],
        };
        assert_eq!(plan_refresh(&before, &after), expected);

        // the 3D orbit ticked
        let ticked = system_bar(&menu_model(&MenuState { orbit: Checked::Yes, ..quiet() }));
        let tick = Change { target: Target::Item(id_of(&MenuAction::Orbit3d)), edit: Edit::Checked(Checked::Yes) };
        assert_eq!(plan_refresh(&before, &ticked), Refresh::Parts { rebuild: Vec::new(), changes: vec![tick] });

        // a file opened: the File menu has a line more, and only it is built anew
        let opened = system_bar(&menu_model(&MenuState { recent: vec!["/tmp/a/box.qcad".into()], ..quiet() }));
        assert_eq!(plan_refresh(&before, &opened), Refresh::Parts { rebuild: vec![NativeRole::Bar(MenuRole::File)], changes: Vec::new() });

        // the check for updates became possible: the Help menu is built anew
        let offered = system_bar(&menu_model(&MenuState { updates: Offered::Yes, ..quiet() }));
        assert_eq!(plan_refresh(&before, &offered), Refresh::Parts { rebuild: vec![NativeRole::Bar(MenuRole::Help)], changes: Vec::new() });

        // a bar with a menu less is a different bar
        assert_eq!(plan_refresh(&before, &before[1..]), Refresh::Whole);
    }

    /// ANOTHER LANGUAGE CHANGES ONLY WORDS: the titles of the menus are renamed in place, nothing is rebuilt.
    #[test]
    fn another_language_renames_in_place() {
        let mut renamed = system_bar(&menu_model(&quiet()));
        let before = renamed.clone();
        renamed[1].caption = "Fichier".into();
        let rename = Change { target: Target::Top(NativeRole::Bar(MenuRole::File)), edit: Edit::Text("Fichier".into()) };
        assert_eq!(plan_refresh(&before, &renamed), Refresh::Parts { rebuild: Vec::new(), changes: vec![rename] });
    }

    /// WHERE THE SYSTEM BAR HOLDS THE MENUS, the window draws none and gives their strip to the panels below:
    /// the title of the tree rises by the height of the menu row and no empty strip is left in its place.
    #[test]
    fn the_window_gives_its_menu_row_to_the_system_bar() {
        use crate::gui::hand::Hand;
        let file = qymcad_i18n::tr("menu-file");
        let tree = qymcad_i18n::tr("tree-title");

        let mut app = crate::gui::App::default();
        let mut hand = Hand::new(&mut app);
        let menu_row = hand.written_at(&file).expect("setup: the window draws its menu bar");
        let below = hand.written_at(&tree).expect("setup: the tree has a title").top();

        pretend_system_bar();
        let mut app = crate::gui::App::default();
        let mut hand = Hand::new(&mut app);
        assert!(!hand.shows(&file), "the window still draws its menu bar while the system bar holds the menus");
        let risen = hand.written_at(&tree).expect("the tree still has a title").top();
        assert!(below - risen >= menu_row.height(), "the tree rose by {} points, less than the menu row's {}: an empty strip is left where the menu was", below - risen, menu_row.height());
    }

    /// The words a frame of the menu bar alone paints.
    fn menu_bar_words(app: &mut crate::gui::App) -> Vec<String> {
        fn walk(shape: &egui::Shape, out: &mut Vec<String>) {
            match shape {
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
                _ => {}
            }
        }
        let ctx = egui::Context::default();
        let mut asks = Vec::new();
        let out = ctx.run_ui(egui::RawInput::default(), |ui| crate::gui::panels_bars::menu_bar(&mut app.bar_ctx(&mut asks), ui));
        let mut words = Vec::new();
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut words));
        words
    }

    /// THE MENU BAR OF THE WINDOW DRAWS NOTHING while the system bar holds the menus - not even into a strip
    /// nobody sees, where its items would still be laid out every frame.
    #[test]
    fn the_window_draws_no_menus_while_the_system_bar_holds_them() {
        let file = qymcad_i18n::tr("menu-file");
        let words = menu_bar_words(&mut crate::gui::App::default());
        assert!(words.iter().any(|w| w.contains(&file)), "setup: the menu bar of the window writes \"{file}\": {words:?}");
        pretend_system_bar();
        let words = menu_bar_words(&mut crate::gui::App::default());
        assert!(words.is_empty(), "the menu bar of the window still draws while the system bar holds the menus: {words:?}");
    }

    /// THE KEYS OF THE MENU BECOME KEYS OF THE SYSTEM: every key the bar carries has a key code there.
    #[cfg(target_os = "macos")]
    #[test]
    fn every_key_of_the_bar_is_a_key_of_the_system() {
        fn keys(nodes: &[NativeNode], out: &mut Vec<Shortcut>) {
            for node in nodes {
                match node {
                    NativeNode::Item(item) | NativeNode::Check { item, .. } => out.extend(item.shortcut),
                    NativeNode::Sub { nodes, .. } => keys(nodes, out),
                    _ => {}
                }
            }
        }
        let mut all = Vec::new();
        system_bar(&menu_model(&busy())).iter().for_each(|t| keys(&t.nodes, &mut all));
        assert!(!all.is_empty(), "setup: the bar carries keys");
        for key in all {
            assert!(mac::key_of(key).is_some(), "{key:?} has no key code in the system bar");
        }
        let quit = mac::key_of(Shortcut::command(egui::Key::Q)).expect("Cmd+Q");
        assert_eq!(quit, "CmdOrCtrl+Q".parse::<muda::accelerator::Accelerator>().expect("the system reads Cmd+Q"), "Command is Cmd on a Mac");
    }
}
