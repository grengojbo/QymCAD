//! A FOLDER OF ITS OWN FOR EVERY CHECK AND EVERY RUN, for the files and the sockets a check makes.
/// The folder of one check in one run, `qymcad-check-<check>-<run>` under the system's temporary folder, the run
/// being the process id: no other check of this run and no check of another run writes there. Emptied when made and
/// removed with everything in it when dropped, a panicking check included: a file under the temporary folder that no
/// check removes is left there by every run.
pub struct CheckFolder {
    path: std::path::PathBuf,
}

impl CheckFolder {
    /// The folder of the check `check` in this run.
    pub fn new(check: &str) -> Self {
        let path = std::env::temp_dir().join(format!("qymcad-check-{check}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a folder for the check");
        Self { path }
    }

    /// Where the folder is.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for CheckFolder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
