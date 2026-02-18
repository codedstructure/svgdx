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

use crate::errors::{Error, Result};
pub use crate::style::types::{Selectable, Stylable};

use autostyle::StyleProvider;

pub use autostyle::StyleRegistry;
pub use themes::{ContextTheme, ThemeType};

/// Auto-style processing mode.
///
/// Auto-styles translate specific element class names (all beginning with `d-`)
/// to corresponding CSS (as part of a `<style>` element) or inline (the `style`
/// attribute) style information.
///
/// Any required `<defs>` entries are also added, unless the mode is set to
/// `None`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum AutoStyleMode {
    /// Don't process auto-style classes.
    None,
    /// Include auto-styles as part of element `<style>` attributes.
    Inline,
    /// Generate CSS auto-style rules in a separate `<style>` element.
    #[default]
    Css,
}

impl std::str::FromStr for AutoStyleMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "none" => Ok(AutoStyleMode::None),
            "inline" => Ok(AutoStyleMode::Inline),
            "css" => Ok(AutoStyleMode::Css),
            _ => Err(Error::InvalidValue("auto-style-mode".into(), s.into())),
        }
    }
}
