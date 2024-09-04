// themes for svgdx
//
// themes provide two outputs: a set of `defs` elements (patterns, markers, gradients etc)
// and a set of `styles` entries (typically CSS rules).

use crate::types::fstr;
use crate::TransformConfig;
use std::{collections::HashSet, str::FromStr};

use crate::colours::{COLOUR_LIST, DARK_COLOURS};
use anyhow::bail;

#[derive(Debug, Clone)]
pub enum ThemeType {
    Default(DefaultTheme),
    Bold(BoldTheme),
    Fine(FineTheme),
    Glass(GlassTheme),
    Light(LightTheme),
    Dark(DarkTheme),
}

impl FromStr for ThemeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(Self::default()),
            "bold" => Ok(Self::Bold(BoldTheme)),
            "fine" => Ok(Self::Fine(FineTheme)),
            "glass" => Ok(Self::Glass(GlassTheme)),
            "light" => Ok(Self::Light(LightTheme)),
            "dark" => Ok(Self::Dark(DarkTheme)),
            _ => bail!(
                "Unknown theme '{}' (available themes: default, bold, fine, glass, light, dark)",
                s
            ),
        }
    }
}

impl Default for ThemeType {
    fn default() -> Self {
        ThemeType::Default(DefaultTheme)
    }
}

fn append_common_styles(tb: &mut ThemeBuilder, fill: &str, stroke: &str, stroke_width: f32) {
    // Default styles suitable for box-and-line diagrams
    for s in [
        format!("rect, circle, ellipse, polygon {{ stroke-width: {stroke_width}; fill: {fill}; stroke: {stroke}; }}"),
        format!("line, polyline, path {{ stroke-width: {stroke_width}; fill: none; stroke: {stroke}; }}"),
    ] {
        tb.add_style(&s);
    }
}

