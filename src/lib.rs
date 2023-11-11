use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::io::{BufReader, Read, Write};

use std::collections::HashMap;

use regex::Regex;

mod types;
use types::{AttrMap, BoundingBox, ClassList};

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

#[derive(Clone, Debug)]
struct SvgElement {
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
    fn with_attr(&self, key: &str, value: &str) -> Self {
        let mut attrs = self.attrs.clone();
        attrs.insert(key, value);
        SvgElement::new(self.name.as_str(), &attrs.to_vec())
    }

    #[allow(dead_code)]
    fn without_attr(&self, key: &str) -> Self {
        let attrs: Vec<(String, String)> = self
            .attrs
            .clone()
            .into_iter()
            .filter(|(k, _v)| k != key)
            .collect();
        SvgElement::new(self.name.as_str(), &attrs)
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

    #[allow(dead_code)]
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

    fn locations(&self) -> &[&str] {
        if self.name == "line" {
            &[
                "xy1", "start", "xy2", "end", "t", "b", "l", "r", "tl", "bl", "tr", "br",
            ]
        } else {
            &["t", "b", "l", "r", "tl", "bl", "tr", "br"]
        }
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
            if let Some(pos) = ref_el.unwrap().coord(loc.unwrap_or(default_rel)) {
                return format!("{} {}", fstr(pos.0 + dx), fstr(pos.1 + dy));
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

    /// Returns iterator cycling over whitespace-separated values
    fn attr_split<'a>(&'a self, input: &'a str) -> impl Iterator<Item = String> + '_ {
        input.split_whitespace().map(|v| v.to_string()).cycle()
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
                    let mut parts = self.attr_split(&value);

                    match self.name.as_str() {
                        "text" | "rect" | "tbox" | "pipeline" => {
                            new_attrs.push(("x".into(), parts.next().unwrap()));
                            new_attrs.push(("y".into(), parts.next().unwrap()));
                        }
                        _ => new_attrs.push((key.clone(), value.clone())),
                    }
                }
                "size" | "wh" => {
                    let mut parts = self.attr_split(&value);

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
                    let mut parts = self.attr_split(&value);

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
                                let mut wh_parts = self.attr_split(&wh_inner);
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
                        let mut parts = self.attr_split(&value);

                        new_attrs.push(("rx".into(), parts.next().unwrap()));
                        new_attrs.push(("ry".into(), parts.next().unwrap()));
                    }
                    _ => new_attrs.push((key.clone(), value)),
                },
                "xy1" => {
                    let mut parts = self.attr_split(&value);
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
                    let mut parts = self.attr_split(&value);
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
                                let mut wh_parts = self.attr_split(&wh_inner);
                                width = Some(strp(&wh_parts.next().unwrap()).unwrap());
                                height = Some(strp(&wh_parts.next().unwrap()).unwrap());
                            }
                            if let Some(xy1_inner) = xy1 {
                                let mut xy1_parts = self.attr_split(&xy1_inner);
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

impl From<&BytesStart<'_>> for SvgElement {
    fn from(e: &BytesStart) -> Self {
        let elem_name: String = String::from_utf8(e.name().into_inner().to_vec()).unwrap();

        let attrs: Vec<(String, String)> = e
            .attributes()
            .map(move |a| {
                let aa = a.unwrap();
                let key = String::from_utf8(aa.key.into_inner().to_vec()).unwrap();
                let value = aa.unescape_value().unwrap().into_owned();
                (key, value)
            })
            .collect();
        SvgElement::new(&elem_name, &attrs)
    }
}

#[derive(Clone, Copy, Debug)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Clone, Copy, Debug)]
struct Endpoint {
    origin: (f32, f32),
    dir: Option<Direction>,
}

impl Endpoint {
    fn new(origin: (f32, f32), dir: Option<Direction>) -> Self {
        Self { origin, dir }
    }
}

#[derive(Clone, Copy, Debug)]
enum ConnectionType {
    Straight,
    Corner,
}

#[derive(Clone, Copy, Debug)]
struct Connector {
    start: Endpoint,
    end: Endpoint,
    conn_type: ConnectionType,
}

impl Connector {
    fn new(start: Endpoint, end: Endpoint, conn_type: ConnectionType) -> Self {
        Self {
            start,
            end,
            conn_type,
        }
    }

    fn from_element(
        element: &SvgElement,
        context: &TransformerContext,
        conn_type: ConnectionType,
    ) -> Self {
        let start_ref = element.attrs.get("start").unwrap();
        let end_ref = element.attrs.get("end").unwrap();

        // This could probably be tidier, trying to deal with lots of combinations.
        // Needs to support explicit coordinate pairs or element references, and
        // for element references support given locations or not (in which case
        // the location is determined automatically to give the shortest distance)
        let mut start_el = None;
        let mut end_el = None;
        let mut start_loc = String::from("");
        let mut end_loc = String::from("");
        let mut start_point: Option<(f32, f32)> = None;
        let mut end_point: Option<(f32, f32)> = None;
        let mut start_dir = None;
        let mut end_dir = None;

        let loc_to_dir = |dir: String| match dir.as_str() {
            "t" => Some(Direction::Up),
            "r" => Some(Direction::Right),
            "b" => Some(Direction::Down),
            "l" => Some(Direction::Left),
            _ => None,
        };

        // Example: "#thing@tl" => top left coordinate of element id="thing"
        let re = Regex::new(r"^#(?<id>[^@]+)(@(?<loc>\S+))?$").unwrap();

        if let Some(caps) = re.captures(start_ref) {
            let name = &caps["id"];
            start_loc = caps.name("loc").map_or("", |v| v.as_str()).to_string();
            start_dir = loc_to_dir(start_loc.clone());
            start_el = context.elem_map.get(name);
        } else {
            let mut parts = element.attr_split(start_ref).map(|v| strp(&v).unwrap());
            start_point = Some((parts.next().unwrap(), parts.next().unwrap()));
        }
        if let Some(caps) = re.captures(end_ref) {
            let name = &caps["id"];
            end_loc = caps.name("loc").map_or("", |v| v.as_str()).to_string();
            end_dir = loc_to_dir(end_loc.clone());
            end_el = context.elem_map.get(name);
        } else {
            let mut parts = element.attr_split(end_ref).map(|v| strp(&v).unwrap());
            end_point = Some((parts.next().unwrap(), parts.next().unwrap()));
        }

        let (start, end) = match (start_point, end_point) {
            (Some(start_point), Some(end_point)) => (
                Endpoint::new(start_point, start_dir),
                Endpoint::new(end_point, end_dir),
            ),
            (Some(start_point), None) => {
                let end_el = end_el.unwrap();
                if end_loc.is_empty() {
                    end_loc = context.closest_loc(end_el, start_point);
                    end_dir = loc_to_dir(end_loc.clone());
                }
                (
                    Endpoint::new(start_point, start_dir),
                    Endpoint::new(end_el.coord(&end_loc).unwrap(), end_dir),
                )
            }
            (None, Some(end_point)) => {
                let start_el = start_el.unwrap();
                if start_loc.is_empty() {
                    start_loc = context.closest_loc(start_el, end_point);
                    start_dir = loc_to_dir(start_loc.clone());
                }
                (
                    Endpoint::new(start_el.coord(&start_loc).unwrap(), start_dir),
                    Endpoint::new(end_point, end_dir),
                )
            }
            (None, None) => {
                let (start_el, end_el) = (start_el.unwrap(), end_el.unwrap());
                if start_loc.is_empty() && end_loc.is_empty() {
                    (start_loc, end_loc) = context.shortest_link(start_el, end_el);
                    start_dir = loc_to_dir(start_loc.clone());
                    end_dir = loc_to_dir(end_loc.clone());
                } else if start_loc.is_empty() {
                    start_loc = context.closest_loc(start_el, end_el.coord(&end_loc).unwrap());
                    start_dir = loc_to_dir(start_loc.clone());
                } else if end_loc.is_empty() {
                    end_loc = context.closest_loc(end_el, start_el.coord(&start_loc).unwrap());
                    end_dir = loc_to_dir(end_loc.clone());
                }
                (
                    Endpoint::new(start_el.coord(&start_loc).unwrap(), start_dir),
                    Endpoint::new(end_el.coord(&end_loc).unwrap(), end_dir),
                )
            }
        };
        Self::new(start, end, conn_type)
    }

    fn render(&self) -> impl Iterator<Item = SvgEvent> + '_ {
        let mut vec = vec![];

        let (x1, y1) = self.start.origin;
        let (x2, y2) = self.end.origin;
        match self.conn_type {
            ConnectionType::Straight => {
                vec.push(SvgEvent::Empty(SvgElement::new(
                    "line",
                    &[
                        ("x1".to_string(), fstr(x1)),
                        ("y1".to_string(), fstr(y1)),
                        ("x2".to_string(), fstr(x2)),
                        ("y2".to_string(), fstr(y2)),
                    ],
                )));
            }
            ConnectionType::Corner => {
                let points;
                if let (Some(start_dir_some), Some(end_dir_some)) = (self.start.dir, self.end.dir) {
                    points = match (start_dir_some, end_dir_some) {
                        // L-shaped connection
                        (Direction::Up | Direction::Down, Direction::Left | Direction::Right) => {
                            vec![(x1, y1), (self.start.origin.0, self.end.origin.1), (x2, y2)]
                        }
                        (Direction::Left | Direction::Right, Direction::Up | Direction::Down) => {
                            vec![(x1, y1), (self.end.origin.0, self.start.origin.1), (x2, y2)]
                        }
                        // Z-shaped connection
                        (Direction::Left, Direction::Right)
                        | (Direction::Right, Direction::Left) => {
                            let mid_x = (self.start.origin.0 + self.end.origin.0) / 2.;
                            vec![(x1, y1), (mid_x, y1), (mid_x, y2), (x2, y2)]
                        }
                        (Direction::Up, Direction::Down) | (Direction::Down, Direction::Up) => {
                            let mid_y = (self.start.origin.1 + self.end.origin.1) / 2.;
                            vec![(x1, y1), (x1, mid_y), (x2, mid_y), (x2, y2)]
                        }
                        // If all else fails, straight line...
                        _ => vec![(x1, y1), (x2, y2)],
                    };
                } else {
                    points = vec![(x1, y1), (x2, y2)];
                }
                if points.len() == 2 {
                    vec.push(SvgEvent::Empty(SvgElement::new(
                        "line",
                        &[
                            ("x1".to_string(), fstr(points[0].0)),
                            ("y1".to_string(), fstr(points[0].1)),
                            ("x2".to_string(), fstr(points[1].0)),
                            ("y2".to_string(), fstr(points[1].1)),
                        ],
                    )));
                } else {
                    vec.push(SvgEvent::Empty(SvgElement::new(
                        "polyline",
                        &[(
                            "points".to_string(),
                            points
                                .into_iter()
                                .map(|(px, py)| format!("{} {}", fstr(px), fstr(py)))
                                .collect::<Vec<String>>()
                                .join(", "),
                        )],
                    )));
                }
            }
        }

        vec.into_iter()
    }
}

#[derive(Debug)]
enum SvgEvent {
    Text(String),
    Start(SvgElement),
    Empty(SvgElement),
    End(String),
}

#[derive(Clone, Default, Debug)]
struct TransformerContext {
    elem_map: HashMap<String, SvgElement>,
    prev_element: Option<SvgElement>,
    last_indent: String,
}

impl TransformerContext {
    fn new() -> Self {
        let elem_map: HashMap<String, SvgElement> = HashMap::new();

        Self {
            elem_map,
            prev_element: None,
            last_indent: String::from(""),
        }
    }

    fn populate(&mut self, events: &EventList) {
        let mut elem_map: HashMap<String, SvgElement> = HashMap::new();

        for ev in events.iter() {
            match ev {
                Event::Eof => {
                    // should never happen, as handled in EventList::from_reader()
                    break;
                }
                Event::Start(e) | Event::Empty(e) => {
                    let mut attr_list = vec![];
                    let mut id_opt = None;
                    for a in e.attributes() {
                        let aa = a.unwrap();

                        let key = String::from_utf8(aa.key.into_inner().to_vec()).unwrap();
                        let value = aa.unescape_value().unwrap().into_owned();

                        if &key == "id" {
                            id_opt = Some(value);
                        } else {
                            attr_list.push((key, value.clone()));
                        }
                    }
                    if let Some(id) = id_opt {
                        let elem_name: String =
                            String::from_utf8(e.name().into_inner().to_vec()).unwrap();
                        let mut elem = SvgElement::new(&elem_name, &attr_list);
                        // Expand anything we can given the current context
                        elem.expand_attributes(true, self);
                        elem_map.insert(id.clone(), elem);
                    }
                }
                _ => {}
            }
        }
        self.elem_map = elem_map;
    }

    fn set_indent(&mut self, indent: String) {
        self.last_indent = indent;
    }

    fn eval_ref(&self, attr: &str) -> Option<(f32, f32)> {
        // Example: "#thing@tl" => top left coordinate of element id="thing"
        let re = Regex::new(r"^#(?<id>[^@]+)(@(?<loc>\S+))?$").unwrap();

        let input = String::from(attr);

        let caps = re.captures(&input)?;
        let name = &caps["id"];
        let loc = caps.name("loc").map_or("", |v| v.as_str());
        if loc.is_empty() {
            // find nearest location to us
        }

        let element = self.elem_map.get(name)?;
        element.coord(loc)
    }

    fn closest_loc(&self, this: &SvgElement, point: (f32, f32)) -> String {
        let mut min_dist_sq = f32::MAX;
        let mut min_loc = "c";

        for loc in this.locations() {
            let this_coord = this.coord(loc);
            if let (Some((x1, y1)), (x2, y2)) = (this_coord, point) {
                let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
                if dist_sq < min_dist_sq {
                    min_dist_sq = dist_sq;
                    min_loc = loc;
                }
            }
        }
        min_loc.to_string()
    }

    fn shortest_link(&self, this: &SvgElement, that: &SvgElement) -> (String, String) {
        let mut min_dist_sq = f32::MAX;
        let mut this_min_loc = "c";
        let mut that_min_loc = "c";
        for this_loc in this.locations() {
            for that_loc in that.locations() {
                let this_coord = this.coord(this_loc);
                let that_coord = that.coord(that_loc);
                if let (Some((x1, y1)), Some((x2, y2))) = (this_coord, that_coord) {
                    let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);
                    if dist_sq < min_dist_sq {
                        min_dist_sq = dist_sq;
                        this_min_loc = this_loc;
                        that_min_loc = that_loc;
                    }
                }
            }
        }
        (this_min_loc.to_owned(), that_min_loc.to_owned())
    }

    fn handle_element(&mut self, e: &SvgElement, empty: bool) -> Vec<SvgEvent> {
        let elem_name = &e.name;

        let mut prev_element = self.prev_element.clone();

        let mut omit = false;
        let mut events = vec![];

        let mut e = e.clone();

        e.expand_attributes(false, self);

        if e.is_connector() {
            let conn = Connector::from_element(
                &e,
                self,
                if e.name == "polyline" {
                    ConnectionType::Corner
                } else {
                    ConnectionType::Straight
                },
            );
            for ev in conn.render() {
                events.push(ev);
            }
            // TODO: ensure original element's attrs (e.g. marker-end) get passed through
            omit = true;
        }

        if let Some((orig_elem, text_elements)) = e.process_text_attr() {
            prev_element = Some(e.clone());
            events.push(SvgEvent::Empty(orig_elem));
            events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
            match text_elements.as_slice() {
                [] => {}
                [elem] => {
                    events.push(SvgEvent::Start(elem.clone()));
                    events.push(SvgEvent::Text(elem.clone().content.unwrap()));
                    events.push(SvgEvent::End("text".to_string()));
                }
                _ => {
                    let text_elem = &text_elements[0];
                    events.push(SvgEvent::Start(text_elem.clone()));
                    events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                    for elem in &text_elements[1..] {
                        // Note: we can't insert a newline/last_indent here as whitespace
                        // following a tspan is compressed to a single space and causes
                        // misalignment - see https://stackoverflow.com/q/41364908
                        events.push(SvgEvent::Start(elem.clone()));
                        events.push(SvgEvent::Text(elem.clone().content.unwrap()));
                        events.push(SvgEvent::End("tspan".to_string()));
                    }
                    events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                    events.push(SvgEvent::End("text".to_string()));
                }
            }
            omit = true;
        }

        // Expand any custom element types
        match elem_name.as_str() {
            "tbox" => {
                let mut rect_attrs = vec![];
                let mut text_attrs = vec![];

                let mut text = None;

                for (key, value) in e.clone().attrs {
                    if key == "text" {
                        // allows an empty element to contain text content directly as an attribute
                        text = Some(value);
                    } else {
                        rect_attrs.push((key.clone(), value.clone()));
                    }
                }

                let rect_elem = SvgElement::new("rect", &rect_attrs).add_class("tbox");
                // Assumption is that text should be centered within the rect,
                // and has styling via CSS to reflect this, e.g.:
                //  text.tbox { dominant-baseline: central; text-anchor: middle; }
                let cxy = rect_elem.bbox().unwrap().center().unwrap();
                text_attrs.push(("x".into(), fstr(cxy.0)));
                text_attrs.push(("y".into(), fstr(cxy.1)));
                prev_element = Some(rect_elem.clone());
                events.push(SvgEvent::Empty(rect_elem));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                let text_elem = SvgElement::new("text", &text_attrs).add_class("tbox");
                events.push(SvgEvent::Start(text_elem));
                // if this *isn't* empty, we'll now expect a text event, which will be passed through.
                // the corresponding </tbox> will be converted into a </text> element.
                if empty {
                    if let Some(tt) = text {
                        events.push(SvgEvent::Text(tt.to_string()));
                        events.push(SvgEvent::End("text".to_string()));
                    }
                }
                omit = true;
            }
            "person" => {
                let mut h: f32 = 100.;
                let mut x1: f32 = 0.;
                let mut y1: f32 = 0.;
                let mut common_attrs = vec![];
                let mut head_attrs = vec![];
                let mut body_attrs = vec![];
                let mut arms_attrs = vec![];
                let mut leg1_attrs: Vec<(String, String)> = vec![];
                let mut leg2_attrs: Vec<(String, String)> = vec![];

                for (key, value) in e.clone().attrs {
                    match key.as_str() {
                        "x" => {
                            x1 = value.clone().parse().unwrap();
                        }
                        "y" => {
                            y1 = value.clone().parse().unwrap();
                        }
                        "height" => {
                            h = value.clone().parse().unwrap();
                        }
                        _ => {
                            common_attrs.push((key.clone(), value.clone()));
                        }
                    }
                }

                head_attrs.push(("cx".into(), fstr(x1 + h / 6.)));
                head_attrs.push(("cy".into(), fstr(y1 + h / 9.)));
                head_attrs.push(("r".into(), fstr(h / 9.)));
                head_attrs.push(("style".into(), "fill:none; stroke-width:0.5".into()));
                head_attrs.extend(common_attrs.clone());

                body_attrs.push(("x1".into(), fstr(x1 + h / 6.)));
                body_attrs.push(("y1".into(), fstr(y1 + h / 9. * 2.)));
                body_attrs.push(("x2".into(), fstr(x1 + h / 6.)));
                body_attrs.push(("y2".into(), fstr(y1 + (5.5 * h) / 9.)));
                body_attrs.extend(common_attrs.clone());

                arms_attrs.push(("x1".into(), fstr(x1)));
                arms_attrs.push(("y1".into(), fstr(y1 + h / 3.)));
                arms_attrs.push(("x2".into(), fstr(x1 + h / 3.)));
                arms_attrs.push(("y2".into(), fstr(y1 + h / 3.)));
                arms_attrs.extend(common_attrs.clone());

                leg1_attrs.push(("x1".into(), fstr(x1 + h / 6.)));
                leg1_attrs.push(("y1".into(), fstr(y1 + (5.5 * h) / 9.)));
                leg1_attrs.push(("x2".into(), fstr(x1)));
                leg1_attrs.push(("y2".into(), fstr(y1 + h)));
                leg1_attrs.extend(common_attrs.clone());

                leg2_attrs.push(("x1".into(), fstr(x1 + h / 6.)));
                leg2_attrs.push(("y1".into(), fstr(y1 + (5.5 * h) / 9.)));
                leg2_attrs.push(("x2".into(), fstr(x1 + h / 3.)));
                leg2_attrs.push(("y2".into(), fstr(y1 + h)));
                leg2_attrs.extend(common_attrs.clone());

                events.push(SvgEvent::Empty(
                    SvgElement::new("circle", &head_attrs).add_class("person"),
                ));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                events.push(SvgEvent::Empty(
                    SvgElement::new("line", &body_attrs).add_class("person"),
                ));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                events.push(SvgEvent::Empty(
                    SvgElement::new("line", &arms_attrs).add_class("person"),
                ));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                events.push(SvgEvent::Empty(
                    SvgElement::new("line", &leg1_attrs).add_class("person"),
                ));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));
                events.push(SvgEvent::Empty(
                    SvgElement::new("line", &leg2_attrs).add_class("person"),
                ));
                events.push(SvgEvent::Text(format!("\n{}", self.last_indent)));

                omit = true;
            }
            "pipeline" => {
                let mut x = 0.;
                let mut y = 0.;
                let mut width = 0.;
                let mut height = 0.;
                let mut common_attrs = vec![];
                for (key, value) in e.clone().attrs {
                    match key.as_str() {
                        "x" => {
                            x = value.clone().parse().unwrap();
                        }
                        "y" => {
                            y = value.clone().parse().unwrap();
                        }
                        "height" => {
                            height = value.clone().parse().unwrap();
                        }
                        "width" => {
                            width = value.clone().parse().unwrap();
                        }
                        _ => {
                            common_attrs.push((key.clone(), value.clone()));
                        }
                    }
                }

                if width < height {
                    // Vertical pipeline
                    let w_by2 = width / 2.;
                    let w_by4 = width / 4.;

                    common_attrs.push((
                        "d".to_string(),
                        format!(
                    "M {} {} a {},{} 0 0,0 {},0 a {},{} 0 0,0 -{},0 v {} a {},{} 0 0,0 {},0 v -{}",
                    x, y + w_by4,
                    w_by2, w_by4, width,
                    w_by2, w_by4, width,
                    height - w_by2,
                    w_by2, w_by4, width,
                    height - w_by2),
                    ));
                } else {
                    // Horizontal pipeline
                    let h_by2 = height / 2.;
                    let h_by4 = height / 4.;

                    common_attrs.push((
                        "d".to_string(),
                        format!(
                    "M {} {} a {},{} 0 0,0 0,{} a {},{} 0 0,0 0,-{} h {} a {},{} 0 0,1 0,{} h -{}",
                    x + h_by4, y,
                    h_by4, h_by2, height,
                    h_by4, h_by2, height,
                    width - h_by2,
                    h_by4, h_by2, height,
                    width - h_by2),
                    ));
                }
                events.push(SvgEvent::Empty(
                    SvgElement::new("path", &common_attrs).add_class("pipeline"),
                ));

                omit = true;
                // Since we're omitting the original element we need to set a separate
                // element to act as the previous element for relative positioning
                let bbox = SvgElement::new(
                    "rect",
                    &[
                        ("x".into(), fstr(x)),
                        ("y".into(), fstr(y)),
                        ("width".into(), fstr(width)),
                        ("height".into(), fstr(height)),
                    ],
                );
                prev_element = Some(bbox);
            }
            _ => {}
        }

