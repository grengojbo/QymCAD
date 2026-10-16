//! WHERE THE MENUS LIVE: in the window's own bar, or in the menu bar of the system on a Mac.

use super::Settings;

/// THE PLACE OF THE MENU BAR. Where the system bar holds the menus, the place stays and takes no room: its
/// filling is where the system bar catches up with the document each frame.
pub(crate) fn menu_place(set: &Settings) -> qymcad_shell::Place {
    use qymcad_shell::{Place, Slot};
    if menus_in_system_bar(set) {
        return Place::new("menubar", Slot::Menu).sized(0.0, false).bare();
    }
    Place::new("menubar", Slot::Menu)
}

/// Whether this window can hand its menus to the menu bar of the system: on a Mac, once it has taken that bar.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn system_bar_offered() -> bool {
    crate::gui::native_menu::installed()
}

/// Whether this window can hand its menus to the menu bar of the system: never, away from a Mac.
#[cfg(not(any(target_os = "macos", test)))]
pub(crate) fn system_bar_offered() -> bool {
    false
}

/// Whether the menus live in the menu bar of the system this frame: where it can hold them and the person keeps
/// them there.
pub(crate) fn menus_in_system_bar(set: &Settings) -> bool {
    system_bar_offered() && set.menu_place == qymcad_ui_state::MenuPlace::SystemBar
}
