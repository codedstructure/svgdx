#[cfg(any(feature = "cli", feature = "server"))]
mod args;
#[cfg(feature = "server")]
pub use args::parse_value;
#[cfg(any(feature = "cli", feature = "server"))]
pub use args::{TransformArgs, common_usage, parse_kv_arg, take_value};

use crate::document::parse_library;
use crate::errors::{Error, Result};
use crate::{AutoStyleMode, ThemeType, VarName};
use std::collections::HashMap;
use std::sync::Arc;

use crate::constants::{
    DEFAULT_BACKGROUND, DEFAULT_BORDER, DEFAULT_DEPTH_LIMIT, DEFAULT_FONT_FAMILY,
    DEFAULT_FONT_SIZE, DEFAULT_LOOP_LIMIT, DEFAULT_PATH_REPEAT_LIMIT, DEFAULT_RNG_SEED,
    DEFAULT_SCALE, DEFAULT_VAR_LIMIT,
};

/// Settings to configure a single transformation.
///
/// Note the settings here are specific to a single transformation; alternate front-ends
/// may use this directly rather than `Config` which wraps this struct when `svgdx` is
/// run as a command-line program.
#[derive(Clone, Debug)]
pub struct TransformConfig {
    /// Add debug info (e.g. input source) to output
    pub debug: bool,
    /// Overall output image scale (in mm as scale of user units)
    pub scale: f32,
    /// Border width (user-units, default 5)
    pub border: u16,
    /// Add style & defs entries based on class usage
    pub auto_style_mode: AutoStyleMode,
    /// Background colour (default "default" - use theme default or none)
    pub background: String, // TODO: sanitize this with a `Colour: FromStr + Display` type
    /// Random seed
    pub seed: u64,
    /// Maximum loop iterations
    pub loop_limit: u32,
    /// Max length of variable
    pub var_limit: u32,
    /// Maximum depth of recursion
    pub depth_limit: u32,
    /// Maximum path repeat expansion (`r` command)
    pub path_repeat_limit: u32,
    /// Add source metadata to output
    pub add_metadata: bool,
    /// Default font-size (in user-units)
    pub font_size: f32,
    /// Default font-family
    pub font_family: String,
    /// Theme to use (default "default")
    pub theme: ThemeType,
    /// Optional style to apply to SVG root element
    pub svg_style: Option<String>,
    /// Error handling mode
    pub error_mode: ErrorMode,
    /// Set of initial variable values
    pub vars: HashMap<VarName, String>,
    /// Included library sources
    pub library_sources: Vec<Arc<String>>,
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self {
            debug: false,
            scale: DEFAULT_SCALE,
            border: DEFAULT_BORDER,
            auto_style_mode: AutoStyleMode::default(),
            background: DEFAULT_BACKGROUND.to_owned(),
            seed: DEFAULT_RNG_SEED,
            loop_limit: DEFAULT_LOOP_LIMIT,
            var_limit: DEFAULT_VAR_LIMIT,
            depth_limit: DEFAULT_DEPTH_LIMIT,
            path_repeat_limit: DEFAULT_PATH_REPEAT_LIMIT,
            add_metadata: false,
            font_size: DEFAULT_FONT_SIZE,
            font_family: DEFAULT_FONT_FAMILY.to_owned(),
            theme: ThemeType::default(),
            svg_style: None,
            error_mode: ErrorMode::default(),
            vars: HashMap::new(),
            library_sources: Vec::new(),
        }
    }
}

impl TransformConfig {
    pub fn load_library(&mut self, source: impl Into<String>) -> Result<()> {
        let source = source.into();
        parse_library(source.clone())?;
        self.library_sources.push(Arc::new(source));
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorMode {
    /// Un-resolved errors prevent processing
    #[default]
    Strict,
    /// Continue with error message in XML comment
    Warn,
    /// Continue silently ignoring errors
    Ignore,
}

impl ErrorMode {
    pub fn variants() -> Vec<String> {
        vec![
            ErrorMode::Strict.to_string(),
            ErrorMode::Warn.to_string(),
            ErrorMode::Ignore.to_string(),
        ]
    }
}

impl std::str::FromStr for ErrorMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            s if s == ErrorMode::Strict.to_string() => Ok(ErrorMode::Strict),
            s if s == ErrorMode::Warn.to_string() => Ok(ErrorMode::Warn),
            s if s == ErrorMode::Ignore.to_string() => Ok(ErrorMode::Ignore),
            _ => Err(Error::InvalidValue(
                format!(
                    "error-mode must be '{}', '{}', or '{}'",
                    ErrorMode::Strict,
                    ErrorMode::Warn,
                    ErrorMode::Ignore,
                ),
                s.to_string(),
            )),
        }
    }
}

impl std::fmt::Display for ErrorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ErrorMode::Strict => "strict",
            ErrorMode::Warn => "warn",
            ErrorMode::Ignore => "ignore",
        };
        f.write_str(s)
    }
}
