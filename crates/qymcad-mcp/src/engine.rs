//! WHERE THE CALLS GO: to a document of the program's own, or over the channel to the open window, whose document the
//! person sees.
//!
//! The choice is said on the command line: `--headless` keeps a document here, `--live` sends every call to the
//! window, `--auto` (the default) takes the window when one listens, and works on a document of its own while none
//! does - until it changes that document, and from then on keeps it for the run.
//!
//! THE FIRST CHANGE DECIDES, NOT THE START AND NOT THE FIRST CALL. Claude Desktop starts the program when Claude
//! Desktop itself starts, which is mostly before QymCAD is opened; decided at the start, the session would keep a
//! document of its own for good. Decided at the first call, a model that first asked what was selected - and was told
//! there was no window - kept an empty document of its own after the person switched the window on. A reading changes
//! nothing, so until the own document is changed every call looks for the window again; once it is changed, moving to
//! a window would drop that work unseen.
//!
//! A WINDOW OPENED AGAIN IS ANNOUNCED. Once the window this session reached has closed, a window opened afterwards
//! may hold another document, and a model working on in it unawares would edit a part it never read. So the first
//! call that reaches the new window is refused untouched with `window-new`, and the calls after it go through: the
//! model reads the document in between, and nobody restarts Claude.

use std::path::PathBuf;
use std::time::Duration;

use qymcad_bridge::{Link, LinkError};
use qymcad_tools::tool::{After, Ctx, Missed, Refusal, Seen, Stage};
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
    /// Not decided yet (`--auto`): a call that reaches a document goes to the window when one listens, and to the
    /// document of its own otherwise, as long as that document is untouched.
    Undecided { window: Window, own: Box<Ctx> },
}

impl Engine {
    /// The engine `start` asks for; `--auto` is left to the first call.
    pub fn start(start: Start) -> Engine {
        let window = Window { path: start.socket.or_else(qymcad_bridge::default_path), late: start.late, link: None, reach: Reach::NotYet };
        match start.mode {
            Mode::Headless => {
                let mut own = Box::new(Ctx::blank());
                own.seen = Seen::NoWindow(Missed { why: format!("{} was started with --headless and does not look for a window", crate::release()) });
                Engine::Here(own)
            }
            Mode::Live => Engine::Window(window),
            Mode::Auto => Engine::Undecided { window, own: Box::new(Ctx::blank()) },
        }
    }

    /// LOOK FOR THE WINDOW, when not yet decided: one that listens now takes the run. Nobody listening, the call goes
    /// to the document of its own, which is told where the window was looked for. A window that listens and could not
    /// be reached is no reason to work on a document of its own unawares: the call is refused with why.
    pub fn decide(&mut self) -> Looked {
        let Engine::Undecided { window, own } = self else { return Looked::Decided };
        match window.connect() {
            Ok(()) => {}
            Err(Missing::NoOne) => {
                own.seen = Seen::NoWindow(Missed { why: format!("{} looked for the window at {} and nobody listens there", crate::release(), window.place()) });
                return Looked::NoOne;
            }
            Err(Missing::Refused(refusal)) => return Looked::Refused(refusal),
        }
        let placeholder = Window { path: None, late: Duration::ZERO, link: None, reach: Reach::NotYet };
        if let Engine::Undecided { window, .. } = std::mem::replace(self, Engine::Window(placeholder)) {
            *self = Engine::Window(window);
        }
        Looked::Decided
    }

    /// SETTLE ON THE DOCUMENT OF ITS OWN once a call has changed it: a model's work there is not left behind for a
    /// window it never read.
    pub fn settle(&mut self) {
        let Engine::Undecided { own, .. } = self else { return };
        if own.path.is_none() && own.doc.history().undo_names().is_empty() && own.doc.history().redo_names().is_empty() {
            return;
        }
        let placeholder = Box::new(Ctx::blank());
        if let Engine::Undecided { own, .. } = std::mem::replace(self, Engine::Here(placeholder)) {
            *self = Engine::Here(own);
        }
    }
}

