//! A FOLDER OF ITS OWN FOR EVERY CHECK AND EVERY RUN, for the files a check writes and then reads back.
//!
//! The checks write under the checkout's `target`, which is not the target of the build: two runs from one checkout
//! (the host and a container, or two containers each with a target volume of its own) share it, and libtest runs the
//! checks of one run side by side. A file with a fixed name is then written by one while the import door reads it on a
//! thread of its own for the other, and the door meets a file just truncated: "3MF: the package holds no model", a cube
//! with no vertex, a whole group of import checks red together in one run and green in the next.
//!
//! The folder is `target/check-files/<check>-<run>`, the run being the process id: no other check of this run and no
//! check of another run writes there. The files keep the names the check gives them, since the door names a part after
//! its file. The folder is emptied when it is made and removed when it is dropped, a panicking check included.
//!
//! Two kinds of file cannot live under the checkout, and their folder is under the system's temporary folder, named
//! the same way: a Unix socket - the checkout of the Linux container is a virtiofs mount, which refuses to bind one,
//! and the path of a socket is held to 104 bytes on macOS and 108 on Linux - and the tree of a packaging script, which
//! asks git about the folder it runs in and under the checkout would be answered about the repository itself.
#[cfg(test)]
pub(crate) mod tests {
    use std::path::{Path, PathBuf};

    /// The folder of one check in one run; removed with everything in it when dropped.
    pub(crate) struct CheckFolder {
        path: PathBuf,
    }

    impl CheckFolder {
        /// The folder of the check `check` in this run.
        pub(crate) fn new(check: &str) -> Self {
            Self::of_run(check, std::process::id())
        }

        /// The folder of the check `check` in the run `run` (a process id). A folder left by a run that ended before
        /// it could remove it, under a process id given out again, is emptied first.
        pub(crate) fn of_run(check: &str, run: u32) -> Self {
            Self::made(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/check-files")).join(format!("{check}-{run}")))
        }

        /// The folder of the check `check` in this run under the system's temporary folder, for a socket or the
        /// tree of a packaging script.
        pub(crate) fn outside_the_checkout(check: &str) -> Self {
            Self::made(std::env::temp_dir().join(format!("qymcad-check-{check}-{}", std::process::id())))
        }

        /// The folder at `path`, emptied and made.
        fn made(path: PathBuf) -> Self {
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a folder for the check");
            Self { path }
        }

        /// Where the folder is.
        pub(crate) fn path(&self) -> &Path {
            &self.path
        }

        /// The file `name` in the folder.
        pub(crate) fn file(&self, name: &str) -> PathBuf {
            self.path.join(name)
        }
    }

    /// A run that is not this one and is no process at all, for a check that stands in for another run. Process ids
    /// stay below 2^31, so the top bit set names no process: `pid + 1` is often the process started next to this one,
    /// and a check standing in for another run there wrote into, and emptied, the folders of a real run beside it.
    pub(crate) fn another_run() -> u32 {
        std::process::id() ^ 0x8000_0000
    }

    impl Drop for CheckFolder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// ANOTHER RUN OF THE SAME CHECK DOES NOT REACH THIS RUN'S FILE. Reported behaviour: in a whole run of the lib in the
    /// Linux container a group of import checks failed together, each with nothing come in, and passed in the next run;
    /// a run on the host from the same checkout overlapped it, and both wrote the same files under the checkout's
    /// `target`. Here another run rewrites the cube of the same check without a pause while this run brings it in by
    /// the door 20 times; with one folder for both runs, 5 to 8 of the 20 cubes did not come in whole, in each of 3
    /// runs.
    #[test]
    fn a_file_comes_in_whole_while_another_run_writes_the_same_check() {
        use crate::gui::import_door::tests::{answer, cube_stl, running, settle};
        let theirs = CheckFolder::of_run("overlapped", another_run());
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let other = {
            let stop = stop.clone();
            let p = theirs.file("cube.stl");
            std::thread::spawn(move || {
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = std::fs::write(&p, cube_stl(10.0));
                }
            })
        };
        let broken = (0..20)
            .filter(|_| {
                let ours = CheckFolder::new("overlapped");
                let p = ours.file("cube.stl");
                std::fs::write(&p, cube_stl(10.0)).expect("written");
                let (mut app, ctx) = running();
                answer(&mut app, &ctx, qymcad_ui_state::Want::Anything, &p.to_string_lossy());
                settle(&mut app, &ctx);
                app.project.bodies.last().map(|b| b.mesh.tris.len()) != Some(12)
            })
            .count();
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        other.join().expect("the other run's writer");
        assert_eq!(broken, 0, "{broken} of 20 cubes did not come in whole while another run wrote the file of the same check");
    }
}
