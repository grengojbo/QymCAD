//! THE FILES A CHECK WRITES, in a folder of its process's own, removed when the check ends.
//!
//! Every check runs in a process of its own (`isolation`), so a folder named after the process is one check's alone.
//! A file or a folder under the temporary folder named after the process and never removed is left once per check
//! and per run: 5505 folders of the tool contracts, 152 chains and 108 homes of the program gathered under the
//! temporary folder of one machine. The process that started the check removes its folder and the home of the
//! program in it when the check ends, however it ended: passed, failed, or stopped past its time or its memory.
use std::path::{Path, PathBuf};

/// The folder of the files the check run by the process `run` writes.
pub(crate) fn folder_of(run: u32) -> PathBuf {
    std::env::temp_dir().join(format!("qymcad-check-{run}"))
}

/// THE FILE `name` IN THE FOLDER OF THIS CHECK, the folder made when it is not there yet. The file keeps its name,
/// since the program names a part after the file it came from.
pub fn file(name: &str) -> String {
    place(name).to_string_lossy().into_owned()
}

/// The place `name` in the folder of this check, for a folder of the check's own: a home of the program.
pub fn place(name: &str) -> PathBuf {
    let dir = folder_of(std::process::id());
    std::fs::create_dir_all(&dir).expect("the folder of the check is made");
    dir.join(name)
}

/// A FOLDER REMOVED WITH EVERYTHING IN IT WHEN DROPPED, a panicking check included.
pub struct Folder {
    path: PathBuf,
}

impl Folder {
    /// The folder at `path`, emptied of what an earlier run left there and made.
    pub fn fresh(path: PathBuf) -> Folder {
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the folder is made");
        Folder { path }
    }

    /// The folder at `path` as it is, made or not, to be removed when dropped.
    pub(crate) fn left_at(path: PathBuf) -> Folder {
        Folder { path }
    }

    /// Where the folder is.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
