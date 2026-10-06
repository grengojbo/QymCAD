//! THE PROGRAM ANSWERS TO ONE NAME, IN EVERY PLACE THAT ASKS.
//!
//! The same reverse-DNS name identifies the program to the desktop, to macOS, to Flathub, and it decides
//! the directory a person's settings, colour schemes, templates and library of parts live in. It is said in
//! four places that cannot share a constant - Rust, a shell script, a YAML manifest, an XML description -
//! so it is compared instead.
//!
//! MEASURED BEFORE IT WAS MADE ONE. Three different names were in the tree at once - one in the code, another
//! in the macOS bundle, a third in the Flatpak manifest. Nothing was broken by it - each half worked alone -
//! which is exactly why it had gone unnoticed through two releases.
//!
//! WHAT IS NOT COMPARED HERE, and deliberately: `grengojbo.QymCAD` in the winget manifest and `qymcad-bin`
//! in the AUR package. Those catalogues have naming rules of their own, `Publisher.Package` and the Arch
//! convention, and forcing a reverse-DNS name on them would be wrong rather than consistent.
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn read(rel: &str) -> String {
        let p = root().join(rel);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{rel} must be readable: {e}"))
    }

    /// EVERY PLACE THAT NAMES THE PROGRAM NAMES THE SAME ONE.
    #[test]
    fn the_program_is_called_the_same_thing_everywhere() {
        let id = qymcad_paths::APP_ID;
        let mut wrong = Vec::new();

        // macOS: what the system remembers permissions and file associations by
        let mac = read("packaging/macos/bundle.sh");
        if !mac.contains(&format!("<key>CFBundleIdentifier</key><string>{id}</string>")) {
            wrong.push("the macOS bundle identifies the program as something else".to_string());
        }

        // Flatpak: the manifest, the desktop entry and the metainfo are named by it and carry it
        for f in [format!("{id}.yml"), format!("{id}.desktop"), format!("{id}.metainfo.xml")] {
            let p = root().join("packaging/flatpak").join(&f);
            if !p.exists() {
                wrong.push(format!("packaging/flatpak/{f} does not exist - the Flatpak files are named by the id"));
                continue;
            }
            if !std::fs::read_to_string(&p).unwrap_or_default().contains(id) {
                wrong.push(format!("packaging/flatpak/{f} does not carry {id}"));
            }
        }

        assert!(wrong.is_empty(), "the program answers to more than one name, and each half works alone - which is why this goes unnoticed:\n{}", wrong.join("\n"));
    }

    /// AND THE OLD NAMES ARE GONE FROM THE CODE.
    ///
    /// The directory is built from the id in one place now. A second `ProjectDirs::from` anywhere is the
    /// same decision written twice, which is how the three names came about.
    #[test]
    fn the_directory_is_decided_in_one_place() {
        let mut twice = Vec::new();
        for entry in walk(&root().join("crates")) {
            // the one place that may decide it, and this file, which only talks about it - it found itself
            // on the first run, which is the oldest joke in this kind of check
            if entry.contains("qymcad-paths") || entry.ends_with("app_identity.rs") {
                continue;
            }
            let src = std::fs::read_to_string(&entry).unwrap_or_default();
            if src.contains("ProjectDirs::from(") {
                twice.push(entry);
            }
        }
        assert!(twice.is_empty(), "the program's own directory is decided outside qymcad-paths:\n{}", twice.join("\n"));
    }

    /// NOTHING POINTS AT THE SITE OR THE REPOSITORY OF THE PROJECT THIS ONE GREW FROM.
    ///
    /// That site is not reachable from Ukraine, and every address the program, its help or its packages hand a
    /// person - an update check, a help page, a tracker, a homepage - leads to this repository instead. A merge
    /// of the upstream project brings its own addresses back with it; this says which file did. The author of
    /// the original is still named, as text.
    ///
    /// ONE ADDRESS IS LET THROUGH, IN ONE KIND OF FILE: a README may link the original's repository on GitHub, which
    /// is reachable - it says where the fork came from. The site stays out of the READMEs too, and the repository
    /// stays out of everything the program, its help and its packages hand a person.
    #[test]
    fn nothing_points_at_the_project_this_one_grew_from() {
        // put together here, so this file does not find itself
        let banned = [concat!("qymis", ".tech"), concat!("QymIs", "-Tech/"), concat!("github.com/QymIs", "-Tech")];
        let mut found = Vec::new();
        let mut files: Vec<std::path::PathBuf> = ["Cargo.toml", "README.md", "README.ru.md", "README.uk.md", "CONTRIBUTING.md", "CONTRIBUTING.ru.md"].iter().map(|f| root().join(f)).collect();
        for dir in ["crates", "docs", "site", "i18n", "packaging", ".github", "tools"] {
            files.extend(every_file(&root().join(dir)));
        }
        let origin = concat!("github.com/QymIs", "-Tech/QymCAD");
        for p in files {
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            let readme = p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("README"));
            for (n, line) in text.lines().enumerate() {
                let line = if readme { line.replace(origin, "") } else { line.to_string() };
                if banned.iter().any(|b| line.contains(b)) {
                    found.push(format!("{}:{}", p.strip_prefix(root()).unwrap_or(&p).display(), n + 1));
                }
            }
        }
        assert!(found.is_empty(), "an address of the project this one grew from is back:\n{}", found.join("\n"));
    }

    /// Every file under a directory, the build output left out.
    fn every_file(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        let Ok(rd) = std::fs::read_dir(dir) else { return out };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if !p.ends_with("target") {
                    out.extend(every_file(&p));
                }
            } else {
                out.push(p);
            }
        }
        out
    }

    /// Every `.rs` file under a directory.
    fn walk(dir: &std::path::Path) -> Vec<String> {
        let mut out = Vec::new();
        let Ok(rd) = std::fs::read_dir(dir) else { return out };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p.to_string_lossy().into_owned());
            }
        }
        out
    }
}
