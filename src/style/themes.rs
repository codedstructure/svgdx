// themes for svgdx
//
// themes provide two outputs: a set of `defs` elements (patterns, markers, gradients etc)
// and a set of `styles` entries (typically CSS rules).

use crate::context::TransformerContext;
use crate::errors::{Result, SvgdxError};
use crate::types::fstr;
use std::{collections::HashSet, str::FromStr};

use super::colours::{COLOUR_LIST, DARK_COLOURS};

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

fn append_common_styles(tb: &mut ThemeBuilder, fill: &str, stroke: &str, stroke_width: f32) {
    // Default styles suitable for box-and-line diagrams
    let font_family = &tb.font_family;
    let font_size = tb.font_size;
    let all_elements = if tb.local_style_id.is_some() {
        "*"
    } else {
        "svg *"
    };
    for s in [
        format!("{all_elements} {{ stroke-linecap: round; stroke-linejoin: round; }}"),
        format!("rect, circle, ellipse, polygon {{ stroke-width: {stroke_width}; fill: {fill}; stroke: {stroke}; }}"),
        format!("line, polyline, path {{ stroke-width: {stroke_width}; fill: none; stroke: {stroke}; }}"),
        format!("text, tspan {{ stroke-width: 0; font-family: {font_family}; font-size: {font_size}px; fill: {stroke}; paint-order: stroke; stroke: {fill}; }}"),
    ] {
        tb.add_style(&s);
    }
}

fn append_text_styles(tb: &mut ThemeBuilder) {
    if !tb.has_element("text") {
        return;
    }
    for (class, rule) in [
        // Text alignment - default centered horizontally and vertically
        // These are intended to be composable, e.g. "d-text-top d-text-right"
        ("d-text", "text-anchor: middle; dominant-baseline: central;"),
        ("d-text-top", "dominant-baseline: text-before-edge;"),
        ("d-text-bottom", "dominant-baseline: text-after-edge;"),
        ("d-text-left", "text-anchor: start;"),
        ("d-text-right", "text-anchor: end;"),
        ("d-text-top-vertical", "text-anchor: start;"),
        ("d-text-bottom-vertical", "text-anchor: end;"),
        (
            "d-text-left-vertical",
            "dominant-baseline: text-after-edge;",
        ),
        (
            "d-text-right-vertical",
            "dominant-baseline: text-before-edge;",
        ),
        // Default is sans-serif 'normal' text.
        ("d-text-bold", "font-weight: bold;"),
        // Allow explicitly setting 'normal' font-weight, as themes may set a non-normal default.
        ("d-text-normal", "font-weight: normal;"),
        ("d-text-light", "font-weight: 100;"),
        ("d-text-italic", "font-style: italic;"),
        ("d-text-monospace", "font-family: monospace;"),
        ("d-text-pre", "font-family: monospace;"),
    ] {
        if tb.has_class(class) {
            tb.add_style(&format!(
                "text.{class}, tspan.{class}, text.{class} * {{ {rule} }}"
            ));
        }
    }

    let text_sizes = vec![
        ("d-text-smallest", tb.font_size * 0.333333),
        ("d-text-smaller", tb.font_size * 0.5),
        ("d-text-small", tb.font_size * 0.666666),
        ("d-text-medium", tb.font_size), // Default, but include explicitly for completeness
        ("d-text-large", tb.font_size * 1.5),
        ("d-text-larger", tb.font_size * 2.),
        ("d-text-largest", tb.font_size * 3.),
    ];
    for (class, size) in text_sizes {
        if tb.has_class(class) {
            let size = fstr(size);
            tb.add_style(&format!(
                "text.{class}, tspan.{class}, text.{class} * {{ font-size: {size}px; }}"
            ));
        }
    }
    let text_ol_widths = vec![
        ("d-text-ol", 0.5), // Must be first, so other classes can override
        ("d-text-ol-thinner", 0.125),
        ("d-text-ol-thin", 0.25),
        ("d-text-ol-medium", 0.5), // Default, but include explicitly for completeness
        ("d-text-ol-thick", 1.),
        ("d-text-ol-thicker", 2.),
    ];
    for (class, width) in text_ol_widths {
        if tb.has_class(class) {
            // Selector must be more specific than e.g. `d-thinner`,
            // and must appear after any colour styles, where
            // `d-text-ol-[colour]` provides a default stroke-width.
            let width = fstr(width);
            tb.add_style(&format!(
                "text.{class}, tspan.{class}, text.{class} * {{ stroke-width: {width}; }}",
            ));
        }
    }
}

fn append_stroke_width_styles(tb: &mut ThemeBuilder, base: f32) {
    for (class, width) in [
        ("d-thinner", base * 0.25),
        ("d-thin", base * 0.5),
        ("d-thick", base * 2.),
        ("d-thicker", base * 4.),
    ] {
        if tb.has_class(class) {
            let width = fstr(width);
            tb.add_style(&format!(".{class} {{ stroke-width: {width}; }}"));
        }
    }
}

