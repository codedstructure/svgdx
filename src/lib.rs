use std::io::{Read, Write};

use regex::Regex;

mod types;
use types::{AttrMap, BoundingBox, ClassList};
mod transform;
pub(crate) use transform::{Transformer, TransformerContext};
mod connector;

pub fn svg_transform(reader: &mut dyn Read, writer: &mut dyn Write) -> Result<(), String> {
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

/// Returns iterator cycling over whitespace-or-comma separated values
fn attr_split(input: &str) -> impl Iterator<Item = String> + '_ {
    input
        .split_whitespace()
        .flat_map(|v| v.split(','))
        .map(|v| v.to_string())
        .cycle()
}

#[derive(Clone, Copy, Debug)]
enum Length {
    Absolute(f32),
    Ratio(f32),
}

#[allow(dead_code)]
impl Length {
    fn ratio(&self) -> Option<f32> {
        if let Length::Ratio(result) = self {
            Some(*result)
        } else {
            None
        }
    }

    fn absolute(&self) -> Option<f32> {
        if let Length::Absolute(result) = self {
            Some(*result)
        } else {
            None
        }
    }

    fn calc_offset(&self, start: f32, end: f32) -> f32 {
        match self {
            Length::Absolute(abs) => {
                if abs < &0. {
                    // '+' here since abs is negative and
                    // we're going 'back' from the end.
                    end + abs
                } else {
                    start + abs
                }
            }
            Length::Ratio(ratio) => start + (end - start) * ratio,
        }
    }
}

