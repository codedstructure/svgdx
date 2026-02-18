use itertools::Itertools;

use super::SvgElement;
use crate::context::ElementMap;
use crate::geometry::LocSpec;
use crate::types::{attr_split_cycle, fstr, strp, ElRef};

use crate::elements::markdown::{get_md_value, MdSpan};
use crate::errors::{Error, Result};

fn get_text_value(element: &mut SvgElement) -> String {
    let text_value = element
        .pop_attr("text")
        .expect("no text attr in process_text_attr");
    text_string(&text_value)
}

/// Convert unescaped r"\n" into newline characters for multi-line text
fn text_string(text_value: &str) -> String {
    let mut result = String::new();
    let mut remain = text_value;
    while !remain.is_empty() {
        if let Some(idx) = remain.find("\\n") {
            let (start, new_remain) = remain.split_at(idx);
            remain = &new_remain[2..]; // Skip the two chars '\', 'n'
            if idx > 0 && start.ends_with('\\') {
                // Escaped newline; re-apply the backslash and continue
                result.push_str(&start[..idx - 1]);
                result.push_str("\\n");
            } else {
                result.push_str(start);
                result.push('\n');
            }
        } else {
            // No more newlines
            result.push_str(remain);
            break;
        }
    }
    result
}

fn get_text_position(
    element: &mut SvgElement,
    ctx: &impl ElementMap,
) -> Result<(f32, f32, bool, LocSpec, Vec<String>)> {
    let mut t_dx = 0.;
    let mut t_dy = 0.;
    {
        let dx = element.pop_attr("text-dx");
        let dy = element.pop_attr("text-dy");
        let dxy = element.pop_attr("text-dxy");
        if let Some(dxy) = dxy {
            let mut parts = attr_split_cycle(&dxy);
            if let Some(pdx) = parts.next() {
                t_dx = strp(&pdx)?;
            }
            if let Some(pdy) = parts.next() {
                t_dy = strp(&pdy)?;
            }
        }
        if let Some(dx) = dx {
            t_dx = strp(&dx)?;
        }
        if let Some(dy) = dy {
            t_dy = strp(&dy)?;
        }
    }

    // If a 'rel' attribute is present on a <text> element, resolve it to
    // determine the bounding box and element type (for inside/outside default).
    let mut text_ref_element = element.clone();
    if element.name() == "text" {
        if let Some(ref_str) = element.pop_attr("rel") {
            let elref: ElRef = ref_str.parse()?;
            text_ref_element = ctx
                .get_element(&elref)
                .ok_or_else(|| Error::Reference(elref))?
                .clone();
        }
    }

    let mut text_classes = vec!["d-text".to_owned()];
    let text_loc_str = element.pop_attr("text-loc").unwrap_or("c".into());
    let text_anchor = text_loc_str.parse::<LocSpec>()?;

    // Default dx/dy to push it in slightly from the edge (or out for lines);
    // Without offset text squishes to the edge and can be unreadable
    // Any specified dx/dy override this behaviour.
    let text_offset = strp(&element.pop_attr("text-offset").unwrap_or("1".to_string()))?;

    let vertical = element.has_class("d-text-vertical");
    // text associated with a line, point or text element is pushed 'outside';
    // for other shapes it's pulled 'inside'. This can be overridden with
    // the 'd-text-inside' and 'd-text-outside' classes. Anchor classes and
    // text_offset direction are affected by the value of 'outside'.
    let outside = if element.pop_class("d-text-outside") {
        true
    } else if element.pop_class("d-text-inside") {
        false
    } else {
        matches!(text_ref_element.name(), "line" | "point" | "text")
    };
    match text_anchor {
        ls if ls.is_top() => {
            text_classes.push(
                match (outside, vertical) {
                    (false, false) => "d-text-top",
                    (true, false) => "d-text-bottom",
                    (false, true) => "d-text-top-vertical",
                    (true, true) => "d-text-bottom-vertical",
                }
                .to_owned(),
            );
            t_dy += if outside { -text_offset } else { text_offset };
        }
        ls if ls.is_bottom() => {
            text_classes.push(
                match (outside, vertical) {
                    (false, false) => "d-text-bottom",
                    (true, false) => "d-text-top",
                    (false, true) => "d-text-bottom-vertical",
                    (true, true) => "d-text-top-vertical",
                }
                .to_owned(),
            );
            t_dy += if outside { text_offset } else { -text_offset };
        }
        _ => (),
    }

    match text_anchor {
        ls if ls.is_left() => {
            text_classes.push(
                match (outside, vertical) {
                    (false, false) => "d-text-left",
                    (true, false) => "d-text-right",
                    (false, true) => "d-text-left-vertical",
                    (true, true) => "d-text-right-vertical",
                }
                .to_owned(),
            );
            t_dx += if outside { -text_offset } else { text_offset };
        }
        ls if ls.is_right() => {
            text_classes.push(
                match (outside, vertical) {
                    (false, false) => "d-text-right",
                    (true, false) => "d-text-left",
                    (false, true) => "d-text-right-vertical",
                    (true, true) => "d-text-left-vertical",
                }
                .to_owned(),
            );
            t_dx += if outside { text_offset } else { -text_offset };
        }
        _ => (),
    }

    // Assumption is that text should be centered within the rect,
    // and has styling via CSS to reflect this, e.g.:
    //  text.d-text { dominant-baseline: central; text-anchor: middle; }
    let (mut tdx, mut tdy) = text_ref_element
        .bbox()?
        .ok_or_else(|| Error::MissingBBox(element.to_string()))?
        .locspec(text_anchor);

    tdx += t_dx;
    tdy += t_dy;
    Ok((tdx, tdy, outside, text_anchor, text_classes))
}

