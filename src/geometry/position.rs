use crate::elements::{Element, SvgElement};
use crate::geometry::{BoundingBox, Size};
use crate::types::{fstr, strp};

#[derive(Clone, Default)]
pub struct Position {
    pub xmin: Option<f32>,
    pub ymin: Option<f32>,
    pub xmax: Option<f32>,
    pub ymax: Option<f32>,
    pub cx: Option<f32>,
    pub cy: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub dx: Option<f32>,
    pub dy: Option<f32>,

    shape: String,
}

impl Position {
    pub fn new(shape: impl Into<String>) -> Self {
        Self {
            shape: shape.into(),
            ..Default::default()
        }
    }

    fn extent(
        &self,
        start: Option<f32>,
        end: Option<f32>,
        middle: Option<f32>,
        length: Option<f32>,
    ) -> Option<(f32, f32)> {
        match (start, end, middle, length) {
            (Some(s), Some(e), _, _) => Some((s, e)),
            (Some(s), _, Some(m), _) => Some((s, s + (m - s) * 2.0)),
            (_, Some(e), Some(m), _) => Some((e - (e - m) * 2.0, e)),
            (Some(s), _, _, Some(l)) => Some((s, s + l)),
            (_, Some(e), _, Some(l)) => Some((e - l, e)),
            (_, _, Some(m), Some(l)) => Some((m - l / 2.0, m + l / 2.0)),
            // The following cases allow lines to be specified by a single start/mid/end
            // value in one dimension, as long as there's only a single such value.
            (Some(m), None, None, None)
            | (None, Some(m), None, None)
            | (None, None, Some(m), None)
                if self.shape == "line" =>
            {
                Some((m, m))
            }
            _ => None,
        }
    }

    // For 'square' elements, a single value in one dimension plus extent
    // in the other leads to extent in both dimensions.
    fn three_point(
        &self,
        extent: f32,
        start: Option<f32>,
        middle: Option<f32>,
        end: Option<f32>,
    ) -> Option<(f32, f32)> {
        let (s, e) = match (start, middle, end) {
            (Some(s), _, _) => (s, s + extent),
            (_, Some(m), _) => (m - extent / 2., m + extent / 2.),
            (_, _, Some(e)) => (e - extent, e),
            _ => return None,
        };
        Some((s, e))
    }

    fn x_def(&self) -> Option<(f32, f32)> {
        self.extent(self.xmin, self.xmax, self.cx, self.width)
    }

    fn y_def(&self) -> Option<(f32, f32)> {
        self.extent(self.ymin, self.ymax, self.cy, self.height)
    }

    pub fn to_bbox(&self) -> Option<BoundingBox> {
        let x_ext = self.x_def();
        let y_ext = self.y_def();
        if let (Some((x1, x2)), Some((y1, y2))) = (x_ext, y_ext) {
            Some(BoundingBox::new(x1, y1, x2, y2))
        } else if self.shape == "point" {
            // For points, we don't need extent at all, just at least one x and at least one y
            let px = self.xmin.or(self.xmax.or(self.cx));
            let py = self.ymin.or(self.ymax.or(self.cy));
            if let (Some(x), Some(y)) = (px, py) {
                Some(BoundingBox::new(x, y, x, y))
            } else {
                None
            }
        } else if self.shape == "circle" {
            // For circles, width and height are the same, so we only need one plus a single
            // other value to define the circle. The same logic would apply for squares,
            // but that's not an SVG primitive.
            if let Some((x1, x2)) = x_ext {
                if let Some((y1, y2)) = self.three_point(x2 - x1, self.ymin, self.cy, self.ymax) {
                    Some(BoundingBox::new(x1, y1, x2, y2))
                } else {
                    let radius = (x2 - x1) / 2.;
                    Some(BoundingBox::new(x1, -radius, x2, radius))
                }
            } else if let Some((y1, y2)) = y_ext {
                if let Some((x1, x2)) = self.three_point(y2 - y1, self.xmin, self.cx, self.xmax) {
                    Some(BoundingBox::new(x1, y1, x2, y2))
                } else {
                    let radius = (y2 - y1) / 2.;
                    Some(BoundingBox::new(-radius, y1, radius, y2))
                }
            } else {
                // if cx/cy (etc) are absent, SVG says they are treated as zero.
                if let Some(diameter) = self.width.or(self.height) {
                    let r = diameter / 2.;
                    Some(BoundingBox::new(-r, -r, r, r))
                } else {
                    None
                }
            }
        } else if let (Some(w), Some(h)) = (self.width, self.height) {
            if let Some((x1, x2)) = x_ext {
                Some(BoundingBox::new(x1, 0., x2, h))
            } else if let Some((y1, y2)) = y_ext {
                Some(BoundingBox::new(0., y1, w, y2))
            } else {
                // if x/y (etc) are absent, SVG says they are treated as zero.
                Some(BoundingBox::new(0., 0., w, h))
            }
        } else {
            None
        }
    }