/// Parse a ratio (float or %age) to an f32
/// Note this deliberately does not clamp to 0..1
fn strp_length(s: &str) -> Option<Length> {
    let mut s = s.clone();
    if s.ends_with('%') {
        s = s.trim_end_matches('%');
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

    #[allow(dead_code)]
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
        // This assumes a rectangular bounding box
        // TODO: support per-shape locs - e.g. "in" / "out" for pipeline
        if let Some(BoundingBox::BBox(x1, y1, x2, y2)) = self.bbox() {
            match loc {
                "tl" => Some((x1, y1)),
                "t" => Some(((x1 + x2) / 2., y1)),
                "tr" => Some((x2, y1)),
                "r" => Some((x2, (y1 + y2) / 2.)),
                "br" => Some((x2, y2)),
                "b" => Some(((x1 + x2) / 2., y2)),
                "bl" => Some((x1, y2)),
                "l" => Some((x1, (y1 + y2) / 2.)),
                "c" => Some(((x1 + x2) / 2., (y1 + y2) / 2.)),
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
        let mut attrs = vec![];
        for (key, mut value) in self.attrs.clone() {
            match key.as_str() {
                "x" | "cx" | "x1" | "x2" => {
                    value = fstr(strp(&value).unwrap() + dx);
                }
                "y" | "cy" | "y1" | "y2" => {
                    value = fstr(strp(&value).unwrap() + dy);
                }
                _ => (),
            }
            attrs.push((key.clone(), value.clone()));
        }
        SvgElement::new(self.name.as_str(), &attrs)
    }

    #[allow(dead_code)]
    fn positioned(&self, x: f32, y: f32) -> Self {
        // TODO: should allow specifying which loc this positions; this currently
        // sets the top-left (for rect/tbox), but for some scenarios it might be
        // necessary to set e.g. the bottom-right loc.
        let mut result = self.clone();
        match self.name.as_str() {
            "rect" | "tbox" | "pipeline" => {
                result = result.with_attr("x", &fstr(x)).with_attr("y", &fstr(y));
            }
            _ => {
                todo!()
            }
        }
        result
    }

    fn process_text_attr(&self) -> Option<(SvgElement, Vec<SvgElement>)> {
        let mut orig_elem = self.clone();

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
        let mut dx = orig_elem.pop_attr("dx").map(|dx| strp(&dx).unwrap());
        let mut dy = orig_elem.pop_attr("dy").map(|dy| strp(&dy).unwrap());

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
            if dy.is_none() {
                dy = Some(if is_line { -text_inset } else { text_inset });
            }
        } else if ["b", "bl", "br"].iter().any(|&s| s == text_loc) {
            text_classes.push(if is_line { "text-top" } else { "text-bottom" });
            if dy.is_none() {
                dy = Some(if is_line { text_inset } else { -text_inset });
            }
        }
        if ["l", "tl", "bl"].iter().any(|&s| s == text_loc) {
            text_classes.push(if is_line { "text-right" } else { "text-left" });
            if dx.is_none() {
                dx = Some(if is_line { -text_inset } else { text_inset });
            }
        } else if ["r", "br", "tr"].iter().any(|&s| s == text_loc) {
            text_classes.push(if is_line { "text-left" } else { "text-right" });
            if dx.is_none() {
                dx = Some(if is_line { text_inset } else { -text_inset });
            }
        }

        // Different conversions from line count to first-line offset based on whether
        // top, center, or bottom justification.
        const WRAP_DOWN: fn(usize, f32) -> f32 = |_count, _spacing| 0.;
        const WRAP_UP: fn(usize, f32) -> f32 = |count, spacing| -(count as f32 - 1.) * spacing;
        const WRAP_MID: fn(usize, f32) -> f32 =
            |count, spacing| -(count as f32 - 1.) / 2. * spacing;
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
        if let Some(dx) = dx {
            tdx += dx;
        }
        if let Some(dy) = dy {
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
        // line spacing (in 'em'). TODO: allow configuring this...
        let line_spacing = 1.05;

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

    fn eval_pos(&self, input: &str, context: &TransformerContext) -> String {
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
        let rel_re = Regex::new(r"^(relv|relh|(?<id>#[^@]+)?(?<loc>@\S+)?)(\s+(?<dx>[-0-9\.]+))?(\s+(?<dy>[-0-9\.]+))?$").unwrap();
        if let Some(caps) = rel_re.captures(input) {
            // TODO: relv should be equivalent to xy="@b" xy-loc="@t" and similar
            // for relh being xy="@r" xy-loc="@l". Otherwise relh would also be
            // affected by margin-y.
            let default_rel = match input {
                "relv" => "bl",
                "relh" => "tr",
                _ => "tr",
            };
            let dx = strp(caps.name("dx").map_or("0", |v| v.as_str())).unwrap();
            let dy = strp(caps.name("dy").map_or("0", |v| v.as_str())).unwrap();

            let mut ref_el = context.prev_element.as_ref();
            let opt_id = caps
                .name("id")
                .map(|v| v.as_str().strip_prefix('#').unwrap());
            let loc = caps
                .name("loc")
                .map(|v| v.as_str().strip_prefix('@').unwrap());
            if let Some(name) = opt_id {
                ref_el = Some(context.elem_map.get(name).unwrap());
            }
            if let Some(ref_el) = ref_el {
                let mut margin_x = 0.;
                let mut margin_y = 0.;
                if let Some(margin) = ref_el.get_attr("margin") {
                    let mut margin_parts = attr_split(&margin);
                    margin_x = strp(&margin_parts.next().unwrap()).unwrap();
                    margin_y = strp(&margin_parts.next().unwrap()).unwrap();
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
                    return format!(
                        "{} {}",
                        fstr(pos.0 + margin_x + dx),
                        fstr(pos.1 + margin_y + dy)
                    );
                }
            }
        }
        input.to_owned()
    }

    fn eval_size(&self, input: &str, context: &TransformerContext) -> String {
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
        let rel_re =
            Regex::new(r"^(?<ref>(#\S+|\^))(\s+(?<dw>[-0-9\.]+%?)\s+(?<dh>[-0-9\.]+%?))?$")
                .unwrap();
        if let Some(caps) = rel_re.captures(input) {
            let dw = caps.name("dw").map_or("0", |v| v.as_str());
            let dh = caps.name("dh").map_or("0", |v| v.as_str());
            let mut ref_el = context.prev_element.as_ref();
            let ref_str = caps
                .name("ref")
                .expect("ref is mandatory in regex")
                .as_str();
            if ref_str.starts_with('#') {
                ref_el = Some(
                    context
                        .elem_map
                        .get(ref_str.strip_prefix('#').unwrap())
                        .unwrap(),
                );
            }

            if let Some(inner) = ref_el {
                if let (Some(w), Some(h)) = (inner.get_attr("width"), inner.get_attr("height")) {
                    let mut w = strp(&w).unwrap();
                    let mut h = strp(&h).unwrap();
                    if dw.ends_with('%') {
                        let dw = strp(dw.trim_end_matches('%')).unwrap() / 100.0;
                        w *= dw;
                    } else {
                        w += strp(dw).unwrap();
                    }
                    if dh.ends_with('%') {
                        let dh = strp(dh.trim_end_matches('%')).unwrap() / 100.0;
                        h *= dh;
                    } else {
                        h += strp(dh).unwrap();
                    }

                    return format!("{w} {h}");
                }
            }
        }
        input.to_owned()
    }

    /// Process and expand attributes as needed
    fn expand_attributes(&mut self, simple: bool, context: &mut TransformerContext) {
        let mut new_attrs = vec![];

        // Every attribute is either replaced by one or more other attributes,
        // or copied as-is into `new_attrs`.
        for (key, value) in self.attrs.clone() {
            let mut value = value.clone();
            if !simple {
                match key.as_str() {
                    "xy" | "cxy" | "xy1" | "xy2" => {
                        value = self.eval_pos(value.as_str(), context);
                    }
                    "size" | "wh" => {
                        // TODO: support rxy for ellipses, with scaling factor
                        value = self.eval_size(value.as_str(), context);
                    }
                    _ => (),
                }
            }
            // TODO: should expand in a given order to avoid repetition?
            match key.as_str() {
                "xy" => {
                    let mut parts = attr_split(&value);

                    match self.name.as_str() {
                        "text" | "rect" | "tbox" | "pipeline" => {
                            new_attrs.push(("x".into(), parts.next().unwrap()));
                            new_attrs.push(("y".into(), parts.next().unwrap()));
                        }
                        _ => new_attrs.push((key.clone(), value.clone())),
                    }
                }
                "size" | "wh" => {
                    let mut parts = attr_split(&value);

                    match self.name.as_str() {
                        "rect" | "tbox" | "pipeline" => {
                            new_attrs.push(("width".into(), parts.next().unwrap()));
                            new_attrs.push(("height".into(), parts.next().unwrap()));
                        }
                        "circle" => {
                            let diameter: f32 = strp(&parts.next().unwrap()).unwrap();
                            new_attrs.push(("r".into(), fstr(diameter / 2.)));
                        }
                        "ellipse" => {
                            let dia_x: f32 = strp(&parts.next().unwrap()).unwrap();
                            let dia_y: f32 = strp(&parts.next().unwrap()).unwrap();
                            new_attrs.push(("rx".into(), fstr(dia_x / 2.)));
                            new_attrs.push(("ry".into(), fstr(dia_y / 2.)));
                        }
                        _ => new_attrs.push((key.clone(), value.clone())),
                    }
                }
                "cxy" => {
                    let mut parts = attr_split(&value);

                    match self.name.as_str() {
                        "rect" | "tbox" | "pipeline" => {
                            // Requires wh (/ width&height) be specified in order to evaluate
                            // the centre point.
                            // TODO: also support specifying other attributes; xy+cxy should be sufficient
                            let wh = self.attrs.get("wh").map(|z| z.to_string());
                            let mut width = self.attrs.get("width").map(|z| strp(z).unwrap());
                            let mut height = self.attrs.get("height").map(|z| strp(z).unwrap());
                            let cx = strp(&parts.next().unwrap()).unwrap();
                            let cy = strp(&parts.next().unwrap()).unwrap();
                            if let Some(wh_inner) = wh {
                                let mut wh_parts = attr_split(&wh_inner);
                                width = Some(strp(&wh_parts.next().unwrap()).unwrap());
                                height = Some(strp(&wh_parts.next().unwrap()).unwrap());
                            }
                            if let (Some(width), Some(height)) = (width, height) {
                                new_attrs.push(("x".into(), fstr(cx - width / 2.)));
                                new_attrs.push(("y".into(), fstr(cy - height / 2.)));
                                // wh / width&height will be handled separately
                            }
                        }
                        "circle" | "ellipse" => {
                            new_attrs.push(("cx".into(), parts.next().unwrap()));
                            new_attrs.push(("cy".into(), parts.next().unwrap()));
                        }
                        _ => new_attrs.push((key.clone(), value.clone())),
                    }
                }
                "rxy" => match self.name.as_str() {
                    "ellipse" => {
                        let mut parts = attr_split(&value);

                        new_attrs.push(("rx".into(), parts.next().unwrap()));
                        new_attrs.push(("ry".into(), parts.next().unwrap()));
                    }
                    _ => new_attrs.push((key.clone(), value)),
                },
                "xy1" => {
                    let mut parts = attr_split(&value);
                    match self.name.as_str() {
                        "line" => {
                            new_attrs.push(("x1".into(), parts.next().unwrap()));
                            new_attrs.push(("y1".into(), parts.next().unwrap()));
                        }
                        "rect" => {
                            new_attrs.push(("x".into(), parts.next().unwrap()));
                            new_attrs.push(("y".into(), parts.next().unwrap()));
                        }
                        _ => new_attrs.push((key.clone(), value.clone())),
                    }
                }
                "xy2" => {
                    let mut parts = attr_split(&value);
                    match self.name.as_str() {
                        "line" => {
                            new_attrs.push(("x2".into(), parts.next().unwrap()));
                            new_attrs.push(("y2".into(), parts.next().unwrap()));
                        }
                        "rect" => {
                            // must have xy1 (/ x&y) or wh (/ width&height)
                            let wh = self.attrs.get("wh").map(|z| z.to_string());
                            let xy1 = self.attrs.get("xy1").map(|z| z.to_string());
                            let mut width = self.attrs.get("width").map(|z| strp(z).unwrap());
                            let mut height = self.attrs.get("height").map(|z| strp(z).unwrap());
                            let mut x = self.attrs.get("x").map(|z| strp(z).unwrap());
                            let mut y = self.attrs.get("y").map(|z| strp(z).unwrap());
                            let x2 = strp(&parts.next().unwrap()).unwrap();
                            let y2 = strp(&parts.next().unwrap()).unwrap();
                            if let Some(wh_inner) = wh {
                                let mut wh_parts = attr_split(&wh_inner);
                                width = Some(strp(&wh_parts.next().unwrap()).unwrap());
                                height = Some(strp(&wh_parts.next().unwrap()).unwrap());
                            }
                            if let Some(xy1_inner) = xy1 {
                                let mut xy1_parts = attr_split(&xy1_inner);
                                x = Some(strp(&xy1_parts.next().unwrap()).unwrap());
                                y = Some(strp(&xy1_parts.next().unwrap()).unwrap());
                            }
                            if let (Some(w), Some(h)) = (width, height) {
                                new_attrs.push(("x".into(), fstr(x2 - w)));
                                new_attrs.push(("y".into(), fstr(y2 - h)));
                                // width / height either already exist in the target or will be expanded from a wh.
                            } else if let (Some(x), Some(y)) = (x, y) {
                                new_attrs.push(("width".into(), fstr(x2 - x)));
                                new_attrs.push(("height".into(), fstr(y2 - y)));
                                // x/y either already exist in the target or will be expanded from a xy1.
                            }
                        }
                        _ => new_attrs.push((key.clone(), value.clone())),
                    }
                }
                _ => new_attrs.push((key.clone(), value)),
            }
        }

        self.attrs = new_attrs.into();
        if let Some(elem_id) = self.get_attr("id") {
            let mut updated = SvgElement::new(&self.name, &self.attrs.to_vec());
            updated.add_classes(&self.classes);
            *context.elem_map.get_mut(&elem_id).unwrap() = updated;
        }
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
}
