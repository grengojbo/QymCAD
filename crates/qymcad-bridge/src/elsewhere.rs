//! A SYSTEM WITH NO CHANNEL YET (Windows, until its named pipe is written): the window's end does not open and the
//! program's end finds no window, each saying so, so both build and run and neither pretends.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use crate::{LinkError, Opened, Wait};

/// One call for the window; none ever comes here.
pub struct Call {
    pub tool: String,
    pub arguments: Value,
}

impl Call {
    pub fn answer(self, _result: Value) {}
}

pub struct Listener;

impl Listener {
    pub fn open(_path: &Path, _wait: Wait) -> Result<Listener, Opened> {
        Err(Opened::Unsupported)
    }

    pub fn next(&self) -> Option<Call> {
        None
    }
}

pub struct Link;

impl Link {
    pub fn connect(_path: &Path, _late: Duration) -> Result<Link, LinkError> {
        Err(LinkError::Unsupported)
    }

    pub fn call(&mut self, _tool: &str, _arguments: Value) -> Result<Value, LinkError> {
        Err(LinkError::Unsupported)
    }
}
