use crate::element::{ContentType, SvgElement};
use crate::types::{attr_split_cycle, fstr, strp, LocSpec};

use anyhow::{Context, Result};
use lazy_regex::regex;
use regex::Captures;

fn get_text_value(element: &mut SvgElement) -> String {
    let text_value = element
        .pop_attr("text")
        .expect("no text attr in process_text_attr");
    // Convert unescaped '\n' into newline characters for multi-line text
    let re = regex!(r"\\n");
    let text_value = re.replace_all(&text_value, |caps: &Captures| {
        let inner = caps.get(0).expect("Matched regex must have this group");
        // Check if the newline is escaped; do this here rather than within the regex
        // to avoid the need for an extra initial character which can cause matches
        // to overlap and fail replacement. We're safe to look at the previous byte
        // since Match.start() is guaranteed to be a utf8 char boundary, and '\' has
        // the top bit clear, so will only match on a one-byte utf8 char.
        let start = inner.start();
        if start > 0 && text_value.as_bytes()[start - 1] == b'\\' {
            inner.as_str().to_string()
        } else {
            "\n".to_owned()
        }
    });
    // Following that, replace any escaped "\\n" into literal '\'+'n' characters
    let re = regex!(r"\\\\n");
    re.replace_all(&text_value, "\\n").into_owned()
}

fn get_text_position<'a>(element: &mut SvgElement) -> Result<(f32, f32, LocSpec, Vec<&'a str>)> {
    let mut t_dx = None;
    let mut t_dy = None;
    {
        let dx = element.pop_attr("text-dx");
        let dy = element.pop_attr("text-dy");
        let dxy = element.pop_attr("text-dxy");
        if let Some(dxy) = dxy {
            let mut parts = attr_split_cycle(&dxy).map_while(|v| strp(&v).ok());
            t_dx = Some(parts.next().context("dx from text-dxy should be numeric")?);
            t_dy = Some(parts.next().context("dy from text-dxy should be numeric")?);
        }
        if let Some(dx) = dx {
            t_dx = Some(strp(&dx)?);
        }
        if let Some(dy) = dy {
            t_dy = Some(strp(&dy)?);
        }
    }

    let mut text_classes = vec!["d-tbox"];
    let text_loc = LocSpec::try_from(element.pop_attr("text-loc").unwrap_or("c".into()))?;

    // Default dx/dy to push it in slightly from the edge (or out for lines);
    // Without inset text squishes to the edge and can be unreadable
    // Any specified dx/dy override this behaviour.
    let text_inset = 1.;

    let is_line = element.name == "line";
    // text associated with a line is pushed 'outside' the line,
    // where with other shapes it's pulled 'inside'. The classes
    // and dx/dy values are opposite.
    match text_loc {
        LocSpec::TopLeft | LocSpec::Top | LocSpec::TopRight => {
            text_classes.push(if is_line {
                "d-text-bottom"
            } else {
                "d-text-top"
            });
            if t_dy.is_none() {
                t_dy = Some(if is_line { -text_inset } else { text_inset });
            }
        }
        LocSpec::BottomRight | LocSpec::Bottom | LocSpec::BottomLeft => {
            text_classes.push(if is_line {
                "d-text-top"
            } else {
                "d-text-bottom"
            });
            if t_dy.is_none() {
                t_dy = Some(if is_line { text_inset } else { -text_inset });
            }
        }
        _ => (),
    }

    match text_loc {
        LocSpec::TopLeft | LocSpec::Left | LocSpec::BottomLeft => {
            text_classes.push(if is_line {
                "d-text-right"
            } else {
                "d-text-left"
            });
            if t_dx.is_none() {
                t_dx = Some(if is_line { -text_inset } else { text_inset });
            }
        }
        LocSpec::TopRight | LocSpec::Right | LocSpec::BottomRight => {
            text_classes.push(if is_line {
                "d-text-left"
            } else {
                "d-text-right"
            });
            if t_dx.is_none() {
                t_dx = Some(if is_line { text_inset } else { -text_inset });
            }
        }
        _ => (),
    }

    // Assumption is that text should be centered within the rect,
    // and has styling via CSS to reflect this, e.g.:
    //  text.d-tbox { dominant-baseline: central; text-anchor: middle; }
    let (mut tdx, mut tdy) = element.bbox()?.context("No BoundingBox")?.locspec(text_loc);
    if let Some(dx) = t_dx {
        tdx += dx;
    }
    if let Some(dy) = t_dy {
        tdy += dy;
    }

    Ok((tdx, tdy, text_loc, text_classes))
}

