//! WHAT DRAWS THE WINDOW: the modern drawing interfaces through wgpu (Direct3D 12, Vulkan, Metal), or OpenGL through
//! the framework's second renderer. Decided once, before the window opens; the setting takes effect on a restart.
//!
//! Reported behaviour: on Windows with an Intel HD Graphics 2500 the whole interface was slow - menus, panels, the
//! pointer over buttons - and stayed slow with the 3D view on "GPU" or on "CPU": a quarter of the screen was quick,
//! the whole screen was not. The card has no Direct3D 12 driver, so wgpu draws the window with Microsoft's software
//! adapter, and the processor pays for every pixel of every frame. The same card has an OpenGL 4.0 driver. wgpu's own
//! OpenGL path is no answer there: on that card it panicked a few seconds into a session (see
//! `rank_backend_and_type`). The framework's OpenGL renderer is a different, smaller path: it draws the interface on
//! the card, and the 3D view goes to the processor's raster, which turned the model quickly on that machine.

use eframe::wgpu::{Backend, DeviceType};
use qymcad_ui_state::WindowDrawer;

use super::Host;
use crate::diagnostics::{Drawer, DrawerKind};
use crate::safe_graphics::Step;

/// Set on the second start this module makes after the OpenGL window would not open: that start takes the modern
/// path, whatever the setting says, so a driver that refuses OpenGL leaves the program as it was rather than closed.
const AFTER_OPENGL_FAILED: &str = "QYMCAD_AFTER_OPENGL_FAILED";

/// One adapter the system offers, as the choice needs it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Offer {
    pub(crate) backend: Backend,
    pub(crate) kind: DeviceType,
}

/// THE RENDERER FOR `chosen` ON `host` AT `step` WITH `offered`. `Auto` takes OpenGL only on Windows, only where no
/// modern interface reaches a graphics card and OpenGL reaches one; on Linux wgpu reaches the card through OpenGL
/// itself, and on macOS Metal is always there.
///
/// A START AFTER A START THAT DIED DRAWING (`safe_graphics`) does not go back to what may have killed it: `Auto` takes
/// OpenGL only as chosen, and OpenGL picked in the settings yields from the step of another backend on, as the adapter
/// picked there does. Otherwise a driver that dies in OpenGL would die at every start, the steps down never reaching it.
pub(crate) fn the_renderer(chosen: WindowDrawer, host: Host, step: Step, offered: &[Offer]) -> eframe::Renderer {
    match chosen {
        WindowDrawer::Modern => eframe::Renderer::Wgpu,
        WindowDrawer::OpenGl if step < Step::OtherBackend => eframe::Renderer::Glow,
        WindowDrawer::OpenGl => eframe::Renderer::Wgpu,
        WindowDrawer::Auto if step > Step::AsChosen => eframe::Renderer::Wgpu,
        WindowDrawer::Auto => {
            let card = |o: &&Offer| matches!(o.kind, DeviceType::DiscreteGpu | DeviceType::IntegratedGpu);
            let modern_card = offered.iter().filter(card).any(|o| o.backend != Backend::Gl);
            let opengl_card = offered.iter().filter(card).any(|o| o.backend == Backend::Gl);
            if host == Host::Windows && !modern_card && opengl_card {
                eframe::Renderer::Glow
            } else {
                eframe::Renderer::Wgpu
            }
        }
    }
}

/// The renderer of this start: the setting read from the settings file, and on Windows with `Auto` the adapters the
/// system offers. Elsewhere nothing is asked: enumerating adapters costs a part of a second at every start.
pub(crate) fn for_this_start(step: Step) -> eframe::Renderer {
    let chosen = if std::env::var_os(AFTER_OPENGL_FAILED).is_some() { WindowDrawer::Modern } else { chosen_in(qymcad_paths::settings_file().as_deref()) };
    let host = if cfg!(windows) { Host::Windows } else { Host::Other };
    let offered = if chosen == WindowDrawer::Auto && host == Host::Windows { offered() } else { Vec::new() };
    the_renderer(chosen, host, step, &offered)
}

/// The one setting read here; every other one in the file is passed over.
#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct Kept {
    window_drawer: WindowDrawer,
}