        if !omit {
            e.expand_attributes(true, self);
            let new_elem = e.clone();
            if empty {
                events.push(SvgEvent::Empty(new_elem.clone()));
            } else {
                events.push(SvgEvent::Start(new_elem.clone()));
            }
            if new_elem.bbox().is_some() {
                // prev_element is only used for relative positioning, so
                // only makes sense if it has a bounding box.
                prev_element = Some(new_elem.clone());
            }
        }
        self.prev_element = prev_element;

        events
    }
}

struct EventList<'a> {
    events: Vec<Event<'a>>,
}

impl EventList<'_> {
    fn from_reader(reader: &mut dyn Read) -> Self {
        let mut reader = Reader::from_reader(BufReader::new(reader));

        let mut events = Vec::new();
        let mut buf = Vec::new();

        loop {
            let ev = reader.read_event_into(&mut buf);
            match &ev {
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Ok(e) => events.push(e.clone().into_owned()),
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            };

            buf.clear();
        }

        Self { events }
    }

    fn write_to(&self, writer: &mut dyn Write) -> Result<(), String> {
        let mut writer = Writer::new(writer);

        for event in &self.events {
            writer.write_event(event).map_err(|e| e.to_string())?
        }
        Ok(())
    }

    fn iter(&self) -> impl Iterator<Item = &Event> + '_ {
        self.events.iter()
    }

    fn to_elements(&self) -> Vec<SvgElement> {
        let mut result = vec![];
        let mut event_iter = self.iter();
        loop {
            match event_iter.next() {
                Some(Event::Empty(e)) | Some(Event::Start(e)) => {
                    result.push(e.into());
                }
                None => {
                    break;
                }
                _ => (),
            }
        }
        result
    }

    fn push(&mut self, ev: &Event) {
        self.events.push(ev.clone().into_owned());
    }
}

