use std::collections::HashSet;

use crate::TransformConfig;

// List taken from https://www.w3.org/TR/SVG11/types.html#ColorKeywords
static COLOUR_LIST: &[&str] = &[
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "grey",
    "green",
    "greenyellow",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
    // Also include 'none' which is a valid stroke & fill value
    "none",
];

// The following - a subset of `COLOUR_LIST` - are (subjectively) 'dark'
// and by default will have white text rather than black when used as a
// fill style (e.g. `d-fill-brown`)
static DARK_COLOURS: &[&str] = &[
    "black",
    "blue",
    "blueviolet",
    "brown",
    "cadetblue",
    "chocolate",
    "coral",
    "cornflowerblue",
    "crimson",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "forestgreen",
    "fuchsia",
    "goldenrod",
    "gray",
    "grey",
    "green",
    "hotpink",
    "indianred",
    "indigo",
    "lightcoral",
    "lightslategray",
    "lightslategrey",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "navy",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palevioletred",
    "purple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "seagreen",
    "sienna",
    "slateblue",
    "slategray",
    "slategrey",
    "steelblue",
    "teal",
    "tomato",
];

pub fn build_styles(
    elements: &HashSet<String>,
    classes: &HashSet<String>,
    config: &TransformConfig,
) -> Vec<String> {
    let mut result = Vec::new();

    // Default styles suitable for box-and-line diagrams
    if config.background != "none" {
        result.push(format!("svg {{ background: {}; }}", config.background));
    }
    result.extend(vec![
        String::from("rect, circle, ellipse, line, polyline, polygon, path { stroke-width: 0.5; stroke: black; }"),
        String::from("rect, circle, ellipse, polygon { fill: white; }"),
        String::from("line, polyline, path { fill: none; }"),
    ]);
    if elements.contains("text") {
        result.push(String::from(
            "text { font-family: sans-serif; font-size: 3px; }",
        ));
    }

    // stroke-widths
    {
        let line_style = vec![
            ("d-thinner", "0.125"),
            ("d-thin", "0.25"),
            ("d-thick", "1"),
            ("d-thicker", "2"),
        ];
        for (class, width) in line_style {
            if classes.contains(class) {
                result.push(format!(".{class} {{ stroke-width: {width}; }}"));
            }
        }
    }

    if classes.contains("d-tbox") {
        result.push(String::from(
            r#"text.d-tbox, text.d-tbox * { text-anchor: middle; dominant-baseline: central; }"#,
        ));
    }
    if classes.contains("d-text-top") {
        result.push(String::from(
            r#"text.d-text-top, text.d-text-top * { dominant-baseline: text-before-edge; }"#,
        ));
    }
    if classes.contains("d-text-bottom") {
        result.push(String::from(
            r#"text.d-text-bottom, text.d-text-bottom * { dominant-baseline: text-after-edge; }"#,
        ));
    }
    if classes.contains("d-text-left") {
        result.push(String::from(
            r#"text.d-text-left, text.d-text-left * { text-anchor: start; }"#,
        ));
    }
    if classes.contains("d-text-right") {
        result.push(String::from(
            r#"text.d-text-right, text.d-text-right * { text-anchor: end; }"#,
        ));
    }
    if classes.contains("d-text-top-vertical") {
        result.push(String::from(
            r#"text.d-text-top-vertical, text.d-text-top-vertical * { text-anchor: start; }"#,
        ));
    }
    if classes.contains("d-text-bottom-vertical") {
        result.push(String::from(
            r#"text.d-text-bottom-vertical, text.d-text-bottom-vertical * { text-anchor: end; }"#,
        ));
    }
    if classes.contains("d-text-left-vertical") {
        result.push(String::from(
            r#"text.d-text-left-vertical, text.d-text-left-vertical * { dominant-baseline: text-after-edge; }"#,
        ));
    }
    if classes.contains("d-text-right-vertical") {
        result.push(String::from(
            r#"text.d-text-right-vertical, text.d-text-right-vertical * { dominant-baseline: text-before-edge; }"#,
        ));
    }

    // Text size is 3px by default.
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
    // Default is sans-serif 'normal' text.
    let font_style = vec![
        ("d-text-bold", "font-weight: bold"),
        ("d-text-italic", "font-style: italic"),
        ("d-text-monospace", "font-family: monospace"),
    ];
    for (class, style) in font_style {
        if classes.contains(class) {
            result.push(format!("text.{0}, text.{0} * {{ {1}; }}", class, style));
        }
    }

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

    {
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

    {
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

    if classes.contains("d-surround") {
        result.push(String::from(".d-surround:not(text,tspan) { fill: none; }"));
    }
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

    result
}

pub fn build_defs(
    _elements: &HashSet<String>,
    classes: &HashSet<String>,
    _config: &TransformConfig,
) -> Vec<String> {
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

    result
}
