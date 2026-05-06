use crate::elements::SvgElement;
use crate::errors::Error;
use crate::geometry::{BoundingBox, Size};

#[derive(Clone, Default)]
pub struct Position {
    pub x_start: Option<f32>,
    pub y_start: Option<f32>,
    pub x_end: Option<f32>,
    pub y_end: Option<f32>,
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
        // TODO: check for overconstrained / invalid combinations
        // e.g. !(start <= middle <= end), length < 0, start + length != end, etc.
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
        self.extent(self.x_start, self.x_end, self.cx, self.width)
    }

    fn y_def(&self) -> Option<(f32, f32)> {
        self.extent(self.y_start, self.y_end, self.cy, self.height)
    }

    pub fn to_bbox(&self) -> Option<BoundingBox> {
        match (
            self.shape.as_str(),
            self.x_def(),
            self.y_def(),
            self.width,
            self.height,
        ) {
            // Fully specified bbox already.
            (_, Some((x1, x2)), Some((y1, y2)), _, _) => Some(BoundingBox::new(x1, y1, x2, y2)),
            // For points, we don't need extent at all, just at least one x and at least one y.
            ("point", _, _, _, _) => {
                let px = self.x_start.or(self.x_end.or(self.cx));
                let py = self.y_start.or(self.y_end.or(self.cy));
                px.zip(py).map(|(x, y)| BoundingBox::new(x, y, x, y))
            }
            // For circles, width and height are the same, so we only need one plus a single
            // other value to define the circle. The same logic would apply for squares,
            // but that's not an SVG primitive.
            ("circle", Some((x1, x2)), _, _, _) => {
                if let Some((y1, y2)) = self.three_point(x2 - x1, self.y_start, self.cy, self.y_end)
                {
                    Some(BoundingBox::new(x1, y1, x2, y2))
                } else {
                    let radius = (x2 - x1) / 2.;
                    Some(BoundingBox::new(x1, -radius, x2, radius))
                }
            }
            ("circle", _, Some((y1, y2)), _, _) => {
                if let Some((x1, x2)) = self.three_point(y2 - y1, self.x_start, self.cx, self.x_end)
                {
                    Some(BoundingBox::new(x1, y1, x2, y2))
                } else {
                    let radius = (y2 - y1) / 2.;
                    Some(BoundingBox::new(-radius, y1, radius, y2))
                }
            }
            ("circle", _, _, Some(diameter), _) | ("circle", _, _, _, Some(diameter)) => {
                // if cx/cy (etc) are absent, SVG says they are treated as zero.
                let r = diameter / 2.;
                Some(BoundingBox::new(-r, -r, r, r))
            }
            (_, x_ext, y_ext, Some(w), Some(h)) => {
                let (x1, x2) = x_ext.unwrap_or((0., w));
                let (y1, y2) = y_ext.unwrap_or((0., h));
                Some(BoundingBox::new(x1, y1, x2, y2))
            }
            _ => None,
        }
    }

    fn has_x_position(&self) -> bool {
        self.x_start.is_some() || self.x_end.is_some() || self.cx.is_some() || self.dx.is_some()
    }

    fn has_y_position(&self) -> bool {
        self.y_start.is_some() || self.y_end.is_some() || self.cy.is_some() || self.dy.is_some()
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
            self.x_start.unwrap_or(0.)
        }
    }

    /// Get most-reasonable 'y' value for this element,
    /// defaulting to 0 if required. Excludes dy.
    pub fn y(&self) -> f32 {
        if let Some((y1, _)) = self.y_def() {
            y1
        } else {
            self.y_start.unwrap_or(0.)
        }
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.x_start = self.x_start.map(|x| x + dx);
        self.x_end = self.x_end.map(|x| x + dx);
        self.cx = self.cx.map(|x| x + dx);
        self.y_start = self.y_start.map(|y| y + dy);
        self.y_end = self.y_end.map(|y| y + dy);
        self.cy = self.cy.map(|y| y + dy);
    }

    pub fn set_position_attrs(&self, element: &mut SvgElement) {
        // TODO: should this return a Result?
        match (element.name(), self.to_bbox()) {
            ("g" | "path" | "polyline" | "polygon", _) => {
                // These don't need a full bbox, and are always set via transform in any case.
                self.position_via_transform(element);
            }
            ("use" | "point" | "text", _) => {
                // These only have x & y attrs.
                // TODO: should `length` be set to Some(0.) for these shape types
                // so extent() works and e.g. a 'cxy="5"` works on a `<point>`?
                if self.has_x_position() {
                    element.set_num_attr("x", self.x() + self.dx.unwrap_or(0.));
                }
                if self.has_y_position() {
                    element.set_num_attr("y", self.y() + self.dy.unwrap_or(0.));
                }
                element.remove_attrs(&["dw", "dh", "x1", "y1", "x2", "y2", "cx", "cy", "r"]);
                if element.name() != "use" {
                    element.remove_attrs(&["width", "height"]);
                }
                if element.name() != "text" {
                    element.remove_attrs(&["dx", "dy"]);
                }
            }
            ("" | "rect" | "box" | "image" | "svg" | "foreignObject", Some(bbox)) => {
                let width = bbox.width();
                let height = bbox.height();
                let (x1, y1) = bbox.xy1();
                if self.has_x_position() {
                    element.set_num_attr("x", x1 + self.dx.unwrap_or(0.));
                }
                if self.has_y_position() {
                    element.set_num_attr("y", y1 + self.dy.unwrap_or(0.));
                }
                element.set_num_attr("width", width);
                element.set_num_attr("height", height);
                element.remove_attrs(&[
                    "dx", "dy", "dw", "dh", "x1", "y1", "x2", "y2", "cx", "cy", "r",
                ]);
            }
            ("circle", Some(bbox)) => {
                let (cx, cy) = bbox.center();
                let r = bbox.width() / 2.0;
                if self.has_x_position() {
                    element.set_num_attr("cx", cx + self.dx.unwrap_or(0.));
                }
                if self.has_y_position() {
                    element.set_num_attr("cy", cy + self.dy.unwrap_or(0.));
                }
                element.set_num_attr("r", r);
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
                    element.set_num_attr("cx", cx + self.dx.unwrap_or(0.));
                }
                if self.has_y_position() {
                    element.set_num_attr("cy", cy + self.dy.unwrap_or(0.));
                }
                element.set_num_attr("rx", rx);
                element.set_num_attr("ry", ry);
                element.remove_attrs(&[
                    "dx", "dy", "dw", "dh", "x", "y", "x1", "y1", "x2", "y2", "r", "width",
                    "height",
                ]);
            }
            ("line", Some(bbox)) => {
                // NOTE: lines are directional, so we don't want to set x1/y1 from the bbox
                // (which is directionless). Use the signed x_def() / y_def() instead.
                let (x1, x2) = self.x_def().unwrap_or((0., bbox.width()));
                let (y1, y2) = self.y_def().unwrap_or((0., bbox.height()));
                element.set_num_attr("x1", x1 + self.dx.unwrap_or(0.));
                element.set_num_attr("x2", x2 + self.dx.unwrap_or(0.));
                element.set_num_attr("y1", y1 + self.dy.unwrap_or(0.));
                element.set_num_attr("y2", y2 + self.dy.unwrap_or(0.));

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
                xy_xfrm = format!("{exist_xfrm} {xy_xfrm}");
            }
            element.set_attr("transform", &xy_xfrm);
        }
        // assumes element is g/path/polyline/polygon etc where these attrs
        // have no meaning.
        // NOTE: don't try and use variables with these names!
        element.remove_attrs(&[
            "dx", "dy", "dw", "dh", "x", "y", "x1", "y1", "x2", "y2", "cx", "cy", "rx", "ry", "r",
            "width", "height",
        ]);
    }
}

