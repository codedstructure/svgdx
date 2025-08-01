use super::SvgElement;
use crate::geometry::LocSpec;
use crate::types::{attr_split_cycle, fstr, strp};

use crate::errors::{Result, SvgdxError};

fn get_text_value(element: &mut SvgElement) -> String {
    let text_value = element
        .pop_attr("text")
        .expect("no text attr in process_text_attr");
    text_string(&text_value)
}

fn get_md_value(element: &mut SvgElement) -> (Vec<String>, Vec<u32>) {
    let text_value = element
        .pop_attr("md")
        .expect("no md attr in process_text_attr");

    let (parsed_string, sections) = md_parse(&text_value);

    let mut state_per_char = vec![0; parsed_string.len()];

    for i in 0..sections.len() {
        let bit = sections[i].code_bold_italic;
        for j in sections[i].start_ind..sections[i].end_ind {
            state_per_char[j] |= 1 << bit;
        }
    }

    let mut strings = vec![];
    let mut states = vec![];
    for i in 0..parsed_string.len() {
        if i == 0 || states[states.len() - 1] != state_per_char[i] {
            strings.push(String::new());
            states.push(state_per_char[i])
        }
        strings
            .last_mut()
            .expect("filled from i == 0")
            .push(parsed_string[i]);
    }

    return (strings, states);
}

#[derive(Debug)]
struct SectionData {
    start_ind: usize,
    end_ind: usize,
    code_bold_italic: u32,
}

// based on the commonmarkdown implementation
#[derive(Debug)]
struct DelimiterData {
    ind: usize, // goes just before this char
    char_type: char,
    num_delimiters: u32,
    is_active: bool,
    could_open: bool,
    could_close: bool,
}