    fn has_x_position(&self) -> bool {
        self.xmin.is_some() || self.xmax.is_some() || self.cx.is_some() || self.dx.is_some()
    }

    fn has_y_position(&self) -> bool {
        self.ymin.is_some() || self.ymax.is_some() || self.cy.is_some() || self.dy.is_some()
    }

    pub fn update_size(&mut self, sz: &Size) {
        self.width = Some(sz.width);
        self.height = Some(sz.height);
        // TODO: check not overconstrained
    }

    pub fn update_shape(&mut self, shape: &str) {
        self.shape = shape.to_owned();
    }

    /// Get most-reasonable 'x' value for this element,
    /// defaulting to 0 if required. Excludes dx.
    pub fn x(&self) -> f32 {
        if let Some((x1, _)) = self.x_def() {
            x1
        } else {
            self.xmin.unwrap_or(0.)
        }
    }

    /// Get most-reasonable 'y' value for this element,
    /// defaulting to 0 if required. Excludes dy.
    pub fn y(&self) -> f32 {
        if let Some((y1, _)) = self.y_def() {
            y1
        } else {
            self.ymin.unwrap_or(0.)
        }
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.xmin = self.xmin.map(|x| x + dx);
        self.xmax = self.xmax.map(|x| x + dx);
        self.cx = self.cx.map(|x| x + dx);
        self.ymin = self.ymin.map(|y| y + dy);
        self.ymax = self.ymax.map(|y| y + dy);
        self.cy = self.cy.map(|y| y + dy);
    }

    pub fn set_position_attrs(&self, element: &mut SvgElement) {
        // TODO: should this return a Result?
        match (element.name.as_str(), self.to_bbox()) {
            ("g" | "path" | "polyline" | "polygon", _) => {
                // These don't need a full bbox, and are always set via transform in any case.
                self.position_via_transform(element);
            }
            ("use" | "point" | "text", _) => {
                // These only have x & y attrs.
                // TODO: should `length` be set to Some(0.) for these shape types
                // so extent() works and e.g. a 'cxy="5"` works on a `<point>`?
                if self.has_x_position() {
                    element.set_attr("x", &fstr(self.x() + self.dx.unwrap_or(0.)));
                }
                if self.has_y_position() {
                    element.set_attr("y", &fstr(self.y() + self.dy.unwrap_or(0.)));
                }
                element.remove_attrs(&["dw", "dh", "x1", "y1", "x2", "y2", "cx", "cy", "r"]);
                if element.name != "use" {
                    element.remove_attrs(&["width", "height"]);
                }
                if element.name != "text" {
                    element.remove_attrs(&["dx", "dy"]);
                }
            }
            ("" | "rect" | "box" | "image" | "svg" | "foreignObject", Some(bbox)) => {
                let width = bbox.width();
                let height = bbox.height();
                let (x1, y1) = bbox.xy1();
                if self.has_x_position() {
                    element.set_attr("x", &fstr(x1 + self.dx.unwrap_or(0.)));
                }
                if self.has_y_position() {
                    element.set_attr("y", &fstr(y1 + self.dy.unwrap_or(0.)));
                }
                element.set_attr("width", &fstr(width));
                element.set_attr("height", &fstr(height));
                element.remove_attrs(&[
                    "dx", "dy", "dw", "dh", "x1", "y1", "x2", "y2", "cx", "cy", "r",
                ]);
            }
            ("circle", Some(bbox)) => {
                let (cx, cy) = bbox.center();
                let r = bbox.width() / 2.0;
                if self.has_x_position() {
                    element.set_attr("cx", &fstr(cx + self.dx.unwrap_or(0.)));
                }
                if self.has_y_position() {
                    element.set_attr("cy", &fstr(cy + self.dy.unwrap_or(0.)));
                }
                element.set_attr("r", &fstr(r));
                element.remove_attrs(&[
                    "dx", "dy", "dw", "dh", "x", "y", "x1", "y1", "x2", "y2", "rx", "ry", "width",
                    "height",
                ]);
            }
            ("ellipse", Some(bbox)) => {
                let (cx, cy) = bbox.center();
                let rx = bbox.width() / 2.0;
                let ry = bbox.height() / 2.0;
                if self.has_x_position() {
                    element.set_attr("cx", &fstr(cx + self.dx.unwrap_or(0.)));
                }
                if self.has_y_position() {
                    element.set_attr("cy", &fstr(cy + self.dy.unwrap_or(0.)));
                }
                element.set_attr("rx", &fstr(rx));
                element.set_attr("ry", &fstr(ry));
                element.remove_attrs(&[
                    "dx", "dy", "dw", "dh", "x", "y", "x1", "y1", "x2", "y2", "r", "width",
                    "height",
                ]);
            }
            ("line", Some(bbox)) => {
                // NOTE: lines are directional, so we don't want to set x1/y1 from the bbox
                // if they're already set, but we do need to add dx/dy to any existing attrs.
                let zstr = "0".to_owned();
                let (x1, y1) = bbox.xy1();
                if element.get_attr("x1").is_none() {
                    element.set_attr("x1", &fstr(x1 + self.dx.unwrap_or(0.)));
                } else if let Some(dx) = self.dx {
                    if let Ok(x1) = strp(element.get_attr("x1").unwrap_or(&zstr)) {
                        element.set_attr("x1", &fstr(x1 + dx));
                    }
                }
                if element.get_attr("y1").is_none() {
                    element.set_attr("y1", &fstr(y1 + self.dy.unwrap_or(0.)));
                } else if let Some(dy) = self.dy {
                    if let Ok(y1) = strp(element.get_attr("y1").unwrap_or(&zstr)) {
                        element.set_attr("y1", &fstr(y1 + dy));
                    }
                }
                let (x2, y2) = bbox.xy2();
                if element.get_attr("x2").is_none() {
                    element.set_attr("x2", &fstr(x2 + self.dx.unwrap_or(0.)));
                } else if let Some(dx) = self.dx {
                    if let Ok(x2) = strp(element.get_attr("x2").unwrap_or(&zstr)) {
                        element.set_attr("x2", &fstr(x2 + dx));
                    }
                }
                if element.get_attr("y2").is_none() {
                    element.set_attr("y2", &fstr(y2 + self.dy.unwrap_or(0.)));
                } else if let Some(dy) = self.dy {
                    if let Ok(y2) = strp(element.get_attr("y2").unwrap_or(&zstr)) {
                        element.set_attr("y2", &fstr(y2 + dy));
                    }
                }
                element.remove_attrs(&[
                    "dx", "dy", "dw", "dh", "x", "y", "cx", "cy", "rx", "ry", "r", "width",
                    "height",
                ]);
            }
            _ => (),
        }
    }

