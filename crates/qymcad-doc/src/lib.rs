//! THE DOCUMENT WITHOUT A WINDOW: open it, change it as one action, rebuild it, take the action back, bring a file in
//! and write one out - the same steps the window takes, with no screen, no camera and no thread of the interface.
//!
//! Why a crate of its own. These steps were written inside the interface, beside the camera, the selection and the
//! status line, and every caller that drives a document without a window had to copy them. The copies drifted: the
//! reproduction harness read an imported solid back from its source and dropped the factor the person took it at,
//! so a 10 mm cube taken at 25.4 came back at 1000 mm^3 instead of 16 387 064 mm^3, and a check measured a document
//! the window never shows. One crate, called by the window and by every headless caller, leaves nothing to drift.
//!
//! A module, not a workbench: no words of the interface, no egui, no dependency on any workbench. A name shown to a
//! person is asked of the caller, which knows the language.

pub mod brep;
pub mod engine;
pub mod history;
pub mod regen;

pub use engine::{DocEngine, DocError};