/// WHAT `file` CHOSE TO DRAW THE WINDOW: `Auto` when there is no file, it cannot be read, or the setting is not in it.
fn chosen_in(file: Option<&std::path::Path>) -> WindowDrawer {
    file.and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|text| ron::from_str::<std::collections::HashMap<String, String>>(&text).ok())
        .and_then(|kept| kept.get("settings").and_then(|s| ron::from_str::<Kept>(s).ok()))
        .map(|k| k.window_drawer)
        .unwrap_or_default()
}

/// The adapters the system offers through the modern interfaces and OpenGL, with no window yet. On a desktop the
/// answer is ready at once; one that is not is taken as none, which leaves the modern path.
fn offered() -> Vec<Offer> {
    use eframe::wgpu;
    use std::future::Future;
    let backends = wgpu::Backends::PRIMARY | wgpu::Backends::GL;
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backends;
    let instance = wgpu::Instance::new(desc);
    let mut asked = std::pin::pin!(instance.enumerate_adapters(backends));
    match asked.as_mut().poll(&mut std::task::Context::from_waker(std::task::Waker::noop())) {
        std::task::Poll::Ready(adapters) => adapters
            .iter()
            .map(|a| {
                let i = a.get_info();
                Offer { backend: i.backend, kind: i.device_type }
            })
            .collect(),
        std::task::Poll::Pending => Vec::new(),
    }
}

/// THE WINDOW ON OPENGL DID NOT OPEN: the program starts once more on the modern path and ends with that start's
/// code. Only for a start `Auto` sent to OpenGL - a person who chose OpenGL is told why it failed instead.
pub(crate) fn start_again_on_the_modern_path(renderer: eframe::Renderer) {
    let auto = chosen_in(qymcad_paths::settings_file().as_deref()) == WindowDrawer::Auto;
    if renderer != eframe::Renderer::Glow || !auto || std::env::var_os(AFTER_OPENGL_FAILED).is_some() {
        return;
    }
    let Ok(exe) = std::env::current_exe() else { return };
    if let Ok(status) = std::process::Command::new(exe).args(std::env::args_os().skip(1)).env(AFTER_OPENGL_FAILED, "1").status() {
        std::process::exit(status.code().unwrap_or(1));
    }
}

