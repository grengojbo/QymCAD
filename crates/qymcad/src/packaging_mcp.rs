//! THE SERVER FOR CLAUDE TRAVELS IN EVERY PACKAGE, beside the program.
//!
//! A person connects Claude to QymCAD by pointing it at the server, and the server is built against the same kernel
//! libraries as the program - so it travels in the same package, where those libraries already are, and nothing more
//! has to be downloaded. Three systems pack three ways, in three scripts edited by hand; a server left out of one of
//! them is found by whoever downloads that one, after a whole release run.
//!
//! The AppImage is the odd one: it is mounted at a new path every time it starts, so nothing outside can name the
//! server inside it. The package's own door, `AppRun`, starts the server when the package is run with `mcp` - that
//! script is run here, in a sandbox, with the two binaries stood in for by scripts that say who they are.
#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use crate::gui::check_folder::tests::CheckFolder;
    #[cfg(unix)]
    use std::path::Path;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn read(rel: &str) -> String {
        std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel} must be readable: {e}"))
    }

    /// An AppImage's tree as it is mounted: its door, and the two binaries as scripts that print who they are and
    /// what they were given.
    #[cfg(unix)]
    fn mounted(case: &str) -> CheckFolder {
        use std::os::unix::fs::PermissionsExt;
        let folder = CheckFolder::new(&format!("apprun-{case}"));
        let dir = folder.path();
        std::fs::create_dir_all(dir.join("usr/bin")).expect("the sandbox is writable");
        let put = |rel: &str, text: &str| {
            let p = dir.join(rel);
            std::fs::write(&p, text).expect("the file is written");
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("the bit is set");
        };
        put("AppRun", &read("packaging/linux/AppRun"));
        put("usr/bin/qymcad", "#!/bin/sh\necho \"program $*\"\n");
        put("usr/bin/qymcad-mcp", "#!/bin/sh\necho \"server $*\"\n");
        folder
    }

    #[cfg(unix)]
    fn run(dir: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new(dir.join("AppRun")).args(args).output().expect("the door runs");
        assert!(out.status.success(), "the door failed: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// `QymCAD.AppImage mcp` STARTS THE SERVER, with whatever follows; anything else opens the program as before -
    /// a file to open, or nothing.
    #[cfg(unix)]
    #[test]
    fn the_appimage_starts_the_server_when_run_with_mcp() {
        let folder = mounted("door");
        let dir = folder.path();
        assert_eq!(run(dir, &["mcp"]), "server", "the package run with mcp did not start the server");
        assert_eq!(run(dir, &["mcp", "--verbose"]), "server --verbose", "what follows mcp did not reach the server");
        assert_eq!(run(dir, &["part.qcad"]), "program part.qcad", "a file to open did not reach the program");
        assert_eq!(run(dir, &[]), "program", "the package run bare did not open the program");
    }

    /// EVERY PACKAGE BUILDS THE SERVER AND PUTS IT BESIDE THE PROGRAM: the release run compiles both binaries for
    /// Windows and macOS, the AppImage script compiles both, carries the server and hands linuxdeploy its door, and the
    /// Windows folder - the zip and the MSI alike - holds `qymcad-mcp.exe`. (The macOS bundle is run, not read:
    /// `packaging_macos`.)
    #[test]
    fn every_package_builds_and_carries_the_server() {
        let mut missing = Vec::new();
        let workflow = read(".github/workflows/release.yml");
        let builds: Vec<&str> = workflow.lines().filter(|l| l.contains("cargo build --release")).collect();
        if builds.len() < 2 || !builds.iter().all(|l| l.contains("--bin qymcad-mcp")) {
            missing.push(format!("release.yml compiles without the server: {builds:?}"));
        }
        /// What the AppImage script must do, and the words it is done by.
        struct Needle {
            what: &'static str,
            text: &'static str,
        }
        let appimage = read("packaging/linux/build-appimage.sh");
        for n in [
            Needle { what: "compiles the server", text: "--bin qymcad-mcp" },
            Needle { what: "carries the server", text: "usr/bin/qymcad-mcp\"" },
            Needle { what: "hands linuxdeploy the server", text: "--executable \"$APPDIR/usr/bin/qymcad-mcp\"" },
            Needle { what: "hands linuxdeploy its door", text: "--custom-apprun packaging/linux/AppRun" },
            Needle { what: "asks the built package for a handshake", text: "AppRun\" mcp" },
        ] {
            if !appimage.contains(n.text) {
                missing.push(format!("build-appimage.sh no longer {} ({})", n.what, n.text));
            }
        }
        if !read("packaging/win/bundle.ps1").contains("Copy-Item $server $out") {
            missing.push("bundle.ps1 does not put qymcad-mcp.exe beside qymcad.exe".into());
        }
        assert!(missing.is_empty(), "a package goes out without the server for Claude:\n{}", missing.join("\n"));
    }
}