pub fn process_text_attr(
    element: &SvgElement,
    ctx: &impl ElementMap,
) -> Result<(SvgElement, Vec<SvgElement>)> {
    // Different conversions from line count to first-line offset based on whether
    // top, center, or bottom justification.
    const WRAP_DOWN: fn(usize, f32) -> f32 = |_count, _spacing| 0.;
    const WRAP_UP: fn(usize, f32) -> f32 = |count, spacing| -(count as f32 - 1.) * spacing;
    const WRAP_MID: fn(usize, f32) -> f32 = |count, spacing| -(count as f32 - 1.) / 2. * spacing;

    const ZWSP: &str = "\u{200B}"; // Zero-width space
    const NBSP: &str = "\u{00A0}"; // Non-breaking space

    let mut orig_elem = element.clone();

    let spans;
    if let (Some(_), Some(_)) = (orig_elem.get_attr("text"), orig_elem.get_attr("md")) {
        return Err(Error::Parse(
            "has both attributes of text and md".to_owned(),
        ));
    } else if orig_elem.get_attr("text").is_some() {
        if orig_elem.has_class("d-markdown") {
            // as to call must have one of them
            spans = get_md_value(&mut orig_elem);
        } else {
            spans = vec![MdSpan {
                code: false,
                bold: false,
                italic: false,
                text: get_text_value(&mut orig_elem),
            }];
        }
    } else {
        // as to call must have one of them
        spans = get_md_value(&mut orig_elem);
    }
    let full_text_parsed_string = spans.iter().map(|s| s.text.clone()).join("");

    let (tdx, tdy, outside, text_loc, mut text_classes) = get_text_position(&mut orig_elem, ctx)?;

    let x_str = fstr(tdx);
    let y_str = fstr(tdy);
    let mut text_elements = Vec::new();
    // lines is a vec of (line)s
    // a line is a vec of spans
    // it starts with a single empty line
    let mut lines = vec![vec![]];
    for span in spans.iter() {
        let mut segments = span.text.lines();
        if let Some(first) = segments.next() {
            if !first.is_empty() {
                lines
                    .last_mut()
                    .expect("added item not removed")
                    .push(MdSpan {
                        code: span.code,
                        bold: span.bold,
                        italic: span.italic,
                        text: first.to_string(),
                    });
            }
        }

        for s in segments {
            lines.push(vec![MdSpan {
                code: span.code,
                bold: span.bold,
                italic: span.italic,
                text: s.to_string(),
            }]);
        }

        if let Some(last_char) = span.text.chars().last() {
            if last_char == '\n' {
                lines.push(vec![]);
            }
        }
    }
    // if last char is newline dont do the new line
    if let Some(last_span) = spans.last() {
        if let Some(last_char) = last_span.text.chars().last() {
            if last_char == '\n' {
                lines.pop();
            }
        }
    }

    // fill empty lines with empty strings
    for l in &mut lines {
        if l.is_empty() {
            l.push(MdSpan {
                code: false,
                bold: false,
                italic: false,
                text: String::new(),
            });
        }
    }
    let line_count = lines.len();

    let multielement = line_count > 1 || spans.len() > 1;
    let vertical = orig_elem.has_class("d-text-vertical");
    // Whether text is pre-formatted (i.e. spaces are not collapsed)
    let text_pre = orig_elem.has_class("d-text-pre");

    // There will always be a text element; if not multielement this is the only element.
    let mut text_elem = if orig_elem.name() == "text" {
        orig_elem.clone()
    } else {
        SvgElement::new("text", &[])
    };
    text_elem.set_attr("x", &x_str);
    text_elem.set_attr("y", &y_str);

    // handle text-rotate; note this is ignored by positioning and alignment
    // logic and generally assumes central text anchoring...
    if let Some(rotate) = orig_elem.pop_attr("text-rotate") {
        // move to text element, then process as if it were there to begin with
        text_elem.set_attr("rotate", &rotate);
        text_elem.handle_rotation()?;
    }

    // line spacing (in 'em').
    let line_spacing = strp(&orig_elem.pop_attr("text-lsp").unwrap_or("1.05".to_owned()))?;
    // Extract style and class(es) from original element. Note we use
    // `text-style` for styling text rather than copying `style` to both outer
    // element and generated text, as is likely there will be conflicts with
    // the original element's desired style (e.g. setting `style="fill:red"`
    // on a rect with `text` present would cause red-on-red invisible text).
    let text_style = orig_elem.pop_attr("text-style");
    if let Some(ref style) = text_style {
        text_elem.set_style_from(style);
    }

    // The following should *not* be inherited by the text element.
    // Ideally we'd just have a list of classes to *include*, but this would
    // match all the d-<colour> classes which would be very extensive.
    //
    // In an SVG + CSS3 context, could just use `[selector]:not(text,tspan)`
    // instead of this, but that doesn't work in e.g. Inkscape.
    let text_ignore_classes = [
        "d-softshadow",
        "d-hardshadow",
        "d-grid",
        "d-hatch",
        "d-crosshatch",
        "d-stipple",
        "d-surround",
        "d-flow",
        "d-dot",
        "d-dash",
        "d-dot-dash",
        "d-thinner",
        "d-thin",
        "d-thick",
        "d-thicker",
    ];
    let text_ignore_class_fns = [
        |c: &str| c.starts_with("d-flow-"),
        |c: &str| c.starts_with("d-grid-"),
        |c: &str| c.starts_with("d-crosshatch-"),
        |c: &str| c.starts_with("d-hatch-"),
        |c: &str| c.starts_with("d-stipple-"),
    ];
    // Split classes into text-related and non-text-related and
    // assign to appropriate elements.
    for class in orig_elem.get_classes() {
        if class.starts_with("d-text-") {
            orig_elem.pop_class(&class);
        }
        if !text_ignore_classes.contains(&class.as_str())
            && !text_ignore_class_fns.iter().any(|f| f(&class))
        {
            text_classes.push(class);
        }
    }

    if !multielement {
        if lines[0][0].code {
            text_classes.push("d-text-monospace".to_string());
        }
        if lines[0][0].bold {
            text_classes.push("d-text-bold".to_string());
        }
        if lines[0][0].italic {
            text_classes.push("d-text-italic".to_string());
        }
    }
    text_elem.src_line = orig_elem.src_line;
    text_elem.set_classes(&text_classes);

    // Add this prior to copying over presentation attrs which take precedence
    if vertical {
        text_elem.set_attr("writing-mode", "tb");
    }
    // Move text-related presentation attributes from original element to text element
    let text_presentation_attrs = [
        "alignment-baseline",
        "font-family",
        "font-size",
        "font-size-adjust",
        "font-stretch",
        "font-style",
        "font-variant",
        "font-weight",
        "text-decoration",
        "text-rendering",
        "text-anchor",
        "textLength",
        "lengthAdjust",
        "word-spacing",
        "letter-spacing",
        "writing-mode",
        "unicode-bidi",
    ];
    for text_attr in text_presentation_attrs.iter() {
        if let Some(attr) = orig_elem.pop_attr(text_attr) {
            text_elem.set_attr(text_attr, &attr);
        }
    }
    text_elem.text_content = Some(full_text_parsed_string.clone());
    text_elements.push(text_elem);
    if multielement {
        // Determine position of first text line; others follow this based on line spacing
        let first_line_offset = match (outside, vertical, text_loc) {
            // shapes - text 'inside'
            (false, false, ls) if ls.is_top() => WRAP_DOWN,
            (false, false, ls) if ls.is_bottom() => WRAP_UP,
            (false, true, ls) if ls.is_left() => WRAP_DOWN,
            (false, true, ls) if ls.is_right() => WRAP_UP,
            // lines - text 'beyond'
            (true, false, ls) if ls.is_top() => WRAP_UP,
            (true, false, ls) if ls.is_bottom() => WRAP_DOWN,
            (true, true, ls) if ls.is_left() => WRAP_UP,
            (true, true, ls) if ls.is_right() => WRAP_DOWN,
            (_, _, _) => WRAP_MID,
        };

        let mut tspan_elem = SvgElement::new("tspan", &[]);
        if let Some(ref style) = text_style {
            tspan_elem.set_style_from(style);
        }
        tspan_elem.src_line = orig_elem.src_line;
        if vertical {
            lines = lines.into_iter().rev().collect();
        }

        for (idx, line) in lines.into_iter().enumerate() {
            for (idn, md_span) in line.into_iter().enumerate() {
                let mut text_fragment = md_span.text;
                let mut tspan = tspan_elem.clone();
                if idn == 0 {
                    if vertical {
                        tspan.set_attr("y", &y_str);
                    } else {
                        tspan.set_attr("x", &x_str);
                    }
                }

                if md_span.code {
                    tspan.add_class("d-text-monospace");
                }
                if md_span.bold {
                    tspan.add_class("d-text-bold");
                }
                if md_span.italic {
                    tspan.add_class("d-text-italic");
                }

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

                if idn == 0 {
                    tspan.set_attr(
                        if vertical { "dx" } else { "dy" },
                        &format!("{}em", fstr(line_offset)),
                    );
                }

                tspan.text_content = Some(if text_fragment.is_empty() {
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
    }
    Ok((orig_elem, text_elements))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_string() {
        let text = r"Hello, \nworld!";
        assert_eq!(text_string(text), "Hello, \nworld!");

        // when not part of a '\n', '\' is not special
        let text = r"Hello, world! \1";
        assert_eq!(text_string(text), "Hello, world! \\1");

        // when precedes '\n', '\' escapes it.
        let text = r"Hello, \\nworld!";
        assert_eq!(text_string(text), r"Hello, \nworld!");
    }
}
