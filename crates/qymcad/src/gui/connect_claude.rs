//! HELP -> CONNECT TO CLAUDE: the server that travels in this package, given to Claude Desktop and to Claude Code
//! without a person looking for a path.
//!
//! The program knows where it stands, so it knows where its server is: beside itself on macOS and Windows, and on
//! Linux the AppImage itself, run with `mcp` - an AppImage is mounted at a new path every run, and nothing outside can
//! name a file inside it.
//!
//! Claude Desktop reads one settings file. The window adds the server to it under the name `qymcad`, changes nothing
//! else there, and keeps the file as it was beside it first. A file it cannot read is left alone and said so: a
//! person's settings overwritten by a guess are worse than a server not added. Claude Code is told by a command in a
//! terminal, so the window shows that command, ready to copy.

use std::path::{Path, PathBuf};

use egui_phosphor::regular as ph;
use qymcad_ui_state::{WinKind, Windows};

/// The system the program runs on, as far as where Claude keeps its settings and how the server is started.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Os {
    Mac,
    Windows,
    Linux,
}

impl Os {
    pub(crate) fn this_one() -> Os {
        if cfg!(target_os = "macos") {
            Os::Mac
        } else if cfg!(windows) {
            Os::Windows
        } else {
            Os::Linux
        }
    }
}

/// How Claude starts the server: the program to run and what to give it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ServerCommand {
    pub command: String,
    pub args: Vec<String>,
}

/// WHERE THE SERVER OF THIS PACKAGE IS, for a program running from `exe`. `appimage` is the path the AppImage runtime
/// says the package was started from (its `APPIMAGE`), when it was.
pub(crate) fn server_for(exe: &Path, appimage: Option<&str>, os: Os) -> ServerCommand {
    if let (Os::Linux, Some(package)) = (os, appimage.filter(|p| !p.is_empty())) {
        return ServerCommand { command: package.to_string(), args: vec!["mcp".into()] };
    }
    let name = if os == Os::Windows { "qymcad-mcp.exe" } else { "qymcad-mcp" };
    let beside = exe.parent().map_or_else(|| PathBuf::from(name), |dir| dir.join(name));
    ServerCommand { command: beside.to_string_lossy().into_owned(), args: Vec::new() }
}

/// WHERE CLAUDE DESKTOP KEEPS ITS SETTINGS on `os`, read from the environment `env` gives: the files Claude Desktop
/// may read there, each given the server. Empty when the environment does not say where home is.
///
/// ON WINDOWS CLAUDE DESKTOP MAY BE A PACKAGE, and a packaged program reads its settings from a copy of its own,
/// `%LOCALAPPDATA%\Packages\Claude_<publisher>\LocalCache\Roaming\Claude`, not from `%APPDATA%\Claude`. Reported
/// behaviour: the button wrote the server into `%APPDATA%\Claude\claude_desktop_config.json`, the packaged Claude Desktop
/// (`WindowsApps\Claude_..._pzs8sxrjxfjjc`) read its own copy, which had no `mcpServers` at all, and never started the
/// server. Where the package's folder is, its copy comes first; `%APPDATA%` is written as well, because a package's
/// folder outlives the package, and a Claude Desktop installed otherwise afterwards reads `%APPDATA%`.
pub(crate) fn desktop_configs(os: Os, env: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let file = |dir: PathBuf| dir.join("Claude").join("claude_desktop_config.json");
    match os {
        Os::Mac => env("HOME").map(|h| file(PathBuf::from(h).join("Library").join("Application Support"))).into_iter().collect(),
        Os::Windows => {
            let packaged = env("LOCALAPPDATA").and_then(|l| packaged_roaming(Path::new(&l))).map(file);
            packaged.into_iter().chain(env("APPDATA").map(|a| file(PathBuf::from(a)))).collect()
        }
        Os::Linux => env("XDG_CONFIG_HOME").filter(|x| !x.is_empty()).map(PathBuf::from).or_else(|| env("HOME").map(|h| PathBuf::from(h).join(".config"))).map(file).into_iter().collect(),
    }
}