/// WHAT LOOKING FOR THE WINDOW CAME TO.
pub enum Looked {
    /// Where the calls go is settled: the window, or a document of its own.
    Decided,
    /// Nobody listens: the call goes to the document of its own, as yet untouched.
    NoOne,
    /// A window listens and could not be reached: the call is refused with this.
    Refused(Refusal),
}

/// Why a connection to the window was not made.
enum Missing {
    /// Nobody listens on the channel.
    NoOne,
    /// Something listens and the connection failed, or the window is gone or new: the call is refused with this.
    Refused(Refusal),
}

/// How far the session got with the window.
#[derive(Clone, Copy, PartialEq)]
enum Reach {
    /// No window answered yet: the next call looks for one.
    NotYet,
    /// A window answered; the link to it may be dropped and made again, to the same window.
    Reached,
    /// The window answered once and has closed since: the next window reached is announced before it takes a call.
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
    /// Where the window is looked for, in words a person can look up.
    fn place(&self) -> String {
        self.path.as_deref().map_or_else(|| "no place: this system names no folder for the program".to_string(), qymcad_bridge::channel_name)
    }

    fn connect(&mut self) -> Result<(), Missing> {
        let Some(path) = &self.path else { return Err(Missing::NoOne) };
        match Link::connect(path, self.late) {
            Ok(link) => {
                self.link = Some(link);
                let was = std::mem::replace(&mut self.reach, Reach::Reached);
                if was == Reach::Lost {
                    return Err(Missing::Refused(new_window()));
                }
                Ok(())
            }
            Err(_) if self.reach == Reach::Lost => Err(Missing::Refused(gone())),
            Err(_) if self.reach == Reach::Reached => {
                self.reach = Reach::Lost;
                Err(Missing::Refused(gone()))
            }
            Err(LinkError::NoWindow) => Err(Missing::NoOne),
            Err(e) => Err(Missing::Refused(unreached(&self.place(), &format!("{e:?}")))),
        }
    }

    /// PASS ONE CALL to the window: its answer, or the refusal and what it left of the document.
    pub fn pass(&mut self, name: &str, arguments: Value) -> Result<Value, Cut> {
        if self.link.is_none() {
            let place = self.place();
            self.connect().map_err(|missing| Cut {
                refusal: match missing {
                    Missing::NoOne => no_window(&place),
                    Missing::Refused(refusal) => refusal,
                },
                after: After::Untouched,
            })?;
        }
        let Some(link) = self.link.as_mut() else { return Err(Cut { refusal: no_window(&self.place()), after: After::Untouched }) };
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

fn no_window(place: &str) -> Refusal {
    let switch = qymcad_tools::person::the_switch(&qymcad_tools::person::language());
    let message = format!("No QymCAD window listens at {place}: none is open on this machine, or the switch for Claude in it is off. This is {}.", crate::release());
    Refusal::new("no-window", &message, Stage::Window).with_hint(&format!("Ask the person to open QymCAD and turn on {switch} - in these words, their window shows them so - then call again."))
}

/// A window listens at `place` and the connection to it failed: `what` the channel said.
fn unreached(place: &str, what: &str) -> Refusal {
    Refusal::new("window-broken", &format!("A QymCAD window listens at {place}, and {} could not reach it: {what}. Nothing was done.", crate::release()), Stage::Window)
        .with_hint("Tell the person these words: the window and this server cannot talk, so nothing can be done in the window until that is mended. They can send them through Help -> Report a problem in QymCAD.")
}

fn gone() -> Refusal {
    Refusal::new("window-gone", "The QymCAD window this session worked with has closed.", Stage::Window)
        .with_hint("Ask the person to open QymCAD again; the next call reaches the window opened, after a warning that it may hold another document.")
}

fn new_window() -> Refusal {
    Refusal::new("window-new", "A QymCAD window was opened again after the one this session worked with closed; it may hold another document. Nothing was done.", Stage::Window)
        .with_hint("Read get_document before changing anything; the calls after this one go to this window.")
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