pub fn process_text_attr(element: &SvgElement) -> Result<(SvgElement, Vec<SvgElement>)> {
    // Different conversions from line count to first-line offset based on whether
    // top, center, or bottom justification.
    const WRAP_DOWN: fn(usize, f32) -> f32 = |_count, _spacing| 0.;
    const WRAP_UP: fn(usize, f32) -> f32 = |count, spacing| -(count as f32 - 1.) * spacing;
    const WRAP_MID: fn(usize, f32) -> f32 = |count, spacing| -(count as f32 - 1.) / 2. * spacing;

    const ZWSP: &str = "\u{200B}"; // Zero-width space
    const NBSP: &str = "\u{00A0}"; // Non-breaking space

    let mut orig_elem = element.clone();

    let text_value = get_text_value(&mut orig_elem);

    let (tdx, tdy, text_loc, mut text_classes) = get_text_position(&mut orig_elem)?;
    text_classes.push("d-tbox");

    let text_attrs = vec![("x".into(), fstr(tdx)), ("y".into(), fstr(tdy))];
    let mut text_elements = Vec::new();
    let lines: Vec<_> = text_value.lines().collect();
    let line_count = lines.len();

    let multiline = line_count > 1;

    // There will always be a text element; if not multiline this is the only element.
    let mut text_elem = SvgElement::new("text", &text_attrs);
    // line spacing (in 'em').
    let line_spacing = strp(&orig_elem.pop_attr("text-lsp").unwrap_or("1.05".to_owned()))?;
    // Whether text is pre-formatted (i.e. spaces are not collapsed)
    let text_pre = orig_elem.pop_attr("text-pre").is_some();

    // Copy style and class(es) from original element
    if let Some(style) = orig_elem.get_attr("style") {
        text_elem.set_attr("style", &style);
    }
    text_elem.classes = orig_elem.classes.clone();
    for class in text_classes {
        text_elem.add_class(class);
    }
    if !multiline {
        text_elem.content = ContentType::Ready(text_value.clone());
    }
    text_elements.push(text_elem);
    if multiline {
        let is_line = element.name == "line";
        let first_line_offset = match (is_line, text_loc) {
            // shapes - text 'inside'
            (false, LocSpec::TopLeft | LocSpec::Top | LocSpec::TopRight) => WRAP_DOWN,
            (false, LocSpec::BottomLeft | LocSpec::Bottom | LocSpec::BottomRight) => WRAP_UP,
            // lines - text 'beyond'
            (true, LocSpec::TopLeft | LocSpec::Top | LocSpec::TopRight) => WRAP_UP,
            (true, LocSpec::BottomLeft | LocSpec::Bottom | LocSpec::BottomRight) => WRAP_DOWN,
            (_, _) => WRAP_MID,
        };

        let mut tspan_elem = SvgElement::new("tspan", &text_attrs);
        tspan_elem.attrs.pop("y");
        for (idx, text_fragment) in lines.into_iter().enumerate() {
            let mut text_fragment = text_fragment.to_string();
            let mut tspan = tspan_elem.clone();
            let line_offset = if idx == 0 {
                first_line_offset(line_count, line_spacing)
            } else {
                line_spacing
            };

            if text_pre {
                // Replace spaces with non-breaking spaces so they aren't collapsed
                // by XML processing. This allows pre-formatted multi-line text (e.g. for
                // code listings)
                text_fragment = text_fragment.replace(' ', NBSP);
            }

            tspan.attrs.insert("dy", format!("{}em", fstr(line_offset)));
            tspan.content = ContentType::Ready(if text_fragment.is_empty() {
                // Empty tspans don't take up vertical space, so use a zero-width space.
                // Without this "a\n\nb" would render three tspans, but it would appear
                // to have 'b' immediately below 'a' without a blank line between them.
                ZWSP.to_string()
            } else {
                text_fragment.to_string()
            });
            text_elements.push(tspan);
        }
    }
    Ok((orig_elem, text_elements))
}
