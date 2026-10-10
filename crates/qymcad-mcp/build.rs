// THE RELEASE THE SERVER BELONGS TO, stamped as the window's own build stamps it: `QYMCAD_VERSION` carries the tag at
// the release run's step that compiles both programs. A refusal that names the server's release tells an old server
// from a window that is not there; without it the two read the same.
//
// `rerun-if-env-changed` keeps a warm target directory from carrying the previous tag.
fn main() {
    println!("cargo:rerun-if-env-changed=QYMCAD_VERSION");
    if let Ok(tag) = std::env::var("QYMCAD_VERSION") {
        if !tag.trim().is_empty() {
            println!("cargo:rustc-env=QYMCAD_RELEASE={}", tag.trim());
        }
    }
}