impl TryFrom<&SvgElement> for Position {
    type Error = Error;

    /// assumes SvgElement has already had compound attributes split
    fn try_from(value: &SvgElement) -> Result<Self, Self::Error> {
        let mut p = Position::new(value.name());

        // some elements have a special meaning for dx/dx; we don't
        // do anything in that case.
        if !matches!(value.name(), "text" | "tspan" | "feOffset") {
            if let Some(dx) = value.get_num_attr("dx")? {
                p.dx = Some(dx);
            }
            if let Some(dy) = value.get_num_attr("dy")? {
                p.dy = Some(dy);
            }
        }

        if let Some(x) = value.get_num_attr("x1")?.or(value.get_num_attr("x")?) {
            p.x_start = Some(x);
        }
        if let Some(y) = value.get_num_attr("y1")?.or(value.get_num_attr("y")?) {
            p.y_start = Some(y);
        }

        if let Some(x2) = value.get_num_attr("x2")? {
            p.x_end = Some(x2);
        }
        if let Some(y2) = value.get_num_attr("y2")? {
            p.y_end = Some(y2);
        }

        if let Some(cx) = value.get_num_attr("cx")? {
            p.cx = Some(cx);
        }
        if let Some(cy) = value.get_num_attr("cy")? {
            p.cy = Some(cy);
        }

        // In theory `<use>` elements can have width/height attrs, but only if
        // they target an `<svg>`/`<symbol>` element with a `viewPort` attr,
        // where it is overwritten. We don't support that, and instead allow
        // width/height to be used as context variables.
        // See https://www.w3.org/TR/SVG2/struct.html#UseElement
        if !matches!(p.shape.as_str(), "reuse" | "use") {
            if let Some(w) = value.get_num_attr("width")? {
                p.width = Some(w);
            }
            if let Some(h) = value.get_num_attr("height")? {
                p.height = Some(h);
            }
        }

        // if circle / ellipse, get width / height from r / rx / ry
        // These attributes are not symmetric; while circles/ellipses in svgdx
        // can be defined by x/y/width/height etc, non-circle/ellipse elements
        // cannot use r/rx/ry. This is due to rx/ry having different meaning in
        // the context of rect elements.
        if let "circle" | "ellipse" = value.name() {
            if let Some(rx) = value.get_num_attr("rx")?.or(value.get_num_attr("r")?) {
                p.width = Some(rx * 2.);
            }
            if let Some(ry) = value.get_num_attr("ry")?.or(value.get_num_attr("r")?) {
                p.height = Some(ry * 2.);
            }
        }

        Ok(p)
    }
}

