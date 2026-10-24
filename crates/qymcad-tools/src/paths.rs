//! WHERE A PATH A MODEL GIVES POINTS, and whether a file may be written there.
//!
//! The server runs in the folder its client started it in, which is rarely the folder the person thinks of - on Windows
//! it is the program's own folder under Program Files. A file to read is taken as absolute or from that folder; a file
//! to write named without a folder goes beside the document's own file, or into the person's Downloads while the
//! document has none. `~` is the home folder, and every answer names the absolute path it used. A file that is there
//! already is written over only when the call says so: a model guessing a name must not wipe a person's work. Nothing
//! is written into the folder the program is installed in: an update replaces that folder, and on Windows a write there
//! without the rights of an administrator fails or goes to a shadow folder nobody looks in.

use std::path::{Path, PathBuf};

use crate::tool::{Refusal, Stage};

/// `raw` with `~` read as the home folder.
fn expand(raw: &str) -> PathBuf {
    let home = || std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    match raw.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with(['/', '\\']) => match home() {
            Some(h) => h.join(rest.trim_start_matches(['/', '\\'])),
            None => PathBuf::from(raw),
        },
        _ => PathBuf::from(raw),
    }
}

/// The absolute path of a file to read `raw` names.
pub fn resolve(raw: &str) -> PathBuf {
    let path = expand(raw);
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir().map(|d| d.join(&path)).unwrap_or(path)
    }
}

/// THE FOLDERS A FILE TO WRITE IS PLACED BY.
pub struct Places {
    /// The folder of the document's own file, while it has one.
    pub document: Option<PathBuf>,
    /// Where a file goes while the document has no file (`qymcad_paths::downloads`).
    pub downloads: Option<PathBuf>,
    /// The folder the program is installed in: the `.app` bundle on macOS, the folder of the program elsewhere.
    pub program: Option<PathBuf>,
}

impl Places {
    /// The places of this machine, for a document whose own file is `document`.
    pub fn here(document: Option<&str>) -> Self {
        let program = std::env::current_exe().ok().and_then(|exe| {
            let bundle = exe.ancestors().find(|a| a.extension().is_some_and(|e| e == "app")).map(Path::to_path_buf);
            bundle.or_else(|| exe.parent().map(Path::to_path_buf))
        });
        Places { document: document.and_then(|d| Path::new(d).parent().map(Path::to_path_buf)), downloads: qymcad_paths::downloads(), program }
    }
}

/// THE ABSOLUTE PATH OF A FILE TO WRITE `raw` names: as it is when absolute, else beside the document, else in
/// Downloads; refused inside the folder of the program.
pub fn to_write(raw: &str, places: &Places) -> Result<PathBuf, Refusal> {
    let path = expand(raw);
    let path = if path.is_absolute() {
        path
    } else {
        match places.document.as_ref().or(places.downloads.as_ref()) {
            Some(folder) => folder.join(&path),
            None => resolve(raw),
        }
    };
    match places.program.as_ref().filter(|p| path.starts_with(p)) {
        None => Ok(path),
        Some(program) => {
            let name = path.file_name().map(PathBuf::from).unwrap_or_default();
            let instead = places.downloads.as_ref().map(|d| d.join(&name)).unwrap_or(name);
            Err(Refusal::new("path-in-app-dir", &format!("{} is inside the folder QymCAD is installed in, {}: an update replaces that folder.", path.display(), program.display()), Stage::Io)
                .with_hint(&format!("Write it to the person's Downloads: {}", instead.display())))
        }
    }
}

/// THE FOLDER OF `path` IS THERE AND TAKES A FILE: the reason named in words, where the writer would only give a code.
pub fn may_write_in(path: &Path) -> Result<(), Refusal> {
    let folder = path.parent().unwrap_or(Path::new("."));
    if !folder.is_dir() {
        return Err(
            Refusal::new("path-no-folder", &format!("There is no folder {}.", folder.display()), Stage::Io).with_hint("Name a folder that is there, or a bare name to write beside the document.")
        );
    }
    let was_there = path.exists();
    match std::fs::OpenOptions::new().append(true).create(true).open(path) {
        Ok(_) => {
            if !was_there {
                let _ = std::fs::remove_file(path);
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            Err(Refusal::new("path-no-permission", &format!("{} cannot be written: the folder or the file is not open to writing for this user.", path.display()), Stage::Io)
                .with_hint("Write it to the person's Downloads or Documents."))
        }
        Err(e) => Err(Refusal::new("path-busy", &format!("{} cannot be written: {e}.", path.display()), Stage::Io).with_hint("Close the program that holds the file, or write to another name.")),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// An absolute folder on any system, named by `name`; nothing is written there.
    fn at(name: &str) -> PathBuf {
        std::env::temp_dir().join("qymcad-places").join(name)
    }

    fn places(document: Option<&str>) -> Places {
        Places { document: document.map(at), downloads: Some(at("Downloads")), program: Some(at("QymCAD")) }
    }

    fn named(p: &Path) -> String {
        p.to_string_lossy().into_owned()
    }

    /// A BARE NAME goes beside the document while it has a file, into Downloads while it has none; an absolute path is
    /// kept as it is.
    #[test]
    fn a_bare_name_goes_beside_the_document_or_into_downloads() {
        assert_eq!(to_write("a.step", &places(Some("work"))).ok(), Some(at("work").join("a.step")));
        assert_eq!(to_write("a.step", &places(None)).ok(), Some(at("Downloads").join("a.step")));
        let elsewhere = at("elsewhere").join("a.step");
        assert_eq!(to_write(&named(&elsewhere), &places(Some("work"))).ok(), Some(elsewhere));
    }

    /// THE FOLDER OF THE PROGRAM takes no file, and the refusal names the Downloads path to write instead.
    #[test]
    fn the_folder_of_the_program_is_refused_with_downloads_named() {
        let inside = named(&at("QymCAD").join("test2.step"));
        let refused = to_write(&inside, &places(None)).expect_err("a file went into the folder of the program");
        assert_eq!(refused.code, "path-in-app-dir");
        let instead = named(&at("Downloads").join("test2.step"));
        assert!(refused.hint.as_deref().is_some_and(|h| h.contains(&instead)), "the hint names no place to write instead: {:?}", refused.hint);
        let beside = to_write("test2.step", &places(Some("QymCAD"))).expect_err("a bare name beside a document in the folder of the program went through");
        assert_eq!(beside.code, "path-in-app-dir");
    }
}
