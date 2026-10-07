//! WHERE THE CALLS GO: to a document of the program's own, or over the channel to the open window, whose document the
//! person sees.
//!
//! The choice is made once, at the start, and said on the command line: `--headless` keeps a document here, `--live`
//! sends every call to the window, `--auto` (the default) takes the window when one listens at the start and a
//! document of its own otherwise.
//!
//! A WINDOW LOST IS NOT REPLACED. Once the window this session reached has closed, every call is refused: a window
//! opened afterwards may hold another document, and a model working on in it unawares would edit a part it never
//! read. The program is started again for a new window.

use std::path::PathBuf;
use std::time::Duration;

use qymcad_bridge::{Link, LinkError};
use qymcad_tools::tool::{After, Ctx, Refusal, Stage};
use serde_json::Value;

/// HOW LONG THE PROGRAM WAITS for the window to finish a call it took. A rebuild of a heavy file or the import of a
/// large STEP takes minutes, not seconds; past this the call is told late, and it may still finish.
pub const LATE: Duration = Duration::from_secs(600);

/// Where the calls go, as asked on the command line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Live,
    Headless,
    Auto,
}

/// HOW THE PROGRAM STARTS: where the calls go, the window's socket when it is not the usual one, and how long a call
/// in the window may take.
#[derive(Debug)]
pub struct Start {
    pub mode: Mode,
    pub socket: Option<PathBuf>,
    pub late: Duration,
}

/// READ THE COMMAND LINE (without the program's own name): `--live`, `--headless` or `--auto`, and `--socket <path>`.
///
/// # Errors
/// The words for a person: an argument the program does not know, two modes at once, a socket with no path.
pub fn parse(args: &[String]) -> Result<Start, String> {
    let mut mode = None;
    let mut socket = None;
    let mut rest = args.iter();
    while let Some(a) = rest.next() {
        let asked = match a.as_str() {
            "--live" => Mode::Live,
            "--headless" => Mode::Headless,
            "--auto" => Mode::Auto,
            "--socket" => {
                socket = Some(PathBuf::from(rest.next().ok_or("--socket wants a path")?));
                continue;
            }
            other => return Err(format!("unknown argument {other}; the program takes --live, --headless, --auto and --socket <path>")),
        };
        if mode.is_some_and(|m| m != asked) {
            return Err("one of --live, --headless and --auto, not two".into());
        }
        mode = Some(asked);
    }
    Ok(Start { mode: mode.unwrap_or(Mode::Auto), socket, late: LATE })
}

/// WHERE THE CALLS GO for the whole run.
pub enum Engine {
    /// A document of the program's own; held boxed, being many times the size of the window's end.
    Here(Box<Ctx>),
    /// The window's document, over the channel.
    Window(Window),
}

impl Engine {
    /// The engine `start` asks for. `--auto` looks for a window once, now.
    pub fn start(start: Start) -> Engine {
        let window = Window { path: start.socket.or_else(qymcad_bridge::default_path), late: start.late, link: None, reach: Reach::NotYet };
        match start.mode {
            Mode::Headless => Engine::Here(Box::new(Ctx::blank())),
            Mode::Live => Engine::Window(window),
            Mode::Auto => {
                let mut window = window;
                match window.connect() {
                    Ok(()) => Engine::Window(window),
                    Err(_) => Engine::Here(Box::new(Ctx::blank())),
                }
            }
        }
    }
}

/// How far the session got with the window.
#[derive(Clone, Copy, PartialEq)]
enum Reach {
    /// No window answered yet: the next call looks for one.
    NotYet,
    /// A window answered; the link to it may be dropped and made again, to the same window.
    Reached,
    /// The window answered once and has closed since: nothing goes to a window again this run.
    Lost,
}

/// THE WINDOW'S END OF THE PROGRAM.
pub struct Window {
    path: Option<PathBuf>,
    late: Duration,
    link: Option<Link>,
    reach: Reach,
}

impl Window {
    fn connect(&mut self) -> Result<(), Refusal> {
        if self.reach == Reach::Lost {
            return Err(gone());
        }
        let Some(path) = &self.path else { return Err(no_window()) };
        match Link::connect(path, self.late) {
            Ok(link) => {
                self.link = Some(link);
                self.reach = Reach::Reached;
                Ok(())
            }
            Err(_) if self.reach == Reach::Reached => {
                self.reach = Reach::Lost;
                Err(gone())
            }
            Err(LinkError::NoWindow | LinkError::Unsupported) => Err(no_window()),
            Err(e) => Err(broken(&format!("{e:?}"))),
        }
    }

    /// PASS ONE CALL to the window: its answer, or the refusal and what it left of the document.
    pub fn pass(&mut self, name: &str, arguments: Value) -> Result<Value, Cut> {
        if self.link.is_none() {
            self.connect().map_err(|refusal| Cut { refusal, after: After::Untouched })?;
        }
        let Some(link) = self.link.as_mut() else { return Err(Cut { refusal: no_window(), after: After::Untouched }) };
        match link.call(name, arguments) {
            Ok(answer) => Ok(answer),
            Err(LinkError::Busy) => Err(Cut { refusal: busy(), after: After::Untouched }),
            Err(LinkError::Gone) => {
                self.link = None;
                self.reach = Reach::Lost;
                Err(Cut { refusal: gone(), after: After::Unknown })
            }
            // the answer still owed on this link would be read as the next call's: the link is dropped, and the next
            // call makes a new one to the same window
            Err(LinkError::Late) => {
                self.link = None;
                Err(Cut { refusal: late(self.late), after: After::Unknown })
            }
            Err(LinkError::Refused { code, message }) => Err(Cut { refusal: Refusal::new(&code, &message, Stage::Window), after: After::Unknown }),
            Err(e) => {
                self.link = None;
                Err(Cut { refusal: broken(&format!("{e:?}")), after: After::Unknown })
            }
        }
    }
}

/// A CALL THE WINDOW DID NOT ANSWER: why, and what it left of the document.
pub struct Cut {
    pub refusal: Refusal,
    pub after: After,
}

fn no_window() -> Refusal {
    Refusal::new("no-window", "No QymCAD window listens: none is open, or its connection to Claude is switched off.", Stage::Window).with_hint(
        "Ask the person to open QymCAD and switch the connection to Claude on (Help -> Connect to Claude), then call again; or start the server with --headless to work on a document of its own.",
    )
}

fn gone() -> Refusal {
    Refusal::new("window-gone", "The QymCAD window this session worked with has closed.", Stage::Window)
        .with_hint("Nothing goes to a window again in this session: another window may hold another document. Ask the person to reopen QymCAD and restart the QymCAD server.")
}

fn busy() -> Refusal {
    Refusal::new("window-busy", "The window is busy with the person's own action; the call was withdrawn and did not happen.", Stage::Window).with_hint("Call again in a moment.")
}

fn late(late: Duration) -> Refusal {
    Refusal::new("window-late", &format!("The window took the call and has not answered in {} s; it may still finish.", late.as_secs()), Stage::Window)
        .with_hint("Read get_document before calling again, to see whether it did.")
}

fn broken(what: &str) -> Refusal {
    Refusal::new("window-broken", &format!("The answer from the window could not be read: {what}"), Stage::Window).with_hint("Read get_document to see where the document stands.")
}