fn append_text_styles(tb: &mut ThemeBuilder, text_colour: &str) {
    if !tb.has_element("text") {
        return;
    }
    tb.add_style(&format!("text, tspan {{ stroke-width: 0; font-family: sans-serif; font-size: 3px; fill: {text_colour} }}"));
    for (class, rule) in [
        // Text alignment - default centered horizontally and vertically
        // These are intended to be composable, e.g. "d-text-top d-text-right"
        ("d-tbox", "text.d-tbox, text.d-tbox * { text-anchor: middle; dominant-baseline: central; }"),
        ("d-text-top", "text.d-text-top, text.d-text-top * { dominant-baseline: text-before-edge; }"),
        ("d-text-bottom", "text.d-text-bottom, text.d-text-bottom * { dominant-baseline: text-after-edge; }"),
        ("d-text-left", "text.d-text-left, text.d-text-left * { text-anchor: start; }"),
        ("d-text-right", "text.d-text-right, text.d-text-right * { text-anchor: end; }"),
        ("d-text-top-vertical", "text.d-text-top-vertical, text.d-text-top-vertical * { text-anchor: start; }"),
        ("d-text-bottom-vertical", "text.d-text-bottom-vertical, text.d-text-bottom-vertical * { text-anchor: end; }"),
        ("d-text-left-vertical", "text.d-text-left-vertical, text.d-text-left-vertical * { dominant-baseline: text-after-edge; }"),
        ("d-text-right-vertical", "text.d-text-right-vertical, text.d-text-right-vertical * { dominant-baseline: text-before-edge; }"),
        // Default is sans-serif 'normal' text.
        ("d-text-bold", "text.d-text-bold, text.d-text-bold * { font-weight: bold; }"),
        // Allow explicitly setting 'normal' font-weight, as themes may set a non-normal default.
        ("d-text-normal", "text.d-text-normal, text.d-text-normal * { font-weight: normal; }"),
        ("d-text-light", "text.d-text-light, text.d-text-light * { font-weight: 100; }"),
        ("d-text-italic", "text.d-text-italic, text.d-text-italic * { font-style: italic; }"),
        ("d-text-monospace", "text.d-text-monospace, text.d-text-monospace * { font-family: monospace; }"),
    ] {
        if tb.has_class(class) {
            tb.add_style(rule);
        }
    }

    let text_sizes = vec![
        ("d-text-smallest", 1.),
        ("d-text-smaller", 1.5),
        ("d-text-small", 2.),
        ("d-text-medium", 3.), // Default, but include explicitly for completeness
        ("d-text-large", 4.5),
        ("d-text-larger", 7.),
        ("d-text-largest", 10.),
    ];
    for (class, size) in text_sizes {
        if tb.has_class(class) {
            tb.add_style(&format!(
                "text.{0}, text.{0} * {{ font-size: {1}px; }}",
                class, size
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
            tb.add_style(&format!(".{class} {{ stroke-width: {}; }}", fstr(width)));
        }
    }
}

fn append_colour_styles(tb: &mut ThemeBuilder) {
    //, classes: &HashSet<String>) {
    // Colours
    // - d-colour sets a 'default' colour for shape outlines and text
    // - d-fill-colour sets the colour for shape fills, and sets a text colour
    //   to an appropriate contrast colour.
    // - d-text-colour sets the colour for text elements, which overrides any
    //   colours set by d-colour or d-fill-colour.
    for colour in COLOUR_LIST {
        if tb.has_class(&format!("d-fill-{colour}")) {
            tb.add_style(&format!(
                ".d-fill-{colour}:not(text,tspan) {{ fill: {colour}; }}"
            ));
            let text_colour = if DARK_COLOURS.contains(colour) {
                "white"
            } else {
                "black"
            };
            tb.add_style(&format!(
                "text.d-fill-{colour}, text.d-fill-{colour} * {{ fill: {text_colour}; }}"
            ));
        }
    }
    for colour in COLOUR_LIST {
        if tb.has_class(&format!("d-{colour}")) {
            tb.add_style(&format!(
                ".d-{colour}:not(text,tspan) {{ stroke: {colour}; }}"
            ));
            // By default text is the same colour as shape stroke, but may be
            // overridden by d-text-colour (e.g. for text attrs on shapes)
            // Also special-case 'none'; there are many use-cases for not having
            // a stroke colour (using `d-none`), but text should always have a colour.
            if *colour != "none" {
                tb.add_style(&format!(
                    "text.d-{colour}, text.d-{colour} * {{ fill: {colour}; }}"
                ));
            }
        }
    }
    for colour in COLOUR_LIST {
        if tb.has_class(&format!("d-text-{colour}")) {
            // Must be at least as specific as d-fill-colour
            tb.add_style(&format!(
                "text.d-text-{colour}, text.d-text-{colour} * {{ fill: {colour}; }}"
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
            r#"<marker id="d-arrow" refX="0.8" refY="0.5" orient="auto-start-reverse" markerWidth="6" markerHeight="5" viewBox="0 0 0.4 1">
  <path d="M 0 0 1 0.5 0 1" style="stroke: none; fill: context-stroke;"/>
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
            tb.add_style(&format!(".{class} {{ animation: {speed}s linear 0s infinite running d-flow-animation; stroke-dasharray: 1.5 0.5; }}"));
            has_flow = true;
        }
    }
    if has_flow {
        tb.add_style("@keyframes d-flow-animation { from {stroke-dashoffset: 4;} to {stroke-dashoffset: 0;} }");
    }
    if tb.has_class("d-flow-rev") {
        tb.add_style(".d-flow-rev { animation-direction: reverse; }");
    }
    // NOTE: these are after the d-flow-* classes, as they provide a default dasharray these may override.
    if tb.has_class("d-dash") {
        tb.add_style(".d-dash { stroke-dasharray: 1.5 0.5; }");
    }
    if tb.has_class("d-dot") {
        tb.add_style(".d-dot { stroke-dasharray: 0.5 0.5; }");
    }
}

fn d_stipple(tb: &mut ThemeBuilder, t_stroke: &str) {
    tb.add_style(".d-stipple:not(text,tspan) {fill: url(#stipple)}");
    tb.add_defs(&format!(
r#"<pattern id="stipple" x="0" y="0" width="1" height="1" patternTransform="rotate(45)" patternUnits="userSpaceOnUse" >
  <rect width="100%" height="100%" style="stroke: none"/>
  <circle cx="0.5" cy="0.5" r="0.1" style="stroke: none; fill: {t_stroke}"/>
</pattern>"#,
    ));
}

fn d_crosshatch(tb: &mut ThemeBuilder, t_stroke: &str) {
    tb.add_style(".d-crosshatch:not(text,tspan) {fill: url(#crosshatch)}");
    tb.add_defs(&format!(
r#"<pattern id="crosshatch" x="0" y="0" width="1" height="1" patternTransform="rotate(75)" patternUnits="userSpaceOnUse" >
  <rect width="100%" height="100%" style="stroke: none"/>
  <line x1="0" y1="0" x2="1" y2="0" style="stroke-width: 0.1; stroke: {t_stroke}"/>
  <line x1="0" y1="0" x2="0" y2="1" style="stroke-width: 0.1; stroke: {t_stroke}"/>
</pattern>"#,
    ));
}

fn d_hatch(tb: &mut ThemeBuilder, t_stroke: &str) {
    tb.add_style(".d-hatch:not(text,tspan) {fill: url(#hatch)}");
    tb.add_defs(&format!(
r#"<pattern id="hatch" x="0" y="0" width="1" height="1" patternTransform="rotate(120)" patternUnits="userSpaceOnUse" >
  <rect width="100%" height="100%" style="stroke: none"/>
  <line x1="0" y1="0" x2="1" y2="0" style="stroke-width: 0.1; stroke: {t_stroke}"/>
</pattern>"#,
    ));
}

fn d_softshadow(tb: &mut ThemeBuilder, _: &str) {
    tb.add_style(".d-softshadow:not(text,tspan) { filter: url(#d-softshadow); }");
    tb.add_defs(
        r#"<filter id="d-softshadow" x="-50%" y="-50%" width="200%" height="200%">
  <feGaussianBlur in="SourceAlpha" stdDeviation="0.7"/>
  <feOffset dx="1" dy="1"/>
  <feComposite in2="SourceGraphic" operator="arithmetic" k1="0" k2="0.4" k3="1" k4="0"/>
</filter>"#,
    );
}

fn d_hardshadow(tb: &mut ThemeBuilder, _: &str) {
    tb.add_style(".d-hardshadow:not(text,tspan) { filter: url(#d-hardshadow); }");
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
        if tb.background != "default" {
            tb.add_style(&format!("svg {{ background: {}; }}", tb.background));
        } else {
            tb.add_style(&format!(
                "svg {{ background: {}; }}",
                self.default_background()
            ));
        }
        self.append_early_styles(tb);
        append_common_styles(
            tb,
            &self.default_fill(),
            &self.default_stroke(),
            self.default_stroke_width(),
        );
        append_stroke_width_styles(tb, self.default_stroke_width());
        if tb.elements.contains("text") {
            append_text_styles(tb, &self.default_stroke());
        }

        append_arrow_styles(tb);
        append_dash_styles(tb);

        if tb.has_class("d-surround") {
            tb.add_style(".d-surround:not(text,tspan) { fill: none; }");
        }

        append_colour_styles(tb);

        type Tfn = dyn Fn(&mut ThemeBuilder, &str);
        for (class, build_fn) in [
            ("d-softshadow", &d_softshadow as &Tfn),
            ("d-hardshadow", &d_hardshadow as &Tfn),
            ("d-stipple", &d_stipple as &Tfn),
            ("d-hatch", &d_hatch as &Tfn),
            ("d-crosshatch", &d_crosshatch as &Tfn),
        ] {
            if tb.has_class(class) {
                build_fn(tb, &self.default_stroke());
            }
        }
        self.append_late_styles(tb);
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
    styles: Vec<String>,
    defs: Vec<String>,

    background: String,
    theme: ThemeType,
    classes: HashSet<String>,
    elements: HashSet<String>,
}

impl ThemeBuilder {
    pub fn new(
        config: &TransformConfig,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
    ) -> Self {
        Self {
            styles: Vec::new(),
            defs: Vec::new(),
            background: config.background.clone(),
            theme: config.theme.clone(),
            classes: classes.to_owned(),
            elements: elements.to_owned(),
        }
    }
    pub fn build(&mut self) {
        match self.theme {
            ThemeType::Default(_) => DefaultTheme {}.build(self),
            ThemeType::Bold(_) => BoldTheme {}.build(self),
            ThemeType::Fine(_) => FineTheme {}.build(self),
            ThemeType::Glass(_) => GlassTheme {}.build(self),
            ThemeType::Light(_) => LightTheme {}.build(self),
            ThemeType::Dark(_) => DarkTheme {}.build(self),
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
