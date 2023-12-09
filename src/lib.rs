use core::fmt::Display;
use std::io::{Read, Write};

use regex::Regex;

use anyhow::{Context, Result};

mod types;
use types::{AttrMap, BoundingBox, ClassList};
mod transform;
use expression::eval_attr;
pub(crate) use transform::{Transformer, TransformerContext};
mod connector;
mod expression;

pub fn svg_transform(reader: &mut dyn Read, writer: &mut dyn Write) -> Result<()> {
    let mut t = Transformer::new();
    t.transform(reader, writer)
}

/// Return a 'minimal' representation of the given number
fn fstr(x: f32) -> String {
    if x == (x as u32) as f32 {
        return (x as u32).to_string();
    }
    let result = format!("{x:.3}");
    if result.contains('.') {
        result.trim_end_matches('0').trim_end_matches('.').into()
    } else {
        result
    }
}

/// Parse a string to an f32
fn strp(s: &str) -> Option<f32> {
    s.parse().ok()
}

/// Returns iterator over whitespace-or-comma separated values
fn attr_split(input: &str) -> impl Iterator<Item = String> + '_ {
    input
        .split_whitespace()
        .flat_map(|v| v.split(','))
        .map(|v| v.to_string())
}

/// Returns iterator *cycling* over whitespace-or-comma separated values
fn attr_split_cycle(input: &str) -> impl Iterator<Item = String> + '_ {
    let x: Vec<String> = attr_split(input).collect();
    x.into_iter().cycle()
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
enum Length {
    Absolute(f32),
    Ratio(f32),
}

impl Default for Length {
    fn default() -> Self {
        Self::Absolute(0.)
    }
}

impl Length {
    #[allow(dead_code)]
    const fn ratio(&self) -> Option<f32> {
        if let Self::Ratio(result) = self {
            Some(*result)
        } else {
            None
        }
    }

    const fn absolute(&self) -> Option<f32> {
        if let Self::Absolute(result) = self {
            Some(*result)
        } else {
            None
        }
    }

    /// Given a single value, update it (scale or addition) from
    /// the current Length value
    fn adjust(&self, value: f32) -> f32 {
        match self {
            Self::Absolute(abs) => value + abs,
            Self::Ratio(ratio) => value * ratio,
        }
    }

    /// Given a range, return a value (typically) in the range
    /// where a positive Absolute is 'from start', a negative Absolute
    /// is 'backwards from end' and Ratios scale as 0%=start, 100%=end
    /// but ratio values are not limited to 0..100 at either end.
    fn calc_offset(&self, start: f32, end: f32) -> f32 {
        match self {
            Self::Absolute(abs) => {
                let mult = if end < start { -1. } else { 1. };
                if abs < &0. {
                    // '+' here since abs is negative and
                    // we're going 'back' from the end.
                    end + abs * mult
                } else {
                    start + abs * mult
                }
            }
            Self::Ratio(ratio) => start + (end - start) * ratio,
        }
    }
}

/// Parse a ratio (float or %age) to an f32
/// Note this deliberately does not clamp to 0..1
fn strp_length(s: &str) -> Option<Length> {
    if let Some(s) = s.strip_suffix('%') {
        Some(Length::Ratio(strp(s)? * 0.01))
    } else {
        Some(Length::Absolute(strp(s)?))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SvgElement {
    name: String,
    attrs: AttrMap,
    classes: ClassList,
    content: Option<String>,
}

impl Display for SvgElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.attrs)?;
        if !self.classes.is_empty() {
            write!(f, r#" class="{}""#, self.classes.to_vec().join(" "))?;
        }
        Ok(())
    }
}

impl SvgElement {
    fn new(name: &str, attrs: &[(String, String)]) -> Self {
        let mut attr_map = AttrMap::new();
        let mut classes = ClassList::new();

        for (key, value) in attrs {
            if key == "class" {
                for c in value.split(' ') {
                    classes.insert(c.to_string());
                }
            } else {
                attr_map.insert(key.to_string(), value.to_string());
            }
        }
        Self {
            name: name.to_string(),
            attrs: attr_map,
            classes,
            content: None,
        }
    }

