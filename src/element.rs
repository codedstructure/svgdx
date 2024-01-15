use crate::expression::eval_attr;
use crate::transform::TransformerContext;
use crate::types::{
    attr_split, attr_split_cycle, fstr, strp, strp_length, AttrMap, BoundingBox, ClassList,
    EdgeSpec, Length, LocSpec,
};
use anyhow::{bail, Context, Result};
use core::fmt::Display;
use lazy_regex::regex;
use regex::Captures;

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

#[derive(Clone)]
pub struct SvgElement {
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
        let mut element = Self::new(self.name.as_str(), &attrs.to_vec());
        element.add_classes(&self.classes);
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
        let mut element = Self::new(self.name.as_str(), &attrs);
        element.add_classes(&self.classes);
        element
    }

    /// copy attributes and classes from another element, returning the merged element
    #[must_use]
    pub fn with_attrs_from(&self, other: &Self) -> Self {
        let mut attrs = self.attrs.clone();
        for (k, v) in &other.attrs {
            attrs.insert(k, v);
        }
        let mut element = Self::new(self.name.as_str(), &attrs.to_vec());
        element.add_classes(&other.classes);
        element
    }

    pub fn pop_attr(&mut self, key: &str) -> Option<String> {
        self.attrs.pop(key)
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
            "person" => {
                if let (Some(x), Some(y), Some(h)) = (
                    self.attrs.get("x"),
                    self.attrs.get("y"),
                    self.attrs.get("height"),
                ) {
                    let x = strp(x)?;
                    let y = strp(y)?;
                    let h = strp(h)?;
                    Ok(Some(BoundingBox::new(x, y, x + h / 3., y + h)))
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

    fn eval_pos(&mut self, input: &str, context: &TransformerContext) -> Result<String> {
        // Relative positioning:
        //   ID LOC DX DY
        // [#id][@loc] [dx] [dy]
        //   or
        // (^|#id:)(h|v|H|V) [gap]

        // #abc@tl - top-left point of #abc
        // @tl - prev element
        // #abc:h - to-the-right of #abc
        // ^h - prev element

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
        let rel_re = regex!(
            r"^((?<relhv>(\^|#[[:word:]]+:)[hvHV])|(?<id>#[[:word:]]+)?((?<loc>@[tbrlc]+)(:(?<len>[-0-9\.]+%?))?)?)"
        );
        let mut parts = attr_split(input);
        let ref_loc = parts.next().context("Empty attribute in eval_pos()")?;
        if let Some(caps) = rel_re.captures(&ref_loc) {
            if caps.get(0).expect("Should always have group 0").is_empty() {
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

            let mut ref_el = context.get_prev_element();
            let default_rel;
            let mut loc = None;
            let mut len = None;

            if let Some(relhv) = caps.name("relhv") {
                let rel_dir;
                if let Some(relhv_id) = relhv.as_str().strip_prefix('#') {
                    let mut parts = relhv_id.split(':');
                    let ref_el_id = parts.next().expect("Regex match");
                    rel_dir = parts.next();
                    ref_el = context.get_element(ref_el_id);
                } else {
                    rel_dir = relhv.as_str().strip_prefix('^');
                }

                default_rel = match rel_dir
                    .context("Invalid relative direction (expected: hHvV)")?
                {
                    "h" => {
                        dx = d1.context(format!(r#"{ref_loc} gap error("{input}")"#))?;
                        LocSpec::Right
                    }
                    "H" => {
                        dx = -d1.context(format!(r#"{ref_loc} gap error("{input}")"#))?;
                        LocSpec::Left
                    }
                    "v" => {
                        dy = d1.context(format!(r#"{ref_loc} gap error("{input}")"#))?;
                        LocSpec::Bottom
                    }
                    "V" => {
                        dy = -d1.context(format!(r#"{ref_loc} gap error("{input}")"#))?;
                        LocSpec::Top
                    }
                    _ => {
                        dx = d1
                            .context(format!(r#"Could not determine dx in eval_pos("{input}")"#))?;
                        dy = d2
                            .context(format!(r#"Could not determine dy in eval_pos("{input}")"#))?;
                        LocSpec::TopRight
                    }
                };

                // This is similar to the more generic `xy-loc` processing.
                // assumes the bounding-box is well-defined by this point.
                if let Some(bbox) = self.bbox()? {
                    let width = bbox.width();
                    let height = bbox.height();
                    let (xy_dx, xy_dy) = match default_rel {
                        LocSpec::Bottom => (width / 2., 0.),
                        LocSpec::Left => (width, height / 2.),
                        LocSpec::Top => (width / 2., height),
                        LocSpec::Right => (0., height / 2.),
                        _ => (0., 0.),
                    };
                    dx -= xy_dx;
                    dy -= xy_dy;
                }
            } else {
                dx = d1.context(format!(r#"Could not determine dx in eval_pos("{input}")"#))?;
                dy = d2.context(format!(r#"Could not determine dy in eval_pos("{input}")"#))?;
                default_rel = LocSpec::TopRight;
                let loc_str = caps
                    .name("loc")
                    .map(|v| v.as_str().strip_prefix('@').expect("Regex match"));
                if let Some(loc_str) = loc_str {
                    loc = Some(LocSpec::try_from(loc_str)?);
                }
                if let Some(len_str) = caps.name("len") {
                    len = Some(strp_length(len_str.as_str()).context("Invalid Length")?);
                }
            }

            let opt_id = caps
                .name("id")
                .map(|v| v.as_str().strip_prefix('#').unwrap());
            if let Some(name) = opt_id {
                ref_el = Some(
                    context
                        .get_element(name)
                        .context(format!(r#"id '{name}' not found in eval_pos("{input}")"#))?,
                );
            }
            if let Some(ref_el) = ref_el {
                let mut margin_x = 0.;
                let mut margin_y = 0.;
                if let Some(margin) = ref_el.get_attr("margin") {
                    let mut margin_parts = attr_split_cycle(&margin);
                    margin_x = strp(&margin_parts.next().expect("cycle"))?;
                    margin_y = strp(&margin_parts.next().expect("cycle"))?;
                }
                let loc = loc.unwrap_or(default_rel);
                margin_y = match loc {
                    LocSpec::TopLeft | LocSpec::Top | LocSpec::TopRight => -margin_y,
                    LocSpec::BottomLeft | LocSpec::Bottom | LocSpec::BottomRight => margin_y,
                    _ => 0.,
                };
                margin_x = match loc {
                    LocSpec::TopLeft | LocSpec::Left | LocSpec::BottomLeft => -margin_x,
                    LocSpec::TopRight | LocSpec::Right | LocSpec::BottomRight => margin_x,
                    _ => 0.,
                };

                if let Some(bb) = ref_el.bbox()? {
                    let pos = if let (Ok(es), Some(len)) = (EdgeSpec::try_from(loc), len) {
                        bb.edgespec(es, len)
                    } else {
                        bb.locspec(loc)
                    };
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
        let rel_re = regex!(r"^(?<ref>(#\S+|\^))");
        if let Some(caps) = rel_re.captures(&ref_loc) {
            let dw = parts.next().unwrap_or("0".to_owned());
            let dh = parts.next().unwrap_or("0".to_owned());
            let mut ref_el = context.get_prev_element();
            let ref_str = caps
                .name("ref")
                .context("ref is mandatory in regex")?
                .as_str();
            if let Some(ref_str) = ref_str.strip_prefix('#') {
                ref_el = Some(context.get_element(ref_str).context(format!(
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
                let mut parts = attr_split_cycle(&dwh).map(|v| strp_length(&v).unwrap());
                d_w = parts.next();
                d_h = parts.next();
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
                "xy" | "cxy" | "xy1" | "xy2" => {
                    // TODO: maybe split up? pos may depende on size, but size doesn't depend on pos
                    value = self.eval_pos(value.as_str(), context)?;
                }
                _ => (),
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
