//! THE DOCUMENT CRATE STAYS A MODULE: no window, no words of the interface, no workbench. Every caller without a
//! window - the reproduction harness, the protocol server - builds on it, and a dependency on the interface here
//! would pull a window into each of them and tie the module to one workbench.

#[test]
fn the_document_crate_stays_a_module() {
    let manifest = include_str!("../Cargo.toml");
    let deps: Vec<&str> = manifest
        .split("[dependencies]")
        .nth(1)
        .expect("the manifest has dependencies")
        .lines()
        .take_while(|l| !l.trim_start().starts_with('['))
        .filter_map(|l| l.split(['=', '.']).next())
        .map(str::trim)
        .filter(|n| !n.is_empty() && !n.starts_with('#'))
        .collect();
    assert!(deps.len() >= 3, "the dependencies are no longer found by this check: {deps:?}");
    const BANNED: [&str; 8] = ["egui", "eframe", "qymcad", "qymcad-ui-state", "qymcad-i18n", "qymcad-part", "qymcad-sketch", "qymcad-assembly"];
    let wrong: Vec<&&str> = deps.iter().filter(|d| BANNED.contains(d)).collect();
    assert!(wrong.is_empty(), "the document crate depends on the interface or a workbench: {wrong:?}");
}