/// THE ROAMING FOLDER OF A PACKAGED CLAUDE DESKTOP under `local` (`%LOCALAPPDATA%`): the first package named
/// `Claude_<publisher>` that holds `LocalCache\Roaming\Claude`, the folder the package reads in place of `%APPDATA%`.
fn packaged_roaming(local: &Path) -> Option<PathBuf> {
    let mut packages: Vec<PathBuf> = std::fs::read_dir(local.join("Packages"))
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("Claude_"))
        .map(|e| e.path().join("LocalCache").join("Roaming"))
        .filter(|roaming| roaming.join("Claude").is_dir())
        .collect();
    packages.sort();
    packages.into_iter().next()
}

/// The settings could not be read as settings: what was found instead.
#[derive(Debug, PartialEq)]
pub(crate) struct Unreadable {
    pub why: String,
}

/// THE SETTINGS OF CLAUDE DESKTOP WITH THE SERVER IN THEM: `existing` (none, or empty, for a file not there yet) with
/// `mcpServers.qymcad` set to `server`, every other setting and server kept. Written back with its keys in order of
/// the alphabet - Claude Desktop reads them by name.
pub(crate) fn merged(existing: Option<&str>, server: &ServerCommand) -> Result<String, Unreadable> {
    let text = existing.map(str::trim).filter(|t| !t.is_empty()).unwrap_or("{}");
    let mut root: serde_json::Value = serde_json::from_str(text).map_err(|e| Unreadable { why: e.to_string() })?;
    let top = root.as_object_mut().ok_or_else(|| Unreadable { why: "the file is not a JSON object".into() })?;
    let servers = top.entry("mcpServers").or_insert_with(|| serde_json::json!({}));
    let servers = servers.as_object_mut().ok_or_else(|| Unreadable { why: "mcpServers is not a JSON object".into() })?;
    let mut entry = serde_json::json!({ "command": server.command });
    if !server.args.is_empty() {
        entry["args"] = serde_json::json!(server.args);
    }
    servers.insert("qymcad".into(), entry);
    serde_json::to_string_pretty(&root).map(|s| s + "\n").map_err(|e| Unreadable { why: e.to_string() })
}

/// What adding the server to Claude Desktop did.
#[derive(Debug, PartialEq)]
pub(crate) struct Added {
    /// Where the file as it was before is kept; none when there was no file.
    pub kept: Option<PathBuf>,
}

/// ADD THE SERVER TO CLAUDE DESKTOP'S SETTINGS at `config`: the file as it was is copied beside it first
/// (`claude_desktop_config.json.qymcad-backup`), then the settings are written with the server in them. Refused, and the
/// file untouched, when it cannot be read as settings.
pub(crate) fn add_to_desktop(config: &Path, server: &ServerCommand) -> Result<Added, String> {
    let existing = match std::fs::read_to_string(config) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    let text = merged(existing.as_deref(), server).map_err(|u| u.why)?;
    let kept = match &existing {
        Some(before) => {
            let backup = config.with_extension("json.qymcad-backup");
            std::fs::write(&backup, before).map_err(|e| e.to_string())?;
            Some(backup)
        }
        None => None,
    };
    if let Some(dir) = config.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(config, text).map_err(|e| e.to_string())?;
    Ok(Added { kept })
}

/// THE COMMAND THAT TELLS CLAUDE CODE: `claude mcp add qymcad -- <server> [args]`, a path with spaces in quotes.
pub(crate) fn claude_code_command(server: &ServerCommand) -> String {
    let quoted = |s: &str| if s.contains(' ') { format!("\"{s}\"") } else { s.to_string() };
    let mut line = format!("claude mcp add qymcad -- {}", quoted(&server.command));
    for a in &server.args {
        line.push(' ');
        line.push_str(&quoted(a));
    }
    line
}

/// THIS MACHINE as the window sees it: where Claude Desktop's settings are, and the server of this package - with
/// whether it is there (a build run from the sources may have no server built beside it).
pub(crate) struct Machine {
    pub configs: Vec<PathBuf>,
    pub server: ServerCommand,
    pub present: Presence,
}

/// Whether the server is where the program says it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Presence {
    There,
    Missing,
}

