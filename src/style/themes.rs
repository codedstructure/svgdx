// themes for svgdx

use std::rc::Rc;

use crate::context::TransformerContext;
use crate::errors::{Result, SvgdxError};

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum ThemeType {
    #[default]
    Default,
    Bold,
    Fine,
    Glass,
    Light,
    Dark,
}

impl std::str::FromStr for ThemeType {
    type Err = SvgdxError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "default" => Ok(Self::default()),
            "bold" => Ok(Self::Bold),
            "fine" => Ok(Self::Fine),
            "glass" => Ok(Self::Glass),
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(SvgdxError::InvalidData(format!(
                "Unknown theme '{s}' (available themes: default, bold, fine, glass, light, dark)",
            ))),
        }
    }
}

pub(super) trait Theme {
    fn default_fill(&self) -> String {
        String::from("white")
    }
    fn default_stroke(&self) -> String {
        String::from("black")
    }
    fn default_background(&self) -> String {
        String::from("none")
    }
    fn default_stroke_width(&self) -> f32 {
        0.5
    }
    fn default_font_weight(&self) -> Option<String> {
        None
    }
}

// fn build_theme(theme: &dyn Theme, tb: &mut ThemeBuilder) {
//     let mut outer_svg = String::from("svg");
//     if let Some(id) = &tb.local_style_id {
//         outer_svg = format!("svg#{id}");
//     }
//     // Any background style needs to be prior to potential CSS nesting from local_id
//     // - it isn't a descendant of the local_id element, but that element itself.
//     if tb.background != "default" {
//         tb.add_style(&format!(
//             "{} {{ background: {}; }}",
//             outer_svg, tb.background
//         ));
//     } else {
//         tb.add_style(&format!(
//             "{} {{ background: {}; }}",
//             outer_svg,
//             theme.default_background()
//         ));
//     }
//     if let Some(id) = &tb.local_style_id {
//         // Start a nested CSS block for styles to ensure they don't leak
//         // to surrounding document.
//         tb.add_style(&format!("#{id} {{"));
//     }
//  ...
//     // Close the nested CSS block if we opened one.
//     if tb.local_style_id.is_some() {
//         tb.add_style("}");
//     }
// }

#[derive(Clone)]
pub struct ContextTheme {
    // local_style_id: Option<String>,
    pub font_size: f32,
    pub font_family: String,
    pub(super) background: String,
    pub(super) theme: Rc<Box<dyn Theme>>,
}

impl Default for ContextTheme {
    fn default() -> Self {
        Self {
            // local_style_id: None,
            font_size: 3.0,
            font_family: String::from("sans-serif"),
            background: String::from("default"),
            theme: Rc::new(Box::new(DefaultTheme {})),
        }
    }
}

impl ContextTheme {
    // Should every theme type be overridable?
    pub fn from_context(context: &TransformerContext) -> Self {
        let theme: Rc<Box<dyn Theme>> = Rc::new(match context.config.theme {
            ThemeType::Default => Box::new(DefaultTheme {}) as Box<dyn Theme>,
            ThemeType::Bold => Box::new(BoldTheme {}) as Box<dyn Theme>,
            ThemeType::Fine => Box::new(FineTheme {}) as Box<dyn Theme>,
            ThemeType::Glass => Box::new(GlassTheme {}) as Box<dyn Theme>,
            ThemeType::Light => Box::new(LightTheme {}) as Box<dyn Theme>,
            ThemeType::Dark => Box::new(DarkTheme {}) as Box<dyn Theme>,
        });
        Self {
            // local_style_id: context.local_style_id.clone(),
            font_size: context.config.font_size,
            font_family: context.config.font_family.clone(),
            background: if context.config.background == "default" {
                theme.default_background()
            } else {
                context.config.background.clone()
            },
            theme,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DefaultTheme;

impl Theme for DefaultTheme {}

#[derive(Debug, Clone)]
pub struct FineTheme;

impl Theme for FineTheme {
    fn default_font_weight(&self) -> Option<String> {
        Some(String::from("100"))
    }
    fn default_stroke_width(&self) -> f32 {
        0.2
    }
}

#[derive(Debug, Clone)]
pub struct BoldTheme;
impl Theme for BoldTheme {
    fn default_font_weight(&self) -> Option<String> {
        Some(String::from("900"))
    }
    fn default_stroke_width(&self) -> f32 {
        1.
    }
}

#[derive(Debug, Clone)]
pub struct GlassTheme;
impl Theme for GlassTheme {
    // TODO: consider opacity: 0.7 for enclosed elements; though
    // maybe just having the default fill translucent is enough.
    // (and possibly any additional colour styles we introduce)
    fn default_fill(&self) -> String {
        String::from("rgba(0, 30, 50, 0.15)")
    }
    fn default_background(&self) -> String {
        String::from("rgba(200, 230, 220, 0.5)")
    }
}

#[derive(Debug, Clone)]
pub struct LightTheme;
impl Theme for LightTheme {
    fn default_stroke(&self) -> String {
        String::from("#657b83")
    }
    fn default_fill(&self) -> String {
        String::from("#fdf6e3")
    }
    fn default_background(&self) -> String {
        String::from("#eee8d5")
    }
}

#[derive(Debug, Clone)]
pub struct DarkTheme;
impl Theme for DarkTheme {
    fn default_stroke(&self) -> String {
        String::from("#93a1a1")
    }
    fn default_fill(&self) -> String {
        String::from("#002b36")
    }
    fn default_background(&self) -> String {
        String::from("#073642")
    }
}