#[derive(Default, Debug)]
pub struct Transformer {
    context: TransformerContext,
}

impl Transformer {
    pub fn new() -> Self {
        Self {
            context: TransformerContext::new(),
        }
    }

    pub fn transform(
        &mut self,
        reader: &mut dyn Read,
        writer: &mut dyn Write,
    ) -> Result<(), String> {
        let input = EventList::from_reader(reader);
        let mut output = EventList { events: vec![] };

        self.context.populate(&input);

        for ev in input.iter() {
            match ev {
                Event::Eof => {
                    // should never happen, as handled in EventList::from_reader()
                    break;
                }
                Event::Start(e) | Event::Empty(e) => {
                    let is_empty = matches!(ev, Event::Empty(_));
                    let mut repeat = 1;
                    let mut event_element = SvgElement::from(e);
                    if let Some(rep_count) = event_element.pop_attr("repeat") {
                        if is_empty {
                            repeat = rep_count.parse().unwrap_or(1);
                        } else {
                            todo!("Repeat is not implemented for non-empty elements");
                        }
                    }
                    for rep_idx in 0..repeat {
                        transform_element(&event_element, &mut self.context, &mut output, is_empty);

                        if rep_idx < (repeat - 1) {
                            output.push(&Event::Text(BytesText::new(&format!(
                                "\n{}",
                                self.context.last_indent
                            ))));
                        }
                    }
                }
                Event::End(e) => {
                    let mut ee_name = String::from_utf8(e.name().as_ref().to_vec()).unwrap();
                    if ee_name.as_str() == "tbox" {
                        ee_name = String::from("text");
                    }
                    output.push(&Event::End(BytesEnd::new(ee_name)))
                }
                Event::Text(e) => {
                    // Extract any trailing whitespace following newlines as the current indentation level
                    let re = Regex::new(r"(?ms)\n.*^(\s+)*").unwrap();
                    let text = String::from_utf8(e.to_vec()).expect("Non-UTF8 in input file");
                    if let Some(captures) = re.captures(&text) {
                        let indent = captures
                            .get(1)
                            .map_or(String::from(""), |m| m.as_str().into());
                        self.context.set_indent(indent);
                    }

                    output.push(&Event::Text(e.borrow()))
                }
                _ => {
                    output.push(ev);
                }
            }
        }

        // Calculate bounding box of diagram and use as new viewBox for the image.
        // This also allows just using `<svg>` as the root element.
        // TODO: preserve other attributes from a given svg root.
        let mut extent = BoundingBox::new();
        for el in output.to_elements() {
            if let Some(bb) = &el.bbox() {
                extent.extend(bb);
            }
        }
        // Expand by 10, then add 10%. Ensures small images get more than a couple
        // of pixels border, and large images still get a (relatively) decent border.
        extent.expand(10.);
        extent.scale(1.1);

        if let BoundingBox::BBox(minx, miny, maxx, maxy) = extent {
            let width = fstr(maxx - minx);
            let height = fstr(maxy - miny);
            let mut bs = BytesStart::new("svg");
            bs.push_attribute(Attribute::from(("version", "1.1")));
            bs.push_attribute(Attribute::from(("xmlns", "http://www.w3.org/2000/svg")));
            bs.push_attribute(Attribute::from(("width", format!("{width}mm").as_str())));
            bs.push_attribute(Attribute::from(("height", format!("{height}mm").as_str())));
            bs.push_attribute(Attribute::from((
                "viewBox",
                format!("{} {} {} {}", fstr(minx), fstr(miny), width, height).as_str(),
            )));
            let new_svg = Event::Start(bs);

            for item in &mut output.events.iter_mut() {
                if let Event::Start(x) = item {
                    if x.name().into_inner() == "svg".as_bytes() {
                        *item = new_svg.clone().into_owned();
                        break;
                    }
                }
            }
        }

        output.write_to(writer).map_err(|e| e.to_string())
    }
}

