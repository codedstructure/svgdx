// themes for svgdx

use std::sync::OnceLock;

use crate::builtin::themes_source;
use crate::context::{TransformerContext, VariableMap};
use crate::document::{EventKind, InputEvent, InputList};
use crate::elements::{SvgElement, apply_theme_var};
use crate::errors::{Error, Result};

const THEME_FILL: &str = "_theme_fill";
const THEME_STROKE: &str = "_theme_stroke";
const THEME_STROKE_WIDTH: &str = "_theme_stroke_width";
const THEME_BACKGROUND: &str = "_theme_background";
const THEME_FONT_WEIGHT: &str = "_theme_font_weight";

/// Auto-style themes for svgdx.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum ThemeType {
    /// The default theme, suitable for line diagrams.
    #[default]
    Default,
    /// The default theme with more bold lines and text.
    Bold,
    /// The default theme with thinner lines and text.
    Fine,
    /// A glass theme with translucent elements.
    Glass,
    /// A theme inspired by solarized light mode.
    Light,
    /// A dark theme inspired by solarized dark mode.
    Dark,
    /// A theme with any other name; must be defined in a `<themes>` element.
    Custom(String),
}

impl ThemeType {
    pub fn variants() -> Vec<String> {
        vec![
            ThemeType::Default.to_string(),
            ThemeType::Bold.to_string(),
            ThemeType::Fine.to_string(),
            ThemeType::Glass.to_string(),
            ThemeType::Light.to_string(),
            ThemeType::Dark.to_string(),
        ]
    }
}

impl std::str::FromStr for ThemeType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            s if s == Self::Default.to_string() => Ok(Self::Default),
            s if s == Self::Bold.to_string() => Ok(Self::Bold),
            s if s == Self::Fine.to_string() => Ok(Self::Fine),
            s if s == Self::Glass.to_string() => Ok(Self::Glass),
            s if s == Self::Light.to_string() => Ok(Self::Light),
            s if s == Self::Dark.to_string() => Ok(Self::Dark),
            _ => Err(Error::InvalidValue(
                format!(
                    "theme ({}, {}, {}, {}, {}, {})",
                    ThemeType::Default,
                    ThemeType::Bold,
                    ThemeType::Fine,
                    ThemeType::Glass,
                    ThemeType::Light,
                    ThemeType::Dark
                ),
                s.into(),
            )),
        }
    }
}

impl std::fmt::Display for ThemeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ThemeType::Default => "default",
            ThemeType::Bold => "bold",
            ThemeType::Fine => "fine",
            ThemeType::Glass => "glass",
            ThemeType::Light => "light",
            ThemeType::Dark => "dark",
            ThemeType::Custom(name) => name,
        };
        f.write_str(s)
    }
}

/// Theme attributes; each sets the corresponding `_theme_*` variable, e.g.
/// `stroke-width` sets `_theme_stroke_width`.
const THEME_ATTRS: &[&str] = &[
    "fill",
    "stroke",
    "stroke-width",
    "background",
    "font-weight",
];

/// A named theme, stored as the attributes of its `<theme>` element.
#[derive(Debug, Clone, Default)]
pub struct ThemeDef {
    pub name: String,
    attrs: Vec<(String, String)>,
}

impl ThemeDef {
    /// Evaluate the theme in the given context, setting its (global) variables.
    pub fn apply(&self, context: &mut TransformerContext) -> Result<()> {
        let mut vars = Vec::with_capacity(self.attrs.len());
        for (key, value) in &self.attrs {
            if !THEME_ATTRS.contains(&key.as_str()) {
                return Err(Error::InvalidAttr(key.clone()));
            }
            vars.push((format!("_theme_{}", key.replace('-', "_")), value.clone()));
        }
        apply_theme_var(&SvgElement::new("var", &vars), context)
    }
}

/// Extract all `<theme>` definitions from a sequence of events.
///
/// `base_level` is the element nesting level at which `<theme>` elements appear
/// in the given events, e.g. 1 for a complete document with a root `<themes>`
/// element, and 0 for the inner events of a `<themes>` element.
pub fn extract_themes<'a>(
    events: impl IntoIterator<Item = &'a InputEvent>,
    base_level: usize,
) -> Vec<ThemeDef> {
    let mut themes = Vec::new();
    let mut level = 0;
    for ev in events {
        match &ev.event {
            EventKind::Start(e) | EventKind::Empty(e) => {
                if level == base_level && e.name() == "theme" {
                    let attrs = e.get_attrs();
                    if let Some((_, name)) = attrs.iter().find(|(k, _)| k == "name") {
                        themes.push(ThemeDef {
                            name: name.clone(),
                            attrs: attrs.iter().filter(|(k, _)| k != "name").cloned().collect(),
                        });
                    }
                }
                if matches!(ev.event, EventKind::Start(_)) {
                    level += 1;
                }
            }
            EventKind::End(_) => level = level.saturating_sub(1),
            _ => {}
        }
    }
    themes
}

fn builtin_themes() -> &'static [ThemeDef] {
    static THEMES: OnceLock<Vec<ThemeDef>> = OnceLock::new();
    THEMES.get_or_init(|| {
        let events: InputList = themes_source().parse().expect("themes parse");
        extract_themes(events.iter(), 1)
    })
}

/// Find a built-in theme definition by name.
pub fn builtin_theme(name: &str) -> Option<&'static ThemeDef> {
    builtin_themes().iter().find(|t| t.name == name)
}

/// Resolved theme values used when generating auto-styles and the background.
#[derive(Clone)]
pub struct ContextTheme {
    pub font_size: f32,
    pub font_family: String,
    pub(super) background: String,
    pub(super) fill: String,
    pub(super) stroke: String,
    pub(super) stroke_width: String,
    pub(super) font_weight: Option<String>,
}

impl Default for ContextTheme {
    fn default() -> Self {
        Self {
            font_size: 3.0,
            font_family: String::from("sans-serif"),
            background: String::from("none"),
            fill: String::from("white"),
            stroke: String::from("black"),
            stroke_width: String::from("0.5"),
            font_weight: None,
        }
    }
}

impl ContextTheme {
    /// Build from the `_theme_*` variables set when the current theme was evaluated.
    pub fn from_context(context: &TransformerContext) -> Self {
        let fallback = Self::default();
        let var = |name: &str, default: &str| {
            context.get_var(name).unwrap_or_else(|| default.to_string())
        };
        let theme_background = var(THEME_BACKGROUND, &fallback.background);
        Self {
            font_size: context.config.font_size,
            font_family: context.config.font_family.clone(),
            background: if context.config.background == "default" {
                theme_background
            } else {
                context.config.background.clone()
            },
            fill: var(THEME_FILL, &fallback.fill),
            stroke: var(THEME_STROKE, &fallback.stroke),
            stroke_width: var(THEME_STROKE_WIDTH, &fallback.stroke_width),
            font_weight: context
                .get_var(THEME_FONT_WEIGHT)
                .filter(|w| !w.trim().is_empty()),
        }
    }

    pub fn background(&self) -> &str {
        &self.background
    }
}
