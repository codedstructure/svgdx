use crate::element::SvgElement;
use crate::position::LocSpec;
use crate::types::{attr_split_cycle, fstr, strp};

use crate::errors::{Result, SvgdxError};

pub fn process_text_attr(element: &SvgElement) -> Result<(SvgElement, Vec<SvgElement>)> {
    let mut orig_elem = element.clone();
    let ts = build_text_spec(&mut orig_elem)?;
    let mut text_elements = Vec::new();
    for t in ts {
        text_elements.extend(t.text_spec_elements()?);
    }
    Ok((orig_elem, text_elements))
}

struct TextPos {
    x: f32,
    y: f32,
    outside: bool,
    vertical: bool,
    text_loc: LocSpec,
}

enum WrapMode {
    Down,
    Up,
    Mid,
}

impl TextPos {
    fn wrap_mode(&self) -> WrapMode {
        match (self.outside, self.vertical, self.text_loc) {
            // shapes - text 'inside'
            (false, false, loc) if loc.is_top() => WrapMode::Down,
            (false, false, loc) if loc.is_bottom() => WrapMode::Up,
            (false, true, loc) if loc.is_left() => WrapMode::Down,
            (false, true, loc) if loc.is_right() => WrapMode::Up,
            // lines - text 'beyond'
            (true, false, loc) if loc.is_top() => WrapMode::Up,
            (true, false, loc) if loc.is_bottom() => WrapMode::Down,
            (true, true, loc) if loc.is_left() => WrapMode::Up,
            (true, true, loc) if loc.is_right() => WrapMode::Down,
            (_, _, _) => WrapMode::Mid,
        }
    }
}