    fn position_via_transform(&self, element: &mut SvgElement) {
        let (mut x, mut y) = (self.x(), self.y());
        if let Some(dx) = self.dx {
            x += dx;
        }
        if let Some(dy) = self.dy {
            y += dy;
        }
        if x != 0. || y != 0. {
            let mut xy_xfrm = format!("translate({x}, {y})");
            if let Some(exist_xfrm) = element.get_attr("transform") {
                xy_xfrm = format!("{} {}", exist_xfrm, xy_xfrm);
            }
            element.set_attr("transform", &xy_xfrm);
            element.remove_attrs(&[
                "dx", "dy", "dw", "dh", "x", "y", "x1", "y1", "x2", "y2", "cx", "cy", "rx", "ry",
                "r", "width", "height",
            ]);
        }
    }
}

impl From<&SvgElement> for Position {
    /// assumes SvgElement has already had compound attributes split
    fn from(value: &SvgElement) -> Self {
        let mut p = Position::new(&value.name);

        p.dx = value.get_attr("dx").and_then(|dx| strp(dx).ok());
        p.dy = value.get_attr("dy").and_then(|dy| strp(dy).ok());

        let x = value.get_attr("x1").or(value.get_attr("x"));
        let y = value.get_attr("y1").or(value.get_attr("y"));
        if let Some(Ok(x)) = x.map(strp) {
            p.xmin = Some(x);
        }
        if let Some(Ok(y)) = y.map(strp) {
            p.ymin = Some(y);
        }

        let x2 = value.get_attr("x2");
        let y2 = value.get_attr("y2");
        if let Some(Ok(x2)) = x2.map(strp) {
            p.xmax = Some(x2);
        }
        if let Some(Ok(y2)) = y2.map(strp) {
            p.ymax = Some(y2);
        }

        let cx = value.get_attr("cx");
        let cy = value.get_attr("cy");
        if let Some(Ok(cx)) = cx.map(strp) {
            p.cx = Some(cx);
        }
        if let Some(Ok(cy)) = cy.map(strp) {
            p.cy = Some(cy);
        }

        // In theory `<use>` elements can have width/height attrs, but only if
        // they target an `<svg>`/`<symbol>` element with a `viewPort` attr,
        // where it is overwritten. We don't support that, and instead allow
        // width/height to be used as context variables.
        // See https://www.w3.org/TR/SVG2/struct.html#UseElement
        if !matches!(p.shape.as_str(), "reuse" | "use") {
            let w = value.get_attr("width");
            let h = value.get_attr("height");
            if let Some(Ok(w)) = w.map(strp) {
                p.width = Some(w);
            }
            if let Some(Ok(h)) = h.map(strp) {
                p.height = Some(h);
            }
        }

        // if circle / ellipse, get width / height from r / rx / ry
        // These attributes are not symmetric; while circles/ellipses in svgdx
        // can be defined by x/y/width/height etc, non-circle/ellipse elements
        // cannot use r/rx/ry. This is due to rx/ry having different meaning in
        // the context of rect elements.
        if let "circle" | "ellipse" = value.name.as_str() {
            let rx = value.get_attr("rx").or(value.get_attr("r"));
            let ry = value.get_attr("ry").or(value.get_attr("r"));
            if let Some(Ok(r)) = rx.map(strp) {
                p.width = Some(r * 2.);
            }
            if let Some(Ok(r)) = ry.map(strp) {
                p.height = Some(r * 2.);
            }
        }

        p
    }
}

