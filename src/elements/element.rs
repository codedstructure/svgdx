use crate::constants::{
    EDGESPEC_SEP, ELREF_ID_PREFIX, ELREF_PREVIOUS, LOCSPEC_SEP, RELPOS_SEP, SCALARSPEC_SEP,
    VAR_PREFIX,
};
use crate::context::{ContextView, ElementMap, TransformerContext};
use crate::elements::{
    path_bbox, process_path_bearing, process_text_attr, ConnectionType, Connector,
};
use crate::errors::{Result, SvgdxError};
use crate::events::{InputList, OutputEvent};
use crate::expression::eval_attr;
use crate::geometry::{
    strp_length, BoundingBox, DirSpec, LocSpec, Position, ScalarSpec, Size, TransformAttr,
    TrblLength,
};
use crate::types::{
    attr_split, attr_split_cycle, extract_elref, fstr, strp, AttrMap, ClassList, OrderIndex,
};

use core::fmt::Display;
use std::collections::HashMap;
use std::f32::consts::{FRAC_1_SQRT_2, SQRT_2};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub struct SvgElement {
    pub name: String,
    pub original: String,
    pub attrs: AttrMap,
    pub classes: ClassList,
    pub text_content: Option<String>,
    pub order_index: OrderIndex,
    pub indent: usize,
    pub src_line: usize,
    pub event_range: Option<(usize, usize)>,
    pub content_bbox: Option<BoundingBox>,
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

/// Split a possible relspec into the reference element (if it exists) and remainder
///
/// `input`: the relspec string
///
/// If the input is not a relspec, the reference element is `None` and the remainder
/// is the entire input string.
fn split_relspec<'a, 'b>(
    input: &'b str,
    ctx: &'a impl ElementMap,
) -> Result<(Option<&'a SvgElement>, &'b str)> {
    if let Ok((elref, remain)) = extract_elref(input) {
        if let Some(el) = ctx.get_element(&elref) {
            // Note: essential we don't trim `remain` - if it starts
            // with whitespace, that is significant.
            Ok((Some(el), remain))
        } else {
            Err(SvgdxError::ReferenceError(elref))
        }
    } else {
        Ok((None, input))
    }
}

/// Replace all refspec entries in a string with lookup results
/// Suitable for use with path `d` or polyline `points` attributes
/// which may contain many such entries.
///
/// Infallible; any invalid refspec will be left unchanged (other
/// than whitespace)
fn expand_relspec(value: &str, ctx: &impl ElementMap) -> String {
    // For most elements either a `#elem~X` or `#elem@X` form is required,
    // but for `<point>` elements a standalone `#elem` suffices.

    let word_break = |c: char| {
        !(
            // not ideal, e.g. a second '.' *would* be a word break.
            c.is_alphanumeric()
                || c == '_'
                || c == '-'
                || c == SCALARSPEC_SEP
                || c == RELPOS_SEP
                || c == LOCSPEC_SEP
                || c == EDGESPEC_SEP
                || c == '%'
        )
    };
    let mut result = String::new();
    let mut value = value;
    while !value.is_empty() {
        if let Some(idx) = value.find([ELREF_ID_PREFIX, ELREF_PREVIOUS]) {
            result.push_str(&value[..idx]);
            value = &value[idx..];
            if let Some(mut idx) = value[1..].find(word_break) {
                idx += 1; // account for ignoring #/^ in word break search
                result.push_str(&expand_single_relspec(&value[..idx], ctx));
                value = &value[idx..];
            } else {
                result.push_str(&expand_single_relspec(value, ctx));
                break;
            };
        } else {
            result.push_str(value);
            break;
        }
    }
    result
}

fn expand_single_relspec(value: &str, ctx: &impl ElementMap) -> String {
    let elem_loc = |elem: &SvgElement, loc: LocSpec| {
        ctx.get_element_bbox(elem)
            .map(|bb| bb.map(|bb| bb.locspec(loc)))
    };
    if let Ok((Some(elem), rest)) = split_relspec(value, ctx) {
        if rest.is_empty() && elem.name == "point" {
            if let Ok(Some(point)) = elem_loc(elem, LocSpec::Center) {
                return format!("{} {}", fstr(point.0), fstr(point.1));
            }
        } else if let Some(loc) = rest.strip_prefix(LOCSPEC_SEP).and_then(|s| s.parse().ok()) {
            if let Ok(Some(point)) = elem_loc(elem, loc) {
                return format!("{} {}", fstr(point.0), fstr(point.1));
            }
        } else if let Some(scalar) = rest
            .strip_prefix(SCALARSPEC_SEP)
            .and_then(|s| s.parse().ok())
        {
            if let Ok(Some(pos)) = ctx
                .get_element_bbox(elem)
                .map(|bb| bb.map(|bb| bb.scalarspec(scalar)))
            {
                return fstr(pos);
            }
        }
    }
    value.to_string()
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
            original: format!("<{name} {}>", attr_map),
            attrs: attr_map.clone(),
            classes,
            text_content: None,
            order_index: OrderIndex::default(),
            indent: 0,
            src_line: 0,
            event_range: None,
            content_bbox: None,
        }
    }

    pub fn transmute(&mut self, ctx: &impl ContextView) -> Result<()> {
        if self.name == "path" {
            if let Some(d) = self.get_attr("d") {
                if d.chars().any(|c| c == 'b' || c == 'B') {
                    self.set_attr("d", &process_path_bearing(&d)?)
                }
            }
        }

        if self.is_connector() {
            if let Ok(conn) = Connector::from_element(
                self,
                ctx,
                if let Some(e_type) = self.get_attr("edge-type") {
                    ConnectionType::from_str(&e_type)
                } else if self.name == "polyline" {
                    ConnectionType::Corner
                } else {
                    ConnectionType::Straight
                },
            ) {
                // replace with rendered connection element
                *self = conn.render(ctx)?.without_attr("edge-type");
            } else {
                return Err(SvgdxError::InvalidData(
                    "Cannot create connector".to_owned(),
                ));
            }
        }

        // Process dx / dy as translation offsets if not an element
        // where they already have intrinsic meaning.
        // TODO: would be nice to get rid of this; it's mostly handled
        // in `set_position_attrs`, but if there is no bbox (e.g. no width/height)
        // then that won't do anything and this does.
        if !matches!(self.name.as_str(), "text" | "tspan" | "feOffset") {
            let dx = self.pop_attr("dx");
            let dy = self.pop_attr("dy");
            let mut d_x = None;
            let mut d_y = None;
            if let Some(dx) = dx {
                d_x = Some(strp(&dx)?);
            }
            if let Some(dy) = dy {
                d_y = Some(strp(&dy)?);
            }
            if d_x.is_some() || d_y.is_some() {
                *self = self.translated(d_x.unwrap_or_default(), d_y.unwrap_or_default())?;
            }
        }

        if self.name == "use" {
            // rotation requires a bbox to identify center of rotation; for `<use>`
            // elements derive from context and inject via `content_bbox`. Allows
            // handle_rotation to be independent of context.
            if let Some(bbox) = ctx.get_element_bbox(self)? {
                self.content_bbox = Some(bbox);
            }
        }
        self.handle_rotation()?;

        Ok(())
    }

    pub fn inner_events(&self, context: &TransformerContext) -> Option<InputList> {
        if let Some((start, end)) = self.event_range {
            // empty events will have end == start
            if end > start {
                return Some(InputList::from(&context.events[start + 1..end]));
            }
        }
        None
    }

    pub fn all_events(&self, context: &TransformerContext) -> InputList {
        if let Some((start, end)) = self.event_range {
            InputList::from(&context.events[start..end + 1])
        } else {
            InputList::new()
        }
    }

    /// Process a given `SvgElement` into a list of `SvgEvent`s
    // TODO: would be nice to make this infallible and have any potential errors handled earlier.
    pub fn element_events(&self, ctx: &mut TransformerContext) -> Result<Vec<OutputEvent>> {
        let mut events = vec![];

        if ctx.config.debug {
            // Prefix replaced element(s) with a representation of the original element
            //
            // Replace double quote with backtick to avoid messy XML entity conversion
            // (i.e. &quot; or &apos; if single quotes were used)
            events.push(OutputEvent::Comment(
                format!(" {} ", self.original)
                    .replace('"', "`")
                    .replace(['<', '>'], ""),
            ));
            events.push(OutputEvent::Text(format!("\n{}", " ".repeat(self.indent))));
        }

        // Standard comment: expressions & variables are evaluated.
        if let Some(comment) = self.get_attr("_") {
            // Expressions in comments are evaluated
            let value = eval_attr(&comment, ctx)?;
            events.push(OutputEvent::Comment(format!(" {value} ")));
            events.push(OutputEvent::Text(format!("\n{}", " ".repeat(self.indent))));
        }

        // 'Raw' comment: no evaluation of expressions occurs here
        if let Some(comment) = self.get_attr("__") {
            events.push(OutputEvent::Comment(format!(" {comment} ")));
            events.push(OutputEvent::Text(format!("\n{}", " ".repeat(self.indent))));
        }

        // Some elements don't generate text themselves, but can have
        // associated text.
        // TODO: refactor this method to handle text event gen better
        let phantom = matches!(self.name.as_str(), "point" | "box");

        if self.has_attr("text") {
            let (orig_elem, text_elements) = process_text_attr(self)?;
            if orig_elem.name != "text" && !phantom {
                // We only care about the original element if it wasn't a text element
                // (otherwise we generate a useless empty text element for the original)
                events.push(OutputEvent::Empty(orig_elem));
                events.push(OutputEvent::Text(format!("\n{}", " ".repeat(self.indent))));
            }
            match text_elements.as_slice() {
                [] => {}
                [elem] => {
                    events.push(OutputEvent::Start(elem.clone()));
                    if let Some(value) = &elem.text_content {
                        events.push(OutputEvent::Text(value.clone()));
                    } else {
                        return Err(SvgdxError::InvalidData(
                            "Text element should have content".to_owned(),
                        ));
                    }
                    events.push(OutputEvent::End("text".to_string()));
                }
                _ => {
                    // Multiple text spans
                    let text_elem = &text_elements[0];
                    events.push(OutputEvent::Start(text_elem.clone()));
                    events.push(OutputEvent::Text(format!("\n{}", " ".repeat(self.indent))));
                    for elem in &text_elements[1..] {
                        // Note: we can't insert a newline/last_indent here as whitespace
                        // following a tspan is compressed to a single space and causes
                        // misalignment - see https://stackoverflow.com/q/41364908
                        events.push(OutputEvent::Start(elem.clone()));
                        if let Some(value) = &elem.text_content {
                            events.push(OutputEvent::Text(value.clone()));
                        } else {
                            return Err(SvgdxError::InvalidData(
                                "Text element should have content".to_owned(),
                            ));
                        }
                        events.push(OutputEvent::End("tspan".to_string()));
                    }
                    events.push(OutputEvent::Text(format!("\n{}", " ".repeat(self.indent))));
                    events.push(OutputEvent::End("text".to_string()));
                }
            }
        } else if !phantom {
            if self.is_empty_element() {
                events.push(OutputEvent::Empty(self.clone()));
            } else {
                events.push(OutputEvent::Start(self.clone()));
            }
        }

        Ok(events)
    }

    pub fn resolve_position(&mut self, ctx: &impl ContextView) -> Result<()> {
        // Evaluate any expressions (e.g. var lookups or {{..}} blocks) in attributes
        // TODO: this is not idempotent in the case of e.g. RNG lookups, so should be
        // moved out of this function and called once per element (or this function
        // should be called once per element...)
        self.eval_attributes(ctx)?;

        self.handle_containment(ctx)?;

        // Need size before can evaluate relative position
        self.expand_compound_size();
        self.eval_rel_attributes(ctx)?;
        self.resolve_size_delta();

        // ensure relatively-positioned text elements have appropriate anchors
        if self.name == "text" && self.has_attr("text") {
            self.eval_text_anchor(ctx)?;
        }

        if let ("polyline" | "polygon", Some(points)) =
            (self.name.as_str(), self.get_attr("points"))
        {
            self.set_attr("points", &expand_relspec(&points, ctx));
        }
        if let ("path", Some(d)) = (self.name.as_str(), self.get_attr("d")) {
            self.set_attr("d", &expand_relspec(&d, ctx));
        }

        // TODO: issue is that this could fail with a reference error
        // which would be resolved by expand_relspec, though that requires
        // eval_rel_attributes to be called first...
        if let Some(pos) = self.pos_from_dirspec(ctx)? {
            self.pop_attr("xy");
            pos.set_position_attrs(self);
        }
        // Compound attributes, e.g. xy="#o 2" -> x="#o 2", y="#o 2"
        self.expand_compound_pos();
        self.eval_rel_attributes(ctx)?;

        // if let ("polyline" | "polygon", Some(points)) =
        //     (self.name.as_str(), self.get_attr("points"))
        // {
        //     self.set_attr("points", &expand_relspec(&points, ctx));
        // }
        // if let ("path", Some(d)) = (self.name.as_str(), self.get_attr("d")) {
        //     self.set_attr("d", &expand_relspec(&d, ctx));
        // }

        let mut p = Position::from(self as &SvgElement);
        if self.name == "use" {
            let el = self.get_target_element(ctx)?;
            if let Some(sz) = el.size(ctx)? {
                p.update_size(&sz);
                if el.name == "circle" || el.name == "ellipse" {
                    // The referenced element is defined by its center,
                    // but use elements are defined by top-left pos.
                    p.translate(sz.width / 4., sz.height / 4.);
                }
            }
        }
        p.set_position_attrs(self);

        Ok(())
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

    /// Remove a class from the element, returning `true` if the class was present
    pub fn pop_class(&mut self, class: &str) -> bool {
        self.classes.remove(class)
    }

    pub fn get_classes(&self) -> Vec<String> {
        self.classes.to_vec()
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

    pub fn remove_attrs(&mut self, keys: &[&str]) {
        for key in keys {
            self.pop_attr(key);
        }
    }

    pub fn get_attr(&self, key: &str) -> Option<String> {
        self.attrs.get(key).map(|x| x.to_owned())
    }

    pub fn set_attr(&mut self, key: &str, value: &str) {
        self.attrs.insert(key, value);
    }

    /// set an attribute key/value if the key does not already exist
    pub fn set_default_attr(&mut self, key: &str, value: &str) {
        if !self.has_attr(key) {
            self.set_attr(key, value);
        }
    }

    pub fn get_attrs(&self) -> HashMap<String, String> {
        self.attrs.to_vec().into_iter().collect()
    }

    /// Resolve any expressions in attributes.
    pub fn eval_attributes(&mut self, ctx: &impl ContextView) -> Result<()> {
        // Resolve any attributes
        for (key, value) in self.attrs.clone() {
            if key == "__" {
                // Raw comments are not evaluated
                continue;
            }
            let replace = eval_attr(&value, ctx)?;
            self.attrs.insert(&key, &replace);
        }
        // Classes are handled separately to other attributes
        for class in &self.classes.clone() {
            self.classes.replace(class, eval_attr(class, ctx)?);
        }

        Ok(())
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

    pub fn is_empty_element(&self) -> bool {
        if let Some((start, end)) = self.event_range {
            start == end
        } else {
            true
        }
    }

    pub fn is_connector(&self) -> bool {
        self.has_attr("start")
            && self.has_attr("end")
            && (self.name == "line" || self.name == "polyline")
    }

    pub fn handle_rotation(&mut self) -> Result<()> {
        let angle = self.pop_attr("rotate");
        if angle.is_none() {
            return Ok(());
        }
        let angle = angle.unwrap();
        let angle = strp(&angle)?;
        if let Some((cx, cy)) = self.bbox()?.map(|bb| bb.center()) {
            let mut rot_xfrm = TransformAttr::new();
            rot_xfrm.rotate_around(angle, cx, cy);
            if let Some(xfrm) = self.pop_attr("transform") {
                // rotation should be the outermost transform, so prepend it
                self.set_attr("transform", &format!("{rot_xfrm} {xfrm}"));
            } else {
                self.set_attr("transform", &rot_xfrm.to_string());
            }
        }
        Ok(())
    }

    fn handle_containment(&mut self, ctx: &dyn ContextView) -> Result<()> {
        let (surround, inside) = (self.get_attr("surround"), self.get_attr("inside"));

        if surround.is_some() && inside.is_some() {
            return Err(SvgdxError::InvalidData(
                "Cannot have 'surround' and 'inside' on an element".to_owned(),
            ));
        }
        if surround.is_none() && inside.is_none() {
            return Ok(());
        }

        let is_surround = surround.is_some();
        let contain_str = if is_surround { "surround" } else { "inside" };
        let ref_list = surround.unwrap_or_else(|| inside.unwrap());

        let mut bbox_list = vec![];

        for elref in attr_split(&ref_list) {
            let elref = elref.parse()?;
            let el = ctx
                .get_element(&elref)
                .ok_or_else(|| SvgdxError::ReferenceError(elref.clone()))?;
            {
                let bb = if is_surround {
                    ctx.get_element_bbox(el)
                } else {
                    // TODO: this doesn't handle various cases when at least one
                    // circle/ellipses are is present and ref_list.len() > 1.
                    // Should probably fold the list and provide next element type
                    // as the target shape here
                    el.inscribed_bbox(&self.name)
                };
                if let Ok(Some(el_bb)) = bb {
                    bbox_list.push(el_bb);
                } else {
                    return Err(SvgdxError::MissingBoundingBox(el.to_string()));
                }
            }
        }
        let mut bbox = if is_surround {
            BoundingBox::union(bbox_list)
        } else {
            BoundingBox::intersection(bbox_list)
        };

        if let Some(margin) = self.get_attr("margin") {
            let margin: TrblLength = margin.parse()?;

            if let Some(bb) = &mut bbox {
                if is_surround {
                    bb.expand_trbl_length(margin);
                } else {
                    bb.shrink_trbl_length(margin);
                }
            }
        }
        if let Some(bb) = bbox {
            self.position_from_bbox(&bb, !is_surround);
        }
        self.add_class(&format!("d-{contain_str}"));
        self.remove_attrs(&["surround", "inside", "margin"]);
        Ok(())
    }

    /// Calculate bounding box of target_shape inside self
    pub fn inscribed_bbox(&self, target_shape: &str) -> Result<Option<BoundingBox>> {
        let zstr = "0".to_owned();
        match (target_shape, self.name.as_str()) {
            // rect inside circle
            ("rect", "circle") => {
                if let Some(r) = self.attrs.get("r") {
                    let cx = self.attrs.get("cx").unwrap_or(&zstr);
                    let cy = self.attrs.get("cy").unwrap_or(&zstr);
                    let cx = strp(cx)?;
                    let cy = strp(cy)?;
                    let r = strp(r)? * FRAC_1_SQRT_2;
                    Ok(Some(BoundingBox::new(cx - r, cy - r, cx + r, cy + r)))
                } else {
                    Ok(None)
                }
            }
            // rect inside ellipse
            ("rect", "ellipse") => {
                if let (Some(rx), Some(ry)) = (self.attrs.get("rx"), self.attrs.get("ry")) {
                    let cx = self.attrs.get("cx").unwrap_or(&zstr);
                    let cy = self.attrs.get("cy").unwrap_or(&zstr);
                    let cx = strp(cx)?;
                    let cy = strp(cy)?;
                    let rx = strp(rx)? * FRAC_1_SQRT_2;
                    let ry = strp(ry)? * FRAC_1_SQRT_2;
                    Ok(Some(BoundingBox::new(cx - rx, cy - ry, cx + rx, cy + ry)))
                } else {
                    Ok(None)
                }
            }
            // Trivial cases: same shape
            _ => self.bbox(),
        }
    }

    pub fn get_target_element(&self, ctx: &impl ElementMap) -> Result<SvgElement> {
        // TODO: this uses OrderIndex to uniquely identify elements, but that's a bit
        // of a hack. In particular using `id` or `href` is insufficient, as doesn't
        // cope with '^' where the target might not even have an id. Would be better
        // to assign a dedicated internal ID to every element and use that.
        // TODO: in addition to the above, '^' is already broken since it doesn't get
        // captured in the 'remain' thing for deferred elements, and is always the same
        // element as evaluated here. Probably need to store a 'prev' (and later, 'next')
        // internal ID with each element so can follow a chain of these.
        let mut seen: Vec<OrderIndex> = vec![];
        let mut element = self;

        while element.name == "use" || element.name == "reuse" {
            let href = element
                .get_attr("href")
                .ok_or_else(|| SvgdxError::MissingAttribute("href".to_owned()))?;
            let elref = href.parse()?;
            if let Some(el) = ctx.get_element(&elref) {
                if seen.contains(&el.order_index) {
                    return Err(SvgdxError::CircularRefError(format!(
                        "{} already seen",
                        elref
                    )));
                }
                seen.push(el.order_index.clone());
                element = el;
            } else {
                return Err(SvgdxError::ReferenceError(elref));
            }
        }
        Ok(element.clone())
    }

    pub fn size(&self, ctx: &impl ElementMap) -> Result<Option<Size>> {
        // NOTE: unlike bbox, this does not replace missing values with '0'.
        // Assumes any dw / dh have already been applied.

        // The width/height cases cover rect-like elements, but they are also used
        // as intermediate (e.g. `wh` expansion) size attributes for other elements.
        let mut width = None;
        let mut height = None;
        if let Some(w) = self.attrs.get("width") {
            width = Some(strp(w)?);
        }
        if let Some(h) = self.attrs.get("height") {
            height = Some(strp(h)?);
        }
        match self.name.as_str() {
            "use" | "reuse" => {
                let target_el = self.get_target_element(ctx)?;
                // Take a _copy_ of the target element and evaluate attributes
                // (should really only evaluate those which contribute to size...)
                // This allows 'reuse' attributes which appear as vars within the
                // target's context to determine size.
                // let mut target_el = target_el.clone();
                // target_el.eval_attributes(ctx)?;
                if let Some(sz) = target_el.size(ctx)? {
                    width = Some(sz.width);
                    height = Some(sz.height);
                }
            }
            "g" | "symbol" => {
                if let Some(bb) = self.content_bbox {
                    width = Some(bb.width());
                    height = Some(bb.height());
                }
            }
            "point" | "text" => {
                width = Some(0.);
                height = Some(0.);
            }
            "circle" => {
                if let Some(r) = self.attrs.get("r").map(|n| strp(n)).transpose()? {
                    width = Some(r * 2.0);
                    height = Some(r * 2.0);
                }
            }
            "ellipse" => {
                let rx = self.attrs.get("rx").map(|n| strp(n)).transpose()?;
                let ry = self.attrs.get("ry").map(|n| strp(n)).transpose()?;
                if let Some(rx) = rx {
                    width = Some(rx * 2.0);
                }
                if let Some(ry) = ry {
                    height = Some(ry * 2.0);
                }
            }
            "line" => {
                let x1 = self.attrs.get("x1").map(|n| strp(n)).transpose()?;
                let x2 = self.attrs.get("x2").map(|n| strp(n)).transpose()?;
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    width = Some((x2 - x1).abs());
                }
                let y1 = self.attrs.get("y1").map(|n| strp(n)).transpose()?;
                let y2 = self.attrs.get("y2").map(|n| strp(n)).transpose()?;
                if let (Some(y1), Some(y2)) = (y1, y2) {
                    height = Some((y2 - y1).abs());
                }
            }
            _ => {
                if let Some(bb) = self.bbox()? {
                    width = Some(bb.width());
                    height = Some(bb.height());
                }
            }
        }
        if let (Some(width), Some(height)) = (width, height) {
            Ok(Some(Size::new(width, height)))
        } else {
            Ok(None)
        }
    }

    pub fn bbox(&self) -> Result<Option<BoundingBox>> {
        let mut el_bbox = if self.content_bbox.is_some() {
            // container elements (`g`, `symbol`, `clipPath` etc) set this
            // to the bbox of their contents
            self.content_bbox
        } else {
            self.bbox_raw()?
        };
        // apply any `transform` attr transformations to the bbox
        if let (Some(transform), Some(ref mut bbox)) = (self.get_attr("transform"), &mut el_bbox) {
            let transform: TransformAttr = transform.parse()?;
            el_bbox = Some(transform.apply(bbox));
        }
        Ok(el_bbox)
    }

    fn bbox_raw(&self) -> Result<Option<BoundingBox>> {
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
                && !(value.contains(VAR_PREFIX)
                    || value.contains(ELREF_ID_PREFIX)
                    || value.contains(ELREF_PREVIOUS))
        }
        Ok(match self.name.as_str() {
            "point" | "text" => {
                let x = self.attrs.get("x").unwrap_or(&zstr);
                let y = self.attrs.get("y").unwrap_or(&zstr);
                if passthrough(x) || passthrough(y) {
                    return Ok(None);
                }
                let x = strp(x)?;
                let y = strp(y)?;
                Some(BoundingBox::new(x, y, x, y))
            }
            "box" | "rect" | "image" | "svg" | "foreignObject" => {
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
                    Some(BoundingBox::new(x, y, x + w, y + h))
                } else {
                    None
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
                Some(BoundingBox::new(
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
                        Some(BoundingBox::new(min_x, min_y, max_x, max_y))
                    } else {
                        None // Insufficient points for bbox
                    }
                } else {
                    None
                }
            }
            "path" => path_bbox(self)?,
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
                    Some(BoundingBox::new(cx - r, cy - r, cx + r, cy + r))
                } else {
                    None
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
                    Some(BoundingBox::new(cx - rx, cy - ry, cx + rx, cy + ry))
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    fn translated(&self, dx: f32, dy: f32) -> Result<Self> {
        let mut new_elem = self.clone();
        for (key, value) in &self.attrs {
            match key.as_str() {
                "x" | "cx" | "x1" | "x2" => {
                    new_elem.set_attr(key, &fstr(strp(value)? + dx));
                }
                "y" | "cy" | "y1" | "y2" => {
                    new_elem.set_attr(key, &fstr(strp(value)? + dy));
                }
                _ => (),
            }
        }
        Ok(new_elem)
    }

    fn position_from_bbox(&mut self, bb: &BoundingBox, inscribe: bool) {
        let width = bb.width();
        let height = bb.height();
        let (cx, cy) = bb.center();
        let (x1, y1) = bb.locspec(LocSpec::TopLeft);
        match self.name.as_str() {
            "rect" | "box" => {
                self.attrs.insert("x", fstr(x1));
                self.attrs.insert("y", fstr(y1));
                self.attrs.insert("width", fstr(width));
                self.attrs.insert("height", fstr(height));
            }
            "circle" => {
                self.attrs.insert("cx", fstr(cx));
                self.attrs.insert("cy", fstr(cy));
                let r = if inscribe {
                    0.5 * width.min(height)
                } else {
                    0.5 * width.max(height) * SQRT_2
                };
                self.attrs.insert("r", fstr(r));
            }
            "ellipse" => {
                self.attrs.insert("cx", fstr(cx));
                self.attrs.insert("cy", fstr(cy));
                let rx = if inscribe {
                    0.5 * width
                } else {
                    0.5 * width * SQRT_2
                };
                let ry = if inscribe {
                    0.5 * height
                } else {
                    0.5 * height * SQRT_2
                };
                self.attrs.insert("rx", fstr(rx));
                self.attrs.insert("ry", fstr(ry));
            }
            _ => {}
        }
    }

    fn is_size_attr(&self, name: &str) -> bool {
        if self.name == "text" || self.name == "point" {
            return false;
        }
        let mut size_attr = matches!(name, "width" | "height");
        size_attr = size_attr || (self.name == "circle" && name == "r");
        size_attr = size_attr || (self.name == "ellipse" && (name == "rx" || name == "ry"));
        size_attr
    }

    fn is_pos_attr(&self, name: &str) -> bool {
        // Position attributes are identical for all element types
        matches!(name, "x" | "y" | "x1" | "y1" | "x2" | "y2" | "cx" | "cy")
    }

    fn eval_size_attr(&self, name: &str, value: &str, ctx: &impl ElementMap) -> Result<String> {
        if let Ok(attr_ss) = ScalarSpec::from_str(name) {
            if let (Some(el), remain) = split_relspec(value, ctx)? {
                if let Ok(Some(bbox)) = ctx.get_element_bbox(el) {
                    // default value - same 'type' as attr name, e.g. y2 => ymax
                    let mut v = bbox.scalarspec(attr_ss);
                    // "[~scalarspec][ delta]"
                    let (ss_str, dxy) = remain.split_once(' ').unwrap_or((remain, ""));
                    if let Some(ss) = ss_str.strip_prefix(SCALARSPEC_SEP) {
                        v = bbox.scalarspec(ss.parse()?);
                    }
                    if let Ok(len) = strp_length(dxy) {
                        v = len.adjust(v);
                    }
                    return Ok(fstr(v));
                }
            }
        }
        Ok(value.to_owned())
    }

    fn eval_pos_attr(&self, name: &str, value: &str, ctx: &impl ElementMap) -> Result<String> {
        if let Ok(attr_ss) = ScalarSpec::from_str(name) {
            if let (Some(el), remain) = split_relspec(value, ctx)? {
                if let Ok(Some(bbox)) = ctx.get_element_bbox(el) {
                    return self.pos_attr_helper(remain, &bbox, attr_ss);
                }
            }
        }
        Ok(value.to_owned())
    }

    fn pos_attr_helper(
        &self,
        remain: &str,
        bbox: &BoundingBox,
        attr_ss: ScalarSpec,
    ) -> Result<String> {
        // default value - same 'type' as attr name, e.g. y2 => ymax
        let mut v = bbox.scalarspec(attr_ss);

        // 'position' attribute, e.g. x/y/cx...
        let (loc_str, dxy) = remain.split_once(' ').unwrap_or((remain, ""));
        if let Some(ss) = loc_str.strip_prefix(SCALARSPEC_SEP) {
            // "[~scalarspec][ delta]"
            v = bbox.scalarspec(ss.parse()?);
            if let Ok(len) = strp_length(dxy) {
                v = len.adjust(v);
            }
        } else {
            let mut loc = if self.name == "text" {
                // text elements (currently) have no size, and default
                // to center of the referenced element
                LocSpec::from_str(&self.get_attr("text-loc").unwrap_or("c".to_owned()))?
            } else {
                // otherwise anchor on the same side as the attribute, e.g. x2="#abc"
                // will set x2 to the 'x2' (i.e. right edge) of #abc
                attr_ss.into()
            };
            // "[@loc][ dx dy]"
            if let Some(ls) = loc_str.strip_prefix(LOCSPEC_SEP) {
                loc = ls.parse()?;
            } else if !loc_str.is_empty() {
                return Err(SvgdxError::ParseError(format!(
                    "Could not parse '{loc_str}' in this context",
                )));
            }
            let (x, y) = bbox.locspec(loc);
            let (dx, dy) = self.extract_dx_dy(dxy)?;
            use ScalarSpec::*;
            v = match attr_ss {
                Minx | Maxx | Cx => x + dx,
                Miny | Maxy | Cy => y + dy,
                _ => v,
            };
        }
        Ok(fstr(v).to_string())
    }

    /// Extract dx/dy from a string such as '10 20' or '10' (in which case both are 10)
    fn extract_dx_dy(&self, input: &str) -> Result<(f32, f32)> {
        let mut parts = attr_split_cycle(input);
        let dx = strp(&parts.next().unwrap_or("0".to_string()))?;
        let dy = strp(&parts.next().unwrap_or("0".to_string()))?;
        Ok((dx, dy))
    }

    pub fn eval_rel_attributes(&mut self, ctx: &impl ElementMap) -> Result<()> {
        for (key, value) in self.attrs.clone() {
            if self.is_size_attr(&key) {
                let computed = self.eval_size_attr(&key, &value, ctx)?;
                if strp(&computed).is_ok() {
                    self.attrs.insert(key.clone(), computed);
                }
            } else if self.is_pos_attr(&key) {
                let computed = self.eval_pos_attr(&key, &value, ctx)?;
                if strp(&computed).is_ok() {
                    self.attrs.insert(key.clone(), computed);
                }
            }
        }
        Ok(())
    }

    fn eval_text_anchor(&mut self, ctx: &impl ContextView) -> Result<()> {
        // we do some of this processing as part of positioning, but don't want
        // to be tightly coupled to that.
        let input = self.attrs.get("xy");
        if let Some(input) = input {
            let (_, rel_loc) = split_relspec(input, ctx)?;
            let rel_loc = rel_loc.split_whitespace().next().unwrap_or_default();
            if let Some(rel) = rel_loc.strip_prefix(RELPOS_SEP) {
                match rel.parse()? {
                    DirSpec::Above => self.set_default_attr("text-loc", "t"),
                    DirSpec::Below => self.set_default_attr("text-loc", "b"),
                    DirSpec::InFront => self.set_default_attr("text-loc", "r"),
                    DirSpec::Behind => self.set_default_attr("text-loc", "l"),
                }
            } else if let Some(loc) = rel_loc.strip_prefix(LOCSPEC_SEP) {
                if let Ok(loc_spec) = loc.parse::<LocSpec>() {
                    match loc_spec {
                        LocSpec::TopLeft => self.set_default_attr("text-loc", "tl"),
                        LocSpec::Top => self.set_default_attr("text-loc", "t"),
                        LocSpec::TopRight => self.set_default_attr("text-loc", "tr"),
                        LocSpec::Right => self.set_default_attr("text-loc", "r"),
                        LocSpec::BottomRight => self.set_default_attr("text-loc", "br"),
                        LocSpec::Bottom => self.set_default_attr("text-loc", "b"),
                        LocSpec::BottomLeft => self.set_default_attr("text-loc", "bl"),
                        LocSpec::Left => self.set_default_attr("text-loc", "l"),
                        LocSpec::Center => self.set_default_attr("text-loc", "c"),
                        LocSpec::TopEdge(_) => self.set_default_attr("text-loc", "t"),
                        LocSpec::BottomEdge(_) => self.set_default_attr("text-loc", "b"),
                        LocSpec::LeftEdge(_) => self.set_default_attr("text-loc", "l"),
                        LocSpec::RightEdge(_) => self.set_default_attr("text-loc", "r"),
                    }
                } else {
                    return Err(SvgdxError::InvalidData(format!(
                        "Could not derive text anchor: '{}'",
                        loc
                    )));
                }
            }
        }
        Ok(())
    }

    /// Direction relative positioning - horizontally below, above, to the left, or to the
    /// right of the referenced element.
    /// ELREF '|' DIR ' ' [gap]
    /// DIR values:
    ///   h - horizontal to the right
    ///   H - horizontal to the left
    ///   v - vertical below
    ///   V - vertical above
    pub fn pos_from_dirspec(&self, ctx: &impl ContextView) -> Result<Option<Position>> {
        let input = self.attrs.get("xy");
        if input.is_none() {
            return Ok(None);
        }
        let input = input.unwrap();
        if !input.contains(RELPOS_SEP) {
            return Ok(None);
        }
        // element-relative position can only be applied via xy attribute
        // containing RELPOS_SEP.
        let (ref_el, remain) = split_relspec(input, ctx)?;
        let ref_el = match ref_el {
            Some(el) => el,
            None => return Ok(None),
        };
        if let (Some(bbox), Some(skip_rp_sep)) = (
            ctx.get_element_bbox(ref_el)?,
            remain.strip_prefix(RELPOS_SEP),
        ) {
            let parts = skip_rp_sep.find(|c: char| c.is_whitespace());
            let (reldir, remain) = if let Some(split_idx) = parts {
                let (a, b) = skip_rp_sep.split_at(split_idx);
                (a, b.trim_start())
            } else {
                (skip_rp_sep, "")
            };
            let rel: DirSpec = reldir.parse()?;
            // We won't have the full *position* of this element at this point, but hopefully
            // we have enough to determine its size.
            let (this_width, this_height) = self.size(ctx)?.unwrap_or(Size::new(0., 0.)).as_wh();
            let gap = if !remain.is_empty() {
                let mut parts = attr_split(remain);
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

            let mut pos = Position::new(self.name.clone());
            if self.name.as_str() == "use" {
                // Need to determine top-left corner of the target bbox which
                // may not be (0, 0), and offset by the equivalent amount.
                if let Some(bbox) = self.get_target_element(ctx)?.bbox()? {
                    let (tx, ty) = bbox.locspec(LocSpec::TopLeft);
                    pos.xmin = Some(x + dx - tx);
                    pos.ymin = Some(y + dy - ty);
                }
            } else {
                pos.xmin = Some(x + dx);
                pos.xmax = Some(x + dx + this_width);
                pos.ymin = Some(y + dy);
                pos.ymax = Some(y + dy + this_height);
            }
            return Ok(Some(pos));
        }
        Ok(None)
    }

    fn split_compound_attr(value: &str) -> (String, String) {
        // wh="10" -> width="10", height="10"
        // wh="10 20" -> width="10", height="20"
        // wh="#thing" -> width="#thing", height="#thing"
        // wh="#thing 50%" -> width="#thing 50%", height="#thing 50%"
        // wh="#thing 10 20" -> width="#thing 10", height="#thing 20"
        if value.starts_with([ELREF_ID_PREFIX, ELREF_PREVIOUS]) {
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

    fn resolve_size_delta(&mut self) {
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
        // NOTE: must have already done any relative positioning (e.g. `xy="#abc|h"`)
        // before this point as xy is not considered a compound attribute in that case.
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
    use std::collections::HashMap;

    use crate::types::ElRef;

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
    fn test_eval_size_attr() {
        let mut ctx = TestContext::default();

        ctx.add(
            "abc",
            SvgElement::new(
                "rect",
                &[
                    (String::from("x"), String::from("0")),
                    (String::from("y"), String::from("0")),
                    (String::from("width"), String::from("10")),
                    (String::from("height"), String::from("20")),
                ],
            ),
        );

        let element = SvgElement::new("rect", &[]);

        let result = element.eval_size_attr("width", "#abc", &ctx);
        assert_eq!(result.unwrap(), "10");
        let result = element.eval_size_attr("height", "#abc", &ctx);
        assert_eq!(result.unwrap(), "20");
        let result = element.eval_size_attr("width", "#abc~h", &ctx);
        assert_eq!(result.unwrap(), "20");
        let result = element.eval_size_attr("width", "#abc~h 25%", &ctx);
        assert_eq!(result.unwrap(), "5");
        let result = element.eval_size_attr("height", "#abc~w -3", &ctx);
        assert_eq!(result.unwrap(), "7");
        let result = element.eval_size_attr("width", "#abc~hkjhdsfg", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_pos_edge() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        // Test with edge positioning
        let result = element.pos_attr_helper("@t:25%", &bbox, ScalarSpec::Minx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "25");

        let result = element.pos_attr_helper("@t:25% -4", &bbox, ScalarSpec::Minx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "21");

        let result = element.pos_attr_helper("@r:200%", &bbox, ScalarSpec::Minx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "100");
        let result = element.pos_attr_helper("@r:200%", &bbox, ScalarSpec::Miny);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "200");

        let result = element.pos_attr_helper("@l:-1", &bbox, ScalarSpec::Miny);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "99");

        let result = element.pos_attr_helper("@l:37", &bbox, ScalarSpec::Miny);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "37");

        let result = element.pos_attr_helper("@l:37 3 5", &bbox, ScalarSpec::Minx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "3");
        let result = element.pos_attr_helper("@l:37 3 5", &bbox, ScalarSpec::Miny);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "42");
    }

    #[test]
    fn test_eval_pos_loc() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        // Test with location positioning
        let result = element.pos_attr_helper("@tr", &bbox, ScalarSpec::Minx);
        assert_eq!(result.unwrap(), "100");
        let result = element.pos_attr_helper("@tr", &bbox, ScalarSpec::Miny);
        assert_eq!(result.unwrap(), "0");

        let result = element.pos_attr_helper("@bl", &bbox, ScalarSpec::Minx);
        assert_eq!(result.unwrap(), "0");
        let result = element.pos_attr_helper("@bl", &bbox, ScalarSpec::Miny);
        assert_eq!(result.unwrap(), "100");

        let result = element.pos_attr_helper("@c", &bbox, ScalarSpec::Minx);
        assert_eq!(result.unwrap(), "50");
        let result = element.pos_attr_helper("@c", &bbox, ScalarSpec::Miny);
        assert_eq!(result.unwrap(), "50");
    }

    #[test]
    fn test_eval_pos_invalid() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        // empty should be fine
        let result = element.pos_attr_helper("", &bbox, ScalarSpec::Minx);
        assert_eq!(result.unwrap(), "0");

        // Test with invalid input
        let result = element.pos_attr_helper("invalid", &bbox, ScalarSpec::Minx);
        assert!(result.is_err());

        // Scalar-spec isn't valid for pos_attr_helper
        let result = element.pos_attr_helper("~w", &bbox, ScalarSpec::Minx);
        assert_eq!(result.unwrap(), "100");

        let result = element.pos_attr_helper(" 30 20", &bbox, ScalarSpec::Minx);
        assert_eq!(result.unwrap(), "30");
    }

    #[derive(Default)]
    struct TestContext {
        elements: HashMap<String, SvgElement>,
    }

    impl ElementMap for TestContext {
        fn get_element(&self, id: &ElRef) -> Option<&SvgElement> {
            if let ElRef::Id(id) = id {
                return self.elements.get(id);
            }
            None
        }

        fn get_element_bbox(&self, el: &SvgElement) -> Result<Option<BoundingBox>> {
            el.bbox()
        }

        fn get_element_size(&self, el: &SvgElement) -> Result<Option<Size>> {
            el.size(self)
        }
    }

    impl TestContext {
        fn add(&mut self, id: &str, element: SvgElement) {
            self.elements.insert(id.to_owned(), element);
        }
    }

    #[test]
    fn test_expand_relspec() {
        let mut ctx = TestContext::default();

        ctx.add(
            "abc",
            SvgElement::new(
                "rect",
                &[
                    (String::from("x"), String::from("0")),
                    (String::from("y"), String::from("0")),
                    (String::from("width"), String::from("10")),
                    (String::from("height"), String::from("20")),
                ],
            ),
        );

        let out = expand_relspec("#abc~w", &ctx);
        assert_eq!(out, "10");
        let out = expand_relspec("#abc@br", &ctx);
        assert_eq!(out, "10 20");
        let out = expand_relspec("1 2 #abc@t 3 4 #abc~h 5 6 #abc@c", &ctx);
        assert_eq!(out, "1 2 5 0 3 4 20 5 6 5 10");
    }
}
