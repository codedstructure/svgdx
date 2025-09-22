use std::f32::consts::{FRAC_1_SQRT_2, SQRT_2};
use std::str::FromStr;

use super::SvgElement;
use crate::constants::{
    EDGESPEC_SEP, ELREF_ID_PREFIX, ELREF_NEXT, ELREF_PREVIOUS, LOCSPEC_SEP, RELPOS_SEP,
    SCALARSPEC_SEP, VAR_PREFIX,
};
use crate::context::{ContextView, ElementMap};
use crate::elements::line_offset::get_point_along_linelike_type_el;
use crate::elements::path::path_bbox;
use crate::errors::{Result, SvgdxError};
use crate::geometry::{
    strp_length, BoundingBox, DirSpec, ElementLoc, LocSpec, Position, ScalarSpec, Size,
    TransformAttr, TrblLength,
};
use crate::types::{attr_split, attr_split_cycle, extract_elref, fstr, split_compound_attr, strp};

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
        if let Some(idx) = value.find([ELREF_ID_PREFIX, ELREF_PREVIOUS, ELREF_NEXT]) {
            result.push_str(&value[..idx]);
            value = &value[idx..];
            if let Some(mut idx) = value[1..].find(word_break) {
                if value.starts_with(ELREF_ID_PREFIX) {
                    idx += 1; // account for ignoring # in word break search
                } else {
                    // account for ignoring ^/+/^^^ in word break search
                    let elref_char = if value.starts_with(ELREF_PREVIOUS) {
                        ELREF_PREVIOUS
                    } else {
                        ELREF_NEXT
                    };
                    let new_s = value.trim_start_matches(elref_char);
                    idx = value.len() - new_s.len(); // asummes elref_char is 1 byte
                }
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
        if rest.is_empty() && elem.name() == "point" {
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

// Elements which have a bounding box, including graphical elements
// and selected containers.
pub fn is_layout_element(el: &SvgElement) -> bool {
    matches!(
        el.name(),
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
            // Following also have at least some level of layout
            | "g"
            | "symbol"
            | "clipPath"
            | "box"
            | "point"
            | "svg"
            | "foreignObject"
    )
}

// Elements wh
impl SvgElement {
    pub(crate) fn extract_relpos(&mut self) -> Option<String> {
        if self.get_attr("xy").unwrap_or_default().contains(RELPOS_SEP) {
            return Some(self.pop_attr("xy").unwrap().to_string());
        }
        None
    }

    pub fn resolve_position(&mut self, ctx: &mut impl ContextView) -> Result<()> {
        // Ensure relative ElRef offsets are resolved wrt this element
        ctx.set_current_element(self);

        // Evaluate any expressions (e.g. var lookups or {{..}} blocks) in attributes
        // TODO: this is not idempotent in the case of e.g. RNG lookups, so should be
        // moved out of this function and called once per element (or this function
        // should be called once per element...)
        self.eval_attributes(ctx)?;

        self.handle_containment(ctx)?;

        // Need size before can evaluate relative position
        expand_compound_size(self);
        eval_rel_attributes(self, ctx)?;
        resolve_size_delta(self);

        // ensure relatively-positioned text elements have appropriate anchors
        if self.name() == "text" && self.has_attr("text") {
            eval_text_anchor(self, ctx)?;
        }

        if let ("polyline" | "polygon", Some(points)) = (self.name(), self.get_attr("points")) {
            self.set_attr("points", &expand_relspec(points, ctx));
        }
        if let ("path", Some(d)) = (self.name(), self.get_attr("d")) {
            self.set_attr("d", &expand_relspec(d, ctx));
        }

        // TODO: issue is that this could fail with a reference error
        // which would be resolved by expand_relspec, though that requires
        // eval_rel_attributes to be called first...
        if let Some(relpos) = self.extract_relpos() {
            if let Some(mut pos) = self.pos_from_dirspec(&relpos, ctx)? {
                if let Some(bb) = self.content_bbox {
                    pos.translate(-bb.x1, -bb.y1);
                }
                pos.set_position_attrs(self);
            }
        }
        // Compound attributes, e.g. xy="#o 2" -> x="#o 2", y="#o 2"
        expand_compound_pos(self);
        eval_rel_attributes(self, ctx)?;

        let mut p = Position::try_from(self as &SvgElement)?;
        if self.name() == "g" {
            if let Some(bb) = self.content_bbox {
                p.update_size(&Size::new(bb.width(), bb.height()));
                p.translate(-bb.x1, -bb.y1);
            }
        } else if self.name() == "use" {
            let el = ctx.get_target_element(self)?;
            if let Some(sz) = el.size(ctx)? {
                p.update_size(&sz);
                if let "circle" | "ellipse" = el.name() {
                    // The referenced element is defined by its center,
                    // but use elements are defined by top-left pos.
                    p.translate(sz.width / 2., sz.height / 2.);
                }
            }
        }
        p.set_position_attrs(self);

        Ok(())
    }

    pub fn get_element_loc_coord(
        &self,
        elem_map: &impl ElementMap,
        loc: ElementLoc,
    ) -> Result<(f32, f32)> {
        match loc {
            ElementLoc::LineOffset(l) => get_point_along_linelike_type_el(self, l),
            ElementLoc::LocSpec(spec) => Ok(elem_map
                .get_element_bbox(self)?
                .ok_or_else(|| SvgdxError::MissingBoundingBox(self.to_string()))?
                .locspec(spec)),
        }
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

        for elref in attr_split(ref_list) {
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
                    el.inscribed_bbox(self.name())
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
            position_from_bbox(self, &bb, !is_surround);
        }
        self.add_class(&format!("d-{contain_str}"));
        self.remove_attrs(&["surround", "inside", "margin"]);
        Ok(())
    }

    /// Calculate bounding box of target_shape inside self
    fn inscribed_bbox(&self, target_shape: &str) -> Result<Option<BoundingBox>> {
        let zstr = "0";
        match (target_shape, self.name()) {
            // rect inside circ
            ("rect", "circle") => {
                if let Some(r) = self.get_attr("r") {
                    let cx = self.get_attr("cx").unwrap_or(zstr);
                    let cy = self.get_attr("cy").unwrap_or(zstr);
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
                if let (Some(rx), Some(ry)) = (self.get_attr("rx"), self.get_attr("ry")) {
                    let cx = self.get_attr("cx").unwrap_or(zstr);
                    let cy = self.get_attr("cy").unwrap_or(zstr);
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

    pub fn size(&self, ctx: &impl ElementMap) -> Result<Option<Size>> {
        // NOTE: unlike bbox, this does not replace missing values with '0'.
        // Assumes any dw / dh have already been applied.

        // The width/height cases cover rect-like elements, but they are also used
        // as intermediate (e.g. `wh` expansion) size attributes for other elements.
        let mut width = None;
        let mut height = None;
        if let Some(w) = self.get_attr("width") {
            width = Some(strp(w)?);
        }
        if let Some(h) = self.get_attr("height") {
            height = Some(strp(h)?);
        }
        match self.name() {
            "use" | "reuse" => {
                let target_el = ctx.get_target_element(self)?;
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
                if let Some(r) = self.get_attr("r").map(strp).transpose()? {
                    width = Some(r * 2.0);
                    height = Some(r * 2.0);
                }
            }
            "ellipse" => {
                let rx = self.get_attr("rx").map(strp).transpose()?;
                let ry = self.get_attr("ry").map(strp).transpose()?;
                if let Some(rx) = rx {
                    width = Some(rx * 2.0);
                }
                if let Some(ry) = ry {
                    height = Some(ry * 2.0);
                }
            }
            "line" => {
                let x1 = self.get_attr("x1").map(strp).transpose()?;
                let x2 = self.get_attr("x2").map(strp).transpose()?;
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    width = Some((x2 - x1).abs());
                }
                let y1 = self.get_attr("y1").map(strp).transpose()?;
                let y2 = self.get_attr("y2").map(strp).transpose()?;
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
        // TODO: this is a hack; forward references (i.e. '+') may cause bounding box
        // evaluation of an element which hasn't had attributes expanded yet, and if
        // there is otherwise sufficient info (e.g. with x/y etc assumed zero) for a bbox.
        // Should probably pre-expand attributes first...
        // Also, elements with clip-path overload content_bbox to be the clip region,
        // in which case we may still validly have these other attributes...
        // TODO: avoid overloading different content_bbox uses.
        if !self.has_attr("clip-path") {
            for key in ["xy", "cxy", "wh", "dwh", "dw", "dh", "rxy", "xy1", "xy2"] {
                if self.has_attr(key) {
                    return Err(SvgdxError::MissingBoundingBox(key.to_owned()));
                }
            }
            if self.name() == "g" {
                // for group elements, *any* positional attributes imply positioning
                // hasn't yet been resolved.
                for key in ["x", "y", "cx", "cy", "x1", "y1", "x2", "y2"] {
                    if self.has_attr(key) {
                        return Err(SvgdxError::MissingBoundingBox(key.to_owned()));
                    }
                }
            }
        }
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
        let zstr = "0";
        // if not a number and not a refspec, pass it through without computing a bbox
        // this is needed to ultimately pass through e.g. "10cm" or "5%" as-is without
        // attempting to compute a bounding box.
        fn passthrough(value: &str) -> bool {
            // if attrs cannot be converted to f32 *and* do not contain '$'/'#'/'^'/'+'
            // (which might be resolved later) then return Ok(None).
            // This will return `true` for things such as "10%" or "40mm".
            strp(value).is_err()
                && !(value.contains(VAR_PREFIX)
                    || value.contains(ELREF_ID_PREFIX)
                    || value.contains(ELREF_PREVIOUS)
                    || value.contains(ELREF_NEXT))
        }
        Ok(match self.name() {
            "point" | "text" => {
                let x = self.get_attr("x").unwrap_or(zstr);
                let y = self.get_attr("y").unwrap_or(zstr);
                if passthrough(x) || passthrough(y) {
                    return Ok(None);
                }
                let x = strp(x)?;
                let y = strp(y)?;
                Some(BoundingBox::new(x, y, x, y))
            }
            "box" | "rect" | "image" | "svg" | "foreignObject" => {
                if let (Some(w), Some(h)) = (self.get_attr("width"), self.get_attr("height")) {
                    let x = self.get_attr("x").unwrap_or(zstr);
                    let y = self.get_attr("y").unwrap_or(zstr);
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
                let x1 = self.get_attr("x1").unwrap_or(zstr);
                let y1 = self.get_attr("y1").unwrap_or(zstr);
                let x2 = self.get_attr("x2").unwrap_or(zstr);
                let y2 = self.get_attr("y2").unwrap_or(zstr);
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

                if let Some(points) = self.get_attr("points") {
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
                if let Some(r) = self.get_attr("r") {
                    let cx = self.get_attr("cx").unwrap_or(zstr);
                    let cy = self.get_attr("cy").unwrap_or(zstr);
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
                if let (Some(rx), Some(ry)) = (self.get_attr("rx"), self.get_attr("ry")) {
                    let cx = self.get_attr("cx").unwrap_or(zstr);
                    let cy = self.get_attr("cy").unwrap_or(zstr);
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

    pub fn translated(&self, dx: f32, dy: f32) -> Result<Self> {
        let mut new_elem = self.clone();
        for (key, value) in self.get_attrs() {
            match key.as_str() {
                "x" | "cx" | "x1" | "x2" => {
                    new_elem.set_attr(&key, &fstr(strp(&value)? + dx));
                }
                "y" | "cy" | "y1" | "y2" => {
                    new_elem.set_attr(&key, &fstr(strp(&value)? + dy));
                }
                _ => (),
            }
        }
        Ok(new_elem)
    }

    /// Direction relative positioning - horizontally below, above, to the left, or to the
    /// right of the referenced element.
    /// `ELREF '|' DIR ' ' [gap]`
    /// DIR values:
    ///   h - horizontal to the right
    ///   H - horizontal to the left
    ///   v - vertical below
    ///   V - vertical above
    pub fn pos_from_dirspec(
        &self,
        input: &str,
        ctx: &impl ContextView,
    ) -> Result<Option<Position>> {
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
        let remain = remain.trim_start_matches(RELPOS_SEP);
        if let Some(bbox) = ctx.get_element_bbox(ref_el)? {
            let parts = remain.find(|c: char| c.is_whitespace());
            let (reldir, remain) = if let Some(split_idx) = parts {
                let (a, b) = remain.split_at(split_idx);
                (a, b.trim_start())
            } else {
                (remain, "")
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

            let mut pos = Position::new(self.name());
            if self.name() == "use" {
                // Need to determine top-left corner of the target bbox which
                // may not be (0, 0), and offset by the equivalent amount.
                if let Some(bbox) = ctx.get_target_element(self)?.bbox()? {
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
            Ok(Some(pos))
        } else {
            Err(SvgdxError::MissingBoundingBox(format!("No bbox: {input}")))
        }
    }
}

fn position_from_bbox(element: &mut SvgElement, bb: &BoundingBox, inscribe: bool) {
    let width = bb.width();
    let height = bb.height();
    let (cx, cy) = bb.center();
    let (x1, y1) = bb.locspec(LocSpec::TopLeft);
    match element.name() {
        "rect" | "box" => {
            element.set_attr("x", &fstr(x1));
            element.set_attr("y", &fstr(y1));
            element.set_attr("width", &fstr(width));
            element.set_attr("height", &fstr(height));
        }
        "circle" => {
            element.set_attr("cx", &fstr(cx));
            element.set_attr("cy", &fstr(cy));
            let r = if inscribe {
                0.5 * width.min(height)
            } else {
                0.5 * width.max(height) * SQRT_2
            };
            element.set_attr("r", &fstr(r));
        }
        "ellipse" => {
            element.set_attr("cx", &fstr(cx));
            element.set_attr("cy", &fstr(cy));
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
            element.set_attr("rx", &fstr(rx));
            element.set_attr("ry", &fstr(ry));
        }
        _ => {}
    }
}

fn resolve_size_delta(element: &mut SvgElement) {
    // assumes "width"/"height"/"r"/"rx"/"ry" are numeric if present
    let (w, h) = match element.name() {
        "circle" => {
            let diam = element.get_attr("r").map(|r| 2. * strp(r).unwrap_or(0.));
            (diam, diam)
        }
        "ellipse" => (
            element
                .get_attr("rx")
                .and_then(|rx| strp(rx).ok())
                .map(|x| x * 2.),
            element
                .get_attr("ry")
                .and_then(|ry| strp(ry).ok())
                .map(|x| x * 2.),
        ),
        _ => (
            element.get_attr("width").and_then(|w| strp(w).ok()),
            element.get_attr("height").and_then(|h| strp(h).ok()),
        ),
    };

    if let Some(dw) = element.pop_attr("dw") {
        if let Ok(Some(new_w)) = strp_length(&dw).map(|dw| w.map(|x| dw.adjust(x))) {
            element.set_attr("width", &fstr(new_w));
        }
    }
    if let Some(dh) = element.pop_attr("dh") {
        if let Ok(Some(new_h)) = strp_length(&dh).map(|dh| h.map(|x| dh.adjust(x))) {
            element.set_attr("height", &fstr(new_h));
        }
    }
}

fn eval_size_attr(name: &str, value: &str, ctx: &impl ElementMap) -> Result<String> {
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

fn eval_pos_attr(
    element: &SvgElement,
    name: &str,
    value: &str,
    ctx: &impl ElementMap,
) -> Result<String> {
    if let Ok(attr_ss) = ScalarSpec::from_str(name) {
        if let (Some(el), remain) = split_relspec(value, ctx)? {
            if let Ok(Some(bbox)) = ctx.get_element_bbox(el) {
                return pos_attr_helper(element, remain, &bbox, attr_ss);
            }
        }
    }
    Ok(value.to_owned())
}

fn pos_attr_helper(
    element: &SvgElement,
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
        let mut loc = if element.name() == "text" {
            // text elements (currently) have no size, and default
            // to center of the referenced element
            LocSpec::from_str(element.get_attr("text-loc").unwrap_or("c"))?
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
        let (dx, dy) = extract_dx_dy(dxy)?;
        use ScalarSpec::*;
        v = match attr_ss {
            Minx | Maxx | Cx => x + dx,
            Miny | Maxy | Cy => y + dy,
            _ => v,
        };
    }
    Ok(fstr(v).to_string())
}

fn is_size_attr(element: &SvgElement, name: &str) -> bool {
    if element.name() == "text" || element.name() == "point" {
        return false;
    }
    let mut size_attr = matches!(name, "width" | "height");
    size_attr = size_attr || (element.name() == "circle" && name == "r");
    size_attr = size_attr || (element.name() == "ellipse" && (name == "rx" || name == "ry"));
    size_attr
}

fn eval_rel_attributes(element: &mut SvgElement, ctx: &impl ElementMap) -> Result<()> {
    for (key, value) in element.get_attrs() {
        if is_size_attr(element, &key) {
            let computed = eval_size_attr(&key, &value, ctx)?;
            if strp(&computed).is_ok() {
                element.set_attr(&key, &computed);
            }
        } else if is_pos_attr(&key) {
            let computed = eval_pos_attr(element, &key, &value, ctx)?;
            if strp(&computed).is_ok() {
                element.set_attr(&key, &computed);
            }
        }
    }
    Ok(())
}

fn eval_text_anchor(element: &mut SvgElement, ctx: &impl ContextView) -> Result<()> {
    // we do some of this processing as part of positioning, but don't want
    // to be tightly coupled to that.
    let input = element.get_attr("xy");
    if let Some(input) = input {
        let (_, rel_loc) = split_relspec(input, ctx)?;
        let rel_loc = rel_loc.split_whitespace().next().unwrap_or_default();
        if let Some(rel) = rel_loc.strip_prefix(RELPOS_SEP) {
            match rel.parse()? {
                DirSpec::Above => element.set_default_attr("text-loc", "t"),
                DirSpec::Below => element.set_default_attr("text-loc", "b"),
                DirSpec::InFront => element.set_default_attr("text-loc", "r"),
                DirSpec::Behind => element.set_default_attr("text-loc", "l"),
            }
        } else if let Some(loc) = rel_loc.strip_prefix(LOCSPEC_SEP) {
            if let Ok(loc_spec) = loc.parse::<LocSpec>() {
                match loc_spec {
                    LocSpec::TopLeft => element.set_default_attr("text-loc", "tl"),
                    LocSpec::Top => element.set_default_attr("text-loc", "t"),
                    LocSpec::TopRight => element.set_default_attr("text-loc", "tr"),
                    LocSpec::Right => element.set_default_attr("text-loc", "r"),
                    LocSpec::BottomRight => element.set_default_attr("text-loc", "br"),
                    LocSpec::Bottom => element.set_default_attr("text-loc", "b"),
                    LocSpec::BottomLeft => element.set_default_attr("text-loc", "bl"),
                    LocSpec::Left => element.set_default_attr("text-loc", "l"),
                    LocSpec::Center => element.set_default_attr("text-loc", "c"),
                    LocSpec::TopEdge(_) => element.set_default_attr("text-loc", "t"),
                    LocSpec::BottomEdge(_) => element.set_default_attr("text-loc", "b"),
                    LocSpec::LeftEdge(_) => element.set_default_attr("text-loc", "l"),
                    LocSpec::RightEdge(_) => element.set_default_attr("text-loc", "r"),
                }
            } else {
                return Err(SvgdxError::InvalidData(format!(
                    "Could not derive text anchor: '{loc}'"
                )));
            }
        }
    }
    Ok(())
}

fn is_pos_attr(name: &str) -> bool {
    // Position attributes are identical for all element types
    matches!(name, "x" | "y" | "x1" | "y1" | "x2" | "y2" | "cx" | "cy")
}

/// Extract dx/dy from a string such as '10 20' or '10' (in which case both are 10)
fn extract_dx_dy(input: &str) -> Result<(f32, f32)> {
    let mut parts = attr_split_cycle(input);
    let dx = strp(&parts.next().unwrap_or("0".to_string()))?;
    let dy = strp(&parts.next().unwrap_or("0".to_string()))?;
    Ok((dx, dy))
}

fn expand_compound_size(el: &mut SvgElement) {
    if let Some(wh) = el.pop_attr("wh") {
        // Split value into width and height
        let (w, h) = split_compound_attr(&wh);
        el.set_default_attr("width", &w);
        el.set_default_attr("height", &h);
    }
    if el.name() == "ellipse" {
        if let Some(rxy) = el.pop_attr("rxy") {
            // Split value into rx and ry
            let (rx, ry) = split_compound_attr(&rxy);
            el.set_default_attr("rx", &rx);
            el.set_default_attr("ry", &ry);
        }
    }
    if let Some(dwh) = el.pop_attr("dwh") {
        // Split value into dw and dh
        let (dw, dh) = split_compound_attr(&dwh);
        el.set_default_attr("dw", &dw);
        el.set_default_attr("dh", &dh);
    }
}

// Compound attributes, e.g.
// xy="#o" -> x="#o", y="#o"
// xy="#o 2" -> x="#o 2", y="#o 2"
// xy="#o 2 4" -> x="#o 2", y="#o 4"
fn expand_compound_pos(el: &mut SvgElement) {
    // NOTE: must have already done any relative positioning (e.g. `xy="#abc|h"`)
    // before this point as xy is not considered a compound attribute in that case.
    if let Some(xy) = el.pop_attr("xy") {
        if xy.contains(RELPOS_SEP) {
            // xy is a relative position spec, e.g. `xy="#abc|h 5 10"`
            // which is not a compound attribute, so restore it.
            el.set_attr("xy", &xy);
        } else {
            let (x, y) = split_compound_attr(&xy);
            let (x_attr, y_attr) = match el.pop_attr("xy-loc").as_deref() {
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
            el.set_default_attr(x_attr, &x);
            el.set_default_attr(y_attr, &y);
        }
    }
    if let Some(cxy) = el.pop_attr("cxy") {
        let (cx, cy) = split_compound_attr(&cxy);
        el.set_default_attr("cx", &cx);
        el.set_default_attr("cy", &cy);
    }
    if let Some(xy1) = el.pop_attr("xy1") {
        let (x1, y1) = split_compound_attr(&xy1);
        el.set_default_attr("x1", &x1);
        el.set_default_attr("y1", &y1);
    }
    if let Some(xy2) = el.pop_attr("xy2") {
        let (x2, y2) = split_compound_attr(&xy2);
        el.set_default_attr("x2", &x2);
        el.set_default_attr("y2", &y2);
    }
    if let Some(dxy) = el.pop_attr("dxy") {
        let (dx, dy) = split_compound_attr(&dxy);
        el.set_default_attr("dx", &dx);
        el.set_default_attr("dy", &dy);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::types::ElRef;

    use super::*;

    #[test]
    fn test_spread_attr() {
        let (w, h) = split_compound_attr("10");
        assert_eq!(w, "10");
        assert_eq!(h, "10");
        let (w, h) = split_compound_attr("10 20");
        assert_eq!(w, "10");
        assert_eq!(h, "20");
        let (w, h) = split_compound_attr("#thing");
        assert_eq!(w, "#thing");
        assert_eq!(h, "#thing");
        let (w, h) = split_compound_attr("#thing 50%");
        assert_eq!(w, "#thing 50%");
        assert_eq!(h, "#thing 50%");
        let (w, h) = split_compound_attr("#thing 10 20");
        assert_eq!(w, "#thing 10");
        assert_eq!(h, "#thing 20");

        let (x, y) = split_compound_attr("^a@tl");
        assert_eq!(x, "^a@tl");
        assert_eq!(y, "^a@tl");
        let (x, y) = split_compound_attr("^a@tl 5");
        assert_eq!(x, "^a@tl 5");
        assert_eq!(y, "^a@tl 5");
        let (x, y) = split_compound_attr("^a@tl 5 7%");
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

        let result = eval_size_attr("width", "#abc", &ctx);
        assert_eq!(result.unwrap(), "10");
        let result = eval_size_attr("height", "#abc", &ctx);
        assert_eq!(result.unwrap(), "20");
        let result = eval_size_attr("width", "#abc~h", &ctx);
        assert_eq!(result.unwrap(), "20");
        let result = eval_size_attr("width", "#abc~h 25%", &ctx);
        assert_eq!(result.unwrap(), "5");
        let result = eval_size_attr("height", "#abc~w -3", &ctx);
        assert_eq!(result.unwrap(), "7");
        let result = eval_size_attr("width", "#abc~hkjhdsfg", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_pos_edge() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        // Test with edge positioning
        let result = pos_attr_helper(&element, "@t:25%", &bbox, ScalarSpec::Minx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "25");

        let result = pos_attr_helper(&element, "@t:25% -4", &bbox, ScalarSpec::Minx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "21");

        let result = pos_attr_helper(&element, "@r:200%", &bbox, ScalarSpec::Minx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "100");
        let result = pos_attr_helper(&element, "@r:200%", &bbox, ScalarSpec::Miny);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "200");

        let result = pos_attr_helper(&element, "@l:-1", &bbox, ScalarSpec::Miny);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "99");

        let result = pos_attr_helper(&element, "@l:37", &bbox, ScalarSpec::Miny);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "37");

        let result = pos_attr_helper(&element, "@l:37 3 5", &bbox, ScalarSpec::Minx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "3");
        let result = pos_attr_helper(&element, "@l:37 3 5", &bbox, ScalarSpec::Miny);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "42");
    }

    #[test]
    fn test_eval_pos_loc() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        // Test with location positioning
        let result = pos_attr_helper(&element, "@tr", &bbox, ScalarSpec::Minx);
        assert_eq!(result.unwrap(), "100");
        let result = pos_attr_helper(&element, "@tr", &bbox, ScalarSpec::Miny);
        assert_eq!(result.unwrap(), "0");

        let result = pos_attr_helper(&element, "@bl", &bbox, ScalarSpec::Minx);
        assert_eq!(result.unwrap(), "0");
        let result = pos_attr_helper(&element, "@bl", &bbox, ScalarSpec::Miny);
        assert_eq!(result.unwrap(), "100");

        let result = pos_attr_helper(&element, "@c", &bbox, ScalarSpec::Minx);
        assert_eq!(result.unwrap(), "50");
        let result = pos_attr_helper(&element, "@c", &bbox, ScalarSpec::Miny);
        assert_eq!(result.unwrap(), "50");
    }

    #[test]
    fn test_eval_pos_invalid() {
        let element = SvgElement::new("rect", &[]);
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        // empty should be fine
        let result = pos_attr_helper(&element, "", &bbox, ScalarSpec::Minx);
        assert_eq!(result.unwrap(), "0");

        // Test with invalid input
        let result = pos_attr_helper(&element, "invalid", &bbox, ScalarSpec::Minx);
        assert!(result.is_err());

        // Scalar-spec isn't valid for pos_attr_helper
        let result = pos_attr_helper(&element, "~w", &bbox, ScalarSpec::Minx);
        assert_eq!(result.unwrap(), "100");

        let result = pos_attr_helper(&element, " 30 20", &bbox, ScalarSpec::Minx);
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
