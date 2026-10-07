//! THE TOOLS STAY A MODULE: no window and no workbench. The window takes this crate to lend its document to the same
//! tools the program runs; a dependency on the window would be a loop, and one on a workbench would tie every tool to
//! that workbench's interface.

#[test]
fn the_tools_stay_a_module() {
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
    const BANNED: [&str; 6] = ["eframe", "qymcad", "qymcad-mcp", "qymcad-part", "qymcad-sketch", "qymcad-assembly"];
    let wrong: Vec<&&str> = deps.iter().filter(|d| BANNED.contains(d)).collect();
    assert!(wrong.is_empty(), "the tools depend on the window, the program or a workbench: {wrong:?}");
}