impl std::fmt::Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("Position");
        if let Some(xmin) = self.xmin {
            f.field("xmin", &xmin);
        }
        if let Some(ymin) = self.ymin {
            f.field("ymin", &ymin);
        }
        if let Some(xmax) = self.xmax {
            f.field("xmax", &xmax);
        }
        if let Some(ymax) = self.ymax {
            f.field("ymax", &ymax);
        }
        if let Some(cx) = self.cx {
            f.field("cx", &cx);
        }
        if let Some(cy) = self.cy {
            f.field("cy", &cy);
        }
        if let Some(width) = self.width {
            f.field("width", &width);
        }
        if let Some(height) = self.height {
            f.field("height", &height);
        }
        if let Some(dx) = self.dx {
            f.field("dx", &dx);
        }
        if let Some(dy) = self.dy {
            f.field("dy", &dy);
        }
        f.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x1_y1_w_h() {
        let pos = Position {
            xmin: Some(15.0),
            width: Some(5.0),
            ymin: Some(15.0),
            height: Some(5.0),
            ..Default::default()
        };

        let bbox = pos.to_bbox().unwrap();
        assert_eq!(bbox.x1, 15.0);
        assert_eq!(bbox.x2, 20.0);
        assert_eq!(bbox.y1, 15.0);
        assert_eq!(bbox.y2, 20.0);
    }

    #[test]
    fn test_x1_y1_x2_y2() {
        let pos = Position {
            xmin: Some(15.0),
            xmax: Some(20.0),
            ymin: Some(15.0),
            ymax: Some(20.0),
            ..Default::default()
        };

        let bbox = pos.to_bbox().unwrap();
        assert_eq!(bbox.x1, 15.0);
        assert_eq!(bbox.x2, 20.0);
        assert_eq!(bbox.y1, 15.0);
        assert_eq!(bbox.y2, 20.0);
    }

    #[test]
    fn test_x2_y2_w_h() {
        let pos = Position {
            xmax: Some(20.0),
            width: Some(5.0),
            ymax: Some(20.0),
            height: Some(5.0),
            ..Default::default()
        };

        let bbox = pos.to_bbox().unwrap();
        assert_eq!(bbox.x1, 15.0);
        assert_eq!(bbox.x2, 20.0);
        assert_eq!(bbox.y1, 15.0);
        assert_eq!(bbox.y2, 20.0);
    }

    #[test]
    fn test_x1_y1_cx_cy() {
        let pos = Position {
            xmin: Some(15.0),
            ymin: Some(15.0),
            cx: Some(17.5),
            cy: Some(17.5),
            ..Default::default()
        };

        let bbox = pos.to_bbox().unwrap();
        assert_eq!(bbox.x1, 15.0);
        assert_eq!(bbox.x2, 20.0);
        assert_eq!(bbox.y1, 15.0);
        assert_eq!(bbox.y2, 20.0);
    }

    #[test]
    fn test_x2_y2_cx_cy() {
        let pos = Position {
            xmax: Some(20.0),
            ymax: Some(20.0),
            cx: Some(17.5),
            cy: Some(17.5),
            ..Default::default()
        };

        let bbox = pos.to_bbox().unwrap();
        assert_eq!(bbox.x1, 15.0);
        assert_eq!(bbox.x2, 20.0);
        assert_eq!(bbox.y1, 15.0);
        assert_eq!(bbox.y2, 20.0);
    }

    #[test]
    fn test_cx_cy_w_h() {
        let pos = Position {
            cx: Some(17.5),
            cy: Some(17.5),
            width: Some(5.0),
            height: Some(5.0),
            ..Default::default()
        };

        let bbox = pos.to_bbox().unwrap();
        assert_eq!(bbox.x1, 15.0);
        assert_eq!(bbox.x2, 20.0);
        assert_eq!(bbox.y1, 15.0);
        assert_eq!(bbox.y2, 20.0);
    }
}
