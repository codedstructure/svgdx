// Map from classes to inline styles.

use super::colours::{COLOUR_LIST, DARK_COLOURS};
use crate::elements::SvgElement;
use crate::types::StyleMap;

pub fn apply_auto_styles(element: &mut SvgElement) {
    let mut style = StyleMap::new();
    shape_style(element, &mut style);

    colour_styles(element, &mut style);
    if matches!(element.name(), "text" | "tspan") {
        text_styles(element, &mut style);
    }
    arrow_styles(element, &mut style);
    dash_styles(element, &mut style);
    pattern_styles(element, &mut style);
    shadow_styles(element, &mut style);

    tidy_styles(element, &mut style);

    for (key, value) in style {
        element.add_auto_style(&key, &value);
    }
}

fn shape_style(element: &SvgElement, style: &mut StyleMap) {
    let fill = "none";
    let stroke = "black";
    let stroke_width = "0.5";
    let name = element.name();
    match name {
        "rect" | "circle" | "ellipse" | "polygon" => {
            style.insert("stroke-width", stroke_width);
            style.insert("fill", fill);
            style.insert("stroke", stroke);
        }
        "line" | "polyline" | "path" => {
            style.insert("stroke-width", stroke_width);
            if name != "line" {
                style.insert("fill", "none");
            }
            style.insert("stroke", stroke);
        }
        "text" | "tspan" => {
            style.insert("font-family", "sans-serif");
            style.insert("font-size", "3px");
            style.insert("fill", stroke);
        }
        _ => {}
    }
}

fn colour_styles(element: &SvgElement, style: &mut StyleMap) {
    // Colours
    // - d-colour sets a 'default' colour for shape outlines and text
    // - d-fill-colour sets the colour for shape fills, and sets a text colour
    //   to an appropriate contrast colour.
    // - d-text-colour sets the colour for text elements, which overrides any
    //   colours set by d-colour or d-fill-colour.
    // - d-text-ol-colour sets the colour for text outline
    let is_text = matches!(element.name(), "text" | "tspan");

    let mut d_colour = None;
    let mut d_fill_colour = None;
    let mut d_text_colour = None;
    let mut d_text_ol_colour = None;

    fn is_colour(colour: &str) -> bool {
        COLOUR_LIST.binary_search(&colour).is_ok()
    }

    let classes = element.get_classes();
    for class in classes.iter().filter_map(|c| c.strip_prefix("d-")) {
        let z: Vec<&str> = class.split('-').collect();
        match z.as_slice() {
            ["text", "ol", colour] if is_colour(colour) => {
                d_text_ol_colour = Some(*colour);
            }
            ["text", colour] if is_colour(colour) => {
                d_text_colour = Some(*colour);
            }
            ["fill", colour] if is_colour(colour) => {
                d_fill_colour = Some(*colour);
            }
            [colour] if is_colour(colour) => {
                d_colour = Some(*colour);
            }
            _ => {}
        }
    }

    if d_text_colour.is_none() {
        // set appropriate text colour based on shape fill
        if let Some(sc) = d_fill_colour {
            if DARK_COLOURS.binary_search(&sc).is_ok() {
                d_text_colour = Some("white");
                if d_text_ol_colour.is_none() {
                    d_text_ol_colour = Some("black");
                }
            }
        }
    }

    if is_text {
        if let Some(c) = d_text_colour.or(d_colour) {
            style.insert("fill", c);
        }
        if let Some(c) = d_text_ol_colour {
            style.insert("paint-order", "stroke");
            style.insert("stroke", c);
            style.insert("stroke-linecap", "round");
            style.insert("stroke-linejoin", "round");
            if !style.contains_key("stroke-width") {
                // TODO: other text stroke widths
                style.insert("stroke-width", "0.5");
            }
        }
    } else {
        if let Some(c) = d_fill_colour {
            style.insert("fill", c);
        }
        if let Some(c) = d_colour {
            style.insert("stroke", c);
        }
    }
}

