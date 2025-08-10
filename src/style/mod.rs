//! This module supports styling and theming SVG documents, including svgdx 'auto-styles'.
//!
//! The primary mechanism is taking element class names (with the 'd-' prefix) and mapping
//! them to corresponding CSS or inline styles.

mod autostyle;
mod colours;
mod omap;
mod rules;
mod themes;
mod types;

use crate::elements::SvgElement;
use crate::errors::{Result, SvgdxError};

use autostyle::StyleProvider;

pub use themes::{ContextTheme, ThemeType};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum AutoStyleMode {
    None,
    Inline,
    #[default]
    Css,
}

impl std::str::FromStr for AutoStyleMode {
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

pub fn get_css_styles(
    theme: &ContextTheme,
    elements: &[&SvgElement],
) -> (Vec<String>, Vec<String>) {
    let mut registry = autostyle::StyleRegistry::new(theme);
    registry.register_all();
    registry.generate_css(elements)
}

pub fn update_inline_styles(
    theme: &ContextTheme,
    elements: &mut [&mut SvgElement],
) -> (Vec<String>, Vec<String>) {
    let mut registry = autostyle::StyleRegistry::new(theme);
    registry.register_all();
    registry.update_elements(elements)
}