fn get_text_position(element: &mut SvgElement) -> Result<(TextPos, Vec<String>)> {
    let mut t_dx = 0.;
    let mut t_dy = 0.;
    {
        let dx = element.pop_attr("text-dx");
        let dy = element.pop_attr("text-dy");
        let dxy = element.pop_attr("text-dxy");
        if let Some(dxy) = dxy {
            let mut parts = attr_split_cycle(&dxy).map_while(|v| strp(&v).ok());
            t_dx = parts.next().ok_or_else(|| {
                SvgdxError::ParseError("dx from text-dxy should be numeric".to_owned())
            })?;
            t_dy = parts.next().ok_or_else(|| {
                SvgdxError::ParseError("dy from text-dxy should be numeric".to_owned())
            })?;
        }
        if let Some(dx) = dx {
            t_dx = strp(&dx)?;
        }
        if let Some(dy) = dy {
            t_dy = strp(&dy)?;
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
        matches!(element.name.as_str(), "line" | "point" | "text")
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
    let (mut tdx, mut tdy) = element
        .bbox()?
        .ok_or_else(|| SvgdxError::MissingBoundingBox(element.to_string()))?
        .locspec(text_anchor);
    tdx += t_dx;
    tdy += t_dy;

    Ok((
        TextPos {
            x: tdx,
            y: tdy,
            outside,
            vertical,
            text_loc: text_anchor,
        },
        text_classes,
    ))
}

struct TextSpec {
    text: String,
    position: TextPos,
    text_style: Option<String>,
    text_classes: Vec<String>,
    text_attrs: Vec<(String, String)>,
    text_pre: bool,
    line_spacing: f32,
    prototype: SvgElement,
}

fn build_text_spec(element: &mut SvgElement) -> Result<Vec<TextSpec>> {
    let text = text_string(element.pop_attr("text").unwrap_or("".to_owned()).as_str());
    let (position, mut text_classes) = get_text_position(element)?;

    // Extract style and class(es) from original element. Note we use
    // `text-style` for styling text rather than copying `style` to both outer
    // element and generated text, as is likely there will be conflicts with
    // the original element's desired style (e.g. setting `style="fill:red"`
    // on a rect with `text` present would cause red-on-red invisible text).
    let text_style = element.pop_attr("text-style");
    // Whether text is pre-formatted (i.e. spaces are not collapsed)
    let text_pre = element.has_class("d-text-pre");
    // line spacing (in 'em').
    let line_spacing = strp(&element.pop_attr("text-lsp").unwrap_or("1.05".to_owned()))?;
    let mut text_attrs = Vec::new();
    // extract classes beginning with 'd-text-' from the original element
    // and add them to the text element.
    text_classes.extend(extract_text_classes(element));
    // Add this prior to copying over presentation attrs which take precedence
    if position.vertical {
        text_attrs.push(("writing-mode".into(), "tb".into()));
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
    for text_attr in text_presentation_attrs.into_iter() {
        if let Some(attr) = element.pop_attr(text_attr) {
            text_attrs.push((text_attr.to_owned(), attr));
        }
    }
    // TODO: support multiple text fragments from a single source element
    Ok(vec![TextSpec {
        text,
        position,
        text_style,
        text_classes,
        text_attrs: text_attrs
            .into_iter()
            .map(|pair| (pair.0.to_owned(), pair.1.to_owned()))
            .collect(),
        text_pre,
        line_spacing,
        prototype: element.clone(),
    }])
}

fn extract_text_classes(element: &mut SvgElement) -> Vec<String> {
    let mut text_classes = Vec::new();
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
    for class in element.classes.clone().into_iter() {
        if class.starts_with("d-text-") {
            element.pop_class(&class);
        }
        if !text_ignore_classes.contains(&class.as_str())
            && !text_ignore_class_fns.iter().any(|f| f(&class))
        {
            text_classes.push(class);
        }
    }
    text_classes
}

impl TextSpec {
    pub fn text_spec_elements(&self) -> Result<Vec<SvgElement>> {
        let mut text_elements = Vec::new();
        let mut lines: Vec<_> = self.text.lines().collect();
        let line_count = lines.len();
        let multiline = line_count > 1;

        // There will always be a text element; if not multiline this is the only element.
        let mut text_elem = if self.prototype.name == "text" {
            self.prototype.clone()
        } else {
            SvgElement::new("text", &[])
        };

        text_elem.src_line = self.prototype.src_line;
        text_elem.classes = self.text_classes.clone().into();

        let x_str = fstr(self.position.x);
        let y_str = fstr(self.position.y);
        text_elem.set_attr("x", &x_str);
        text_elem.set_attr("y", &y_str);

        if let Some(ref style) = self.text_style {
            text_elem.set_attr("style", style);
        }
        for (attr, value) in self.text_attrs.iter() {
            text_elem.set_attr(attr, value);
        }

        const ZWSP: &str = "\u{200B}"; // Zero-width space
        const NBSP: &str = "\u{00A0}"; // Non-breaking space

        text_elem.text_content = Some(self.text.clone());
        text_elements.push(text_elem);
        if multiline {
            // Determine position of first text line; others follow this based on line spacing
            // Different conversions from line count to first-line offset based on whether
            // top, center, or bottom justification.
            let first_line_offset = match self.position.wrap_mode() {
                WrapMode::Down => 0.,
                WrapMode::Up => -(line_count as f32 - 1.) * self.line_spacing,
                WrapMode::Mid => -(line_count as f32 - 1.) / 2. * self.line_spacing,
            };

            let mut tspan_elem = SvgElement::new("tspan", &[]);
            tspan_elem.src_line = self.prototype.src_line;
            if let Some(ref style) = self.text_style {
                tspan_elem.set_attr("style", style);
            }
            if self.position.vertical {
                tspan_elem.set_attr("y", &y_str);
                lines = lines.into_iter().rev().collect();
            } else {
                tspan_elem.set_attr("x", &x_str);
            }

            for (idx, text_fragment) in lines.into_iter().enumerate() {
                let mut text_fragment = text_fragment.to_string();
                let mut tspan = tspan_elem.clone();
                let line_offset = if idx == 0 {
                    first_line_offset
                } else {
                    self.line_spacing
                };

                if self.text_pre {
                    // Replace spaces with non-breaking spaces so they aren't collapsed
                    // by XML processing. This allows pre-formatted multi-line text (e.g. for
                    // code listings)
                    text_fragment = text_fragment.replace(' ', NBSP);
                }

                tspan.attrs.insert(
                    if self.position.vertical { "dx" } else { "dy" },
                    format!("{}em", fstr(line_offset)),
                );
                tspan.text_content = Some(if text_fragment.is_empty() {
                    // Empty tspans don't take up vertical space, so use a zero-width space.
                    // Without this "a\n\nb" would render three tspans, but it would appear
                    // to have 'b' immediately below 'a' without a blank line between them.
                    ZWSP.to_string()
                } else {
                    text_fragment
                });
                text_elements.push(tspan);
            }
        }

        Ok(text_elements)
    }
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