fn text_styles(element: &mut SvgElement, style: &mut StyleMap) {
    // Text alignment - default centered horizontally and vertically
    // These are intended to be composable, e.g. "d-text-top d-text-right"
    for (c, (key, value)) in [
        ("d-text", ("text-anchor", "middle")),
        ("d-text", ("dominant-baseline", "central")),
        ("d-text-top", ("dominant-baseline", "text-before-edge")),
        ("d-text-bottom", ("dominant-baseline", "text-after-edge")),
        ("d-text-left", ("text-anchor", "start")),
        ("d-text-right", ("text-anchor", "end")),
        ("d-text-top-vertical", ("text-anchor", "start")),
        ("d-text-bottom-vertical", ("text-anchor", "end")),
        (
            "d-text-left-vertical",
            ("dominant-baseline", "text-after-edge"),
        ),
        (
            "d-text-right-vertical",
            ("dominant-baseline", "text-before-edge"),
        ),
        // Default is sans-serif 'normal' text.
        ("d-text-bold", ("font-weight", "bold")),
        // Allow explicitly setting 'normal' font-weight, as themes may set a non-normal default.
        ("d-text-normal", ("font-weight", "normal")),
        ("d-text-light", ("font-weight", "100")),
        ("d-text-italic", ("font-style", "italic")),
        ("d-text-monospace", ("font-family", "monospace")),
        ("d-text-pre", ("font-family", "monospace")),
    ] {
        if element.has_class(c) {
            style.insert(key, value);
        }
    }

    // TODO: text sizes etc
}

fn arrow_styles(element: &mut SvgElement, style: &mut StyleMap) {
    if element.has_class("d-arrow") {
        style.insert("marker-end", "url(#d-arrow)");
    }
    if element.has_class("d-biarrow") {
        style.insert("marker-start", "url(#d-arrow)");
        style.insert("marker-end", "url(#d-arrow)");
    }
}

fn dash_styles(element: &mut SvgElement, style: &mut StyleMap) {
    // TODO: flow classes
    if element.has_class("d-dash") {
        style.insert("stroke-dasharray", "1 1.5");
    }
    if element.has_class("d-dot") {
        // TODO: this only works with stroke-linecap: round...
        // we could include it here, or throughout for lines?
        // maybe all lines should have round linecap/join?
        // I quite like not having it for d-dash though, but
        // may want to adjust the dasharray there.
        style.insert("stroke-linecap", "round");
        style.insert("stroke-dasharray", "0 1");
    }
    if element.has_class("d-dot-dash") {
        style.insert("stroke-linecap", "round");
        style.insert("stroke-dasharray", "0 1 1.5 1 0 1.5");
    }
}

fn pattern_styles(element: &SvgElement, style: &mut StyleMap) {
    const PATTERN_CLASSES: &[&str] = &[
        "d-grid",
        "d-grid-h",
        "d-grid-v",
        "d-hatch",
        "d-crosshatch",
        "d-stipple",
    ];

    for class in element.get_classes() {
        for &ptn_class in PATTERN_CLASSES {
            let is_exact = class == ptn_class;
            let is_numbered = !is_exact
                && class
                    .strip_prefix(&format!("{ptn_class}-"))
                    .and_then(|s| s.parse::<u32>().ok())
                    .is_some_and(|n| n <= 100);

            if is_exact || is_numbered {
                let fill_url = format!("url(#{})", class.trim_start_matches("d-"));
                style.insert("fill", fill_url);
            }
        }
    }
}

fn shadow_styles(element: &mut SvgElement, style: &mut StyleMap) {
    if element.has_class("d-softshadow") {
        style.insert("filter", "url(#d-softshadow)");
    }
    if element.has_class("d-hardshadow") {
        style.insert("filter", "url(#d-hardshadow)");
    }
}

fn tidy_styles(element: &mut SvgElement, style: &mut StyleMap) {
    let is_text = matches!(element.name(), "text" | "tspan");

    // SVG `stroke` defaults to `none`. Do we want to explicitly include
    // setting to none? e.g. if no stroke is set, should stroke-width be
    // omitted? What if there's CSS or a presentation attribute?
    if style.get("stroke") == Some("none") {
        style.pop("stroke-width");
        if is_text {
            // text doesn't have a stroke by default
            style.pop("stroke");
            style.pop("paint-order");
        }
    }
    if style.get("stroke-width") == Some("0") {
        style.pop("stroke");
        if is_text {
            // text doesn't have a stroke by default
            style.pop("stroke-width");
            style.pop("paint-order");
        }
    }

    // Consider removing other default style values, e.g. SVG stroke-width
    // default is 1px (but should we assume 1==1px?)
}