impl Machine {
    pub(crate) fn this_one() -> Machine {
        let os = Os::this_one();
        let exe = std::env::current_exe().unwrap_or_default();
        let appimage = std::env::var("APPIMAGE").ok();
        let server = server_for(&exe, appimage.as_deref(), os);
        let present = if Path::new(&server.command).exists() { Presence::There } else { Presence::Missing };
        Machine { configs: desktop_configs(os, &|k| std::env::var(k).ok()), server, present }
    }
}

/// What the window last did, kept for the frames after the press: egui's memory, not a field of the application - it
/// is the window's own passing state.
#[derive(Clone, Default)]
struct Said {
    text: String,
    trouble: Trouble,
}

/// Whether what was said is good news.
#[derive(Clone, Copy, Default, PartialEq)]
enum Trouble {
    #[default]
    None,
    Some,
}

/// THE SERVER ADDED TO EVERY FILE CLAUDE DESKTOP MAY READ, and what the window says of it: the first refusal, or
/// where the first file as it was is kept.
fn add_to_every(configs: &[PathBuf], server: &ServerCommand) -> Said {
    let mut kept_first = None;
    for config in configs {
        match add_to_desktop(config, server) {
            Ok(Added { kept }) => kept_first = kept_first.or(kept),
            Err(why) => return Said { text: crate::i18n::tr1("claude-desktop-refused", "why", &why), trouble: Trouble::Some },
        }
    }
    match kept_first {
        Some(kept) => Said { text: crate::i18n::tr1("claude-desktop-added-kept", "kept", &crate::crash::without_home(&kept.to_string_lossy())), trouble: Trouble::None },
        None => Said { text: crate::i18n::tr("claude-desktop-added"), trouble: Trouble::None },
    }
}

fn said_id() -> egui::Id {
    egui::Id::new("qym-connect-claude-said")
}

