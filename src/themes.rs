use crate::types::fstr;
use crate::TransformConfig;
use std::{collections::HashSet, str::FromStr};

use crate::colours::{COLOUR_LIST, DARK_COLOURS};
use crate::svg_defs::{build_defs, build_styles};
use anyhow::bail;

pub trait Theme {
    fn build_styles(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String>;
    fn build_defs(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String>;
}

#[derive(Debug, Clone)]
pub enum ThemeType {
    Default(DefaultTheme),
    Bold(BoldTheme),
    Fine(FineTheme),
    Tranlucent(TranslucentTheme),
    Dark(DarkTheme),
}

impl FromStr for ThemeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(Self::default()),
            "bold" => Ok(Self::Bold(BoldTheme)),
            "fine" => Ok(Self::Fine(FineTheme)),
            "translucent" => Ok(Self::Tranlucent(TranslucentTheme)),
            "dark" => Ok(Self::Dark(DarkTheme)),
            _ => bail!(
                "Unknown theme '{}' (available themes: default, bold, fine)",
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

impl Theme for ThemeType {
    fn build_styles(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        match self {
            ThemeType::Default(t) => t.build_styles(elements, classes, config),
            ThemeType::Bold(t) => t.build_styles(elements, classes, config),
            ThemeType::Fine(t) => t.build_styles(elements, classes, config),
            ThemeType::Tranlucent(t) => t.build_styles(elements, classes, config),
            ThemeType::Dark(t) => t.build_styles(elements, classes, config),
        }
    }

    fn build_defs(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        match self {
            ThemeType::Default(t) => t.build_defs(elements, classes, config),
            ThemeType::Bold(t) => t.build_defs(elements, classes, config),
            ThemeType::Fine(t) => t.build_defs(elements, classes, config),
            ThemeType::Tranlucent(t) => t.build_defs(elements, classes, config),
            ThemeType::Dark(t) => t.build_defs(elements, classes, config),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DefaultTheme;

impl Theme for DefaultTheme {
    fn build_styles(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        build_styles(elements, classes, config)
    }

    fn build_defs(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        build_defs(elements, classes, config)
    }
}

#[derive(Debug, Clone)]
pub struct BoldTheme;

impl Theme for BoldTheme {
    fn build_styles(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        let mut styles = Vec::new();
        styles
    }

    fn build_defs(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        build_defs(elements, classes, config)
    }
}

fn append_common_styles(result: &mut Vec<String>, fill: &str, stroke: &str) {
    // Default styles suitable for box-and-line diagrams
    result.extend(
        [
            format!("rect, circle, ellipse, polygon {{ fill: {fill}; stroke: {stroke}; }}"),
            format!("line, polyline, path {{ fill: none; stroke: {stroke}; }}"),
            format!("text, tspan {{ fill: {stroke}; }}"),
        ]
        .to_vec(),
    )
}

fn append_text_styles(result: &mut Vec<String>, classes: &HashSet<String>) {
    result.extend([
        "text, tspan { stroke-width: 0; font-family: sans-serif; font-size: 3px; }",
        // Text alignment - default centered horizontally and vertically
        // These are intended to be composable, e.g. "d-text-top d-text-right"
        "text.d-tbox, text.d-tbox * { text-anchor: middle; dominant-baseline: central; }",
        "text.d-text-top, text.d-text-top * { dominant-baseline: text-before-edge; }",
        "text.d-text-bottom, text.d-text-bottom * { dominant-baseline: text-after-edge; }",
        "text.d-text-left, text.d-text-left * { text-anchor: start; }",
        "text.d-text-right, text.d-text-right * { text-anchor: end; }",
        "text.d-text-top-vertical, text.d-text-top-vertical * { text-anchor: start; }",
        "text.d-text-bottom-vertical, text.d-text-bottom-vertical * { text-anchor: end; }",
        "text.d-text-left-vertical, text.d-text-left-vertical * { dominant-baseline: text-after-edge; }",
        "text.d-text-right-vertical, text.d-text-right-vertical * { dominant-baseline: text-before-edge; }",
        // Default is sans-serif 'normal' text.
        "text.d-text-bold, text.d-text-bold * { font-weight: bold; }",
        "text.d-text-italic, text.d-text-italic * { font-style: italic; }",
        "text.d-text-monospace, text.d-text-monospace * { font-family: monospace; }",
    ].map(|s| s.to_string()).to_vec());

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
        if classes.contains(class) {
            result.push(format!(
                "text.{0}, text.{0} * {{ font-size: {1}px; }}",
                class, size
            ));
        }
    }
}

fn append_stroke_width_styles(result: &mut Vec<String>, base: f32) {
    result.push(format!("* {{ stroke-width: {}; }}", fstr(base)));
    for (class, width) in [
        ("d-thinner", base * 0.25),
        ("d-thin", base * 0.5),
        ("d-thick", base * 2.),
        ("d-thicker", base * 4.),
    ] {
        result.push(format!(".{class} {{ stroke-width: {}; }}", fstr(width)));
    }
}

fn append_colour_styles(result: &mut Vec<String>, classes: &HashSet<String>) {
    // Colours
    // - d-colour sets a 'default' colour for shape outlines and text
    // - d-fill-colour sets the colour for shape fills, and sets a text colour
    //   to an appropriate contrast colour.
    // - d-text-colour sets the colour for text elements, which overrides any
    //   colours set by d-colour or d-fill-colour.
    for colour in COLOUR_LIST {
        if classes.contains(&format!("d-fill-{colour}")) {
            result.push(format!(
                ".d-fill-{colour}:not(text,tspan) {{ fill: {colour}; }}"
            ));
            let text_colour = if DARK_COLOURS.contains(colour) {
                "white"
            } else {
                "black"
            };
            result.push(format!(
                "text.d-fill-{colour}, text.d-fill-{colour} * {{ fill: {text_colour}; }}"
            ));
        }
    }
    for colour in COLOUR_LIST {
        if classes.contains(&format!("d-{colour}")) {
            result.push(format!(
                ".d-{colour}:not(text,tspan) {{ stroke: {colour}; }}"
            ));
            // By default text is the same colour as shape stroke, but may be
            // overridden by d-text-colour (e.g. for text attrs on shapes)
            // Also special-case 'none'; there are many use-cases for not having
            // a stroke colour (using `d-none`), but text should always have a colour.
            if *colour != "none" {
                result.push(format!(
                    "text.d-{colour}, text.d-{colour} * {{ fill: {colour}; }}"
                ));
            }
        }
    }
    for colour in COLOUR_LIST {
        if classes.contains(&format!("d-text-{colour}")) {
            // Must be at least as specific as d-fill-colour
            result.push(format!(
                "text.d-text-{colour}, text.d-text-{colour} * {{ fill: {colour}; }}"
            ));
        }
    }
}
fn append_arrow_styles(result: &mut Vec<String>, classes: &HashSet<String>) {
    let mut has_arrow = false;
    if classes.contains("d-arrow") {
        result.push(String::from(
            "line.d-arrow, polyline.d-arrow, path.d-arrow { marker-end: url(#d-arrow); }",
        ));
        has_arrow = true;
    }
    if classes.contains("d-biarrow") {
        result.push(String::from(
                "line.d-biarrow, polyline.d-biarrow, path.d-biarrow { marker-start: url(#d-arrow); marker-end: url(#d-arrow); }",
            ));
        has_arrow = true;
    }
    if has_arrow {
        // override the default 'fill:none' for markers.
        result.push(String::from("marker path { fill: inherit; }"));
    }
}

fn append_dash_styles(result: &mut Vec<String>, classes: &HashSet<String>) {
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
        if classes.contains(class) {
            // d-flow defaults to equivalent of d-dash, but also works with d-dot.
            result.push(format!(".{class} {{ animation: {speed}s linear 0s infinite running d-flow-animation; stroke-dasharray: 1.5 0.5; }}"));
            has_flow = true;
        }
    }
    if has_flow {
        result.push(String::from("@keyframes d-flow-animation { from {stroke-dashoffset: 4;} to {stroke-dashoffset: 0;} }"));
    }
    if classes.contains("d-flow-rev") {
        result.push(String::from(
            ".d-flow-rev { animation-direction: reverse; }",
        ));
    }
    // NOTE: these are after the d-flow-* classes, as they provide a default dasharray these may override.
    if classes.contains("d-dash") {
        result.push(String::from(".d-dash { stroke-dasharray: 1.5 0.5; }"));
    }
    if classes.contains("d-dot") {
        result.push(String::from(".d-dot { stroke-dasharray: 0.5 0.5; }"));
    }
}

fn append_defs(result: &mut Vec<String>, classes: &HashSet<String>) {
    let mut result = Vec::new();

    if classes.contains("d-arrow") || classes.contains("d-biarrow") {
        // Note use of context-stroke for fill, and setting stroke:none to prevent
        // the marker size extending beyond the path boundary.
        // NOTE: the arrow marker butts up against the end of the line so doesn't have
        // a 'point'. This means the line and arrow both end together and the line is
        // never thicker than the arrow, but isn't ideal visually.
        // A more sophisticated system would have the marker 'after' the line, and
        // reduce the line length by the marker width - but that would be complex
        // in this program. Maybe in the future.
        result.push(String::from(r#"<marker id="d-arrow" refX="0.8" refY="0.5" orient="auto-start-reverse" markerWidth="6" markerHeight="5" viewBox="0 0 0.4 1">
  <path d="M 0 0 1 0.5 0 1" style="stroke: none; fill: context-stroke;"/>
</marker>"#));
    }

    if classes.contains("d-softshadow") {
        result.push(String::from(
            r#"<filter id="d-softshadow" x="-50%" y="-50%" width="200%" height="200%">
  <feGaussianBlur in="SourceAlpha" stdDeviation="0.7"/>
  <feOffset dx="1" dy="1"/>
  <feComposite in2="SourceGraphic" operator="arithmetic" k1="0" k2="0.4" k3="1" k4="0"/>
</filter>"#,
        ));
    }
    if classes.contains("d-hardshadow") {
        result.push(String::from(
            r#"<filter id="d-hardshadow" x="-50%" y="-50%" width="200%" height="200%">
  <feGaussianBlur in="SourceAlpha" stdDeviation="0.2"/>
  <feOffset dx="1" dy="1"/>
  <feComposite in2="SourceGraphic" operator="arithmetic" k1="0" k2="0.6" k3="1" k4="0"/>
</filter>"#,
        ));
    }
}

#[derive(Debug, Clone)]
pub struct FineTheme;

impl Theme for FineTheme {
    fn build_styles(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        let mut result = Vec::new();
        if config.background != "none" {
            result.push(format!("svg {{ background: {}; }}", config.background));
        } else {
            result.push("svg { background: #eee8d5; }".to_string());
        }
        append_common_styles(&mut result, "#fdf6e3", "#657b83");
        append_stroke_width_styles(&mut result, 0.2);
        if elements.contains("text") {
            append_text_styles(&mut result, classes);
        }
        result.push(
            "text, tspan { stroke-width: 0; font-family: sans-serif; font-size: 3px; }".to_string(),
        );

        if classes.contains("d-softshadow") {
            result.push(String::from(
                ".d-softshadow:not(text,tspan) { filter: url(#d-softshadow); }",
            ));
        }
        if classes.contains("d-hardshadow") {
            result.push(String::from(
                ".d-hardshadow:not(text,tspan) { filter: url(#d-hardshadow); }",
            ));
        }

        append_arrow_styles(&mut result, classes);
        append_dash_styles(&mut result, classes);

        if classes.contains("d-surround") {
            result.push(String::from(".d-surround:not(text,tspan) { fill: none; }"));
        }

        append_colour_styles(&mut result, classes);

        result
    }

    fn build_defs(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        build_defs(elements, classes, config)
    }
}

#[derive(Debug, Clone)]
pub struct TranslucentTheme;

impl Theme for TranslucentTheme {
    fn build_styles(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        let mut result = Vec::new();
        if config.background != "none" {
            result.push(format!("svg {{ background: {}; }}", config.background));
        }
        append_common_styles(&mut result, "rgba(0,30,50, 0.15)", "black");
        append_stroke_width_styles(&mut result, 0.5);
        if elements.contains("text") {
            append_text_styles(&mut result, classes);
        }
        result.push(
            "text, tspan { stroke-width: 0; font-family: sans-serif; font-size: 3px; }".to_string(),
        );

        if classes.contains("d-softshadow") {
            result.push(String::from(
                ".d-softshadow:not(text,tspan) { filter: url(#d-softshadow); }",
            ));
        }
        if classes.contains("d-hardshadow") {
            result.push(String::from(
                ".d-hardshadow:not(text,tspan) { filter: url(#d-hardshadow); }",
            ));
        }

        append_arrow_styles(&mut result, classes);
        append_dash_styles(&mut result, classes);

        if classes.contains("d-surround") {
            //result.push(String::from(".d-surround:not(text,tspan) { fill: none; }"));
        }

        append_colour_styles(&mut result, classes);

        result
    }

    fn build_defs(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        build_defs(elements, classes, config)
    }
}

#[derive(Debug, Clone)]
pub struct DarkTheme;

impl Theme for DarkTheme {
    fn build_styles(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        let mut result = Vec::new();
        if config.background != "none" {
            result.push(format!("svg {{ background: {}; }}", config.background));
        }
        append_common_styles(&mut result, "#444", "#eee");
        append_stroke_width_styles(&mut result, 0.5);
        if elements.contains("text") {
            append_text_styles(&mut result, classes);
        }
        result.push(
            "text, tspan { stroke-width: 0; font-family: sans-serif; font-size: 3px; }".to_string(),
        );

        if classes.contains("d-softshadow") {
            result.push(String::from(
                ".d-softshadow:not(text,tspan) { filter: url(#d-softshadow); }",
            ));
        }
        if classes.contains("d-hardshadow") {
            result.push(String::from(
                ".d-hardshadow:not(text,tspan) { filter: url(#d-hardshadow); }",
            ));
        }

        append_arrow_styles(&mut result, classes);
        append_dash_styles(&mut result, classes);

        if classes.contains("d-surround") {
            //result.push(String::from(".d-surround:not(text,tspan) { fill: none; }"));
        }

        append_colour_styles(&mut result, classes);

        result
    }

    fn build_defs(
        &self,
        elements: &HashSet<String>,
        classes: &HashSet<String>,
        config: &TransformConfig,
    ) -> Vec<String> {
        build_defs(elements, classes, config)
    }
}