fn md_parse(text_value: &str) -> (Vec<char>, Vec<SectionData>) {
    let mut sections = vec![];
    let mut result = vec![];
    let mut delimiters = vec![DelimiterData {
        ind: 0,
        char_type: ' ',
        num_delimiters: 0,
        is_active: false,
        could_open: false,
        could_close: false,
    }];
    let mut escaped = false;

    // first pass process \ and find delimiters
    for c in text_value.chars() {
        let mut add = true;
        if c == '\\' {
            if !escaped {
                add = false;
                escaped = true;
            } else {
                escaped = false;
            }
        }
        // the delimiters
        else if c == '`' || c == '_' || c == '*' {
            if !escaped {
                let last = delimiters.last_mut().expect("garenteed not to be empty");
                if c == last.char_type && last.ind == result.len() {
                    // is a continuation
                    last.num_delimiters += 1;
                } else {
                    delimiters.push(DelimiterData {
                        ind: result.len(),
                        char_type: c,
                        num_delimiters: 1,
                        is_active: true,
                        could_open: true,
                        could_close: true,
                    });
                }
                add = false;
            } else {
                escaped = true;
            }
        } else if escaped {
            if c == 'n' {
                add = false;
                result.push('\n');
            } else {
                // was not an escape
                result.push('\\');
            }
            escaped = false;
        }

        if add {
            result.push(c);
        }
    }

    // set could open/close
    for i in 0..delimiters.len() {
        let prev_char;
        let next_char;
        if i != 0 && delimiters[i - 1].ind == delimiters[i].ind {
            prev_char = delimiters[i - 1].char_type;
        } else if delimiters[i].ind == 0 {
            prev_char = ' ';
        } else {
            prev_char = result[delimiters[i].ind - 1];
        }

        if i != delimiters.len() - 1 && delimiters[i + 1].ind == delimiters[i].ind {
            next_char = delimiters[i + 1].char_type;
        } else if delimiters[i].ind == result.len() {
            next_char = ' ';
        } else {
            next_char = result[delimiters[i].ind];
        }

        if next_char.is_whitespace() {
            delimiters[i].could_open = false;
        }
        if prev_char.is_whitespace() {
            delimiters[i].could_close = false;
        }
        if !next_char.is_whitespace()
            && !prev_char.is_whitespace()
            && delimiters[i].char_type == '_'
        {
            delimiters[i].could_open = false;
            delimiters[i].could_close = false;
        }

        if next_char.is_ascii_punctuation()
            && (!prev_char.is_whitespace() || !prev_char.is_ascii_punctuation())
        {
            delimiters[i].could_open = false;
        }
        if prev_char.is_ascii_punctuation()
            && (!next_char.is_whitespace() || !next_char.is_ascii_punctuation())
        {
            delimiters[i].could_close = false;
        }
    }

    let stack_bottom = 0; // because I have a null element in it
    let mut current_position = stack_bottom + 1;
    let mut opener_a = [stack_bottom; 3];
    let mut opener_d = [stack_bottom; 3];
    let mut opener_t = [stack_bottom; 3];

    loop {
        while current_position != delimiters.len()
            && !delimiters[current_position].could_close
            && delimiters[current_position].is_active
        {
            current_position += 1;
        }
        if current_position == delimiters.len() {
            break;
        }
        let opener_min = match delimiters[current_position].char_type {
            '*' => &mut opener_a,
            '_' => &mut opener_d,
            '`' => &mut opener_t,
            _ => panic!(),
        };
        println!("{} {:?}", current_position, delimiters);

        let min = opener_min[(delimiters[current_position].num_delimiters % 3) as usize]
            .max(stack_bottom);
        let mut opener_ind = current_position - 1;
        while opener_ind > min {
            // found opener
            if delimiters[opener_ind].is_active
                && delimiters[opener_ind].could_open
                && delimiters[opener_ind].char_type == delimiters[current_position].char_type
            {
                if (delimiters[opener_ind].could_close || delimiters[current_position].could_open)
                    && delimiters[opener_ind].num_delimiters % 3
                        != delimiters[current_position].num_delimiters % 3
                {
                } else {
                    break;
                }
            }
            opener_ind -= 1;
        }

        if opener_ind == min {
            // not found a opener
            opener_min[(delimiters[current_position].num_delimiters % 3) as usize] =
                current_position - 1;
            current_position += 1;
        } else {
            delimiters[current_position].could_open = false;
            delimiters[opener_ind].could_close = false;
            // did
            let code = delimiters[current_position].char_type == '`';
            let strong = !code
                && delimiters[opener_ind].num_delimiters >= 2
                && delimiters[current_position].num_delimiters >= 2;
            sections.push(SectionData {
                start_ind: delimiters[opener_ind].ind,
                end_ind: delimiters[current_position].ind,
                code_bold_italic: if code {
                    0
                } else if strong {
                    1
                } else {
                    2
                },
            });

            println!("{} {} {}", opener_ind, current_position, strong);
            delimiters[opener_ind].num_delimiters -= 1 + (strong as u32);
            delimiters[current_position].num_delimiters -= 1 + (strong as u32);

            if delimiters[opener_ind].num_delimiters == 0 {
                delimiters[opener_ind].is_active = false;
            }
            if delimiters[current_position].num_delimiters == 0 {
                delimiters[current_position].is_active = false;
                current_position += 1;
            }

            for i in (opener_ind + 1)..current_position {
                delimiters[i].is_active = false;
            }
        }
    }
    println!();

    let mut final_result = vec![];

    // work from the back to avoid index invalidation
    for i in (0..delimiters.len()).rev() {
        while delimiters[i].ind < result.len() {
            if let Some(thing) = result.pop() {
                final_result.push(thing);
            }
        }

        for j in 0..sections.len() {
            // if start needs to be after or equal
            if sections[j].start_ind >= delimiters[i].ind {
                sections[j].start_ind += delimiters[i].num_delimiters as usize;
            }
            if sections[j].end_ind > delimiters[i].ind {
                // if end needs to be after
                sections[j].end_ind += delimiters[i].num_delimiters as usize;
            }
        }
        for _ in 0..delimiters[i].num_delimiters {
            final_result.push(delimiters[i].char_type);
        }
    }

    return (final_result.into_iter().rev().collect(), sections);
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

fn get_text_position(element: &mut SvgElement) -> Result<(f32, f32, bool, LocSpec, Vec<String>)> {
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
        matches!(element.name(), "line" | "point" | "text")
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
    Ok((tdx, tdy, outside, text_anchor, text_classes))
}

fn get_text_len(mono: bool, text: String) -> f32 {
    if mono {
        return 0.6 * text.len() as f32;
    }
    let mut length = 0.0;

    let long = ['m', 'w'];
    let short = ['f', 'i', 'j', 'l', 'r', 't'];
    for i in text.chars() {
        if long.contains(&i) {
            length += 0.8;
        } else if short.contains(&i) {
            length += 0.33;
        } else {
            length += 0.6;
        }
    }

    return length;
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

    let text_values;
    let state_values;
    if let (Some(_), Some(_)) = (orig_elem.get_attr("text"), orig_elem.get_attr("md")) {
        return Err(SvgdxError::ParseError(
            "has both attributes of text and md".to_owned(),
        ));
    } else if let Some(_) = orig_elem.get_attr("text") {
        text_values = vec![get_text_value(&mut orig_elem)];
        state_values = vec![0];
    } else {
        // as to call must have one of them
        (text_values, state_values) = get_md_value(&mut orig_elem);
    }
    let mut full_text_parsed_string = "".to_string();
    for t in text_values.iter() {
        full_text_parsed_string.push_str(&t);
    }

    let (tdx, tdy, outside, text_loc, mut text_classes) = get_text_position(&mut orig_elem)?;

    let x_str = fstr(tdx);
    let y_str = fstr(tdy);
    let mut text_elements = Vec::new();
    let mut lines = vec![vec![]];
    let mut line_types = vec![vec![]];
    for i in 0..text_values.len() {
        let mut segments = text_values[i].lines();

        if let Some(first) = segments.next() {
            if first != "" {
                lines
                    .last_mut()
                    .expect("added item not removed")
                    .push(first);
                line_types
                    .last_mut()
                    .expect("added item not removed")
                    .push(state_values[i]);
            } else if i != 0 {
                lines.push(vec![]);
                line_types.push(vec![]);
            }
        }

        for s in segments {
            lines.push(vec![s]);
            line_types.push(vec![state_values[i]]);
        }

        if let Some(last_char) = text_values[i].chars().last() {
            if last_char == '\n' && i != text_values.len() - 1 {
                lines.push(vec![]);
                line_types.push(vec![]);
            }
        }
    }

    for i in 0..lines.len() {
        if lines[i].len() == 0 {
            lines[i].push("");
            line_types[i].push(0);
        }
    }
    let line_count = lines.len();
    println!("{:?}", text_values[0].lines().collect::<Vec<_>>());
    println!("{:?}", text_values);
    println!("{:?}", lines);

    let multielement = line_count > 1 || text_values.len() > 1;
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
            //let mut dist_along = 0.0;
            for (idn, text_fragment) in line.into_iter().enumerate() {
                let mut text_fragment = text_fragment.to_string();
                let mut tspan = tspan_elem.clone();
                if idn == 0 {
                    if vertical {
                        tspan.set_attr("y", &y_str);
                    } else {
                        tspan.set_attr("x", &x_str);
                    }
                }

                if line_types[idx][idn] & (1 << 0) != 0 {
                    tspan.add_class("d-text-monospace");
                }
                if line_types[idx][idn] & (1 << 1) != 0 {
                    tspan.add_class("d-text-bold");
                }
                if line_types[idx][idn] & (1 << 2) != 0 {
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

                tspan.set_attr(
                    if vertical { "dx" } else { "dy" },
                    &format!("{}em", fstr(line_offset)),
                );

                //tspan.set_attr(
                //    if vertical { "dy" } else { "dx" },
                //    &format!("{}em", fstr(dist_along)),
                //);
                tspan.text_content = Some(if text_fragment.is_empty() {
                    // Empty tspans don't take up vertical space, so use a zero-width space.
                    // Without this "a\n\nb" would render three tspans, but it would appear
                    // to have 'b' immediately below 'a' without a blank line between them.
                    ZWSP.to_string()
                } else {
                    text_fragment.to_string()
                });
                text_elements.push(tspan);

                //dist_along += get_text_len(line_types[idx][idn] & (1 << 0) != 0, text_fragment);
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

    #[test]
    fn test_md_parse() {
        // the basic examples no actual md

        let text = r"Hello, \nworld!";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['H', 'e', 'l', 'l', 'o', ',', ' ', '\\n', 'w', 'o', 'r', 'l', 'd', '!'], [])"
        );

        // when not part of a '\n', '\' is not special
        let text = r"Hello, world! \1";
        assert_eq!(format!("{:?}",md_parse(text)), "(['H', 'e', 'l', 'l', 'o', ',', ' ', 'w', 'o', 'r', 'l', 'd', '!', ' ', '\\\\', '1'], [])");

        // when precedes '\n', '\' escapes it.
        let text = r"Hello, \\nworld!";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['H', 'e', 'l', 'l', 'o', ',', ' ', '\\\\', 'n', 'w', 'o', 'r', 'l', 'd', '!'], [])"
        );

        fn sd(s: i32, e: i32, i: i32) -> String {
            "SectionData { start_ind: ".to_owned()
                + &s.to_string()
                + ", end_ind: "
                + &e.to_string()
                + ", code_bold_italic: "
                + &i.to_string()
                + " }"
        }

        // using the md
        let text = r"He*ll*o, \nworld!";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['H', 'e', 'l', 'l', 'o', ',', ' ', '\\n', 'w', 'o', 'r', 'l', 'd', '!'], ["
                .to_owned()
                + &sd(2, 4, 2)
                + "])"
        );

        // mismatched
        let text = r"*Hello** , \nworld!";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['H', 'e', 'l', 'l', 'o', '*', ' ', ',', ' ', '\\n', 'w', 'o', 'r', 'l', 'd', '!'], ["
                .to_owned()
                + &sd(0, 5, 2) + "])"
        );

        // diff type
        let text = r"He*llo_, \nworld!";
        assert_eq!(format!("{:?}",md_parse(text)), "(['H', 'e', '*', 'l', 'l', 'o', '_', ',', ' ', '\\n', 'w', 'o', 'r', 'l', 'd', '!'], [])");

        // multiple diff type
        let text = r"_hello*";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['_', 'h', 'e', 'l', 'l', 'o', '*'], [])"
        );

        // multiple same type
        let text = r"He*ll*o, \nw*or*ld!";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['H', 'e', 'l', 'l', 'o', ',', ' ', '\\n', 'w', 'o', 'r', 'l', 'd', '!'], ["
                .to_owned()
                + &sd(2, 4, 2)
                + ", "
                + &sd(9, 11, 2)
                + "])"
        );

        // space before
        let text = r"**foo bar **";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['*', '*', 'f', 'o', 'o', ' ', 'b', 'a', 'r', ' ', '*', '*'], [])"
        );

        // punctuation before alphnum after
        let text = r"**(**foo)";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['*', '*', '(', '*', '*', 'f', 'o', 'o', ')'], [])"
        );
    }

    #[test]
    fn test_get_md_value() {
        let mut el = SvgElement::new("text", &[]);
        let text = r"foo";
        el.set_attr("md", text);
        assert_eq!(format!("{:?}", get_md_value(&mut el)), "([\"foo\"], [0])");

        let text = r"**(**foo)";
        el.set_attr("md", text);
        assert_eq!(
            format!("{:?}", get_md_value(&mut el)),
            "([\"**(**foo)\"], [0])"
        );

        let text = r"*foo *bar**";
        el.set_attr("md", text);
        assert_eq!(
            format!("{:?}", get_md_value(&mut el)),
            "([\"foo bar\"], [4])"
        );

        let text = r"*foo**bar**baz*";
        el.set_attr("md", text);
        assert_eq!(
            format!("{:?}", get_md_value(&mut el)),
            "([\"foo\", \"bar\", \"baz\"], [4, 6, 4])"
        );
    }
}
