#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum CoreError {
    NotImplemented(&'static str),
}

impl Display for CoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented(what) => write!(f, "not implemented: {what}"),
        }
    }
}

impl Error for CoreError {}

pub type CoreResult<T> = Result<T, CoreError>;

pub fn list_dir(_path: &str) -> CoreResult<Vec<String>> {
    Err(CoreError::NotImplemented("list_dir"))
}

pub fn read_text(_path: &str) -> CoreResult<String> {
    Err(CoreError::NotImplemented("read_text"))
}

pub fn write_text(_path: &str, _content: &str) -> CoreResult<()> {
    Err(CoreError::NotImplemented("write_text"))
}

pub fn fetch_json(_url: &str) -> CoreResult<String> {
    Err(CoreError::NotImplemented("fetch_json"))
}
