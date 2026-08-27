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
pub enum AutoStyleMode {
    /// Don't process auto-style classes.
    None,
    /// Include auto-styles as part of element `<style>` attributes.
    Inline,
    /// As per `inline`, but avoid 'CDATA' to generate valid HTML
    InlineHtml,
    /// Generate CSS auto-style rules in a separate `<style>` element.
    #[default]
    Css,
    /// As per `css`, but avoid 'CDATA' to generate valid HTML
    CssHtml,
}

impl AutoStyleMode {
    /// Returns `true` if this mode should use CDATA sections for inline styles.
    pub fn use_cdata(&self) -> bool {
        !matches!(self, AutoStyleMode::InlineHtml | AutoStyleMode::CssHtml)
    }

    pub fn variants() -> Vec<String> {
        vec![
            AutoStyleMode::None.to_string(),
            AutoStyleMode::Inline.to_string(),
            AutoStyleMode::InlineHtml.to_string(),
            AutoStyleMode::Css.to_string(),
            AutoStyleMode::CssHtml.to_string(),
        ]
    }
}

impl std::str::FromStr for AutoStyleMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            s if s == AutoStyleMode::None.to_string() => Ok(AutoStyleMode::None),
            s if s == AutoStyleMode::Inline.to_string() => Ok(AutoStyleMode::Inline),
            s if s == AutoStyleMode::InlineHtml.to_string() => Ok(AutoStyleMode::InlineHtml),
            s if s == AutoStyleMode::Css.to_string() => Ok(AutoStyleMode::Css),
            s if s == AutoStyleMode::CssHtml.to_string() => Ok(AutoStyleMode::CssHtml),
            _ => Err(Error::InvalidValue("auto-style-mode".into(), s.into())),
        }
    }
}

impl std::fmt::Display for AutoStyleMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AutoStyleMode::None => "none",
            AutoStyleMode::Inline => "inline",
            AutoStyleMode::InlineHtml => "inline-html",
            AutoStyleMode::Css => "css",
            AutoStyleMode::CssHtml => "css-html",
        };
        f.write_str(s)
    }
}
