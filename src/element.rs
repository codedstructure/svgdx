use crate::expression::eval_attr;
use crate::path::path_bbox;
use crate::transform::TransformerContext;
use crate::types::{
    attr_split, attr_split_cycle, fstr, strp, strp_length, AttrMap, BoundingBox, ClassList,
    DirSpec, EdgeSpec, Length, LocSpec,
};
use anyhow::{bail, Context, Result};
use core::fmt::Display;
use lazy_regex::regex;
use regex::Captures;

/// Replace all refspec entries in a string with lookup results
/// Suitable for use with path `d` or polyline `points` attributes
/// which may contain many such entries.
///
/// Infallible; any invalid refspec will be left unchanged.
fn expand_relspec(value: &str, context: &TransformerContext) -> String {
    let locspec = regex!(r"#(?<id>[[:word:]]+)@(?<loc>[[:word:]]+)");

    let result = locspec.replace_all(value, |caps: &Captures| {
        let elref = caps.name("id").expect("Regex Match").as_str();
        let loc = caps.name("loc").expect("Regex Match").as_str();
        if let Some(elem) = context.get_element(elref) {
            if let Ok(Some(pos)) = elem.coord(loc) {
                format!("{} {}", fstr(pos.0), fstr(pos.1))
            } else {
                value.to_string()
            }
        } else {
            value.to_string()
        }
    });

    result.to_string()
}

#[derive(Debug, Clone)]
pub enum ContentType {
    /// This element is empty, therefore *can't* have any content
    Empty,
    /// This element will have content but it isn't known yet
    Pending,
    /// This element has content and it's ready to be used
    Ready(String),
}

impl ContentType {
    pub fn is_pending(&self) -> bool {
        matches!(self, ContentType::Pending)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, ContentType::Ready(_))
    }
}

#[derive(Clone, Debug)]
pub struct SvgElement {
    pub name: String,
    pub attrs: AttrMap,
    pub classes: ClassList,
    pub content: ContentType,
    pub tail: Option<String>,
    pub order_index: usize,
    pub indent: usize,
    pub src_line: usize,
}

