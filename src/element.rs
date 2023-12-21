use crate::expression::eval_attr;
pub(crate) use crate::transform::TransformerContext;
use crate::types::{AttrMap, BoundingBox, ClassList};
use crate::{attr_split, attr_split_cycle, fstr, strp, strp_length, Length};
use anyhow::{bail, Context, Result};
use core::fmt::Display;
use regex::Regex;

#[derive(Clone, Debug)]
pub(crate) struct SvgElement {
    pub name: String,
    pub attrs: AttrMap,
    pub classes: ClassList,
    pub content: Option<String>,
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
    pub fn new(name: &str, attrs: &[(String, String)]) -> Self {
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

    pub fn add_class(&mut self, class: &str) -> Self {
        self.classes.insert(class.to_string());
        self.clone()
    }

    pub fn add_classes(&mut self, classes: &ClassList) {
        for class in classes {
            self.add_class(class);
        }
    }

    pub fn has_attr(&self, key: &str) -> bool {
        self.attrs.contains_key(key)
    }

    pub fn add_attr(&mut self, key: &str, value: &str) {
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
    pub fn without_attr(&self, key: &str) -> Self {
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
    pub fn with_attrs_from(&self, other: &SvgElement) -> Self {
        let mut attrs = self.attrs.clone();
        for (k, v) in &other.attrs {
            attrs.insert(k, v);
        }
        let mut element = SvgElement::new(self.name.as_str(), &attrs.to_vec());
        element.add_classes(&other.classes);
        element
    }

    pub fn pop_attr(&mut self, key: &str) -> Option<String> {
        self.attrs.remove(key)
    }

    pub fn get_attr(&self, key: &str) -> Option<String> {
        self.attrs.get(key).map(|x| x.to_owned())
    }

    fn set_attr(&mut self, key: &str, value: &str) {
        self.attrs.insert(key, value);
    }

    pub fn is_connector(&self) -> bool {
        self.has_attr("start")
            && self.has_attr("end")
            && (self.name == "line" || self.name == "polyline")
    }

    pub fn bbox(&self) -> Result<Option<BoundingBox>> {
        // For SVG 'Basic shapes' (e.g. rect, circle, ellipse, etc) for x/y and similar:
        // "If the attribute is not specified, the effect is as if a value of "0" were specified."
        // The same is not specified for 'size' attributes (width/height/r etc), so we require
        // these to be set to have a bounding box.
        let zstr = "0".to_owned();
        match self.name.as_str() {
            "rect" | "tbox" | "pipeline" => {
                if let (Some(w), Some(h)) = (self.attrs.get("width"), self.attrs.get("height")) {
                    let x = strp(self.attrs.get("x").unwrap_or(&zstr))?;
                    let y = strp(self.attrs.get("y").unwrap_or(&zstr))?;
                    let w = strp(w)?;
                    let h = strp(h)?;
                    Ok(Some(BoundingBox::BBox(x, y, x + w, y + h)))
                } else {
                    Ok(None)
                }
            }
            "line" => {
                let x1 = strp(self.attrs.get("x1").unwrap_or(&zstr))?;
                let y1 = strp(self.attrs.get("y1").unwrap_or(&zstr))?;
                let x2 = strp(self.attrs.get("x2").unwrap_or(&zstr))?;
                let y2 = strp(self.attrs.get("y2").unwrap_or(&zstr))?;
                Ok(Some(BoundingBox::BBox(
                    x1.min(x2),
                    y1.min(y2),
                    x1.max(x2),
                    y1.max(y2),
                )))
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

                if let Some(points) = self.attrs.get("points") {
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
                        Ok(Some(BoundingBox::BBox(min_x, min_y, max_x, max_y)))
                    } else {
                        bail!("Insufficient points for bbox")
                    }
                } else {
                    Ok(None)
                }
            }
            "circle" => {
                if let Some(r) = self.attrs.get("r") {
                    let cx = strp(self.attrs.get("cx").unwrap_or(&zstr))?;
                    let cy = strp(self.attrs.get("cy").unwrap_or(&zstr))?;
                    let r = strp(r)?;
                    Ok(Some(BoundingBox::BBox(cx - r, cy - r, cx + r, cy + r)))
                } else {
                    Ok(None)
                }
            }
            "ellipse" => {
                if let (Some(rx), Some(ry)) = (self.attrs.get("rx"), self.attrs.get("ry")) {
                    let cx = strp(self.attrs.get("cx").unwrap_or(&zstr))?;
                    let cy = strp(self.attrs.get("cy").unwrap_or(&zstr))?;
                    let rx = strp(rx)?;
                    let ry = strp(ry)?;
                    Ok(Some(BoundingBox::BBox(cx - rx, cy - ry, cx + rx, cy + ry)))
                } else {
                    Ok(None)
                }
            }
            "person" => {
                if let (Some(x), Some(y), Some(h)) = (
                    self.attrs.get("x"),
                    self.attrs.get("y"),
                    self.attrs.get("height"),
                ) {
                    let x = strp(x)?;
                    let y = strp(y)?;
                    let h = strp(h)?;
                    Ok(Some(BoundingBox::BBox(x, y, x + h / 3., y + h)))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    pub fn coord(&self, loc: &str) -> Result<Option<(f32, f32)>> {
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
        if let Some(BoundingBox::BBox(x1, y1, x2, y2)) = self.bbox()? {
            let tl = (x1, y1);
            let tr = (x2, y1);
            let br = (x2, y2);
            let bl = (x1, y2);
            let c = ((x1 + x2) / 2., (y1 + y2) / 2.);
            Ok(match loc {
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
                        if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                            self.attrs.get("x1"),
                            self.attrs.get("y1"),
                            self.attrs.get("x2"),
                            self.attrs.get("y2"),
                        ) {
                            let x1 = strp(x1)?;
                            let y1 = strp(y1)?;
                            let x2 = strp(x2)?;
                            let y2 = strp(y2)?;
                            match loc {
                                "xy1" | "start" => Some((x1, y1)),
                                "xy2" | "end" => Some((x2, y2)),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            })
        } else {
            Ok(None)
        }
    }

    pub fn translated(&self, dx: f32, dy: f32) -> Self {
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

    pub fn resized_by(&self, dw: Length, dh: Length) -> Self {
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

    fn eval_pos(&mut self, input: &str, context: &TransformerContext) -> Result<String> {
        // Relative positioning:
        //   ID LOC DX DY
        // [#id][@loc] [dx] [dy]
        //   or
        // ^(h|v|H|V) [gap]

        // Defaults:
        //   #id = previous element
        //   @loc = tr
        //   dx, dy - offset from the @loc
        // Examples:
        //   xy="^h 10"      - position to right of previous element with gap of 10
        //   xy="^H"         - position immediately to left of previous element
        //   xy="^v"         - position immediately below previous element
        //   xy="^V 5"       - position immediately previous element
        //   xy="@tr 10 0"   - position to right of previous element with gap of 10
        //   cxy="@b"        - position centre at bottom of previous element
        // TODO - extend to allow referencing earlier elements beyond previous
        let rel_re = Regex::new(r"^(\^[hvHV]|(?<id>#[^@]+)?(?<loc>@\S+)?)").expect("Bad Regex");
        let mut parts = attr_split(input);
        let ref_loc = parts.next().context("Empty attribute in eval_pos()")?;
        if let Some(caps) = rel_re.captures(&ref_loc) {
            if caps.get(0).unwrap().is_empty() {
                // We need either id or loc or both; since they are both optional in
                // the regex we check that we did actually match some text here...
                return Ok(input.to_owned());
            }

            // For ^h etc, we only consider the first number as a 'gap' value.
            // For other relative specs these are dx/dy.
            let d1 = strp(&parts.next().unwrap_or("0".to_owned()));
            let d2 = strp(&parts.next().unwrap_or("0".to_owned()));
            let mut dx = 0.;
            let mut dy = 0.;

            let default_rel = match ref_loc.as_str() {
                "^h" => {
                    dx = d1.context(format!(r#"{ref_loc} gap error("{input}")"#))?;
                    "r"
                }
                "^H" => {
                    dx = -d1.context(format!(r#"{ref_loc} gap error("{input}")"#))?;
                    "l"
                }
                "^v" => {
                    dy = d1.context(format!(r#"{ref_loc} gap error("{input}")"#))?;
                    "b"
                }
                "^V" => {
                    dy = -d1.context(format!(r#"{ref_loc} gap error("{input}")"#))?;
                    "t"
                }
                _ => {
                    dx = d1.context(format!(r#"Could not determine dx in eval_pos("{input}")"#))?;
                    dy = d2.context(format!(r#"Could not determine dy in eval_pos("{input}")"#))?;
                    "tr"
                }
            };

            // This is similar to the more generic `xy-loc` processing.
            // assumes the bounding-box is well-defined by this point.
            if let Some(bbox) = self.bbox()? {
                let width = bbox.width().unwrap();
                let height = bbox.height().unwrap();
                let (xy_dx, xy_dy) = match default_rel {
                    "b" => (width / 2., 0.),
                    "l" => (width, height / 2.),
                    "t" => (width / 2., height),
                    "r" => (0., height / 2.),
                    _ => (0., 0.),
                };
                dx -= xy_dx;
                dy -= xy_dy;
            }

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
                if let Some(pos) = ref_el.coord(loc)? {
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
        //   dw% / dh% - scaled width/height
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
    pub fn expand_attributes(
        &mut self,
        simple: bool,
        context: &mut TransformerContext,
    ) -> Result<()> {
        let mut new_attrs = AttrMap::new();

        for (key, value) in self.attrs.clone() {
            let replace = eval_attr(&value, &context.variables, &context.elem_map);
            self.attrs.insert(&key, &replace);
        }

        // In the following steps, every attribute is either replaced by one
        // or more other attributes, or copied as-is into `new_attrs`.

        // Step 1: Evaluate size from wh attributes
        for (key, value) in self.attrs.clone() {
            let mut value = value.clone();
            if !simple && key.as_str() == "wh" {
                // TODO: support rxy for ellipses, with scaling factor
                value = self.eval_size(value.as_str(), context)?;
            }
            let mut parts = attr_split_cycle(&value);
            match (key.as_str(), self.name.as_str()) {
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
                _ => new_attrs.insert(key.clone(), value.clone()),
            }
        }

        self.attrs = new_attrs;
        let mut new_attrs = AttrMap::new();

        // Step 2: Evaluate position
        for (key, value) in self.attrs.clone() {
            let mut value = value.clone();
            if !simple {
                match key.as_str() {
                    "xy" | "cxy" | "xy1" | "xy2" => {
                        // TODO: maybe split up? pos may depende on size, but size doesn't depend on pos
                        value = self.eval_pos(value.as_str(), context)?;
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
            // Note first-pass expansion must be assumed to have occurred, e.g.
            // there will no longer be a "wh" attribute for element types where this
            // is expanded in the first pass.
            let mut pass_two_attrs = AttrMap::new();
            for (key, value) in new_attrs.clone() {
                let mut parts = attr_split_cycle(&value);
                match (key.as_str(), self.name.as_str()) {
                    ("cxy", "rect" | "tbox" | "pipeline") => {
                        // Requires width / height to be specified in order to evaluate
                        // the centre point.
                        // TODO: also support specifying other attributes; xy+cxy should be sufficient
                        let width = new_attrs.get("width").map(|z| strp(z).unwrap());
                        let height = new_attrs.get("height").map(|z| strp(z).unwrap());
                        let cx = strp(&parts.next().expect("cycle"))
                            .context("Could not derive cx from cxy")?;
                        let cy = strp(&parts.next().expect("cycle"))
                            .context("Could not derive cy from cxy")?;
                        if let (Some(width), Some(height)) = (width, height) {
                            pass_two_attrs.insert("x", fstr(cx - width / 2.));
                            pass_two_attrs.insert("y", fstr(cy - height / 2.));
                        }
                    }
                    ("xy", "circle") => {
                        // Requires xy / r
                        let r = new_attrs.get("r").map(|z| strp(z).unwrap());
                        let x = strp(&parts.next().expect("cycle"))
                            .context("Could not derive x from xy")?;
                        let y = strp(&parts.next().expect("cycle"))
                            .context("Could not derive y from xy")?;
                        if let Some(r) = r {
                            pass_two_attrs.insert("cx", fstr(x + r));
                            pass_two_attrs.insert("cy", fstr(y + r));
                        }
                    }
                    ("xy", "ellipse") => {
                        // Requires xy / rx / ry
                        let rx = new_attrs.get("rx").map(|z| strp(z).unwrap());
                        let ry = new_attrs.get("ry").map(|z| strp(z).unwrap());
                        let x = strp(&parts.next().expect("cycle"))
                            .context("Could not derive x from xy")?;
                        let y = strp(&parts.next().expect("cycle"))
                            .context("Could not derive y from xy")?;
                        if let (Some(rx), Some(ry)) = (rx, ry) {
                            pass_two_attrs.insert("cx", fstr(x + rx));
                            pass_two_attrs.insert("cy", fstr(y + ry));
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