    fn add_class(&mut self, class: &str) -> Self {
        self.classes.insert(class.to_string());
        self.clone()
    }

    fn add_classes(&mut self, classes: &ClassList) {
        for class in classes {
            self.add_class(class);
        }
    }

    fn has_attr(&self, key: &str) -> bool {
        self.attrs.contains_key(key)
    }

    fn add_attr(&mut self, key: &str, value: &str) {
        self.attrs.insert(key, value);
    }

    #[allow(dead_code)]
    #[must_use]
    fn with_attr(&self, key: &str, value: &str) -> Self {
        let mut attrs = self.attrs.clone();
        attrs.insert(key, value);
        SvgElement::new(self.name.as_str(), &attrs.to_vec())
    }

    #[must_use]
    fn without_attr(&self, key: &str) -> Self {
        let attrs: Vec<(String, String)> = self
            .attrs
            .clone()
            .into_iter()
            .filter(|(k, _v)| k != key)
            .collect();
        SvgElement::new(self.name.as_str(), &attrs)
    }

    /// copy attributes and classes from another element, returning the merged element
    #[must_use]
    fn with_attrs_from(&self, other: &SvgElement) -> Self {
        let mut attrs = self.attrs.clone();
        for (k, v) in &other.attrs {
            attrs.insert(k, v);
        }
        let mut element = SvgElement::new(self.name.as_str(), &attrs.to_vec());
        element.add_classes(&other.classes);
        element
    }

    fn pop_attr(&mut self, key: &str) -> Option<String> {
        self.attrs.remove(key)
    }

    fn get_attr(&self, key: &str) -> Option<String> {
        self.attrs.get(key).map(|x| x.to_owned())
    }

    fn set_attr(&mut self, key: &str, value: &str) {
        self.attrs.insert(key, value);
    }

    fn is_connector(&self) -> bool {
        self.has_attr("start")
            && self.has_attr("end")
            && (self.name == "line" || self.name == "polyline")
    }

    fn bbox(&self) -> Option<BoundingBox> {
        match self.name.as_str() {
            "rect" | "tbox" | "pipeline" => {
                let (x, y, w, h) = (
                    strp(self.attrs.get("x")?)?,
                    strp(self.attrs.get("y")?)?,
                    strp(self.attrs.get("width")?)?,
                    strp(self.attrs.get("height")?)?,
                );
                Some(BoundingBox::BBox(x, y, x + w, y + h))
            }
            "line" => {
                let (x1, y1, x2, y2) = (
                    strp(self.attrs.get("x1")?)?,
                    strp(self.attrs.get("y1")?)?,
                    strp(self.attrs.get("x2")?)?,
                    strp(self.attrs.get("y2")?)?,
                );
                Some(BoundingBox::BBox(
                    x1.min(x2),
                    y1.min(y2),
                    x1.max(x2),
                    y1.max(y2),
                ))
            }
            "polyline" | "polygon" => {
                let mut min_x = f32::MAX;
                let mut min_y = f32::MAX;
                let mut max_x = f32::MIN;
                let mut max_y = f32::MIN;
                // these are checks to ensure we have some valid points, or we'll
                // end up with an 'interesting' bounding box...
                let mut has_x = false;
                let mut has_y = false;

                let points = self.attrs.get("points")?;
                let mut idx = 0;
                for point_ws in points.split_whitespace() {
                    for point in point_ws.split(',') {
                        let point = point.trim();
                        if point.is_empty() {
                            continue;
                        }
                        let point: f32 = strp(point)?;
                        if idx % 2 == 0 {
                            min_x = min_x.min(point);
                            max_x = max_x.max(point);
                            has_x = true;
                        } else {
                            min_y = min_y.min(point);
                            max_y = max_y.max(point);
                            has_y = true;
                        }
                        idx += 1;
                    }
                }
                if has_x && has_y {
                    Some(BoundingBox::BBox(min_x, min_y, max_x, max_y))
                } else {
                    None
                }
            }
            "circle" => {
                let (cx, cy, r) = (
                    strp(self.attrs.get("cx")?)?,
                    strp(self.attrs.get("cy")?)?,
                    strp(self.attrs.get("r")?)?,
                );
                Some(BoundingBox::BBox(cx - r, cy - r, cx + r, cy + r))
            }
            "ellipse" => {
                let (cx, cy, rx, ry) = (
                    strp(self.attrs.get("cx")?)?,
                    strp(self.attrs.get("cy")?)?,
                    strp(self.attrs.get("rx")?)?,
                    strp(self.attrs.get("ry")?)?,
                );
                Some(BoundingBox::BBox(cx - rx, cy - ry, cx + rx, cy + ry))
            }
            "person" => {
                let (x, y, h) = (
                    strp(self.attrs.get("x")?)?,
                    strp(self.attrs.get("y")?)?,
                    strp(self.attrs.get("height")?)?,
                );
                Some(BoundingBox::BBox(x, y, x + h / 3., y + h))
            }

            _ => None,
        }
    }

