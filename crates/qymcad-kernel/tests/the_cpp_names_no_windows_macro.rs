//! THE C++ OF THE BRIDGE NAMES NOTHING THE WINDOWS HEADERS TAKE AS A MACRO.
//!
//! `<windows.h>` defines `near` and `far` as empty macros, and OCCT pulls it in on Windows: a field `gp_Vec near, far;`
//! becomes `gp_Vec , ;` there and the kernel does not build (MSVC C2059 "syntax error: ','"), while Linux and macOS
//! build the same file clean. Reported behaviour: the release went red on Windows alone. The check reads the sources so
//! the fault shows on every machine, not only on the one that has the headers.

/// The words the Windows headers define as macros that a name in our C++ could take.
const MACROS: [&str; 2] = ["near", "far"];

/// A use of a macro word as code, with where it stands.
struct Found {
    file: String,
    line: usize,
    word: &'static str,
}

/// The code of a line, its `//` comment left out.
struct Code<'a> {
    text: &'a str,
}

impl<'a> Code<'a> {
    fn of(line: &'a str) -> Self {
        Code { text: line.split("//").next().unwrap_or("") }
    }

    /// Whether `word` stands alone in the code: not a part of a longer name.
    fn stands_alone(&self, word: &str) -> bool {
        let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
        self.text.match_indices(word).any(|(at, _)| {
            let before = self.text[..at].chars().next_back();
            let after = self.text[at + word.len()..].chars().next();
            !before.is_some_and(is_name) && !after.is_some_and(is_name)
        })
    }
}

/// Every C++ source the program is built from: the files under `src` of every crate. The generators of data for the
/// probes, under `tests`, are built by hand and never by the release.
fn cpp_sources() -> Vec<std::path::PathBuf> {
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the crates directory");
    let mut stack: Vec<std::path::PathBuf> = std::fs::read_dir(crates).expect("the crates read").flatten().map(|e| e.path().join("src")).filter(|p| p.is_dir()).collect();
    let mut out = Vec::new();
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).expect("the sources read").flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "cpp" || x == "h" || x == "hpp") {
                out.push(p);
            }
        }
    }
    out
}

#[test]
fn no_name_in_the_cpp_is_a_macro_of_the_windows_headers() {
    let sources = cpp_sources();
    assert!(sources.len() > 5, "the C++ of the bridge was not found: {sources:?}");
    let mut found = Vec::new();
    for p in sources {
        let text = std::fs::read_to_string(&p).expect("a source reads");
        for (k, l) in text.lines().enumerate() {
            for word in MACROS {
                if Code::of(l).stands_alone(word) {
                    found.push(Found { file: p.file_name().unwrap_or_default().to_string_lossy().into_owned(), line: k + 1, word });
                }
            }
        }
    }
    let told: Vec<String> = found.iter().map(|f| format!("{}:{} `{}`", f.file, f.line, f.word)).collect();
    assert!(told.is_empty(), "{} names in the C++ are macros of the Windows headers and break the build there:\n{}", told.len(), told.join("\n"));
}
