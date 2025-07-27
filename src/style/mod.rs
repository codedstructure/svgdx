/// This module supports styling and theming SVG documents, including svgdx 'auto-styles'.
mod colours;
mod styles;
mod themes;

use std::str::FromStr;

use crate::errors::{Result, SvgdxError};
pub use styles::apply_auto_styles;
pub use themes::{ThemeBuilder, ThemeType};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum AutoStyleMode {
    None,
    Inline,
    #[default]
    Css,
}

impl FromStr for AutoStyleMode {
    type Err = SvgdxError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "none" => Ok(AutoStyleMode::None),
            "inline" => Ok(AutoStyleMode::Inline),
            "css" => Ok(AutoStyleMode::Css),
            _ => Err(SvgdxError::ParseError(format!(
                "Unknown auto-styles mode: {s}"
            ))),
        }
    }
}