impl std::fmt::Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("Position");
        if let Some(xmin) = self.x_start {
            f.field("xmin", &xmin);
        }
        if let Some(ymin) = self.y_start {
            f.field("ymin", &ymin);
        }
        if let Some(xmax) = self.x_end {
            f.field("xmax", &xmax);
        }
        if let Some(ymax) = self.y_end {
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
            x_start: Some(15.0),
            width: Some(5.0),
            y_start: Some(15.0),
            height: Some(5.0),
            ..Default::default()
        };

        let bbox = pos.to_bbox().unwrap();
        assert_eq!(bbox.x1(), 15.0);
        assert_eq!(bbox.x2(), 20.0);
        assert_eq!(bbox.y1(), 15.0);
        assert_eq!(bbox.y2(), 20.0);
    }

    #[test]
    fn test_x1_y1_x2_y2() {
        let pos = Position {
            x_start: Some(15.0),
            x_end: Some(20.0),
            y_start: Some(15.0),
            y_end: Some(20.0),
            ..Default::default()
        };

        let bbox = pos.to_bbox().unwrap();
        assert_eq!(bbox.x1(), 15.0);
        assert_eq!(bbox.x2(), 20.0);
        assert_eq!(bbox.y1(), 15.0);
        assert_eq!(bbox.y2(), 20.0);
    }

    #[test]
    fn test_x2_y2_w_h() {
        let pos = Position {
            x_end: Some(20.0),
            width: Some(5.0),
            y_end: Some(20.0),
            height: Some(5.0),
            ..Default::default()
        };

        let bbox = pos.to_bbox().unwrap();
        assert_eq!(bbox.x1(), 15.0);
        assert_eq!(bbox.x2(), 20.0);
        assert_eq!(bbox.y1(), 15.0);
        assert_eq!(bbox.y2(), 20.0);
    }

    #[test]
    fn test_x1_y1_cx_cy() {
        let pos = Position {
            x_start: Some(15.0),
            y_start: Some(15.0),
            cx: Some(17.5),
            cy: Some(17.5),
            ..Default::default()
        };

        let bbox = pos.to_bbox().unwrap();
        assert_eq!(bbox.x1(), 15.0);
        assert_eq!(bbox.x2(), 20.0);
        assert_eq!(bbox.y1(), 15.0);
        assert_eq!(bbox.y2(), 20.0);
    }

    #[test]
    fn test_x2_y2_cx_cy() {
        let pos = Position {
            x_end: Some(20.0),
            y_end: Some(20.0),
            cx: Some(17.5),
            cy: Some(17.5),
            ..Default::default()
        };

        let bbox = pos.to_bbox().unwrap();
        assert_eq!(bbox.x1(), 15.0);
        assert_eq!(bbox.x2(), 20.0);
        assert_eq!(bbox.y1(), 15.0);
        assert_eq!(bbox.y2(), 20.0);
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
        assert_eq!(bbox.x1(), 15.0);
        assert_eq!(bbox.x2(), 20.0);
        assert_eq!(bbox.y1(), 15.0);
        assert_eq!(bbox.y2(), 20.0);
    }
}
