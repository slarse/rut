use crate::refs::ParseRevisionError;

use std::{error, fmt, io, result};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    ParseRevision(ParseRevisionError),
    Fatal(Option<Box<dyn std::error::Error>>, String),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::ParseRevision(err) => Some(err),
            Error::Fatal(Some(err), _) => err.source(),
            Error::Fatal(None, _) => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "Unhandled IO error: {}", err),
            Error::ParseRevision(err) => write!(f, "Unhandled parse error: {}", err),
            Error::Fatal(_, msg) => write!(f, "fatal: {}", msg),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<ParseRevisionError> for Error {
    fn from(err: ParseRevisionError) -> Error {
        Error::ParseRevision(err)
    }
}

pub type Result<T> = result::Result<T, Error>;
