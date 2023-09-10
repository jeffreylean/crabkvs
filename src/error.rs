use core::fmt;
use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    KeyNotFound,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::KeyNotFound => write!(f, "Key not found"),
        }
    }
}

impl std::error::Error for Error {}