fn append_colour_styles(tb: &mut ThemeBuilder) {
    // Colours
    // - d-colour sets a 'default' colour for shape outlines and text
    // - d-fill-colour sets the colour for shape fills, and sets a text colour
    //   to an appropriate contrast colour.
    // - d-text-colour sets the colour for text elements, which overrides any
    //   colours set by d-colour or d-fill-colour.
    // - d-text-ol-colour sets the colour for text outline
    for colour in COLOUR_LIST {
        if tb.has_class(&format!("d-fill-{colour}")) {
            tb.add_style(&format!(".d-fill-{colour} {{ fill: {colour}; }}"));
            let (text_fill, text_stroke) = if DARK_COLOURS.contains(colour) {
                ("white", "black")
            } else {
                ("black", "white")
            };
            tb.add_style(&format!(
                "text.d-fill-{colour}, text.d-fill-{colour} * {{ fill: {text_fill}; stroke: {text_stroke}; }}"
            ));
        }
    }
    for colour in COLOUR_LIST {
        if tb.has_class(&format!("d-{colour}")) {
            tb.add_style(&format!(".d-{colour} {{ stroke: {colour}; }}"));
            // By default text is the same colour as shape stroke, but may be
            // overridden by d-text-colour (e.g. for text attrs on shapes)
            // Also special-case 'none'; there are many use-cases for not having
            // a stroke colour (using `d-none`), but text should always have a colour.
            if *colour != "none" {
                let text_stroke = if DARK_COLOURS.contains(colour) {
                    "white"
                } else {
                    "black"
                };
                tb.add_style(&format!(
                    "text.d-{colour}, tspan.d-{colour}, text.d-{colour} * {{ fill: {colour}; stroke: {text_stroke}; }}"
                ));
            }
        }
    }
    for colour in COLOUR_LIST {
        if tb.has_class(&format!("d-text-{colour}")) {
            let text_stroke = if DARK_COLOURS.contains(colour) {
                "white"
            } else {
                "black"
            };
            // Must be at least as specific as d-fill-colour
            tb.add_style(&format!(
                "text.d-text-{colour}, tspan.d-text-{colour}, text.d-text-{colour} * {{ fill: {colour}; stroke: {text_stroke}; }}"
            ));
        }
    }
    for colour in COLOUR_LIST {
        if tb.has_class(&format!("d-text-ol-{colour}")) {
            // Must be at least as specific as d-fill-colour
            tb.add_style(&format!(
                "text.d-text-ol-{colour}, tspan.d-text-ol-{colour}, text.d-text-ol-{colour} * {{ stroke: {colour}; stroke-width: 0.5; }}"
            ));
        }
    }
}

