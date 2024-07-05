use crate::context::{ContextView, ElementMap};
use crate::expression::eval_attr;
use crate::path::path_bbox;
use crate::position::{strp_length, BoundingBox, DirSpec, EdgeSpec, Length, LocSpec, ScalarSpec};
use crate::types::{attr_split, attr_split_cycle, fstr, strp, AttrMap, ClassList, OrderIndex};
use anyhow::{bail, Result};
use core::fmt::Display;
use lazy_regex::regex;
use std::str::FromStr;

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
    pub order_index: OrderIndex,
    pub indent: usize,
    pub src_line: usize,
    pub event_range: Option<(usize, usize)>,
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
            attrs: attr_map.clone(),
            classes,
            content: ContentType::Empty,
            tail: None,
            order_index: OrderIndex::default(),
            indent: 0,
            src_line: 0,
            event_range: None,
        }
    }

    pub fn set_indent(&mut self, indent: usize) {
        self.indent = indent;
    }

    pub fn set_src_line(&mut self, line: usize) {
        self.src_line = line;
    }

    pub fn set_order_index(&mut self, order_index: &OrderIndex) {
        self.order_index = order_index.clone();
    }

    pub fn set_event_range(&mut self, range: (usize, usize)) {
        self.event_range = Some(range);
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

    pub fn has_class(&self, class: &str) -> bool {
        self.classes.contains(class)
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
        element.name.clone_from(&self.name);
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

    /// Resolve any expressions in attributes. Note attributes are unchanged on failure.
    pub fn eval_attributes(&mut self, ctx: &impl ContextView) {
        // Resolve any attributes
        for (key, value) in self.attrs.clone() {
            let replace = eval_attr(&value, ctx);
            self.attrs.insert(&key, &replace);
        }
        // Classes are handled separately to other attributes
        for class in &self.classes.clone() {
            self.classes.replace(class, eval_attr(class, ctx));
        }
    }

    pub fn is_phantom_element(&self) -> bool {
        matches!(self.name.as_str(), "config" | "specs" | "var" | "loop")
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
                // Following are non-standard.
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
                // Following are non-standard.
                | "specs"
        )
    }

    /// Should text content of this element be treated as element text?
    pub fn is_content_text(&self) -> bool {
        // This is present for graphics elements except for text,
        // where we need to be transparent.
        // TODO: except where we don't....
        self.is_graphics_element() && self.name != "text"
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
        // if not a number and not a refspec, pass it through without computing a bbox
        // this is needed to ultimately pass through e.g. "10cm" or "5%" as-is without
        // attempting to compute a bounding box.
        fn passthrough(value: &str) -> bool {
            // if attrs cannot be converted to f32 *and* do not contain '$'/'#'/'^' (which
            // might be resolved later) then return Ok(None).
            // This will return `true` for things such as "10%" or "40mm".
            strp(value).is_err()
                && !(value.contains('$') || value.contains('#') || value.contains('^'))
        }
        match self.name.as_str() {
            "text" => {
                let x = self.attrs.get("x").unwrap_or(&zstr);
                let y = self.attrs.get("y").unwrap_or(&zstr);
                if passthrough(x) || passthrough(y) {
                    return Ok(None);
                }
                let x = strp(x)?;
                let y = strp(y)?;
                Ok(Some(BoundingBox::new(x, y, x, y)))
            }
            "rect" | "use" | "image" | "svg" | "foreignObject" => {
                if let (Some(w), Some(h)) = (self.attrs.get("width"), self.attrs.get("height")) {
                    let x = self.attrs.get("x").unwrap_or(&zstr);
                    let y = self.attrs.get("y").unwrap_or(&zstr);
                    if passthrough(x) || passthrough(y) || passthrough(w) || passthrough(h) {
                        return Ok(None);
                    }
                    let x = strp(x)?;
                    let y = strp(y)?;
                    let w = strp(w)?;
                    let h = strp(h)?;
                    Ok(Some(BoundingBox::new(x, y, x + w, y + h)))
                } else {
                    Ok(None)
                }
            }
            "line" => {
                let x1 = self.attrs.get("x1").unwrap_or(&zstr);
                let y1 = self.attrs.get("y1").unwrap_or(&zstr);
                let x2 = self.attrs.get("x2").unwrap_or(&zstr);
                let y2 = self.attrs.get("y2").unwrap_or(&zstr);
                if passthrough(x1) || passthrough(y1) || passthrough(x2) || passthrough(y2) {
                    return Ok(None);
                }
                let x1 = strp(x1)?;
                let y1 = strp(y1)?;
                let x2 = strp(x2)?;
                let y2 = strp(y2)?;
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
                    let cx = self.attrs.get("cx").unwrap_or(&zstr);
                    let cy = self.attrs.get("cy").unwrap_or(&zstr);
                    if passthrough(cx) || passthrough(cy) || passthrough(r) {
                        return Ok(None);
                    }
                    let cx = strp(cx)?;
                    let cy = strp(cy)?;
                    let r = strp(r)?;
                    Ok(Some(BoundingBox::new(cx - r, cy - r, cx + r, cy + r)))
                } else {
                    Ok(None)
                }
            }
            "ellipse" => {
                if let (Some(rx), Some(ry)) = (self.attrs.get("rx"), self.attrs.get("ry")) {
                    let cx = self.attrs.get("cx").unwrap_or(&zstr);
                    let cy = self.attrs.get("cy").unwrap_or(&zstr);
                    if passthrough(cx) || passthrough(cy) || passthrough(rx) || passthrough(ry) {
                        return Ok(None);
                    }
                    let cx = strp(cx)?;
                    let cy = strp(cy)?;
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
            if let Ok(edge) = loc.parse() {
                Ok(Some(bb.edgespec(edge, len)))
            } else if let Ok(loc) = loc.parse() {
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

    /// Split a possible relspec into the reference element (if it exists) and remainder
    ///
    /// `input`: the relspec string
    ///
    /// If the input is not a relspec, the reference element is `None` and the remainder
    /// is the entire input string.
    fn split_relspec<'a, 'b>(
        &self,
        input: &'b str,
        ctx: &'a impl ElementMap,
    ) -> Result<(Option<&'a SvgElement>, &'b str)> {
        if input.starts_with('^') {
            let skip_prev = input.strip_prefix('^').unwrap_or(input);
            Ok((ctx.get_prev_element(), skip_prev.trim_start()))
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
            if let Some(el) = ctx.get_element(id) {
                Ok((Some(el), remain))
            } else {
                bail!("Reference to unknown element '{}'", id);
            }
        } else {
            Ok((None, input))
        }
    }

    fn eval_rel_attr(&self, name: &str, value: &str, ctx: &impl ElementMap) -> Result<String> {
        if let Ok(ss) = ScalarSpec::from_str(name) {
            if let (Some(el), remain) = self.split_relspec(value, ctx)? {
                if let Ok(Some(bbox)) = el.bbox() {
                    // default value - same 'type' as attr name, e.g. y2 => ymax
                    let mut v = bbox.scalarspec(ss);

                    if let "width" | "height" | "dw" | "dh" = name {
                        if let Ok(len) = strp_length(remain) {
                            v = len.adjust(v);
                        }
                    } else {
                        // position attributes handle dx/dy within eval_pos_helper
                        if let Ok(Some((x, y))) = self.eval_pos_helper(remain, &bbox, ss.into()) {
                            use ScalarSpec::*;
                            v = match ss {
                                Minx | Maxx | Cx => x,
                                Miny | Maxy | Cy => y,
                                _ => v,
                            };
                        }
                    }
                    return Ok(fstr(v).to_string());
                }
            }
        }
        Ok(value.to_owned())
    }

    /// Extract dx/dy from a string such as '10 20' or '10' (in which case both are 10)
    fn extract_dx_dy(&self, input: &str) -> Result<(f32, f32)> {
        let mut parts = attr_split_cycle(input);
        let dx = strp(&parts.next().unwrap_or("0".to_string()))?;
        let dy = strp(&parts.next().unwrap_or("0".to_string()))?;
        Ok((dx, dy))
    }

    /// Location-based positioning - relative to a specific location on the reference element
    /// ```text
    /// (^|#id)[@loc] [dx] [dy]
    /// ```
    ///
    /// Edge-based positioning - relative to a specific location on the reference element
    /// ```text
    /// (^|#id)[@edge][:length] [dx] [dy]
    /// ```
    fn eval_pos_helper(
        &self,
        remain: &str,
        bbox: &BoundingBox,
        anchor: LocSpec,
    ) -> Result<Option<(f32, f32)>> {
        let loc_re = regex!(r"^@(?<loc>[trblc]+)(\s+(?<remain>.*))?$");
        let edge_re = regex!(r"^@(?<edge>[trbl]):(?<len>[-0-9\.]+%?)(\s+(?<remain>.*))?$");
        if let Some((x, y)) = if let Some(caps) = edge_re.captures(remain) {
            let edge: EdgeSpec = caps.name("edge").expect("Regex Match").as_str().parse()?;
            let length = strp_length(caps.name("len").expect("Regex Match").as_str())?;
            let (dx, dy) = if let Some(remain) = caps.name("remain") {
                self.extract_dx_dy(remain.as_str())?
            } else {
                (0., 0.)
            };
            let (x, y) = bbox.edgespec(edge, length);
            Some((x + dx, y + dy))
        } else if let Some(caps) = loc_re.captures(remain) {
            let loc: LocSpec = caps.name("loc").expect("Regex Match").as_str().parse()?;
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
            Ok(Some((x, y)))
        } else {
            Ok(None)
        }
    }

    pub fn eval_rel_attributes(&mut self, ctx: &impl ElementMap) -> Result<()> {
        for (key, value) in self.attrs.clone() {
            if matches!(
                (self.name.as_str(), key.as_str()),
                (
                    "rect" | "use" | "image" | "svg" | "foreignObject" | "line",
                    "x" | "y" | "cx" | "cy" | "x1" | "y1" | "x2" | "y2" | "width" | "height",
                ) | (
                    "circle",
                    "x" | "y" | "cx" | "cy" | "x1" | "y1" | "x2" | "y2" | "width" | "height" | "r",
                ) | (
                    "ellipse",
                    "x" | "y"
                        | "cx"
                        | "cy"
                        | "x1"
                        | "y1"
                        | "x2"
                        | "y2"
                        | "width"
                        | "height"
                        | "r"
                        | "rx"
                        | "ry",
                )
            ) {
                let computed = self.eval_rel_attr(&key, &value, ctx)?;
                if strp(&computed).is_ok() {
                    self.attrs.insert(key.clone(), computed);
                }
            }
        }
        Ok(())
    }

    /// Direction relative positioning - horizontally below, above, to the left, or to the
    /// right of the referenced element.
    /// ```text
    /// (^|#id)(:(h|H|v|V) [gap])
    /// ```
    pub fn eval_rel_position(&mut self, ctx: &impl ContextView) -> Result<()> {
        let rel_re = regex!(r"^:(?<rel>[hHvV])(\s+(?<remain>.*))?$");
        let input = self.attrs.get("xy");
        // element-relative position can only be applied via xy attribute
        if let Some(input) = input {
            let (ref_el, remain) = self.split_relspec(input, ctx)?;
            let ref_el = match ref_el {
                Some(el) => el,
                None => return Ok(()),
            };
            if let (Some(bbox), Some(caps)) = (ref_el.bbox()?, rel_re.captures(remain)) {
                let rel: DirSpec = caps.name("rel").expect("Regex Match").as_str().parse()?;
                // this relies on x / y defaulting to 0 if not present, so we can get a bbox
                // from only having a defined width / height.
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
                let (dx, dy) = match rel {
                    DirSpec::Above => (-this_width / 2., -(this_height + gap)),
                    DirSpec::Below => (-this_width / 2., gap),
                    DirSpec::InFront => (gap, -this_height / 2.),
                    DirSpec::Behind => (-(this_width + gap), -this_height / 2.),
                };
                self.pop_attr("xy"); // don't need this anymore
                self.set_attr("x", &fstr(x + dx));
                self.set_attr("y", &fstr(y + dy));
            }
        }

        Ok(())
    }

    fn split_compound_attr(value: &str) -> (String, String) {
        // wh="10" -> width="10", height="10"
        // wh="10 20" -> width="10", height="20"
        // wh="#thing" -> width="#thing", height="#thing"
        // wh="#thing 50%" -> width="#thing 50%", height="#thing 50%"
        // wh="#thing 10 20" -> width="#thing 10", height="#thing 20"
        if value.starts_with(['#', '^']) {
            let mut parts = value.splitn(2, char::is_whitespace);
            let prefix = parts.next().expect("nonempty");
            if let Some(remain) = parts.next() {
                let mut parts = attr_split_cycle(remain);
                let x_suffix = parts.next().unwrap_or_default();
                let y_suffix = parts.next().unwrap_or_default();
                ([prefix, &x_suffix].join(" "), [prefix, &y_suffix].join(" "))
            } else {
                (value.to_owned(), value.to_owned())
            }
        } else {
            let mut parts = attr_split_cycle(value);
            let x = parts.next().unwrap_or_default();
            let y = parts.next().unwrap_or_default();
            (x, y)
        }
    }

    pub fn expand_compound_size(&mut self) {
        if let Some(wh) = self.attrs.pop("wh") {
            // Split value into width and height
            let (w, h) = Self::split_compound_attr(&wh);
            self.attrs.insert_first("width", w);
            self.attrs.insert_first("height", h);
        }
        if let ("ellipse", Some(rxy)) = (self.name.as_str(), self.attrs.pop("rxy")) {
            // Split value into rx and ry
            let (rx, ry) = Self::split_compound_attr(&rxy);
            self.attrs.insert_first("rx", rx);
            self.attrs.insert_first("ry", ry);
        }
        if let Some(dwh) = self.attrs.pop("dwh") {
            // Split value into dw and dh
            let (dw, dh) = Self::split_compound_attr(&dwh);
            self.attrs.insert_first("dw", dw);
            self.attrs.insert_first("dh", dh);
        }
    }

    pub fn resolve_size_delta(&mut self) {
        // assumes "width"/"height"/"r"/"rx"/"ry" are numeric if present
        let (w, h) = match self.name.as_str() {
            "circle" => {
                let diam = self.get_attr("r").map(|r| 2. * strp(&r).unwrap_or(0.));
                (diam, diam)
            }
            "ellipse" => (
                self.get_attr("rx")
                    .and_then(|rx| strp(&rx).ok())
                    .map(|x| x * 2.),
                self.get_attr("ry")
                    .and_then(|ry| strp(&ry).ok())
                    .map(|x| x * 2.),
            ),
            _ => (
                self.get_attr("width").and_then(|w| strp(&w).ok()),
                self.get_attr("height").and_then(|h| strp(&h).ok()),
            ),
        };

        if let Some(dw) = self.pop_attr("dw") {
            if let Ok(Some(new_w)) = strp_length(&dw).map(|dw| w.map(|x| dw.adjust(x))) {
                self.set_attr("width", &fstr(new_w));
            }
        }
        if let Some(dh) = self.pop_attr("dh") {
            if let Ok(Some(new_h)) = strp_length(&dh).map(|dh| h.map(|x| dh.adjust(x))) {
                self.set_attr("height", &fstr(new_h));
            }
        }
    }

    // Compound attributes, e.g.
    // xy="#o" -> x="#o", y="#o"
    // xy="#o 2" -> x="#o 2", y="#o 2"
    // xy="#o 2 4" -> x="#o 2", y="#o 4"
    pub fn expand_compound_pos(&mut self) {
        if let Some(xy) = self.pop_attr("xy") {
            let (x, y) = Self::split_compound_attr(&xy);
            let (x_attr, y_attr) = match self.pop_attr("xy-loc").as_deref() {
                Some("t") => ("cx", "y1"),
                Some("tr") => ("x2", "y1"),
                Some("r") => ("x2", "cy"),
                Some("br") => ("x2", "y2"),
                Some("b") => ("cx", "y2"),
                Some("bl") => ("x1", "y2"),
                Some("l") => ("x1", "cy"),
                Some("c") => ("cx", "cy"),
                _ => ("x", "y"),
            };
            self.attrs.insert_first(x_attr, x);
            self.attrs.insert_first(y_attr, y);
        }
        if let Some(cxy) = self.pop_attr("cxy") {
            let (cx, cy) = Self::split_compound_attr(&cxy);
            self.attrs.insert_first("cx", cx);
            self.attrs.insert_first("cy", cy);
        }
        if let Some(xy1) = self.pop_attr("xy1") {
            let (x1, y1) = Self::split_compound_attr(&xy1);
            self.attrs.insert_first("x1", x1);
            self.attrs.insert_first("y1", y1);
        }
        if let Some(xy2) = self.pop_attr("xy2") {
            let (x2, y2) = Self::split_compound_attr(&xy2);
            self.attrs.insert_first("x2", x2);
            self.attrs.insert_first("y2", y2);
        }
        if let Some(dxy) = self.pop_attr("dxy") {
            let (dx, dy) = Self::split_compound_attr(&dxy);
            self.attrs.insert_first("dx", dx);
            self.attrs.insert_first("dy", dy);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spread_attr() {
        let (w, h) = SvgElement::split_compound_attr("10");
        assert_eq!(w, "10");
        assert_eq!(h, "10");
        let (w, h) = SvgElement::split_compound_attr("10 20");
        assert_eq!(w, "10");
        assert_eq!(h, "20");
        let (w, h) = SvgElement::split_compound_attr("#thing");
        assert_eq!(w, "#thing");
        assert_eq!(h, "#thing");
        let (w, h) = SvgElement::split_compound_attr("#thing 50%");
        assert_eq!(w, "#thing 50%");
        assert_eq!(h, "#thing 50%");
        let (w, h) = SvgElement::split_compound_attr("#thing 10 20");
        assert_eq!(w, "#thing 10");
        assert_eq!(h, "#thing 20");

        let (x, y) = SvgElement::split_compound_attr("^a@tl");
        assert_eq!(x, "^a@tl");
        assert_eq!(y, "^a@tl");
        let (x, y) = SvgElement::split_compound_attr("^a@tl 5");
        assert_eq!(x, "^a@tl 5");
        assert_eq!(y, "^a@tl 5");
        let (x, y) = SvgElement::split_compound_attr("^a@tl 5 7%");
        assert_eq!(x, "^a@tl 5");
        assert_eq!(y, "^a@tl 7%");
    }

    #[test]
    fn test_eval_pos_edge() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        // Test with edge positioning
        let result = element.eval_pos_helper("@t:25%", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some((25., 0.)));

        let result = element.eval_pos_helper("@t:25% -4", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some((21., -4.)));

        let result = element.eval_pos_helper("@r:200%", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some((100., 200.)));

        let result = element.eval_pos_helper("@l:-1", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some((0., 99.)));

        let result = element.eval_pos_helper("@l:37", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some((0., 37.)));

        let result = element.eval_pos_helper("@l:37 3 5", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some((3., 42.)));
    }

    #[test]
    fn test_eval_pos_loc() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        // Test with location positioning
        let result = element.eval_pos_helper("@tr", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some((100., 0.)));

        let result = element.eval_pos_helper("@bl", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some((0., 100.)));

        let result = element.eval_pos_helper("@c", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some((50., 50.)));
    }

    #[test]
    fn test_eval_pos_invalid() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        // Test with invalid input

        let result = element.eval_pos_helper("invalid", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        let result = element.eval_pos_helper("30 20", &bbox, LocSpec::TopLeft);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some((30., 20.)));
    }
}
