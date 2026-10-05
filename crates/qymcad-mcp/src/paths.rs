//! WHERE A PATH A MODEL GIVES POINTS, and whether a file may be written there.
//!
//! The server runs in the folder its client started it in, which is rarely the folder the person thinks of, so a path
//! is taken as absolute or from that folder, `~` is the home folder, and every answer names the absolute path it used.
//! A file that is there already is written over only when the call says so: a model guessing a name must not wipe a
//! person's work.

use std::path::{Path, PathBuf};

use crate::tool::{Refusal, Stage};

/// The absolute path `raw` names.
pub fn resolve(raw: &str) -> PathBuf {
    let home = || std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    let path = match raw.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with(['/', '\\']) => match home() {
            Some(h) => h.join(rest.trim_start_matches(['/', '\\'])),
            None => PathBuf::from(raw),
        },
        _ => PathBuf::from(raw),
    };
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir().map(|d| d.join(&path)).unwrap_or(path)
    }
}

/// The path ends in one of `allowed` (lower-case, without the dot), whatever its case.
pub fn has_extension(path: &Path, allowed: &[&str]) -> Result<(), Refusal> {
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    if allowed.contains(&ext.as_str()) {
        return Ok(());
    }
    let list = allowed.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(", ");
    Err(Refusal::new("wrong-extension", &format!("{} is not one of {list}.", path.display()), Stage::Io).with_hint(&format!("Name the file with {list}.")))
}

/// What to do with a file that is there already.
#[derive(Clone, Copy, PartialEq)]
pub enum Existing {
    Keep,
    WriteOver,
}

impl Existing {
    /// The `overwrite` field of a call.
    pub fn from_overwrite(overwrite: bool) -> Self {
        if overwrite {
            Existing::WriteOver
        } else {
            Existing::Keep
        }
    }
}

/// A file may go to `path`: nothing is there, or the call allows writing over it.
pub fn may_write(path: &Path, existing: Existing) -> Result<(), Refusal> {
    if existing == Existing::WriteOver || !path.exists() {
        return Ok(());
    }
    Err(Refusal::new("path-taken", &format!("{} is there already.", path.display()), Stage::Io).with_hint("Pass overwrite: true to write over it, or another path."))
}
