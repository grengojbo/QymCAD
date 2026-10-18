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

/// The code of a line without its `//` comment.
fn code_of(line: &str) -> &str {
    line.split("//").next().unwrap_or("")
}

/// Whether `word` stands alone in `code`: not a part of a longer name.
fn stands_alone(code: &str, word: &str) -> bool {
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
    code.match_indices(word).any(|(at, _)| {
        let before = code[..at].chars().next_back();
        let after = code[at + word.len()..].chars().next();
        !before.is_some_and(is_name) && !after.is_some_and(is_name)
    })
}

#[test]
fn no_name_in_the_cpp_is_a_macro_of_the_windows_headers() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    for e in std::fs::read_dir(&src).expect("the sources read").flatten() {
        let p = e.path();
        if !p.extension().is_some_and(|x| x == "cpp" || x == "h" || x == "hpp") {
            continue;
        }
        let text = std::fs::read_to_string(&p).expect("a source reads");
        for (k, l) in text.lines().enumerate() {
            for word in MACROS {
                if stands_alone(code_of(l), word) {
                    found.push(Found { file: p.file_name().unwrap_or_default().to_string_lossy().into_owned(), line: k + 1, word });
                }
            }
        }
    }
    let told: Vec<String> = found.iter().map(|f| format!("{}:{} `{}`", f.file, f.line, f.word)).collect();
    assert!(told.is_empty(), "{} names in the C++ are macros of the Windows headers and break the build there:\n{}", told.len(), told.join("\n"));
}
