/// This module supports styling and theming SVG documents, including svgdx 'auto-styles'.
mod autostyle;
mod colours;
mod oimap;
mod rules;
mod styles;
mod themes;
mod types;

use crate::elements::SvgElement;
use crate::errors::{Result, SvgdxError};

use autostyle::StyleProvider;

pub use styles::apply_auto_styles;
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

pub fn get_css_styles(tb: &ContextTheme, elements: &[&SvgElement]) -> (Vec<String>, Vec<String>) {
    let mut registry = autostyle::StyleRegistry::new(tb);
    registry.register_all();
    registry.generate_css(elements)
}

#[allow(dead_code)]
pub fn update_inline_styles(tb: &ContextTheme, elements: &mut [&mut SvgElement]) -> String {
    let mut registry = autostyle::StyleRegistry::new(tb);
    registry.register_all();
    let defs = registry.update_elements(elements);

    defs.join("\n")
}
