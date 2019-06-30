use std::fmt::{Display, Formatter, Result};
use std::io;
use std::error::Error;

use regex;

#[derive(Debug)]
pub enum SubError {
    FailedToWrite(io::Error),
    InvalidUTF8(io::Error),
    RegexError(regex::Error),
    CouldNotOpenFile(io::Error),
    CouldNotCreateTempFile(io::Error),
    CouldNotModifyInplace(io::Error),
    CouldNotReadMetadata(io::Error),
    CouldNotSetPermissions(io::Error),
}

impl Error for SubError {

    fn description(&self) -> &str {
        use SubError::*;
        match self {
            FailedToWrite(e) => e.description(),
            InvalidUTF8(e) => e.description(),
            RegexError(e) => e.description(),
            CouldNotOpenFile(e) => e.description(),
            CouldNotCreateTempFile(e) => e.description(),
            CouldNotModifyInplace(e) => e.description(),
            CouldNotReadMetadata(e) => e.description(),
            CouldNotSetPermissions(e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        use SubError::*;
        match self {
            FailedToWrite(e) => Some(e),
            InvalidUTF8(e) => Some(e),
            RegexError(e) => Some(e),
            CouldNotOpenFile(e) => Some(e),
            CouldNotCreateTempFile(e) => Some(e),
            CouldNotModifyInplace(e) => Some(e),
            CouldNotReadMetadata(e) => Some(e),
            CouldNotSetPermissions(e) => Some(e),
        }
    }
}

impl Display for SubError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        use SubError::*;

        match self {
            FailedToWrite(e) => write!(f, "Output stream has been closed: {}", e),
            InvalidUTF8(e) => write!(f, "Input contains invalid UTF-8: {}", e),
            RegexError(e) => write!(f, "{}", e),
            CouldNotOpenFile(e) => write!(f, "Could not open file '{}'", e),
            CouldNotCreateTempFile(e) => write!(f, "Failed to create temporary file: {}", e),
            CouldNotModifyInplace(e) => write!(
                f,
                "Could not modify the file in-place: {}",
                //path.to_string_lossy(),
                e
            ),
            CouldNotReadMetadata(e) => write!(
                f,
                "Could not read metadata from file '{}'",
                e
            ),
            CouldNotSetPermissions(e) => write!(
                f,
                "Could not set permissions of file '{}'",
                e
            ),
        }
    }
}