/// THE ADAPTER OF AN OPENGL WINDOW, from the renderer string its driver gives. The drivers that draw on the processor
/// name themselves: "GDI Generic" (Windows' own OpenGL 1.1), "llvmpipe" and "softpipe" (Mesa), "SwiftShader".
pub(crate) fn gl_drawer(renderer: &str) -> Drawer {
    let soft = ["GDI Generic", "llvmpipe", "softpipe", "SwiftShader", "Basic Render"];
    let kind = if soft.iter().any(|s| renderer.contains(s)) { DrawerKind::Processor } else { DrawerKind::Card };
    Drawer { name: renderer.to_string(), interface: "OpenGL".to_string(), kind }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn offer(backend: Backend, kind: DeviceType) -> Offer {
        Offer { backend, kind }
    }

    /// THE WINDOW IS DRAWN BY THE CARD WHERE ONLY OPENGL REACHES IT: the reported machine (Direct3D 12 on the
    /// processor only, the Intel card through OpenGL) opens on OpenGL; a machine whose card answers to a modern
    /// interface, a machine with no card at all, and any machine but Windows keep the modern path; a setting other
    /// than `Auto` is obeyed as it stands; after a start that died drawing, `Auto` keeps the modern path, and OpenGL
    /// picked in the settings yields from the step of another backend.
    #[test]
    fn the_window_is_drawn_by_the_card_where_only_opengl_reaches_it() {
        use eframe::Renderer::{Glow, Wgpu};
        let reported = [offer(Backend::Dx12, DeviceType::Cpu), offer(Backend::Gl, DeviceType::IntegratedGpu)];
        let modern = [offer(Backend::Dx12, DeviceType::IntegratedGpu), offer(Backend::Gl, DeviceType::IntegratedGpu)];
        let no_card = [offer(Backend::Dx12, DeviceType::Cpu), offer(Backend::Gl, DeviceType::Cpu)];
        let linux = [offer(Backend::Vulkan, DeviceType::Cpu), offer(Backend::Gl, DeviceType::IntegratedGpu)];
        struct Case<'a> {
            what: &'a str,
            chosen: WindowDrawer,
            host: Host,
            step: Step,
            offered: &'a [Offer],
            wanted: eframe::Renderer,
        }
        let (auto, first) = (WindowDrawer::Auto, Step::AsChosen);
        let cases = [
            Case { what: "the reported machine", chosen: auto, host: Host::Windows, step: first, offered: &reported, wanted: Glow },
            Case { what: "a card on Direct3D 12", chosen: auto, host: Host::Windows, step: first, offered: &modern, wanted: Wgpu },
            Case { what: "no card at all", chosen: auto, host: Host::Windows, step: first, offered: &no_card, wanted: Wgpu },
            Case { what: "nothing asked", chosen: auto, host: Host::Windows, step: first, offered: &[], wanted: Wgpu },
            Case { what: "not Windows", chosen: auto, host: Host::Other, step: first, offered: &linux, wanted: Wgpu },
            Case { what: "the reported machine after a start that died", chosen: auto, host: Host::Windows, step: Step::NoAntialiasing, offered: &reported, wanted: Wgpu },
            Case { what: "the modern path chosen", chosen: WindowDrawer::Modern, host: Host::Windows, step: first, offered: &reported, wanted: Wgpu },
            Case { what: "OpenGL chosen", chosen: WindowDrawer::OpenGl, host: Host::Other, step: Step::NoAntialiasing, offered: &[], wanted: Glow },
            Case { what: "OpenGL chosen, at the step of another backend", chosen: WindowDrawer::OpenGl, host: Host::Other, step: Step::OtherBackend, offered: &[], wanted: Wgpu },
        ];
        let wrong: Vec<String> = cases
            .iter()
            .filter_map(|c| {
                let got = the_renderer(c.chosen, c.host, c.step, c.offered);
                (got != c.wanted).then(|| format!("{}: {:?} where {:?} is wanted", c.what, got, c.wanted))
            })
            .collect();
        assert!(wrong.is_empty(), "the window is drawn by the wrong renderer:\n  {}", wrong.join("\n  "));
    }

    /// THE SETTING IS READ WHERE THE WINDOW KEEPS IT, before there is a window: the settings file the window writes,
    /// with the other settings around it; no file, or a file that does not parse, is `Auto`.
    #[test]
    fn the_setting_is_read_from_the_settings_file() {
        let folder = super::super::check_folder::tests::CheckFolder::new("window-drawer");
        let kept = folder.file("app.ron");
        let settings = qymcad_ui_state::Settings { window_drawer: WindowDrawer::OpenGl, ..Default::default() };
        let inner = ron::to_string(&settings).expect("the settings as text");
        let outer = ron::to_string(&std::collections::HashMap::from([("settings".to_string(), inner)])).expect("the file as text");
        std::fs::write(&kept, outer).expect("the settings file");
        let broken = folder.file("broken.ron");
        std::fs::write(&broken, "(settings: ").expect("a broken file");

        assert_eq!(chosen_in(Some(&kept)), WindowDrawer::OpenGl, "the choice written by the window is not read");
        assert_eq!(chosen_in(Some(&folder.file("absent.ron"))), WindowDrawer::Auto, "no file is not Auto");
        assert_eq!(chosen_in(Some(&broken)), WindowDrawer::Auto, "a broken file is not Auto");
        assert_eq!(chosen_in(None), WindowDrawer::Auto, "no settings folder is not Auto");
    }

    /// AN OPENGL DRIVER THAT DRAWS ON THE PROCESSOR IS NAMED SO: the line "Draws" says "processor" for Windows' own
    /// OpenGL and for Mesa's software ones, and "graphics card" for a card's driver.
    #[test]
    fn an_opengl_driver_on_the_processor_is_named_so() {
        assert_eq!(gl_drawer("Intel(R) HD Graphics 2500").kind, DrawerKind::Card);
        assert_eq!(gl_drawer("Intel(R) HD Graphics 2500").interface, "OpenGL");
        assert_eq!(gl_drawer("GDI Generic").kind, DrawerKind::Processor);
        assert_eq!(gl_drawer("llvmpipe (LLVM 17.0.6, 256 bits)").kind, DrawerKind::Processor);
    }
}
