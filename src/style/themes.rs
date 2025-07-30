// themes for svgdx
//
// themes provide two outputs: a set of `defs` elements (patterns, markers, gradients etc)
// and a set of `styles` entries (typically CSS rules).

use super::css::{
    append_arrow_styles, append_colour_styles, append_dash_styles, append_pattern_styles,
    append_stroke_width_styles, append_text_styles, d_hardshadow, d_softshadow,
};
use crate::context::TransformerContext;
use crate::errors::{Result, SvgdxError};
use crate::style::css::append_common_styles;
use std::{collections::HashSet, str::FromStr};

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

impl FromStr for ThemeType {
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

trait Theme {
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

fn build_theme(theme: &dyn Theme, tb: &mut ThemeBuilder) {
    let mut outer_svg = String::from("svg");
    if let Some(id) = &tb.local_style_id {
        outer_svg = format!("svg#{id}");
    }
    // Any background style needs to be prior to potential CSS nesting from local_id
    // - it isn't a descendant of the local_id element, but that element itself.
    if tb.background != "default" {
        tb.add_style(&format!(
            "{} {{ background: {}; }}",
            outer_svg, tb.background
        ));
    } else {
        tb.add_style(&format!(
            "{} {{ background: {}; }}",
            outer_svg,
            theme.default_background()
        ));
    }
    if let Some(id) = &tb.local_style_id {
        // Start a nested CSS block for styles to ensure they don't leak
        // to surrounding document.
        tb.add_style(&format!("#{id} {{"));
    }
    // Must be before any colour styles which need to override this
    if tb.has_class("d-surround") {
        tb.add_style(".d-surround { fill: none; }");
    }

    append_common_styles(
        tb,
        &theme.default_fill(),
        &theme.default_stroke(),
        theme.default_stroke_width(),
    );
    // Colour styles must appear before text styles, at least so
    // d-text-ol-[colour] (which sets a default stroke-width) can be
    // overridden by the text style `d-text-ol-[thickness]`.
    append_colour_styles(tb);

    append_stroke_width_styles(tb, theme.default_stroke_width());
    if tb.elements.contains("text") {
        append_text_styles(tb);
        // TODO: theme should be provided to append_text_styles
        if let Some(weight) = theme.default_font_weight() {
            tb.add_style(&format!("text, tspan {{ font-weight: {weight}; }}"));
        }
    }

    append_arrow_styles(tb);
    append_dash_styles(tb);
    append_pattern_styles(tb, &theme.default_stroke());

    type Tfn = dyn Fn(&mut ThemeBuilder, &str);
    for (class, build_fn) in [
        ("d-softshadow", &d_softshadow as &Tfn),
        ("d-hardshadow", &d_hardshadow as &Tfn),
    ] {
        if tb.has_class(class) {
            build_fn(tb, &theme.default_stroke());
        }
    }
    // Close the nested CSS block if we opened one.
    if tb.local_style_id.is_some() {
        tb.add_style("}");
    }
}

pub struct ThemeBuilder {
    pub local_style_id: Option<String>,
    styles: Vec<String>,
    defs: Vec<String>,

    background: String,
    pub font_size: f32,
    pub font_family: String,
    theme: ThemeType,
    pub classes: HashSet<String>,
    elements: HashSet<String>,
}

impl ThemeBuilder {
    pub fn new(
        context: &TransformerContext,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
    ) -> Self {
        Self {
            local_style_id: context.local_style_id.clone(),
            styles: Vec::new(),
            defs: Vec::new(),
            background: context.config.background.clone(),
            font_size: context.config.font_size,
            font_family: context.config.font_family.clone(),
            theme: context.config.theme.clone(),
            classes: classes.to_owned(),
            elements: elements.to_owned(),
        }
    }
    pub fn build(&mut self) {
        let theme: Box<dyn Theme> = match self.theme {
            ThemeType::Default => Box::new(DefaultTheme {}),
            ThemeType::Bold => Box::new(BoldTheme {}),
            ThemeType::Fine => Box::new(FineTheme {}),
            ThemeType::Glass => Box::new(GlassTheme {}),
            ThemeType::Light => Box::new(LightTheme {}),
            ThemeType::Dark => Box::new(DarkTheme {}),
        };
        build_theme(&*theme, self);
    }
    pub(super) fn has_class(&self, s: &str) -> bool {
        self.classes.iter().any(|x| x == s)
    }
    pub(super) fn has_element(&self, s: &str) -> bool {
        self.elements.iter().any(|x| x == s)
    }
    pub(super) fn add_defs(&mut self, s: &str) {
        self.defs.push(s.to_owned());
    }
    pub(super) fn add_style(&mut self, s: &str) {
        self.styles.push(s.to_owned());
    }
    pub fn get_defs(&self) -> Vec<String> {
        self.defs.clone()
    }
    pub fn get_styles(&self) -> Vec<String> {
        self.styles.clone()
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
