//! THE PERSON'S LANGUAGE: the one their QymCAD interface is in. The answers stay English - their reader is a model -
//! but a model that names a menu or a setting to the person names it the way the person's window shows it.
//!
//! The open window says its own language with every call. A program with no window reads it where the window keeps
//! it: the settings file.

use std::collections::HashMap;
use std::path::Path;

/// The one setting read here; every other one in the file is passed over.
#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct Chosen {
    language: String,
}

/// THE LANGUAGE OF THE PERSON'S INTERFACE, from the settings file of this machine.
pub fn language() -> String {
    language_in(qymcad_paths::settings_file().as_deref())
}

/// THE LANGUAGE `file` CHOSE: the code the person picked in the settings, the system's when none was picked - as the
/// window resolves it at its start - and the system's too when there is no such file or it cannot be read.
pub fn language_in(file: Option<&Path>) -> String {
    let chosen = file
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|text| ron::from_str::<HashMap<String, String>>(&text).ok())
        .and_then(|kept| kept.get("settings").and_then(|s| ron::from_str::<Chosen>(s).ok()))
        .map(|c| c.language)
        .unwrap_or_default();
    if qymcad_i18n::available().iter().any(|(code, _)| *code == chosen) {
        chosen
    } else {
        qymcad_i18n::system_default()
    }
}
