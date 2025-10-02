use std::collections::HashMap;
use std::num::{ParseFloatError, ParseIntError};
use std::str::ParseBoolError;
use std::string::FromUtf8Error;

use crate::elements::SvgElement;
use crate::types::{ElRef, OrderIndex};

// type alias for Result for use across the library
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Parse(String),
    InvalidData(String),
    Reference(ElRef),
    VarLimit(String, usize, u32),
    LoopLimit(u32, u32),
    DepthLimit(u32, u32),
    CircularRef(String),
    Document(String),
    MissingAttr(String),
    MissingBBox(String),
    InternalLogic(String),
    Multi(HashMap<OrderIndex, (SvgElement, Error)>),
    Other(Box<dyn std::error::Error>),
    #[cfg(feature = "cli")]
    Cli(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(source) => write!(f, "IO error: {source}"),
            Error::Parse(reason) => write!(f, "Parse error: {reason}"),
            Error::InvalidData(reason) => write!(f, "Invalid data: {reason}"),
            Error::Reference(elref) => write!(f, "Reference error: {elref}"),
            Error::VarLimit(name, len, limit) => {
                write!(f, "Variable '{name}' length ({len}) exceeded limit {limit}")
            }
            Error::LoopLimit(count, limit) => {
                write!(f, "Loop count {count} exceeded limit {limit}")
            }
            Error::DepthLimit(depth, limit) => {
                write!(f, "Depth {depth} exceeded limit {limit}")
            }
            Error::CircularRef(reason) => {
                write!(f, "Circular reference error: {reason}")
            }
            Error::Document(reason) => write!(f, "Document error: {reason}"),
            Error::MissingAttr(attr) => write!(f, "Element missing attribute '{attr}'"),
            Error::MissingBBox(reason) => write!(f, "Missing bounding box: {reason}"),
            Error::InternalLogic(reason) => write!(f, "Internal logic error: {reason}"),
            Error::Multi(errors) => {
                let mut errs = errors.iter().collect::<Vec<_>>();
                errs.sort_by(|a, b| a.0.cmp(b.0));
                for (_, (el, err)) in errs {
                    write!(f, "\n {:>4}: {}: {}", el.src_line, el.original, err)?;
                }
                Ok(())
            }
            Error::Other(source) => source.fmt(f),
            #[cfg(feature = "cli")]
            Error::Cli(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(source) => Some(source),
            Error::Parse(_) => None,
            Error::InvalidData(_) => None,
            Error::Reference(_) => None,
            Error::VarLimit(_, _, _) => None,
            Error::LoopLimit(_, _) => None,
            Error::DepthLimit(_, _) => None,
            Error::CircularRef(_) => None,
            Error::Document(_) => None,
            Error::MissingAttr(_) => None,
            Error::MissingBBox(_) => None,
            Error::InternalLogic(_) => None,
            Error::Multi(_) => None,
            Error::Other(e) => Some(&**e),
            #[cfg(feature = "cli")]
            Error::Cli(_) => None,
        }
    }
}

impl Error {
    pub fn from_err<T>(err: T) -> Error
    where
        T: std::error::Error + 'static,
    {
        Error::Other(Box::new(err))
    }
}

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Error {
        Error::Parse(format!("float: {err}"))
    }
}

impl From<ParseBoolError> for Error {
    fn from(err: ParseBoolError) -> Error {
        Error::Parse(format!("bool: {err}"))
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Error {
        Error::Parse(format!("int: {err}"))
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Error {
        Error::Document(format!("utf8: {err}"))
    }
}
