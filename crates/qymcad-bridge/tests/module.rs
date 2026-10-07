//! THE CHANNEL STAYS A CHANNEL: no window, no kernel, no document. Its checks run where the kernel is not built, and a
//! dependency on any of those would bring the kernel with it.

#[test]
fn the_channel_stays_a_channel() {
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
    assert!(deps.len() >= 2, "the dependencies are no longer found by this check: {deps:?}");
    let allowed = ["qymcad-paths", "serde_json", "serde"];
    let wrong: Vec<&&str> = deps.iter().filter(|d| !allowed.contains(d)).collect();
    assert!(wrong.is_empty(), "the channel took a dependency beyond its own: {wrong:?}");
}