fn transform_element(
    element: &SvgElement,
    context: &mut TransformerContext,
    output: &mut EventList,
    is_empty: bool,
) {
    let ee = context.handle_element(element, is_empty);
    for svg_ev in ee.into_iter() {
        // re-calculate is_empty for each generated event
        let is_empty = matches!(svg_ev, SvgEvent::Empty(_));
        match svg_ev {
            SvgEvent::Empty(e) | SvgEvent::Start(e) => {
                let mut bs = BytesStart::new(e.name);
                // Collect non-'class' attributes
                for (k, v) in e.attrs.into_iter() {
                    if k != "class" {
                        bs.push_attribute(Attribute::from((k.as_bytes(), v.as_bytes())));
                    }
                }
                // Any 'class' attribute values are stored separately as a HashSet;
                // collect those into the BytesStart object
                if !e.classes.is_empty() {
                    bs.push_attribute(Attribute::from((
                        "class".as_bytes(),
                        e.classes
                            .into_iter()
                            .collect::<Vec<String>>()
                            .join(" ")
                            .as_bytes(),
                    )));
                }
                if is_empty {
                    output.push(&Event::Empty(bs));
                } else {
                    output.push(&Event::Start(bs));
                }
            }
            SvgEvent::Text(t) => {
                output.push(&Event::Text(BytesText::new(&t)));
            }
            SvgEvent::End(name) => {
                output.push(&Event::End(BytesEnd::new(name)));
            }
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

    #[test]
    fn test_connector() {
        let c = Connector::new(
            Endpoint::new((10., 10.), Some(Direction::Right)),
            Endpoint::new((20., 20.), Some(Direction::Up)),
            ConnectionType::Corner,
        );
        println!("{:?}", c.render().collect::<Vec<_>>());
    }
}