fn append_arrow_styles(tb: &mut ThemeBuilder) {
    let mut has_arrow = false;
    if tb.has_class("d-arrow") {
        tb.add_style("line.d-arrow, polyline.d-arrow, path.d-arrow { marker-end: url(#d-arrow); }");
        has_arrow = true;
    }
    if tb.has_class("d-biarrow") {
        tb.add_style(
                "line.d-biarrow, polyline.d-biarrow, path.d-biarrow { marker-start: url(#d-arrow); marker-end: url(#d-arrow); }",
            );
        has_arrow = true;
    }
    if has_arrow {
        // override the default 'fill:none' for markers.
        tb.add_style("marker path { fill: inherit; }");
        // Note use of context-stroke for fill, and setting stroke:none to prevent
        // the marker size extending beyond the path boundary.
        // NOTE: the arrow marker butts up against the end of the line so doesn't have
        // a 'point'. This means the line and arrow both end together and the line is
        // never thicker than the arrow, but isn't ideal visually.
        // A more sophisticated system would have the marker 'after' the line, and
        // reduce the line length by the marker width - but that would be complex
        // in this program. Maybe in the future.
        tb.add_defs(
            r#"<marker id="d-arrow" refX="1" refY="0.5" orient="auto-start-reverse" markerWidth="6" markerHeight="5" viewBox="0 0 1 1">
  <path d="M 0 0 1 0.4 1 0.6 0 1" style="stroke: none; fill: context-stroke;"/>
</marker>"#);
    }
}

fn append_dash_styles(tb: &mut ThemeBuilder) {
    // Dash / dot / flow: stroke-dasharray should have an even number of entries and the 'from'
    // keyframe stroke-dashoffset should be (a multiple of) the sum of the dasharray values.
    let flow_style = vec![
        ("d-flow-slower", "4"),
        ("d-flow-slow", "2"),
        ("d-flow", "1"),
        ("d-flow-fast", "0.5"),
        ("d-flow-faster", "0.25"),
    ];
    let mut has_flow = false;
    for (class, speed) in flow_style {
        if tb.has_class(class) {
            // d-flow defaults to equivalent of d-dash, but also works with d-dot.
            tb.add_style(&format!(".{class} {{ animation: {speed}s linear 0s infinite running d-flow-animation; stroke-dasharray: 1 1.5; }}"));
            has_flow = true;
        }
    }
    if has_flow {
        tb.add_style("@keyframes d-flow-animation { from {stroke-dashoffset: 5;} to {stroke-dashoffset: 0;} }");
    }
    if tb.has_class("d-flow-rev") {
        tb.add_style(".d-flow-rev { animation-direction: reverse; }");
    }
    // NOTE: these are after the d-flow-* classes, as they provide a default dasharray these may override.
    if tb.has_class("d-dash") {
        tb.add_style(".d-dash { stroke-dasharray: 1 1.5; }");
    }
    if tb.has_class("d-dot") {
        tb.add_style(".d-dot { stroke-dasharray: 0 1; }");
    }
    if tb.has_class("d-dot-dash") {
        tb.add_style(".d-dot-dash { stroke-dasharray: 0 1 1.5 1 0 1.5; }");
    }
}

#[derive(Debug, Clone, Copy)]
enum PatternType {
    Horizontal,
    Vertical,
    Grid,
    Stipple,
}

fn pattern_defs(
    tb: &mut ThemeBuilder,
    t_stroke: &str,
    class: &str,
    spacing: u32,
    direction: PatternType,
    rotate: Option<i32>,
) {
    let rotate = if let Some(r) = rotate {
        format!(" patternTransform=\"rotate({r})\"")
    } else {
        String::new()
    };
    // This is fairly hacky, but a bigger spacing *probably* means
    // covering a larger area and a thicker stroke width is appropriate.
    let sw = fstr((spacing as f32).sqrt() / 10.);
    let ptn_id = class.trim_start_matches("d-");
    tb.add_style(&format!(".{class} {{fill: url(#{ptn_id})}}"));
    let mut lines = String::new();
    if let PatternType::Horizontal | PatternType::Grid = direction {
        lines.push_str(&format!(
            r#"<line x1="0" y1="0" x2="{spacing}" y2="0" style="stroke-width: {sw}; stroke: {t_stroke}"/>"#
        ));
    }
    if let PatternType::Vertical | PatternType::Grid = direction {
        lines.push_str(&format!(
            r#"<line x1="0" y1="0" x2="0" y2="{spacing}" style="stroke-width: {sw}; stroke: {t_stroke}"/>"#
        ));
    }
    if let PatternType::Stipple = direction {
        let gs = fstr(spacing as f32 / 2.);
        let r = fstr((spacing as f32).sqrt() / 5.);
        lines.push_str(&format!(
            r#"<circle cx="{gs}" cy="{gs}" r="{r}" style="stroke: none; fill: {t_stroke}"/>"#
        ));
    }
    tb.add_defs(&format!(
        r#"<pattern id="{ptn_id}" x="0" y="0" width="{spacing}" height="{spacing}"{rotate} patternUnits="userSpaceOnUse" >
  <rect width="100%" height="100%" style="stroke: none;"/>
  {lines}
</pattern>"#,
    ));
}

fn append_pattern_styles(tb: &mut ThemeBuilder, t_stroke: &str) {
    for (ptn_class, ptn_type, ptn_rotate) in [
        ("d-grid", PatternType::Grid, None),
        ("d-grid-h", PatternType::Horizontal, None),
        ("d-grid-v", PatternType::Vertical, None),
        ("d-hatch", PatternType::Horizontal, Some(-45)),
        ("d-crosshatch", PatternType::Grid, Some(75)),
        ("d-stipple", PatternType::Stipple, Some(45)),
    ] {
        fn get_spacing(prefix: &str, c: &str) -> Option<u32> {
            if let Some(suffix) = c.strip_prefix(prefix) {
                suffix.parse::<u32>().ok().filter(|&n| n <= 100)
            } else {
                None
            }
        }
        if tb.has_class(ptn_class) {
            pattern_defs(tb, t_stroke, ptn_class, 1, ptn_type, ptn_rotate);
        }
        let spec_class = format!("{ptn_class}-");

        let classes: Vec<_> = tb
            .classes
            .iter()
            .filter(|c| c.starts_with(&spec_class))
            .cloned()
            .collect();
        for class in classes {
            if let Some(grid_size) = get_spacing(&spec_class, &class) {
                pattern_defs(tb, t_stroke, &class, grid_size, ptn_type, ptn_rotate);
            }
        }
    }
}

fn d_softshadow(tb: &mut ThemeBuilder, _: &str) {
    tb.add_style(".d-softshadow { filter: url(#d-softshadow); }");
    tb.add_defs(
        r#"<filter id="d-softshadow" x="-50%" y="-50%" width="200%" height="200%">
  <feGaussianBlur in="SourceAlpha" stdDeviation="0.7"/>
  <feOffset dx="1" dy="1"/>
  <feComposite in2="SourceGraphic" operator="arithmetic" k1="0" k2="0.4" k3="1" k4="0"/>
</filter>"#,
    );
}

fn d_hardshadow(tb: &mut ThemeBuilder, _: &str) {
    tb.add_style(".d-hardshadow { filter: url(#d-hardshadow); }");
    tb.add_defs(
        r#"<filter id="d-hardshadow" x="-50%" y="-50%" width="200%" height="200%">
  <feGaussianBlur in="SourceAlpha" stdDeviation="0.2"/>
  <feOffset dx="1" dy="1"/>
  <feComposite in2="SourceGraphic" operator="arithmetic" k1="0" k2="0.6" k3="1" k4="0"/>
</filter>"#,
    );
}

trait Theme: Clone {
    fn build(&self, tb: &mut ThemeBuilder) {
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
                self.default_background()
            ));
        }
        if let Some(id) = &tb.local_style_id {
            // Start a nested CSS block for styles to ensure they don't leak
            // to surrounding document.
            tb.add_style(&format!("#{id} {{"));
        }
        self.append_early_styles(tb);
        // Must be before any colour styles which need to override this
        if tb.has_class("d-surround") {
            tb.add_style(".d-surround { fill: none; }");
        }

        append_common_styles(
            tb,
            &self.default_fill(),
            &self.default_stroke(),
            self.default_stroke_width(),
        );
        // Colour styles must appear before text styles, at least so
        // d-text-ol-[colour] (which sets a default stroke-width) can be
        // overridden by the text style `d-text-ol-[thickness]`.
        append_colour_styles(tb);

        append_stroke_width_styles(tb, self.default_stroke_width());
        if tb.elements.contains("text") {
            append_text_styles(tb);
        }

        append_arrow_styles(tb);
        append_dash_styles(tb);
        append_pattern_styles(tb, &self.default_stroke());

        type Tfn = dyn Fn(&mut ThemeBuilder, &str);
        for (class, build_fn) in [
            ("d-softshadow", &d_softshadow as &Tfn),
            ("d-hardshadow", &d_hardshadow as &Tfn),
        ] {
            if tb.has_class(class) {
                build_fn(tb, &self.default_stroke());
            }
        }
        self.append_late_styles(tb);
        // Close the nested CSS block if we opened one.
        if tb.local_style_id.is_some() {
            tb.add_style("}");
        }
    }
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
    fn append_early_styles(&self, _tb: &mut ThemeBuilder) {}
    fn append_late_styles(&self, _tb: &mut ThemeBuilder) {}
}