impl Display for SvgElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{}", self.name)?;
        if !self.attrs.is_empty() {
            write!(f, " {}", self.attrs)?;
        }
        if !self.classes.is_empty() {
            write!(f, r#" class="{}""#, self.classes.to_vec().join(" "))?;
        }
        write!(f, ">")?;
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
            content: ContentType::Empty,
            tail: None,
            order_index: 0,
            indent: 0,
            src_line: 0,
        }
    }

    pub fn set_indent(&mut self, indent: usize) {
        self.indent = indent;
    }

    pub fn set_src_line(&mut self, line: usize) {
        self.src_line = line;
    }

    pub fn set_order_index(&mut self, order_index: usize) {
        self.order_index = order_index;
    }

    pub fn set_tail(&mut self, tail: &str) {
        self.tail = Some(tail.to_string());
    }

    pub fn add_class(&mut self, class: &str) -> Self {
        self.classes.insert(class.to_string());
        self.clone()
    }

    pub fn add_classes(&mut self, classes: &ClassList) {
        for class in classes {
            // update classes directly rather than use add_class which
            // performs a clone() operation.
            self.classes.insert(class.to_string());
        }
    }

    pub fn has_attr(&self, key: &str) -> bool {
        self.attrs.contains_key(key)
    }

    fn replace_attrs(&mut self, attrs: AttrMap) {
        self.attrs = attrs;
    }

    #[allow(dead_code)]
    #[must_use]
    fn with_attr(&self, key: &str, value: &str) -> Self {
        let mut element = self.clone();
        element.set_attr(key, value);
        element
    }

    #[must_use]
    pub fn without_attr(&self, key: &str) -> Self {
        let attrs: Vec<(String, String)> = self
            .attrs
            .clone()
            .into_iter()
            .filter(|(k, _v)| k != key)
            .collect();
        let mut element = self.clone();
        element.replace_attrs(attrs.into());
        element
    }

    /// copy attributes, classes and indentation from another element,
    /// returning the merged element
    #[must_use]
    pub fn with_attrs_from(&self, other: &Self) -> Self {
        let mut attrs = self.attrs.clone();
        for (k, v) in &other.attrs {
            attrs.insert(k, v);
        }
        let mut element = other.clone();
        element.replace_attrs(attrs);
        // Everything but the name and any attrs unique to the original element
        // is from the other element.
        element.name = self.name.clone();
        element
    }

    pub fn pop_attr(&mut self, key: &str) -> Option<String> {
        self.attrs.pop(key)
    }

    pub fn get_attr(&self, key: &str) -> Option<String> {
        self.attrs.get(key).map(|x| x.to_owned())
    }

    pub fn set_attr(&mut self, key: &str, value: &str) {
        self.attrs.insert(key, value);
    }

    pub fn is_phantom_element(&self) -> bool {
        matches!(self.name.as_str(), "config" | "specs" | "var")
    }

    /// See https://www.w3.org/TR/SVG11/intro.html#TermGraphicsElement
    /// Note `reuse` is not a standard SVG element, but is used here in similar
    /// contexts to the `use` element.
    pub fn is_graphics_element(&self) -> bool {
        matches!(
            self.name.as_str(),
            "circle"
                | "ellipse"
                | "image"
                | "line"
                | "path"
                | "polygon"
                | "polyline"
                | "rect"
                | "text"
                | "use"
                | "reuse"
        )
    }

    /// See https://www.w3.org/TR/SVG11/intro.html#TermContainerElement
    /// Note `specs` is not a standard SVG element, but is used here in similar
    /// contexts to the `defs` element.
    #[allow(dead_code)]
    pub fn is_container_element(&self) -> bool {
        matches!(
            self.name.as_str(),
            "a" | "defs"
                | "glyph"
                | "g"
                | "marker"
                | "mask"
                | "missing-glyph"
                | "pattern"
                | "svg"
                | "switch"
                | "symbol"
                | "specs"
        )
    }

    pub fn is_empty_element(&self) -> bool {
        matches!(self.content, ContentType::Empty)
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
            "text" => {
                let x = strp(self.attrs.get("x").unwrap_or(&zstr))?;
                let y = strp(self.attrs.get("y").unwrap_or(&zstr))?;
                Ok(Some(BoundingBox::new(x, y, x, y)))
            }
            "rect" => {
                if let (Some(w), Some(h)) = (self.attrs.get("width"), self.attrs.get("height")) {
                    let x = strp(self.attrs.get("x").unwrap_or(&zstr))?;
                    let y = strp(self.attrs.get("y").unwrap_or(&zstr))?;
                    let w = strp(w)?;
                    let h = strp(h)?;
                    Ok(Some(BoundingBox::new(x, y, x + w, y + h)))
                } else {
                    Ok(None)
                }
            }
            "line" => {
                let x1 = strp(self.attrs.get("x1").unwrap_or(&zstr))?;
                let y1 = strp(self.attrs.get("y1").unwrap_or(&zstr))?;
                let x2 = strp(self.attrs.get("x2").unwrap_or(&zstr))?;
                let y2 = strp(self.attrs.get("y2").unwrap_or(&zstr))?;
                Ok(Some(BoundingBox::new(
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
                        Ok(Some(BoundingBox::new(min_x, min_y, max_x, max_y)))
                    } else {
                        bail!("Insufficient points for bbox")
                    }
                } else {
                    Ok(None)
                }
            }
            "path" => Ok(Some(path_bbox(self)?)),
            "circle" => {
                if let Some(r) = self.attrs.get("r") {
                    let cx = strp(self.attrs.get("cx").unwrap_or(&zstr))?;
                    let cy = strp(self.attrs.get("cy").unwrap_or(&zstr))?;
                    let r = strp(r)?;
                    Ok(Some(BoundingBox::new(cx - r, cy - r, cx + r, cy + r)))
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
                    Ok(Some(BoundingBox::new(cx - rx, cy - ry, cx + rx, cy + ry)))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    /// Get point from a string such as 'tl' (top-left of this element) or
    /// 'r:30%' (30% down the right edge).
    ///
    /// TODO: should also support e.g. `start:10%` for a line etc
    ///
    /// TODO: support per-shape locs - e.g. "in" / "out" for pipeline
    ///
    /// Return `Err` if invalid string format, `Ok(None)` if no bounding box,
    /// else `Ok(Some(coord))`
    pub fn coord(&self, loc: &str) -> Result<Option<(f32, f32)>> {
        let mut loc = loc;
        let mut len = Length::Ratio(0.5);
        let re = regex!(r"(?<loc>[^:\s]+)(:(?<len>[-0-9\.]+%?))?$");
        if let Some(caps) = re.captures(loc) {
            loc = caps.name("loc").expect("Regex Match").as_str();
            len = caps
                .name("len")
                .map_or(len, |v| strp_length(v.as_str()).expect("Invalid length"));
        }
        if let Some(bb) = self.bbox()? {
            if let Ok(edge) = EdgeSpec::try_from(loc) {
                Ok(Some(bb.edgespec(edge, len)))
            } else if let Ok(loc) = LocSpec::try_from(loc) {
                Ok(Some(bb.locspec(loc)))
            } else {
                bail!("Invalid locspec in coord")
            }
        } else {
            Ok(None)
        }
    }

    pub fn translated(&self, dx: f32, dy: f32) -> Result<Self> {
        let mut new_elem = self.clone();
        for (key, value) in &self.attrs {
            match key.as_str() {
                "x" | "cx" | "x1" | "x2" => {
                    new_elem.set_attr(key, &fstr(strp(value)? + dx));
                }
                "y" | "cy" | "y1" | "y2" => {
                    new_elem.set_attr(key, &fstr(strp(value)? + dy));
                }
                "points" => {
                    let mut values = vec![];
                    for (idx, part) in attr_split(value).enumerate() {
                        values.push(fstr(strp(&part)? + if idx % 2 == 0 { dx } else { dy }));
                    }
                    new_elem.set_attr(key, &values.join(" "));
                }
                _ => (),
            }
        }
        Ok(new_elem)
    }

    pub fn resized_by(&self, dw: Length, dh: Length) -> Result<Self> {
        let mut new_elem = self.clone();
        for (key, value) in &self.attrs {
            match key.as_str() {
                "width" => {
                    new_elem.set_attr(key, &fstr(dw.adjust(strp(value)?)));
                }
                "height" => {
                    new_elem.set_attr(key, &fstr(dh.adjust(strp(value)?)));
                }
                _ => (),
            }
        }
        Ok(new_elem)
    }

    pub fn position_from_bbox(&mut self, bb: &BoundingBox) {
        let width = bb.width();
        let height = bb.height();
        let (cx, cy) = bb.center();
        let (x1, y1) = bb.locspec(LocSpec::TopLeft);
        match self.name.as_str() {
            "rect" => {
                self.attrs.insert("x", fstr(x1));
                self.attrs.insert("y", fstr(y1));
                self.attrs.insert("width", fstr(width));
                self.attrs.insert("height", fstr(height));
            }
            "circle" => {
                self.attrs.insert("cx", fstr(cx));
                self.attrs.insert("cy", fstr(cy));
                self.attrs
                    .insert("r", fstr(0.5 * width.max(height) * 1.414));
            }
            "ellipse" => {
                self.attrs.insert("cx", fstr(cx));
                self.attrs.insert("cy", fstr(cy));
                self.attrs.insert("rx", fstr(0.5 * width * 1.414));
                self.attrs.insert("ry", fstr(0.5 * height * 1.414));
            }
            _ => {}
        }
    }

    /// Convert a (potentially) relspec position string to an absolute position string.
    ///
    /// If it doesn't look like a relspec, it is returned unchanged; if it looks like a
    /// relspec but can't be parsed, an error is returned.
    ///
    /// Examples:
    ///
    /// Direction relative positioning - horizontally below, above, to the left, or to the
    /// right of the referenced element.
    /// ```text
    /// (^|#id)(:(h|H|v|V) [gap])
    /// ```
    ///
    /// Location-based positioning - relative to a specific location on the reference element
    /// ```text
    /// (^|#id)[@loc] [dx] [dy]
    /// ```
    ///
    /// Edge-based positioning - relative to a specific location on the reference element
    /// ```text
    /// (^|#id)[@edge][:length] [dx] [dy]
    /// ```
    ///
    /// `input`: the relspec string
    ///
    /// `anchor`: the anchor point for the position. This is only relevant for dirspec
    /// (e.g. `^:h`) and bare elref (e.g. `#id`), not for locspec or edgespec.
    ///
    /// `context`: the transformer context, used for lookup of the reference element.
    fn eval_pos(
        &self,
        input: &str,
        anchor: LocSpec,
        context: &TransformerContext,
    ) -> Result<String> {
        let input = input.trim();
        let (ref_el, remain) = self.split_relspec(input, context)?;
        let ref_el = match ref_el {
            Some(el) => el,
            None => return Ok(input.to_owned()),
        };

        if let Some(bbox) = ref_el.bbox()? {
            self.eval_pos_helper(remain, &bbox, anchor)
        } else {
            Ok(input.to_owned())
        }
    }

    /// Split a possible relspec into the reference element (if it exists) and remainder
    ///
    /// `input`: the relspec string
    ///
    /// If the input is not a relspec, the reference element is `None` and the remainder
    /// is the entire input string.
    fn split_relspec<'a, 'b>(
        &self,
        input: &'b str,
        context: &'a TransformerContext,
    ) -> Result<(Option<&'a SvgElement>, &'b str)> {
        if input.starts_with('^') {
            let skip_prev = input.strip_prefix('^').unwrap_or(input);
            Ok((context.get_prev_element(), skip_prev.trim_start()))
        } else if input.starts_with('#') {
            let extract_ref_id_re = regex!(r"^#(?<id>[[:word:]]+)\s*(?<remain>.*)$");
            let (id, remain) = extract_ref_id_re
                .captures(input)
                .map(|caps| {
                    (
                        caps.name("id").expect("Regex Match").as_str(),
                        caps.name("remain").expect("Regex Match").as_str(),
                    )
                })
                .unwrap_or(("INVALID ELEMENT ID", input));
            if let Some(el) = context.get_element(id) {
                Ok((Some(el), remain))
            } else {
                bail!("Reference to unknown element '{}'", id);
            }
        } else {
            Ok((None, input))
        }
    }

    /// Extract dx/dy from a string such as '10 20' or '10' (in which case dy is 0)
    fn extract_dx_dy(&self, input: &str) -> Result<(f32, f32)> {
        let mut parts = attr_split(input);
        let dx = strp(&parts.next().unwrap_or("0".to_string()))?;
        let dy = strp(&parts.next().unwrap_or("0".to_string()))?;
        Ok((dx, dy))
    }

    // This is split out for testability
    fn eval_pos_helper(&self, remain: &str, bbox: &BoundingBox, anchor: LocSpec) -> Result<String> {
        let rel_re = regex!(r"^:(?<rel>[hHvV])(\s+(?<remain>.*))?$");
        let loc_re = regex!(r"^@(?<loc>[trblc]+)(\s+(?<remain>.*))?$");
        let edge_re = regex!(r"^@(?<edge>[trbl]):(?<len>[-0-9\.]+%?)(\s+(?<remain>.*))?$");
        if let Some((x, y)) = if let Some(caps) = rel_re.captures(remain) {
            let rel = DirSpec::try_from(caps.name("rel").expect("Regex Match").as_str())?;
            let this_bbox = self.bbox()?;
            let this_width = this_bbox.map(|bb| bb.width()).unwrap_or(0.);
            let this_height = this_bbox.map(|bb| bb.height()).unwrap_or(0.);
            let gap = if let Some(remain) = caps.name("remain") {
                let mut parts = attr_split(remain.as_str());
                strp(&parts.next().unwrap_or("0".to_string()))?
            } else {
                0.
            };
            let (x, y) = bbox.locspec(rel.to_locspec());
            let (mut dx, mut dy) = match rel {
                DirSpec::Above => (-this_width / 2., -(this_height + gap)),
                DirSpec::Below => (-this_width / 2., gap),
                DirSpec::InFront => (gap, -this_height / 2.),
                DirSpec::Behind => (-(this_width + gap), -this_height / 2.),
            };
            if let LocSpec::Center = anchor {
                dx += this_width / 2.;
                dy += this_height / 2.;
            }
            Some((x + dx, y + dy))
        } else if let Some(caps) = edge_re.captures(remain) {
            let edge = EdgeSpec::try_from(caps.name("edge").expect("Regex Match").as_str())?;
            let length = strp_length(caps.name("len").expect("Regex Match").as_str())?;
            let (dx, dy) = if let Some(remain) = caps.name("remain") {
                self.extract_dx_dy(remain.as_str())?
            } else {
                (0., 0.)
            };
            let (x, y) = bbox.edgespec(edge, length);
            Some((x + dx, y + dy))
        } else if let Some(caps) = loc_re.captures(remain) {
            let loc = LocSpec::try_from(caps.name("loc").expect("Regex Match").as_str())?;
            let (dx, dy) = if let Some(remain) = caps.name("remain") {
                self.extract_dx_dy(remain.as_str())?
            } else {
                (0., 0.)
            };
            let (x, y) = bbox.locspec(loc);
            Some((x + dx, y + dy))
        } else if let Ok((dx, dy)) = self.extract_dx_dy(remain) {
            let (x, y) = bbox.locspec(anchor);
            Some((x + dx, y + dy))
        } else {
            None
        } {
            return Ok(format!("{} {}", fstr(x), fstr(y)));
        }

        Ok(remain.to_owned())
    }

    /// Convert a (potentially) relspec size string to an absolute (width, height) string.
    ///
    /// If it doesn't look like a relspec, it is returned unchanged; if it looks like a
    /// relspec but can't be parsed, an error is returned.
    ///
    /// Examples:
    ///
    ///   (#id|^) [DW[%] DH[%]]
    /// Meaning:
    ///   #id - reference to size of another element
    ///   ^ - reference to previous element
    ///   dw / dh - delta width/height (user units; may be negative)
    ///   dw% / dh% - scaled width/height
    ///
    /// `input`: the relspec string
    ///
    /// `context`: the transformer context, used for lookup of the reference element.
    fn eval_size(&self, input: &str, context: &TransformerContext) -> Result<String> {
        let input = input.trim();
        let (ref_el, remain) = self.split_relspec(input, context)?;
        let ref_el = match ref_el {
            Some(el) => el,
            None => return Ok(input.to_owned()),
        };

        let mut parts = attr_split(remain);
        let dw = parts.next().unwrap_or("0".to_owned());
        let dh = parts.next().unwrap_or("0".to_owned());

        if let (Some(w), Some(h)) = (ref_el.get_attr("width"), ref_el.get_attr("height")) {
            let w = strp(&w).context(r#"Could not derive width in eval_size("{input}")"#)?;
            let h = strp(&h).context(r#"Could not derive height in eval_size("{input}")"#)?;
            let dw = strp_length(&dw).context(r#"Could not derive dw in eval_size("{input"})"#)?;
            let dh = strp_length(&dh).context(r#"Could not derive dh in eval_size("{input"})"#)?;
            let w = fstr(dw.adjust(w));
            let h = fstr(dh.adjust(h));

            return Ok(format!("{w} {h}"));
        }
        Ok(input.to_owned())
    }

    /// Process and expand attributes as needed
    pub fn expand_attributes(&mut self, context: &mut TransformerContext) -> Result<()> {
        // Step 0: Resolve any attributes
        for (key, value) in self.attrs.clone() {
            let replace = eval_attr(&value, context);
            self.attrs.insert(&key, &replace);
        }

        // Step 1: Evaluate size from wh attributes
        if let Some((wh, idx)) = self.attrs.pop_idx("wh") {
            let value = self.eval_size(&wh, context)?;
            let mut parts = attr_split_cycle(&value);
            let w = parts.next().expect("cycle");
            let h = parts.next().expect("cycle");
            match self.name.as_str() {
                "rect" => {
                    self.attrs.insert_idx("width", w, idx);
                    self.attrs.insert_idx("height", h, idx + 1);
                }
                "circle" => {
                    self.attrs.insert_idx("r", fstr(strp(&w)? / 2.), idx);
                }
                "ellipse" => {
                    self.attrs.insert_idx("rx", fstr(strp(&w)? / 2.), idx);
                    self.attrs.insert_idx("ry", fstr(strp(&h)? / 2.), idx + 1);
                }
                _ => {}
            }
        }

        let mut new_attrs = AttrMap::new();

        // Size adjustments must be computed before updating position,
        // as they affect any xy-loc other than default top-left.
        // NOTE: these attributes may be removed once variable arithmetic
        // is implemented; currently key use-case is e.g. wh="$var" dw="-4"
        // with $var="20 30" or similar (the reference form of wh already
        // supports inline dw / dh).
        {
            let dw = self.pop_attr("dw");
            let dh = self.pop_attr("dh");
            let dwh = self.pop_attr("dwh");
            let mut d_w = None;
            let mut d_h = None;
            if let Some(dwh) = dwh {
                let mut parts = attr_split_cycle(&dwh).map_while(|v| strp_length(&v).ok());
                d_w = Some(parts.next().context("dw from dwh should be numeric")?);
                d_h = Some(parts.next().context("dh from dwh should be numeric")?);
            }
            if let Some(dw) = dw {
                d_w = Some(strp_length(&dw)?);
            }
            if let Some(dh) = dh {
                d_h = Some(strp_length(&dh)?);
            }
            if d_w.is_some() || d_h.is_some() {
                self.attrs = self
                    .resized_by(d_w.unwrap_or_default(), d_h.unwrap_or_default())?
                    .attrs;
            }
        }

        // Step 2: Evaluate position
        for (key, value) in self.attrs.clone() {
            let mut value = value.clone();
            match key.as_str() {
                "xy" | "xy1" | "xy2" => {
                    // TODO: maybe split up? pos may depend on size, but size doesn't depend on pos
                    value = self.eval_pos(value.as_str(), LocSpec::TopLeft, context)?;
                }
                "cxy" => {
                    value = self.eval_pos(value.as_str(), LocSpec::Center, context)?;
                }
                _ => (),
            }

            // The first pass is straightforward 'expansion', where the current
            // attribute totally determines the resulting value(s).
            let mut parts = attr_split_cycle(&value);
            match (key.as_str(), self.name.as_str()) {
                ("xy", "text" | "rect") => {
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

        {
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
                    ("cxy", "rect") => {
                        // Requires width / height to be specified in order to evaluate
                        // the centre point.
                        // TODO: also support specifying other attributes; xy+cxy should be sufficient
                        let width = new_attrs.get("width").map(|z| strp(z));
                        let height = new_attrs.get("height").map(|z| strp(z));
                        let cx = strp(&parts.next().expect("cycle"))
                            .context("Could not derive cx from cxy")?;
                        let cy = strp(&parts.next().expect("cycle"))
                            .context("Could not derive cy from cxy")?;
                        if let (Some(width), Some(height)) = (width, height) {
                            pass_two_attrs.insert("x", fstr(cx - width? / 2.));
                            pass_two_attrs.insert("y", fstr(cy - height? / 2.));
                        }
                    }
                    ("xy", "circle") => {
                        // Requires xy / r
                        let r = new_attrs.get("r").map(|z| strp(z));
                        let x = strp(&parts.next().expect("cycle"))
                            .context("Could not derive x from xy")?;
                        let y = strp(&parts.next().expect("cycle"))
                            .context("Could not derive y from xy")?;
                        if let Some(r) = r {
                            let r = r?;
                            pass_two_attrs.insert("cx", fstr(x + r));
                            pass_two_attrs.insert("cy", fstr(y + r));
                        }
                    }
                    ("xy", "ellipse") => {
                        // Requires xy / rx / ry
                        let rx = new_attrs.get("rx").map(|z| strp(z));
                        let ry = new_attrs.get("ry").map(|z| strp(z));
                        let x = strp(&parts.next().expect("cycle"))
                            .context("Could not derive x from xy")?;
                        let y = strp(&parts.next().expect("cycle"))
                            .context("Could not derive y from xy")?;
                        if let (Some(rx), Some(ry)) = (rx, ry) {
                            pass_two_attrs.insert("cx", fstr(x + rx?));
                            pass_two_attrs.insert("cy", fstr(y + ry?));
                        }
                    }
                    ("points", "polyline" | "polygon") => {
                        pass_two_attrs.insert("points", expand_relspec(&value, context));
                    }
                    ("d", "path") => {
                        pass_two_attrs.insert("d", expand_relspec(&value, context));
                    }
                    _ => pass_two_attrs.insert(key.clone(), value.clone()),
                }
            }

            new_attrs = pass_two_attrs;
        }

        self.attrs = new_attrs;
        if let Some(elem_id) = self.get_attr("id") {
            let mut updated = Self::new(&self.name, &self.attrs.to_vec());
            updated.add_classes(&self.classes);
            if let Some(elem) = context.get_element_mut(&elem_id) {
                *elem = updated;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_pos_edge() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        // Test with edge positioning
        let result = element.eval_pos_helper("@t:20%", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "20 0");

        let result = element.eval_pos_helper("@t:20% -4", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "16 0");

        let result = element.eval_pos_helper("@r:200%", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "100 200");

        let result = element.eval_pos_helper("@l:-1", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "0 99");

        let result = element.eval_pos_helper("@l:37", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "0 37");

        let result = element.eval_pos_helper("@l:37 3 5", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "3 42");
    }

    #[test]
    fn test_eval_pos_loc() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        // Test with location positioning
        let result = element.eval_pos_helper("@tr", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "100 0");

        let result = element.eval_pos_helper("@bl", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "0 100");

        let result = element.eval_pos_helper("@c", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "50 50");
    }

    #[test]
    fn test_eval_pos_rel() {
        let element = SvgElement::new(
            "rect",
            &[
                (String::from("width"), String::from("100")),
                (String::from("height"), String::from("75")),
            ],
        );
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        // Test with relative positioning
        let result = element.eval_pos_helper(":h 10", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "110 12.5");
    }

    #[test]
    fn test_eval_pos_invalid() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        // Test with invalid input

        let result = element.eval_pos_helper("invalid", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "invalid");

        let result = element.eval_pos_helper("30 20", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "30 20");
    }
}