/// THE WINDOW. Drawn while it is open; `machine` says where things are on this computer.
pub(crate) fn window(win: &mut Windows, scheme: &qymcad_ui_state::SchemeUi, machine: &Machine, ctx: &egui::Context) {
    if !win.is(WinKind::Claude) {
        return;
    }
    let tr = crate::i18n::tr;
    let mut open = true;
    let mut said: Said = ctx.data(|d| d.get_temp(said_id())).unwrap_or_default();
    egui::Window::new(format!("{} {}", ph::PLUGS_CONNECTED, tr("claude-title"))).open(&mut open).collapsible(false).default_width(560.0).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(
        ctx,
        |ui| {
            ui.label(tr("claude-intro"));
            ui.add_space(6.0);
            if machine.present == Presence::Missing {
                ui.colored_label(scheme.pal.warning(), format!("{} {}", ph::WARNING, tr("claude-no-server")));
                ui.label(egui::RichText::new(crate::crash::without_home(&machine.server.command)).monospace().small());
                return;
            }

            ui.add_space(6.0);
            ui.label(egui::RichText::new(tr("claude-desktop")).strong());
            ui.label(tr("claude-desktop-how"));
            if machine.configs.is_empty() {
                ui.label(tr("claude-desktop-no-home"));
            } else {
                if ui.button(format!("{} {}", ph::PLUS_CIRCLE, tr("claude-desktop-add"))).clicked() {
                    said = add_to_every(&machine.configs, &machine.server);
                }
                for config in &machine.configs {
                    ui.label(egui::RichText::new(crate::crash::without_home(&config.to_string_lossy())).monospace().small().weak());
                }
            }
            if !said.text.is_empty() {
                if said.trouble == Trouble::Some {
                    ui.colored_label(scheme.pal.warning(), format!("{} {}", ph::WARNING, said.text));
                } else {
                    ui.label(format!("{} {}", ph::CHECK_CIRCLE, said.text));
                }
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(egui::RichText::new(tr("claude-code")).strong());
            ui.label(tr("claude-code-how"));
            let line = claude_code_command(&machine.server);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&line).monospace());
                if ui.button(format!("{} {}", ph::COPY, tr("claude-code-copy"))).clicked() {
                    ui.output_mut(|o| o.commands.push(egui::OutputCommand::CopyText(line.clone())));
                }
            });

            // CLAUDE IN THIS WINDOW: the same server, sending its calls to the document open here when the setting
            // lets them in. The switch itself stays in the settings, with the others; this says where it is.
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(egui::RichText::new(tr("settings-claude")).strong());
            ui.label(tr("claude-window-how"));
            if ui.button(format!("{} {}", ph::GEAR, tr("claude-window-open"))).clicked() {
                win.open(WinKind::Settings);
            }
        },
    );
    ctx.data_mut(|d| d.insert_temp(said_id(), said));
    if !open {
        win.close(WinKind::Claude);
        ctx.data_mut(|d| d.remove::<Said>(said_id()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::check_folder::tests::CheckFolder;

    fn server() -> ServerCommand {
        ServerCommand { command: "/Applications/QymCAD.app/Contents/MacOS/qymcad-mcp".into(), args: Vec::new() }
    }

    #[test]
    fn the_server_is_found_beside_the_program_and_in_the_appimage() {
        let mac = server_for(Path::new("/Applications/QymCAD.app/Contents/MacOS/qymcad"), None, Os::Mac);
        assert_eq!(mac, ServerCommand { command: "/Applications/QymCAD.app/Contents/MacOS/qymcad-mcp".into(), args: Vec::new() });
        let win = server_for(Path::new("C:/Program Files/QymCAD/qymcad.exe"), None, Os::Windows);
        assert!(win.command.ends_with("qymcad-mcp.exe") && win.command.contains("Program Files"), "{win:?}");
        // the package itself, run with mcp: nothing outside can name a file inside a mounted AppImage
        let linux = server_for(Path::new("/tmp/.mount_qymXYZ/usr/bin/qymcad"), Some("/home/me/Apps/qymcad.AppImage"), Os::Linux);
        assert_eq!(linux, ServerCommand { command: "/home/me/Apps/qymcad.AppImage".into(), args: vec!["mcp".into()] });
        // a Linux build that is no AppImage keeps its server beside it, like the others
        let plain = server_for(Path::new("/usr/bin/qymcad"), None, Os::Linux);
        assert_eq!(plain.command, "/usr/bin/qymcad-mcp");
    }

    #[test]
    fn claude_desktop_settings_are_where_claude_keeps_them() {
        let env = |k: &str| match k {
            "HOME" => Some("/Users/me".to_string()),
            "APPDATA" => Some("C:\\Users\\me\\AppData\\Roaming".to_string()),
            _ => None,
        };
        let one = |os: Os, env: &dyn Fn(&str) -> Option<String>| desktop_configs(os, env).into_iter().next();
        let mac = one(Os::Mac, &env).expect("a home");
        assert_eq!(mac, PathBuf::from("/Users/me/Library/Application Support/Claude/claude_desktop_config.json"));
        let win = one(Os::Windows, &env).expect("an AppData");
        assert!(win.ends_with("Claude/claude_desktop_config.json") || win.to_string_lossy().ends_with("Claude\\claude_desktop_config.json"), "{win:?}");
        assert!(win.starts_with("C:\\Users\\me\\AppData\\Roaming"), "{win:?}");
        let linux = one(Os::Linux, &env).expect("a home");
        assert_eq!(linux, PathBuf::from("/Users/me/.config/Claude/claude_desktop_config.json"));
        let xdg = one(Os::Linux, &|k: &str| (k == "XDG_CONFIG_HOME").then(|| "/cfg".to_string())).expect("XDG");
        assert_eq!(xdg, PathBuf::from("/cfg/Claude/claude_desktop_config.json"));
        assert!(desktop_configs(Os::Mac, &|_: &str| None).is_empty());
    }

    /// A PACKAGED CLAUDE DESKTOP ON WINDOWS IS GIVEN THE SERVER IN ITS OWN COPY OF THE SETTINGS. Reported behaviour: the
    /// server went into `%APPDATA%\Claude`, the packaged Claude Desktop read
    /// `%LOCALAPPDATA%\Packages\Claude_<publisher>\LocalCache\Roaming\Claude` and never started it. With no such package,
    /// or a package of another program, `%APPDATA%` alone; with one, its copy first and `%APPDATA%` too - and the button
    /// gives both the server, keeping the package's own settings.
    #[test]
    fn a_packaged_claude_desktop_on_windows_gets_its_own_settings() {
        let folder = CheckFolder::new("claude-packaged");
        let local = folder.file("Local");
        let roaming = folder.file("Roaming");
        std::fs::create_dir_all(local.join("Packages").join("ClaudeHelper.Other_1").join("LocalCache").join("Roaming").join("Claude")).expect("another package");
        let env = |k: &str| match k {
            "LOCALAPPDATA" => Some(local.to_string_lossy().into_owned()),
            "APPDATA" => Some(roaming.to_string_lossy().into_owned()),
            _ => None,
        };
        let plain = roaming.join("Claude").join("claude_desktop_config.json");
        assert_eq!(desktop_configs(Os::Windows, &env), vec![plain.clone()], "with no Claude package the settings are not %APPDATA%'s alone");

        let package = local.join("Packages").join("Claude_pzs8sxrjxfjjc").join("LocalCache").join("Roaming").join("Claude");
        std::fs::create_dir_all(&package).expect("the package's roaming folder");
        let own = package.join("claude_desktop_config.json");
        let configs = desktop_configs(Os::Windows, &env);
        assert_eq!(configs, vec![own.clone(), plain.clone()], "the packaged Claude Desktop is not given its own copy first");

        // the button: the server lands in both, the package's own settings kept
        std::fs::write(&own, r#"{ "preferences": { "sidebarMode": "chat" } }"#).expect("the package's settings");
        let said = add_to_every(&configs, &server());
        assert!(said.trouble == Trouble::None, "the button refused: {}", said.text);
        for file in [&own, &plain] {
            let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file).expect("read")).expect("JSON");
            assert_eq!(written["mcpServers"]["qymcad"]["command"], serde_json::json!(server().command), "{}: {written}", file.display());
        }
        let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&own).expect("read")).expect("JSON");
        assert_eq!(written["preferences"]["sidebarMode"], "chat", "the package's own settings were lost: {written}");
    }

    /// THE PERSON'S OTHER SETTINGS AND SERVERS STAY: only `mcpServers.qymcad` is set, and set again over an old one.
    #[test]
    fn the_server_is_added_and_nothing_else_is_touched() {
        let before = r#"{ "globalShortcut": "Alt+Space", "mcpServers": { "files": { "command": "npx", "args": ["-y", "fs"] }, "qymcad": { "command": "/old/qymcad-mcp" } } }"#;
        let after: serde_json::Value = serde_json::from_str(&merged(Some(before), &server()).expect("merged")).expect("JSON");
        assert_eq!(after["globalShortcut"], "Alt+Space", "{after}");
        assert_eq!(after["mcpServers"]["files"]["args"], serde_json::json!(["-y", "fs"]), "{after}");
        assert_eq!(after["mcpServers"]["qymcad"], serde_json::json!({ "command": "/Applications/QymCAD.app/Contents/MacOS/qymcad-mcp" }), "{after}");
        // no file yet, or an empty one: settings of the server alone
        for nothing in [None, Some(""), Some("  \n")] {
            let fresh: serde_json::Value = serde_json::from_str(&merged(nothing, &server()).expect("merged")).expect("JSON");
            assert_eq!(fresh, serde_json::json!({ "mcpServers": { "qymcad": { "command": "/Applications/QymCAD.app/Contents/MacOS/qymcad-mcp" } } }));
        }
        // the AppImage: its argument goes with it
        let image = ServerCommand { command: "/home/me/q.AppImage".into(), args: vec!["mcp".into()] };
        let with_args: serde_json::Value = serde_json::from_str(&merged(None, &image).expect("merged")).expect("JSON");
        assert_eq!(with_args["mcpServers"]["qymcad"]["args"], serde_json::json!(["mcp"]));
    }

    #[test]
    fn settings_that_cannot_be_read_are_left_alone() {
        for broken in ["{ not json", "[1, 2]", r#"{ "mcpServers": [] }"#] {
            assert!(merged(Some(broken), &server()).is_err(), "{broken} was taken as settings");
        }
        let folder = CheckFolder::new("claude-broken");
        let config = folder.file("claude_desktop_config.json");
        std::fs::write(&config, "{ not json").expect("written");
        assert!(add_to_desktop(&config, &server()).is_err(), "broken settings were written over");
        assert_eq!(std::fs::read_to_string(&config).expect("read"), "{ not json", "the broken file was touched");
        assert!(!config.with_extension("json.qymcad-backup").exists(), "a copy was made of what was not written over");
    }

    /// THE FILE AS IT WAS IS KEPT BESIDE IT, and a folder that is not there yet is made.
    #[test]
    fn the_old_settings_are_kept_beside_the_new() {
        let folder = CheckFolder::new("claude-kept");
        let config = folder.file("Claude").join("claude_desktop_config.json");
        let first = add_to_desktop(&config, &server()).expect("added to no file");
        assert_eq!(first, Added { kept: None });
        let before = std::fs::read_to_string(&config).expect("written");
        let second = add_to_desktop(&config, &server()).expect("added again");
        let kept = second.kept.expect("the old file kept");
        assert_eq!(std::fs::read_to_string(kept).expect("the copy"), before);
    }

    #[test]
    fn claude_code_is_told_by_a_command_with_its_path_quoted() {
        assert_eq!(claude_code_command(&server()), "claude mcp add qymcad -- /Applications/QymCAD.app/Contents/MacOS/qymcad-mcp");
        let win = ServerCommand { command: "C:\\Program Files\\QymCAD\\qymcad-mcp.exe".into(), args: Vec::new() };
        assert_eq!(claude_code_command(&win), "claude mcp add qymcad -- \"C:\\Program Files\\QymCAD\\qymcad-mcp.exe\"");
        let image = ServerCommand { command: "/home/me/q.AppImage".into(), args: vec!["mcp".into()] };
        assert_eq!(claude_code_command(&image), "claude mcp add qymcad -- /home/me/q.AppImage mcp");
    }

    // THROUGH A REAL FRAME: the window drawn by egui, its button found among the shapes it painted and pressed with
    // pointer events - a call of the handler would skip the frame's own reading of the input.
    const SCREEN: egui::Vec2 = egui::vec2(1400.0, 900.0);

    fn raw() -> egui::RawInput {
        egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), SCREEN)), ..Default::default() }
    }

    /// Every text a frame painted, with where.
    struct Painted {
        text: String,
        at: egui::Rect,
    }

    fn painted(shapes: &[egui::epaint::ClippedShape]) -> Vec<Painted> {
        fn walk(s: &egui::epaint::Shape, out: &mut Vec<Painted>) {
            match s {
                egui::epaint::Shape::Text(t) => out.push(Painted { text: t.galley.text().to_string(), at: egui::Rect::from_min_size(t.pos, t.galley.size()) }),
                egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let mut out = Vec::new();
        for cs in shapes {
            walk(&cs.shape, &mut out);
        }
        out
    }

    /// A machine whose Claude Desktop keeps its settings in the check's own folder.
    fn sandboxed(folder: &CheckFolder, present: Presence) -> Machine {
        Machine { configs: vec![folder.file("Claude").join("claude_desktop_config.json")], server: server(), present }
    }

    struct Frame {
        ctx: egui::Context,
        win: Windows,
        scheme: qymcad_ui_state::SchemeUi,
    }

    impl Frame {
        fn open() -> Frame {
            crate::i18n::set_language("en");
            let ctx = egui::Context::default();
            crate::gui::install_fonts(&ctx);
            let mut win = Windows::default();
            win.open(WinKind::Claude);
            Frame { ctx, win, scheme: qymcad_ui_state::SchemeUi::default() }
        }

        fn run(&mut self, machine: &Machine, input: egui::RawInput) -> egui::FullOutput {
            let (win, scheme) = (&mut self.win, &self.scheme);
            self.ctx.run_ui(input, |c| window(win, scheme, machine, c.ctx()))
        }

        /// Press what reads `label`, with a real press and release.
        fn press(&mut self, machine: &Machine, label: &str) -> egui::FullOutput {
            let out = self.run(machine, raw());
            let spot = painted(&out.shapes).into_iter().find(|p| p.text.contains(label)).map(|p| p.at.center()).unwrap_or_else(|| panic!("nothing reads {label:?}"));
            let down = egui::RawInput {
                events: vec![egui::Event::PointerMoved(spot), egui::Event::PointerButton { pos: spot, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }],
                ..raw()
            };
            let _ = self.run(machine, down);
            let up = egui::RawInput { events: vec![egui::Event::PointerButton { pos: spot, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }], ..raw() };
            self.run(machine, up)
        }
    }

    /// THE WAY TO CLAUDE IN THIS WINDOW IS SHOWN: the window says what the setting does and its button opens the settings
    /// where the switch is.
    #[test]
    fn the_window_leads_to_the_switch_for_claude_in_this_window() {
        let folder = CheckFolder::new("claude-window-switch");
        let machine = sandboxed(&folder, Presence::There);
        let mut f = Frame::open();
        let _ = f.run(&machine, raw()); // the window lays itself out on its first frame
        let texts: Vec<String> = painted(&f.run(&machine, raw()).shapes).into_iter().map(|p| p.text).collect();
        assert!(texts.iter().any(|t| t.contains(&crate::i18n::tr("settings-claude"))), "the window does not name Claude in this window: {texts:?}");
        assert!(!f.win.is(WinKind::Settings), "setup: the settings were open before anything was pressed");
        let _ = f.press(&machine, &crate::i18n::tr("claude-window-open"));
        assert!(f.win.is(WinKind::Settings), "the button did not open the settings");
    }

    /// THE BUTTON ADDS THE SERVER TO CLAUDE DESKTOP'S SETTINGS and the window says so, and how to finish.
    #[test]
    fn the_button_gives_claude_desktop_the_server() {
        let folder = CheckFolder::new("claude-window-add");
        let machine = sandboxed(&folder, Presence::There);
        let mut f = Frame::open();
        let _ = f.run(&machine, raw()); // the first frame lays the window out
        let _ = f.press(&machine, &crate::i18n::tr("claude-desktop-add"));
        let out = f.run(&machine, raw());
        let config = machine.configs[0].clone();
        let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&config).unwrap_or_else(|e| panic!("no settings written at {}: {e}", config.display()))).expect("JSON");
        assert_eq!(written["mcpServers"]["qymcad"]["command"], serde_json::json!(server().command), "{written}");
        let texts = painted(&out.shapes);
        assert!(texts.iter().any(|p| p.text.contains(&crate::i18n::tr("claude-desktop-added"))), "the window does not say it was added: {:?}", texts.iter().map(|p| &p.text).collect::<Vec<_>>());
    }

    /// CLAUDE CODE'S COMMAND IS SHOWN AND COPIED whole.
    #[test]
    fn the_command_for_claude_code_is_shown_and_copied() {
        let folder = CheckFolder::new("claude-window-code");
        let machine = sandboxed(&folder, Presence::There);
        let mut f = Frame::open();
        let _ = f.run(&machine, raw()); // the first frame lays the window out
        let out = f.run(&machine, raw());
        let line = claude_code_command(&machine.server);
        assert!(painted(&out.shapes).iter().any(|p| p.text == line), "the command is not shown");
        let out = f.press(&machine, &crate::i18n::tr("claude-code-copy"));
        let copied: Vec<String> = out.platform_output.commands.iter().filter_map(|c| if let egui::OutputCommand::CopyText(t) = c { Some(t.clone()) } else { None }).collect();
        assert_eq!(copied, vec![line], "the copy button did not hand the command over");
    }

    /// A BUILD WITHOUT THE SERVER SAYS SO, and offers nothing to press that would point Claude at nothing.
    #[test]
    fn a_build_without_the_server_says_so() {
        let folder = CheckFolder::new("claude-window-missing");
        let machine = sandboxed(&folder, Presence::Missing);
        let mut f = Frame::open();
        let _ = f.run(&machine, raw());
        let out = f.run(&machine, raw());
        let texts: Vec<String> = painted(&out.shapes).into_iter().map(|p| p.text).collect();
        assert!(texts.iter().any(|t| t.contains(&crate::i18n::tr("claude-no-server"))), "{texts:?}");
        assert!(!texts.iter().any(|t| t.contains(&crate::i18n::tr("claude-desktop-add"))), "a button to add a server that is not there: {texts:?}");
    }
}