pub struct ThemeBuilder {
    local_style_id: Option<String>,
    styles: Vec<String>,
    defs: Vec<String>,

    background: String,
    font_size: f32,
    font_family: String,
    theme: ThemeType,
    classes: HashSet<String>,
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
        match self.theme {
            ThemeType::Default => DefaultTheme {}.build(self),
            ThemeType::Bold => BoldTheme {}.build(self),
            ThemeType::Fine => FineTheme {}.build(self),
            ThemeType::Glass => GlassTheme {}.build(self),
            ThemeType::Light => LightTheme {}.build(self),
            ThemeType::Dark => DarkTheme {}.build(self),
        }
    }
    fn has_class(&self, s: &str) -> bool {
        self.classes.iter().any(|x| x == s)
    }
    fn has_element(&self, s: &str) -> bool {
        self.elements.iter().any(|x| x == s)
    }
    fn add_defs(&mut self, s: &str) {
        self.defs.push(s.to_owned());
    }
    fn add_style(&mut self, s: &str) {
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
    fn append_early_styles(&self, tb: &mut ThemeBuilder) {
        tb.add_style("text,tspan {font-weight: 100}");
    }
    fn default_stroke_width(&self) -> f32 {
        0.2
    }
}

#[derive(Debug, Clone)]
pub struct BoldTheme;
impl Theme for BoldTheme {
    fn append_early_styles(&self, tb: &mut ThemeBuilder) {
        tb.add_style("text,tspan {font-weight: 900}");
    }
    fn default_stroke_width(&self) -> f32 {
        1.
    }
}

#[derive(Debug, Clone)]
pub struct GlassTheme;
impl Theme for GlassTheme {
    fn append_early_styles(&self, tb: &mut ThemeBuilder) {
        tb.add_style("rect, circle, ellipse, polygon { opacity: 0.7; }");
    }
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