    fn coord(&self, loc: &str) -> Option<(f32, f32)> {
        let mut loc = loc;
        let mut len = Length::Ratio(0.5);
        let re = Regex::new(r"(?<loc>[^:\s]+)(:(?<len>[-0-9\.]+%?))?$").expect("Bad Regex");
        if let Some(caps) = re.captures(loc) {
            loc = caps.name("loc").unwrap().as_str();
            len = caps
                .name("len")
                .map_or(len, |v| strp_length(v.as_str()).expect("Invalid length"));
        }
        // This assumes a rectangular bounding box
        // TODO: support per-shape locs - e.g. "in" / "out" for pipeline
        if let Some(BoundingBox::BBox(x1, y1, x2, y2)) = self.bbox() {
            let tl = (x1, y1);
            let tr = (x2, y1);
            let br = (x2, y2);
            let bl = (x1, y2);
            let c = ((x1 + x2) / 2., (y1 + y2) / 2.);
            match loc {
                "tl" => Some(tl),
                "t" => Some((len.calc_offset(tl.0, tr.0), tl.1)),
                "tr" => Some(tr),
                "r" => Some((tr.0, len.calc_offset(tr.1, br.1))),
                "br" => Some(br),
                "b" => Some((len.calc_offset(bl.0, br.0), bl.1)),
                "bl" => Some(bl),
                "l" => Some((tl.0, len.calc_offset(tl.1, bl.1))),
                "c" => Some(c),
                _ => {
                    if self.name == "line" {
                        let x1 = strp(self.attrs.get("x1")?)?;
                        let y1 = strp(self.attrs.get("y1")?)?;
                        let x2 = strp(self.attrs.get("x2")?)?;
                        let y2 = strp(self.attrs.get("y2")?)?;
                        match loc {
                            "xy1" | "start" => Some((x1, y1)),
                            "xy2" | "end" => Some((x2, y2)),
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        }
    }

    fn translated(&self, dx: f32, dy: f32) -> Self {
        let mut new_elem = self.clone();
        for (key, value) in &self.attrs {
            match key.as_str() {
                "x" | "cx" | "x1" | "x2" => {
                    new_elem.set_attr(key, &fstr(strp(value).unwrap() + dx));
                }
                "y" | "cy" | "y1" | "y2" => {
                    new_elem.set_attr(key, &fstr(strp(value).unwrap() + dy));
                }
                _ => (),
            }
        }
        new_elem
    }

    fn resized_by(&self, dw: Length, dh: Length) -> Self {
        let mut new_elem = self.clone();
        for (key, value) in &self.attrs {
            match key.as_str() {
                "width" => {
                    new_elem.set_attr(key, &fstr(dw.adjust(strp(value).unwrap())));
                }
                "height" => {
                    new_elem.set_attr(key, &fstr(dh.adjust(strp(value).unwrap())));
                }
                _ => (),
            }
        }
        new_elem
    }

    fn process_text_attr(&self) -> Option<(SvgElement, Vec<SvgElement>)> {
        // Different conversions from line count to first-line offset based on whether
        // top, center, or bottom justification.
        const WRAP_DOWN: fn(usize, f32) -> f32 = |_count, _spacing| 0.;
        const WRAP_UP: fn(usize, f32) -> f32 = |count, spacing| -(count as f32 - 1.) * spacing;
        const WRAP_MID: fn(usize, f32) -> f32 =
            |count, spacing| -(count as f32 - 1.) / 2. * spacing;

        let mut orig_elem = self.clone();

        let mut t_dx = None;
        let mut t_dy = None;
        {
            let dx = orig_elem.pop_attr("text-dx");
            let dy = orig_elem.pop_attr("text-dy");
            let dxy = orig_elem.pop_attr("text-dxy");
            if let Some(dxy) = dxy {
                let mut parts = attr_split_cycle(&dxy).map(|v| strp(&v).unwrap());
                t_dx = parts.next();
                t_dy = parts.next();
            }
            if let Some(dx) = dx {
                t_dx = strp(&dx);
            }
            if let Some(dy) = dy {
                t_dy = strp(&dy);
            }
        }

        let text_value = orig_elem.pop_attr("text")?;
        // Convert unescaped '\n' into newline characters for multi-line text
        let re = Regex::new(r"([^\\])\\n").expect("invalid regex");
        let text_value = re.replace_all(&text_value, "$1\n").into_owned();
        // Following that, replace any escaped "\\n" into literal '\'+'n' characters
        let re = Regex::new(r"\\\\n").expect("invalid regex");
        let text_value = re.replace_all(&text_value, "\\n").into_owned();

        let mut text_attrs = vec![];
        let mut text_classes = vec!["tbox"];
        let text_loc = orig_elem.pop_attr("text-loc").unwrap_or("c".into());

        // Default dx/dy to push it in slightly from the edge (or out for lines);
        // Without inset text squishes to the edge and can be unreadable
        // Any specified dx/dy override this behaviour.
        let text_inset = 1.;

        let is_line = orig_elem.name == "line";
        // text associated with a line is pushed 'outside' the line,
        // where with other shapes it's pulled 'inside'. The classes
        // and dx/dy values are opposite.
        if ["t", "tl", "tr"].iter().any(|&s| s == text_loc) {
            text_classes.push(if is_line { "text-bottom" } else { "text-top" });
            if t_dy.is_none() {
                t_dy = Some(if is_line { -text_inset } else { text_inset });
            }
        } else if ["b", "bl", "br"].iter().any(|&s| s == text_loc) {
            text_classes.push(if is_line { "text-top" } else { "text-bottom" });
            if t_dy.is_none() {
                t_dy = Some(if is_line { text_inset } else { -text_inset });
            }
        }
        if ["l", "tl", "bl"].iter().any(|&s| s == text_loc) {
            text_classes.push(if is_line { "text-right" } else { "text-left" });
            if t_dx.is_none() {
                t_dx = Some(if is_line { -text_inset } else { text_inset });
            }
        } else if ["r", "br", "tr"].iter().any(|&s| s == text_loc) {
            text_classes.push(if is_line { "text-left" } else { "text-right" });
            if t_dx.is_none() {
                t_dx = Some(if is_line { text_inset } else { -text_inset });
            }
        }

        let first_line_offset = match (is_line, text_loc.as_str()) {
            // shapes - text 'inside'
            (false, "tl" | "t" | "tr") => WRAP_DOWN,
            (false, "bl" | "b" | "br") => WRAP_UP,
            // lines - text 'beyond'
            (true, "tl" | "t" | "tr") => WRAP_UP,
            (true, "bl" | "b" | "br") => WRAP_DOWN,
            (_, _) => WRAP_MID,
        };

        // Assumption is that text should be centered within the rect,
        // and has styling via CSS to reflect this, e.g.:
        //  text.tbox { dominant-baseline: central; text-anchor: middle; }
        let (mut tdx, mut tdy) = orig_elem.coord(&text_loc).unwrap();
        if let Some(dx) = t_dx {
            tdx += dx;
        }
        if let Some(dy) = t_dy {
            tdy += dy;
        }
        text_attrs.push(("x".into(), fstr(tdx)));
        text_attrs.push(("y".into(), fstr(tdy)));
        let mut text_elements = Vec::new();
        let lines: Vec<_> = text_value.lines().collect();
        let line_count = lines.len();

        let multiline = line_count > 1;

        // There will always be a text element; if not multiline this is the only element.
        let mut text_elem = SvgElement::new("text", &text_attrs);
        // line spacing (in 'em').
        let line_spacing =
            strp(&orig_elem.pop_attr("text-lsp").unwrap_or("1.05".to_owned())).unwrap();

        // Copy style and class(es) from original element
        if let Some(style) = orig_elem.get_attr("style") {
            text_elem.add_attr("style", &style);
        }
        text_elem.classes = orig_elem.classes.clone();
        for class in &text_classes {
            text_elem.add_class(class);
        }
        if !multiline {
            text_elem.content = Some(text_value.clone());
        }
        text_elements.push(text_elem);
        if multiline {
            let mut tspan_elem = SvgElement::new("tspan", &text_attrs);
            tspan_elem.attrs.remove("y");
            for (idx, text_fragment) in lines.iter().enumerate() {
                let mut tspan = tspan_elem.clone();
                let line_offset = if idx == 0 {
                    first_line_offset(line_count, line_spacing)
                } else {
                    line_spacing
                };
                tspan.attrs.insert("dy", format!("{}em", fstr(line_offset)));
                tspan.content = Some(text_fragment.to_string());
                text_elements.push(tspan);
            }
        }
        Some((orig_elem, text_elements))
    }

    fn eval_pos(&self, input: &str, context: &TransformerContext) -> Result<String> {
        // Relative positioning:
        //   ID LOC DX DY
        // (relv|relh|[#id])[@loc] [dx] [dy]
        // Defaults:
        //   #id = previous element
        //   @loc = tr; equivalent to @tr for relh, @bl for relv)
        //   dx, dy - offset from the @loc
        // Examples:
        //   xy="relv"       - position immediately below previous element
        //   xy="relh"       - position immediately to right of previous element
        //   xy="@tr 10 0"   - position to right of previous element with gap of 10
        //   cxy="@b"        - position centre at bottom of previous element
        // TODO - extend to allow referencing earlier elements beyond previous
        let rel_re = Regex::new(r"^(relv|relh|(?<id>#[^@]+)?(?<loc>@\S+)?)").expect("Bad Regex");
        let mut parts = attr_split(input);
        let ref_loc = parts.next().context("Empty attribute in eval_pos()")?;
        if let Some(caps) = rel_re.captures(&ref_loc) {
            if caps.get(0).unwrap().is_empty() {
                // We need either id or loc or both; since they are both optional in
                // the regex we check that we did actually match some text here...
                return Ok(input.to_owned());
            }
            // TODO: relv should be equivalent to xy="@b" xy-loc="@t" and similar
            // for relh being xy="@r" xy-loc="@l". Otherwise relh would also be
            // affected by margin-y.
            let default_rel = match ref_loc.as_str() {
                "relv" => "bl",
                "relh" => "tr",
                _ => "tr",
            };
            let dx = strp(&parts.next().unwrap_or("0".to_owned()))
                .context(format!(r#"Could not determine dx in eval_pos("{input}")"#))?;
            let dy = strp(&parts.next().unwrap_or("0".to_owned()))
                .context(format!(r#"Could not determine dy in eval_pos("{input}")"#))?;

            let mut ref_el = context.prev_element.as_ref();
            let opt_id = caps
                .name("id")
                .map(|v| v.as_str().strip_prefix('#').unwrap());
            let loc = caps
                .name("loc")
                .map(|v| v.as_str().strip_prefix('@').unwrap());
            if let Some(name) = opt_id {
                ref_el = Some(
                    context
                        .elem_map
                        .get(name)
                        .context(format!(r#"id '{name}' not found in eval_pos("{input}")"#))?,
                );
            }
            if let Some(ref_el) = ref_el {
                let mut margin_x = 0.;
                let mut margin_y = 0.;
                if let Some(margin) = ref_el.get_attr("margin") {
                    let mut margin_parts = attr_split_cycle(&margin);
                    margin_x = strp(&margin_parts.next().expect("cycle")).unwrap();
                    margin_y = strp(&margin_parts.next().expect("cycle")).unwrap();
                }
                let loc = loc.unwrap_or(default_rel);
                margin_y = match loc {
                    "tl" | "t" | "tr" => -margin_y,
                    "bl" | "b" | "br" => margin_y,
                    _ => 0.,
                };
                margin_x = match loc {
                    "tl" | "l" | "bl" => -margin_x,
                    "tr" | "r" | "br" => margin_x,
                    _ => 0.,
                };
                if let Some(pos) = ref_el.coord(loc) {
                    return Ok(format!(
                        "{} {}",
                        fstr(pos.0 + margin_x + dx),
                        fstr(pos.1 + margin_y + dy)
                    ));
                }
            }
        }
        Ok(input.to_owned())
    }

    fn eval_size(&self, input: &str, context: &TransformerContext) -> Result<String> {
        // Relative size:
        //   (#id|^) [DW[%] DH[%]]
        // Meaning:
        //   #id - reference to size of another element
        //   ^ - reference to previous element
        //   dw / dh - delta width/height (user units; may be negative)
        //   dw% / dh% - scaled width/height (range 0..1000%)
        // Note: unlike in eval_pos, the id section is mandatory to distinguish from
        //       a numeric `wh` pair.
        // Examples:
        //   wh="#thing 2 110%"  - size of #thing plus 2 units width, *1.1 height
        // TODO: extend to allow referencing earlier elements beyond previous
        // TODO: allow mixed relative and absolute values...
        let mut parts = attr_split(input);
        let ref_loc = parts.next().expect("always at least one");
        let rel_re = Regex::new(r"^(?<ref>(#\S+|\^))").expect("Bad Regex");
        if let Some(caps) = rel_re.captures(&ref_loc) {
            let dw = parts.next().unwrap_or("0".to_owned());
            let dh = parts.next().unwrap_or("0".to_owned());
            let mut ref_el = context.prev_element.as_ref();
            let ref_str = caps
                .name("ref")
                .context("ref is mandatory in regex")?
                .as_str();
            if let Some(ref_str) = ref_str.strip_prefix('#') {
                ref_el = Some(context.elem_map.get(ref_str).context(format!(
                    "Could not find reference '{ref_str}' in eval_size({input})"
                ))?);
            }

            if let Some(inner) = ref_el {
                if let (Some(w), Some(h)) = (inner.get_attr("width"), inner.get_attr("height")) {
                    let w =
                        strp(&w).context(r#"Could not derive width in eval_size("{input}")"#)?;
                    let h =
                        strp(&h).context(r#"Could not derive height in eval_size("{input}")"#)?;
                    let dw = strp_length(&dw)
                        .context(r#"Could not derive dw in eval_size("{input"})"#)?;
                    let dh = strp_length(&dh)
                        .context(r#"Could not derive dh in eval_size("{input"})"#)?;
                    let w = fstr(dw.adjust(w));
                    let h = fstr(dh.adjust(h));

                    return Ok(format!("{w} {h}"));
                }
            }
        }
        Ok(input.to_owned())
    }

    /// Process and expand attributes as needed
    fn expand_attributes(&mut self, simple: bool, context: &mut TransformerContext) -> Result<()> {
        let mut new_attrs = AttrMap::new();

        for (key, value) in self.attrs.clone() {
            let replace = eval_attr(&value, &context.variables);
            self.attrs.insert(&key, &replace);
        }

        // Every attribute is either replaced by one or more other attributes,
        // or copied as-is into `new_attrs`.
        for (key, value) in self.attrs.clone() {
            let mut value = value.clone();
            if !simple {
                match key.as_str() {
                    "xy" | "cxy" | "xy1" | "xy2" => {
                        value = self.eval_pos(value.as_str(), context)?;
                    }
                    "wh" => {
                        // TODO: support rxy for ellipses, with scaling factor
                        value = self.eval_size(value.as_str(), context)?;
                    }
                    _ => (),
                }
            }

            // The first pass is straightforward 'expansion', where the current
            // attribute totally determines the resulting value(s).
            let mut parts = attr_split_cycle(&value);
            match (key.as_str(), self.name.as_str()) {
                ("xy", "text" | "rect" | "pipeline") => {
                    new_attrs.insert("x", parts.next().expect("cycle"));
                    new_attrs.insert("y", parts.next().expect("cycle"));
                }
                ("wh", "rect" | "tbox" | "pipeline") => {
                    new_attrs.insert("width", parts.next().expect("cycle"));
                    new_attrs.insert("height", parts.next().expect("cycle"));
                }
                ("wh", "circle") => {
                    let diameter: f32 = strp(&parts.next().expect("cycle")).unwrap();
                    new_attrs.insert("r", fstr(diameter / 2.));
                }
                ("wh", "ellipse") => {
                    let dia_x: f32 = strp(&parts.next().expect("cycle")).unwrap();
                    let dia_y: f32 = strp(&parts.next().expect("cycle")).unwrap();
                    new_attrs.insert("rx", fstr(dia_x / 2.));
                    new_attrs.insert("ry", fstr(dia_y / 2.));
                }
                ("cxy", "circle" | "ellipse") => {
                    new_attrs.insert("cx", parts.next().expect("cycle"));
                    new_attrs.insert("cy", parts.next().expect("cycle"));
                }
                ("rxy", "ellipse") => {
                    new_attrs.insert("rx", parts.next().expect("cycle"));
                    new_attrs.insert("ry", parts.next().expect("cycle"));
                }
                ("xy1", "line") => {
                    new_attrs.insert("x1", parts.next().expect("cycle"));
                    new_attrs.insert("y1", parts.next().expect("cycle"));
                }
                ("xy2", "line") => {
                    new_attrs.insert("x2", parts.next().expect("cycle"));
                    new_attrs.insert("y2", parts.next().expect("cycle"));
                }
                _ => new_attrs.insert(key.clone(), value.clone()),
            }
        }

        if !simple {
            // A second pass is used where the processed values of other attributes
            // (which may be given in any order and so not available on first pass)
            // are required, e.g. updating cxy for rect-like objects, which requires
            // width & height to already be determined.
            let mut pass_two_attrs = AttrMap::new();
            for (key, value) in new_attrs.clone() {
                let mut parts = attr_split_cycle(&value);
                match (key.as_str(), self.name.as_str()) {
                    ("cxy", "rect" | "tbox" | "pipeline") => {
                        // Requires wh (/ width&height) be specified in order to evaluate
                        // the centre point.
                        // TODO: also support specifying other attributes; xy+cxy should be sufficient
                        let wh = new_attrs.get("wh").map(|z| z.to_string());
                        let mut width = new_attrs.get("width").map(|z| strp(z).unwrap());
                        let mut height = new_attrs.get("height").map(|z| strp(z).unwrap());
                        let cx = strp(&parts.next().expect("cycle"))
                            .context("Could not derive x from cxy")?;
                        let cy = strp(&parts.next().expect("cycle"))
                            .context("Could not derive y from cxy")?;
                        if let Some(wh_inner) = wh {
                            let mut wh_parts = attr_split_cycle(&wh_inner);
                            width = Some(
                                strp(&wh_parts.next().expect("cycle"))
                                    .context("Could not derive width during cxy processing")?,
                            );
                            height = Some(
                                strp(&wh_parts.next().expect("cycle"))
                                    .context("Could not derive height during cxy processing")?,
                            );
                        }
                        if let (Some(width), Some(height)) = (width, height) {
                            pass_two_attrs.insert("x", fstr(cx - width / 2.));
                            pass_two_attrs.insert("y", fstr(cy - height / 2.));
                            // wh / width&height will be handled separately
                        }
                    }
                    _ => pass_two_attrs.insert(key.clone(), value.clone()),
                }
            }

            new_attrs = pass_two_attrs;
        }

        self.attrs = new_attrs;
        if let Some(elem_id) = self.get_attr("id") {
            let mut updated = SvgElement::new(&self.name, &self.attrs.to_vec());
            updated.add_classes(&self.classes);
            if let Some(elem) = context.elem_map.get_mut(&elem_id) {
                *elem = updated;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fstr() {
        assert_eq!(fstr(1.0), "1");
        assert_eq!(fstr(-100.0), "-100");
        assert_eq!(fstr(1.2345678), "1.235");
        assert_eq!(fstr(-1.2345678), "-1.235");
        assert_eq!(fstr(91.0004), "91");
        // Large-ish integers (up to 24 bit mantissa) should be fine
        assert_eq!(fstr(12345678.0), "12345678");
        assert_eq!(fstr(12340000.0), "12340000");
    }

    #[test]
    fn test_strp() {
        assert_eq!(strp("1"), Some(1.));
        assert_eq!(strp("100"), Some(100.));
        assert_eq!(strp("-100"), Some(-100.));
        assert_eq!(strp("-0.00123"), Some(-0.00123));
        assert_eq!(strp("1234567.8"), Some(1234567.8));
    }

    #[test]
    fn test_strp_length() {
        assert_eq!(strp_length("1"), Some(Length::Absolute(1.)));
        assert_eq!(strp_length("123"), Some(Length::Absolute(123.)));
        assert_eq!(strp_length("-0.0123"), Some(Length::Absolute(-0.0123)));
        assert_eq!(strp_length("0.5%"), Some(Length::Ratio(0.005)));
        assert_eq!(strp_length("150%"), Some(Length::Ratio(1.5)));
        assert_eq!(strp_length("1.2.3"), None);
        assert_eq!(strp_length("a"), None);
        assert_eq!(strp_length("a%"), None);
    }

    #[test]
    fn test_length_calc_offset() {
        assert_eq!(strp_length("25%").expect("test").calc_offset(10., 50.), 20.);
        assert_eq!(
            strp_length("50%").expect("test").calc_offset(-10., -9.),
            -9.5
        );
        assert_eq!(
            strp_length("200%").expect("test").calc_offset(10., 50.),
            90.
        );
        assert_eq!(
            strp_length("-3.5").expect("test").calc_offset(10., 50.),
            46.5
        );
        assert_eq!(
            strp_length("3.5").expect("test").calc_offset(-10., 90.),
            -6.5
        );
    }

    #[test]
    fn test_length_adjust() {
        assert_eq!(strp_length("25%").expect("test").adjust(10.), 2.5);
        assert_eq!(strp_length("-50%").expect("test").adjust(150.), -75.);
        assert_eq!(strp_length("125%").expect("test").adjust(20.), 25.);
        assert_eq!(strp_length("1").expect("test").adjust(23.), 24.);
        assert_eq!(strp_length("-12").expect("test").adjust(123.), 111.);
    }
}
