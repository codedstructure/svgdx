use std::collections::HashSet;

static COLOUR_LIST: &[&str] = &[
    "red", "green", "blue", "cyan", "magenta", "yellow", "black", "white", "none",
];
static DARK_COLOURS: &[&str] = &["red", "green", "blue", "magenta", "black"];

pub(crate) fn build_styles(
    elements: &HashSet<String>,
    classes: &HashSet<String>,
    indent: &str,
) -> String {
    let mut result = Vec::new();

    result.push(String::from(
        "rect, circle, ellipse, line, polyline, polygon { stroke-width: 0.5; stroke: black; fill: none; }",
    ));
    if elements.contains("text") {
        result.push(String::from(
            "text { font-family: sans-serif; font-size: 3px; }",
        ));
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

    if classes.contains("d-softshadow") {
        result.push(String::from(".d-softshadow { filter: url(#d-softshadow) }"));
    }
    if classes.contains("d-hardshadow") {
        result.push(String::from(".d-hardshadow { filter: url(#d-hardshadow) }"));
    }
    if classes.contains("d-arrow") {
        result.push(String::from(
            "line.d-arrow, polyline.d_arrow { marker-end: url(#d-arrow) }",
        ));
    }
    if classes.contains("d-dash") {
        result.push(String::from(r#".d-dash { stroke-dasharray: 2 1.5; }"#));
    }
    if classes.contains("d-dot") {
        result.push(String::from(r#".d-dot { stroke-dasharray: 0.5 1; }"#));
    }
    for colour in COLOUR_LIST {
        if classes.contains(&format!("d-{colour}")) {
            result.push(format!(".d-{colour} {{ stroke: {colour}; }}"));
            result.push(format!("text.d-{colour}, text.d-{colour} * {{ stroke: none; }}"));
        }
    }
    for colour in COLOUR_LIST {
        if classes.contains(&format!("d-fill-{colour}")) {
            result.push(format!(".d-fill-{colour} {{ fill: {colour}; }}"));
            let text_colour = if DARK_COLOURS.contains(colour) { "white" } else { "black" };
            result.push(format!(
                "text.d-fill-{colour}, text.d-fill-{colour} * {{ fill: {text_colour}; }}"
            ));
        }
    }
    if !result.is_empty() {
        let mut style = String::from(&format!("<style>{indent}"));
        for rule in result {
            style.push_str(&format!("  {rule}{indent}"));
        }
        style.push_str("</style>");
        style
    } else {
        String::new()
    }
}

pub(crate) fn build_defs(
    _elements: &HashSet<String>,
    classes: &HashSet<String>,
    indent: &str,
) -> String {
    let mut result = Vec::new();

    if classes.contains("d-arrow") {
        result.push(String::from(r#"<marker id="d-arrow" refX="1" refY="0.5" orient="auto-start-reverse" markerWidth="5" markerHeight="5" viewBox="0 0 1 1">
      <path d="M 0 0 1 0.5 0 1" style="stroke-width: 0.2;"/>
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

    if !result.is_empty() {
        let mut defs = String::from(&format!("<defs>{indent}"));
        for rule in result {
            defs.push_str(&format!("  {rule}{indent}"));
        }
        defs.push_str(&format!("</defs>{indent}").to_owned());
        defs
    } else {
        String::new()
    }
}
