use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::num::{ParseFloatError, ParseIntError};
use std::str::ParseBoolError;
use std::string::FromUtf8Error;

use itertools::Itertools;

use crate::element::SvgElement;
use crate::types::{ElRef, OrderIndex};

// type alias for Result for use across the library
pub type Result<T> = std::result::Result<T, SvgdxError>;

#[derive(Debug)]
pub enum SvgdxError {
    IoError(std::io::Error),
    ParseError(String),
    InvalidData(String),
    ReferenceError(ElRef),
    VarLimitError(String, usize, u32),
    LoopLimitError(u32, u32),
    DepthLimitExceeded(u32, u32),
    CircularRefError(String),
    DocumentError(String),
    MissingAttribute(String),
    GeometryError(String),
    MessageError(String),
    MultiError(HashMap<OrderIndex, (SvgElement, SvgdxError)>),
    OtherError(Box<dyn std::error::Error>),
}

impl fmt::Display for SvgdxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SvgdxError::IoError(source) => write!(f, "IO error: {}", source),
            SvgdxError::ParseError(reason) => write!(f, "Parse error: {}", reason),
            SvgdxError::InvalidData(reason) => write!(f, "Invalid data: {}", reason),
            SvgdxError::ReferenceError(elref) => write!(f, "Reference error: {}", elref),
            SvgdxError::VarLimitError(name, len, limit) => {
                write!(
                    f,
                    "Variable '{}' length ({}) exceeded limit {}",
                    name, len, limit
                )
            }
            SvgdxError::LoopLimitError(count, limit) => {
                write!(f, "Loop count {} exceeded limit {}", count, limit)
            }
            SvgdxError::DepthLimitExceeded(depth, limit) => {
                write!(f, "Depth {} exceeded limit {}", depth, limit)
            }
            SvgdxError::CircularRefError(reason) => {
                write!(f, "Circular reference error: {}", reason)
            }
            SvgdxError::DocumentError(reason) => write!(f, "Document error: {}", reason),
            SvgdxError::MissingAttribute(attr) => write!(f, "Element missing attribute '{}'", attr),
            SvgdxError::GeometryError(reason) => write!(f, "Geometry error: {}", reason),
            SvgdxError::MessageError(reason) => write!(f, "{}", reason),
            SvgdxError::MultiError(errors) => {
                for (_, (el, err)) in errors.iter().sorted_by(|a, b| a.0.cmp(b.0)) {
                    write!(f, "\n {:>4}: {}: {}", el.src_line, el.original, err)?;
                }
                Ok(())
            }
            SvgdxError::OtherError(source) => write!(f, "{}", source),
        }
    }
}

impl Error for SvgdxError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SvgdxError::IoError(source) => Some(source),
            SvgdxError::ParseError(_) => None,
            SvgdxError::InvalidData(_) => None,
            SvgdxError::ReferenceError(_) => None,
            SvgdxError::VarLimitError(_, _, _) => None,
            SvgdxError::LoopLimitError(_, _) => None,
            SvgdxError::DepthLimitExceeded(_, _) => None,
            SvgdxError::CircularRefError(_) => None,
            SvgdxError::DocumentError(_) => None,
            SvgdxError::MissingAttribute(_) => None,
            SvgdxError::GeometryError(_) => None,
            SvgdxError::MessageError(_) => None,
            SvgdxError::MultiError(_) => None,
            SvgdxError::OtherError(e) => Some(&**e),
        }
    }
}

impl SvgdxError {
    pub fn from_err<T>(err: T) -> SvgdxError
    where
        T: std::error::Error + 'static,
    {
        SvgdxError::OtherError(Box::new(err))
    }
}

impl From<std::io::Error> for SvgdxError {
    fn from(err: std::io::Error) -> SvgdxError {
        SvgdxError::IoError(err)
    }
}

impl From<ParseFloatError> for SvgdxError {
    fn from(err: ParseFloatError) -> SvgdxError {
        SvgdxError::ParseError(format!("float: {}", err))
    }
}

impl From<ParseBoolError> for SvgdxError {
    fn from(err: ParseBoolError) -> SvgdxError {
        SvgdxError::ParseError(format!("bool: {}", err))
    }
}

impl From<ParseIntError> for SvgdxError {
    fn from(err: ParseIntError) -> SvgdxError {
        SvgdxError::ParseError(format!("int: {}", err))
    }
}

impl From<FromUtf8Error> for SvgdxError {
    fn from(err: FromUtf8Error) -> SvgdxError {
        SvgdxError::ParseError(format!("utf8: {}", err))
    }
}

impl From<&str> for SvgdxError {
    fn from(err: &str) -> SvgdxError {
        SvgdxError::MessageError(err.to_string())
    }
}
