use std::str::FromStr;

use crate::element::SvgElement;
use crate::errors::{Result, SvgdxError};
use crate::types::{attr_split, fstr, strp};

#[derive(Clone, Debug, Default)]
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
            // The following case allows lines to be specified by cx/cy in one dimension
            (None, None, Some(m), None) => {
                if self.shape == "line" {
                    Some((m, m))
                } else {
                    None
                }
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

    pub fn set_position_attrs(&self, element: &mut SvgElement) {
        if let Some(bbox) = self.to_bbox() {
            match element.name.as_str() {
                "" | "rect" | "use" | "image" | "svg" | "foreignObject" => {
                    let width = bbox.width();
                    let height = bbox.height();
                    let (x1, y1) = bbox.locspec(LocSpec::TopLeft);
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
                "circle" => {
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
                        "dx", "dy", "dw", "dh", "x", "y", "x1", "y1", "x2", "y2", "rx", "ry",
                        "width", "height",
                    ]);
                }
                "ellipse" => {
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
                "line" => {
                    // NOTE: lines are directional, so we don't want to set x1/y1 from the bbox
                    // if they're already set, but we do need to add dx/dy to any existing attrs.
                    let zstr = "0".to_owned();
                    let (x1, y1) = bbox.locspec(LocSpec::TopLeft);
                    if element.get_attr("x1").is_none() {
                        element.set_attr("x1", &fstr(x1 + self.dx.unwrap_or(0.)));
                    } else if let Some(dx) = self.dx {
                        if let Ok(x1) = strp(&element.get_attr("x1").unwrap_or(zstr.clone())) {
                            element.set_attr("x1", &fstr(x1 + dx));
                        }
                    }
                    if element.get_attr("y1").is_none() {
                        element.set_attr("y1", &fstr(y1 + self.dy.unwrap_or(0.)));
                    } else if let Some(dy) = self.dy {
                        if let Ok(y1) = strp(&element.get_attr("y1").unwrap_or(zstr.clone())) {
                            element.set_attr("y1", &fstr(y1 + dy));
                        }
                    }
                    let (x2, y2) = bbox.locspec(LocSpec::BottomRight);
                    if element.get_attr("x2").is_none() {
                        element.set_attr("x2", &fstr(x2 + self.dx.unwrap_or(0.)));
                    } else if let Some(dx) = self.dx {
                        if let Ok(x2) = strp(&element.get_attr("x2").unwrap_or(zstr.clone())) {
                            element.set_attr("x2", &fstr(x2 + dx));
                        }
                    }
                    if element.get_attr("y2").is_none() {
                        element.set_attr("y2", &fstr(y2 + self.dy.unwrap_or(0.)));
                    } else if let Some(dy) = self.dy {
                        if let Ok(y2) = strp(&element.get_attr("y2").unwrap_or(zstr.clone())) {
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
    }
}

impl SvgElement {
    pub fn remove_attrs(&mut self, keys: &[&str]) {
        for key in keys {
            self.pop_attr(key);
        }
    }
}

impl From<&SvgElement> for Position {
    fn from(value: &SvgElement) -> Self {
        let mut p = Position::new(&value.name);

        p.dx = value.get_attr("dx").and_then(|dx| strp(dx.as_ref()).ok());
        p.dy = value.get_attr("dy").and_then(|dy| strp(dy.as_ref()).ok());

        let x = value.get_attr("x1").or(value.get_attr("x"));
        let y = value.get_attr("y1").or(value.get_attr("y"));
        if let Some(Ok(x)) = x.map(|x| strp(x.as_ref())) {
            p.xmin = Some(x);
        }
        if let Some(Ok(y)) = y.map(|y| strp(y.as_ref())) {
            p.ymin = Some(y);
        }

        let x2 = value.get_attr("x2");
        let y2 = value.get_attr("y2");
        if let Some(Ok(x2)) = x2.map(|x2| strp(x2.as_ref())) {
            p.xmax = Some(x2);
        }
        if let Some(Ok(y2)) = y2.map(|y2| strp(y2.as_ref())) {
            p.ymax = Some(y2);
        }

        let cx = value.get_attr("cx");
        let cy = value.get_attr("cy");
        if let Some(Ok(cx)) = cx.map(|cx| strp(cx.as_ref())) {
            p.cx = Some(cx);
        }
        if let Some(Ok(cy)) = cy.map(|cy| strp(cy.as_ref())) {
            p.cy = Some(cy);
        }

        let w = value.get_attr("width");
        let h = value.get_attr("height");
        if let Some(Ok(w)) = w.map(|w| strp(w.as_ref())) {
            p.width = Some(w);
        }
        if let Some(Ok(h)) = h.map(|h| strp(h.as_ref())) {
            p.height = Some(h);
        }

        // if circle / ellipse, get width / height from r / rx / ry
        // These attributes are not symmetric; while circles/ellipses in svgdx
        // can be defined by x/y/width/height etc, non-circle/ellipse elements
        // cannot use r/rx/ry. This is due to rx/ry having different meaning in
        // the context of rect elements.
        if let "circle" | "ellipse" = value.name.as_str() {
            let rx = value.get_attr("rx").or(value.get_attr("r"));
            let ry = value.get_attr("ry").or(value.get_attr("r"));
            if let Some(Ok(r)) = rx.map(|r| strp(r.as_ref())) {
                p.width = Some(r * 2.);
            }
            if let Some(Ok(r)) = ry.map(|r| strp(r.as_ref())) {
                p.height = Some(r * 2.);
            }
        }

        p
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Length {
    Absolute(f32),
    Ratio(f32),
}

impl Default for Length {
    fn default() -> Self {
        Self::Absolute(0.)
    }
}

impl Length {
    #[allow(dead_code)]
    pub const fn ratio(&self) -> Option<f32> {
        if let Self::Ratio(result) = self {
            Some(*result)
        } else {
            None
        }
    }

    pub const fn absolute(&self) -> Option<f32> {
        if let Self::Absolute(result) = self {
            Some(*result)
        } else {
            None
        }
    }

    /// Convert a `Length` to a value, taking a base value as input
    /// in case a ratio length is used.
    pub fn evaluate(&self, base: f32) -> f32 {
        match self {
            Self::Absolute(abs) => *abs,
            Self::Ratio(ratio) => base * ratio,
        }
    }

    /// Given a single value, update it (scale or addition) from
    /// the current Length value
    pub fn adjust(&self, value: f32) -> f32 {
        match self {
            Self::Absolute(abs) => value + abs,
            Self::Ratio(ratio) => value * ratio,
        }
    }

    /// Given a range, return a value (typically) in the range
    /// where a positive Absolute is 'from start', a negative Absolute
    /// is 'backwards from end' and Ratios scale as 0%=start, 100%=end
    /// but ratio values are not limited to 0..100 at either end.
    pub fn calc_offset(&self, start: f32, end: f32) -> f32 {
        match self {
            Self::Absolute(abs) => {
                let mult = if end < start { -1. } else { 1. };
                if abs < &0. {
                    // '+' here since abs is negative and
                    // we're going 'back' from the end.
                    end + abs * mult
                } else {
                    start + abs * mult
                }
            }
            Self::Ratio(ratio) => start + (end - start) * ratio,
        }
    }
}

pub fn strp_length(s: &str) -> Result<Length> {
    s.parse::<Length>()
}

impl FromStr for Length {
    type Err = SvgdxError;

    /// Parse a ratio (float or %age) to an f32
    /// Note this deliberately does not clamp to 0..1
    fn from_str(value: &str) -> Result<Self> {
        let value = value.trim();
        if let Some(pc) = value.strip_suffix('%') {
            Ok(Length::Ratio(strp(pc)? * 0.01))
        } else {
            Ok(Length::Absolute(strp(value)?))
        }
    }
}

/// `DirSpec` defines a location relative to an element's `BoundingBox`
#[derive(Clone, Copy, Debug)]
pub enum DirSpec {
    InFront,
    Behind,
    Below,
    Above,
}

impl FromStr for DirSpec {
    type Err = SvgdxError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "h" => Ok(Self::InFront),
            "H" => Ok(Self::Behind),
            "v" => Ok(Self::Below),
            "V" => Ok(Self::Above),
            _ => Err(SvgdxError::InvalidData(format!(
                "Invalid DirSpec format {value}"
            ))),
        }
    }
}

impl DirSpec {
    pub fn to_locspec(self) -> LocSpec {
        match self {
            Self::InFront => LocSpec::Right,
            Self::Behind => LocSpec::Left,
            Self::Below => LocSpec::Bottom,
            Self::Above => LocSpec::Top,
        }
    }
}

/// `EdgeSpec` defines an offset along one edge of a `BoundingBox`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EdgeSpec {
    Top(Length),
    Right(Length),
    Bottom(Length),
    Left(Length),
}

impl FromStr for EdgeSpec {
    type Err = SvgdxError;

    fn from_str(value: &str) -> Result<Self> {
        if let Some((edge, len)) = value.split_once(':') {
            let len = len.parse::<Length>()?;
            match edge {
                "t" => Ok(Self::Top(len)),
                "r" => Ok(Self::Right(len)),
                "b" => Ok(Self::Bottom(len)),
                "l" => Ok(Self::Left(len)),
                _ => Err(SvgdxError::InvalidData(format!(
                    "Invalid EdgeSpec format {value}"
                ))),
            }
        } else {
            Err(SvgdxError::InvalidData(format!(
                "Invalid EdgeSpec format {value}"
            )))
        }
    }
}

impl EdgeSpec {
    pub fn as_loc(&self) -> LocSpec {
        match self {
            Self::Top(_) => LocSpec::Top,
            Self::Right(_) => LocSpec::Right,
            Self::Bottom(_) => LocSpec::Bottom,
            Self::Left(_) => LocSpec::Left,
        }
    }
}

/// `LocSpec` defines a location on a `BoundingBox`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocSpec {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    Center,
}

impl FromStr for LocSpec {
    type Err = SvgdxError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "tl" => Ok(Self::TopLeft),
            "t" => Ok(Self::Top),
            "tr" => Ok(Self::TopRight),
            "r" => Ok(Self::Right),
            "br" => Ok(Self::BottomRight),
            "b" => Ok(Self::Bottom),
            "bl" => Ok(Self::BottomLeft),
            "l" => Ok(Self::Left),
            "c" => Ok(Self::Center),
            _ => Err(SvgdxError::InvalidData(format!(
                "Invalid LocSpec format {value}"
            ))),
        }
    }
}

impl From<ScalarSpec> for LocSpec {
    fn from(value: ScalarSpec) -> Self {
        match value {
            ScalarSpec::Minx => Self::Left,
            ScalarSpec::Maxx => Self::Right,
            ScalarSpec::Cx => Self::Center,
            ScalarSpec::Miny => Self::Top,
            ScalarSpec::Maxy => Self::Bottom,
            ScalarSpec::Cy => Self::Center,
            ScalarSpec::Width => Self::Right,
            ScalarSpec::Rx => Self::Right,
            ScalarSpec::Height => Self::Bottom,
            ScalarSpec::Ry => Self::Bottom,
        }
    }
}

/// `ScalarSpec` defines a single value from a `BoundingBox`
///
/// These are the min and max x & y values, together with width and height.
#[derive(Clone, Copy)]
pub enum ScalarSpec {
    Minx,
    Maxx,
    Cx,
    Miny,
    Maxy,
    Cy,
    Width,
    Rx,
    Height,
    Ry,
}

impl FromStr for ScalarSpec {
    type Err = SvgdxError;

    fn from_str(value: &str) -> Result<Self> {
        // TODO: 'r' here is ambiguous vs circle's radius attribute.
        // Perhaps require uppercase 'T/R/B/L' for edge values.
        // TODO: consider x1/x2/y1/y2: note that for e.g. a line it is
        // *not* required that the *attributes* x1 < x2 or y1 < y2.
        // Perhaps a separate 'attribute spec' concept is needed...
        match value {
            "x" | "x1" | "l" => Ok(Self::Minx),
            "y" | "y1" | "t" => Ok(Self::Miny),
            "cx" => Ok(Self::Cx),
            "x2" | "r" => Ok(Self::Maxx),
            "y2" | "b" => Ok(Self::Maxy),
            "cy" => Ok(Self::Cy),
            "w" | "width" => Ok(Self::Width),
            "rx" => Ok(Self::Rx),
            "h" | "height" => Ok(Self::Height),
            "ry" => Ok(Self::Ry),
            _ => Err(SvgdxError::InvalidData(format!(
                "Invalid ScalarSpec format {value}"
            ))),
        }
    }
}

/// `BoundingBox` defines an axis-aligned rectangular region is user coordinates.
///
/// Many (not all) `SvgElement` instances will have a corresponding
/// `BoundingBox`, indicating the position and size of the rendered
/// element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Debug, Clone, Default)]
pub struct BoundingBoxBuilder {
    // TODO: NewType?
    bbox: Option<BoundingBox>,
}

impl BoundingBoxBuilder {
    pub fn new() -> Self {
        Self { bbox: None }
    }

    pub fn extend(&mut self, bbox: BoundingBox) -> &Self {
        if let Some(ref mut b) = self.bbox {
            *b = b.combine(&bbox);
        } else {
            self.bbox = Some(bbox);
        }
        self
    }

    pub fn build(self) -> Option<BoundingBox> {
        self.bbox
    }
}

impl BoundingBox {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn locspec(&self, ls: LocSpec) -> (f32, f32) {
        let tl = (self.x1, self.y1);
        let tr = (self.x2, self.y1);
        let br = (self.x2, self.y2);
        let bl = (self.x1, self.y2);
        let c = ((self.x1 + self.x2) / 2., (self.y1 + self.y2) / 2.);
        match ls {
            LocSpec::TopLeft => tl,
            LocSpec::Top => ((self.x1 + self.x2) / 2., self.y1),
            LocSpec::TopRight => tr,
            LocSpec::Right => (self.x2, (self.y1 + self.y2) / 2.),
            LocSpec::BottomRight => br,
            LocSpec::Bottom => ((self.x1 + self.x2) / 2., self.y2),
            LocSpec::BottomLeft => bl,
            LocSpec::Left => (self.x1, (self.y1 + self.y2) / 2.),
            LocSpec::Center => c,
        }
    }

    pub fn scalarspec(&self, ss: ScalarSpec) -> f32 {
        match ss {
            ScalarSpec::Minx => self.x1,
            ScalarSpec::Maxx => self.x2,
            ScalarSpec::Miny => self.y1,
            ScalarSpec::Maxy => self.y2,
            ScalarSpec::Width => (self.x2 - self.x1).abs(),
            ScalarSpec::Height => (self.y2 - self.y1).abs(),
            ScalarSpec::Cx => (self.x1 + self.x2) / 2.,
            ScalarSpec::Cy => (self.y1 + self.y2) / 2.,
            ScalarSpec::Rx => (self.x2 - self.x1).abs() / 2.,
            ScalarSpec::Ry => (self.y2 - self.y1).abs() / 2.,
        }
    }

    pub fn edgespec(&self, es: EdgeSpec) -> (f32, f32) {
        match es {
            EdgeSpec::Top(len) => (len.calc_offset(self.x1, self.x2), self.y1),
            EdgeSpec::Right(len) => (self.x2, len.calc_offset(self.y1, self.y2)),
            EdgeSpec::Bottom(len) => (len.calc_offset(self.x1, self.x2), self.y2),
            EdgeSpec::Left(len) => (self.x1, len.calc_offset(self.y1, self.y2)),
        }
    }

    /// Get point from a string such as 'tl' (top-left of this bbox) or
    /// 'r:30%' (30% down the right edge).
    pub fn get_point(&self, s: &str) -> Result<(f32, f32)> {
        if let Ok(es) = s.parse::<EdgeSpec>() {
            Ok(self.edgespec(es))
        } else if let Ok(ls) = s.parse::<LocSpec>() {
            Ok(self.locspec(ls))
        } else {
            Err(SvgdxError::InvalidData(format!("Invalid point spec {s}")))
        }
    }

    pub fn union(bb_iter: impl IntoIterator<Item = Self>) -> Option<Self> {
        let bb_iter = bb_iter.into_iter();
        bb_iter.reduce(|bb1, bb2| bb1.combine(&bb2))
    }

    pub fn combine(&self, other: &Self) -> Self {
        Self::new(
            self.x1.min(other.x1),
            self.y1.min(other.y1),
            self.x2.max(other.x2),
            self.y2.max(other.y2),
        )
    }

    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let result = Self::new(
            self.x1.max(other.x1),
            self.y1.max(other.y1),
            self.x2.min(other.x2),
            self.y2.min(other.y2),
        );
        if result.width() >= 0. && result.height() >= 0. {
            Some(result)
        } else {
            None
        }
    }

    pub fn intersection(bb_iter: impl IntoIterator<Item = Self>) -> Option<Self> {
        // Ideally want to use `reduce()` here, but want to exit early on None,
        // so do it long-hand.
        let mut bb_iter = bb_iter.into_iter();
        let mut bb = bb_iter.next();
        while bb.is_some() {
            if let Some(other) = bb_iter.next() {
                bb = bb?.intersect(&other);
            } else {
                break;
            }
        }
        bb
    }

    /// dilate the bounding box by the given absolute amount in each direction
    pub fn expand(&mut self, exp_x: f32, exp_y: f32) -> &Self {
        *self = Self {
            x1: self.x1 - exp_x,
            y1: self.y1 - exp_y,
            x2: self.x2 + exp_x,
            y2: self.y2 + exp_y,
        };
        self
    }

    pub fn expand_trbl_length(&mut self, trbl: TrblLength) -> &Self {
        // NOTE: not clear if x values should use width and y values use
        // height, or if having consistent values (as here) is better.
        // Current approach ensures a single-valued `TrblLength`` input
        // has a consistent border on all sides, which is probably the
        // expectation, and matches CSS (where all %ages are in terms
        // of inline-size - typically width - of parent element).
        let base = self.width().max(self.height());
        *self = Self {
            x1: self.x1 - trbl.left.evaluate(base),
            y1: self.y1 - trbl.top.evaluate(base),
            x2: self.x2 + trbl.right.evaluate(base),
            y2: self.y2 + trbl.bottom.evaluate(base),
        };
        self
    }

    pub fn shrink_trbl_length(&mut self, trbl: TrblLength) -> &Self {
        // NOTE: not clear if x values should use width and y values use
        // height, or if having consistent values (as here) is better.
        // Current approach ensures a single-valued `TrblLength`` input
        // has a consistent border on all sides, which is probably the
        // expectation, and matches CSS (where all %ages are in terms
        // of inline-size - typically width - of parent element).

        // Where 'expand_trbl_length' takes the max of width / height,
        // this takes the minimum, so shrink up to 100% still leave some
        // box present.
        let base = self.width().min(self.height());
        *self = Self {
            x1: self.x1 + trbl.left.evaluate(base),
            y1: self.y1 + trbl.top.evaluate(base),
            x2: self.x2 - trbl.right.evaluate(base),
            y2: self.y2 - trbl.bottom.evaluate(base),
        };
        self
    }

    pub fn translated(&self, dx: f32, dy: f32) -> Self {
        Self {
            x1: self.x1 + dx,
            y1: self.y1 + dy,
            x2: self.x2 + dx,
            y2: self.y2 + dy,
        }
    }

    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }

    pub fn height(&self) -> f32 {
        self.y2 - self.y1
    }

    pub fn center(&self) -> (f32, f32) {
        (
            self.x1 + (self.x2 - self.x1) / 2.,
            self.y1 + (self.y2 - self.y1) / 2.,
        )
    }

    /// Scale the bounding box by the given amount with origin at the center
    #[allow(dead_code)]
    pub fn scale(&mut self, amount: f32) -> &Self {
        let width = self.x2 - self.x1;
        let height = self.y2 - self.y1;
        let dx_by_2 = (width * amount - width) / 2.;
        let dy_by_2 = (height * amount - height) / 2.;
        *self = Self {
            x1: self.x1 - dx_by_2,
            y1: self.y1 - dy_by_2,
            x2: self.x2 + dx_by_2,
            y2: self.y2 + dy_by_2,
        };
        self
    }

    /// Expand (floor/ceil) `BBox` to integer coords surrounding current extent.
    pub fn round(&mut self) -> &Self {
        *self = Self {
            x1: self.x1.floor(),
            y1: self.y1.floor(),
            x2: self.x2.ceil(),
            y2: self.y2.ceil(),
        };
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TrblLength {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}

impl TrblLength {
    pub fn new(top: Length, right: Length, bottom: Length, left: Length) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

impl FromStr for TrblLength {
    type Err = SvgdxError;
    fn from_str(value: &str) -> Result<Self> {
        // convert parts to Length, fail if any conversion fails.
        let parts: Result<Vec<_>> = attr_split(value).map(|v| strp_length(&v)).collect();
        let parts = parts?;

        Ok(match parts.len() {
            1 => TrblLength::new(parts[0], parts[0], parts[0], parts[0]),
            2 => TrblLength::new(parts[0], parts[1], parts[0], parts[1]),
            3 => TrblLength::new(parts[0], parts[1], parts[2], parts[1]),
            4 => TrblLength::new(parts[0], parts[1], parts[2], parts[3]),
            _ => Err(SvgdxError::InvalidData(
                "Incorrect number of values".to_owned(),
            ))?,
        })
    }
}

#[cfg(test)]
mod test {
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

    #[test]
    fn test_bbox() {
        let mut bb = BoundingBox::new(10., 0., 10., 10.);
        bb = bb.combine(&BoundingBox::new(20., 10., 30., 15.));
        bb = bb.combine(&BoundingBox::new(25., 20., 25., 30.));
        assert_eq!(bb, BoundingBox::new(10., 0., 30., 30.));

        bb.expand(10., 10.);
        assert_eq!(bb, BoundingBox::new(0., -10., 40., 40.));

        bb.scale(1.1);
        assert_eq!(bb, BoundingBox::new(-2., -12.5, 42., 42.5));
    }

    #[test]
    fn test_strp_length() {
        assert_eq!(strp_length("1").ok(), Some(Length::Absolute(1.)));
        assert_eq!(strp_length("123").ok(), Some(Length::Absolute(123.)));
        assert_eq!(strp_length("-0.0123").ok(), Some(Length::Absolute(-0.0123)));
        assert_eq!(strp_length("0.5%").ok(), Some(Length::Ratio(0.005)));
        assert_eq!(strp_length("150%").ok(), Some(Length::Ratio(1.5)));
        assert_eq!(strp_length("1.2.3").ok(), None);
        assert_eq!(strp_length("a").ok(), None);
        assert_eq!(strp_length("a%").ok(), None);
    }

    #[test]
    fn test_length() {
        let def_len = Length::default();
        assert_eq!(def_len.absolute(), Some(0.));
        assert_eq!(def_len.ratio(), None);

        let abs_len = Length::Absolute(123.5);
        assert_eq!(abs_len.absolute(), Some(123.5));
        assert_eq!(abs_len.ratio(), None);
        assert_eq!(abs_len.adjust(3.125), 123.5 + 3.125);

        let ratio_len = Length::Ratio(0.75);
        assert_eq!(ratio_len.absolute(), None);
        assert_eq!(ratio_len.ratio(), Some(0.75));
        assert_eq!(ratio_len.adjust(3.125), 0.75 * 3.125);
    }

    #[test]
    fn test_length_calc_offset() {
        assert_eq!(strp_length("25%").expect("test").calc_offset(10., 50.), 20.);
        assert_eq!(
            strp_length("50%").expect("test").calc_offset(-10., -9.),
            -9.5
        );
        assert_eq!(
            strp_length("200%").expect("test").calc_offset(10., 50.),
            90.
        );
        assert_eq!(
            strp_length("-3.5").expect("test").calc_offset(10., 50.),
            46.5
        );
        assert_eq!(
            strp_length("3.5").expect("test").calc_offset(-10., 90.),
            -6.5
        );
        // Test with start > end
        assert_eq!(
            strp_length("3.5").expect("test").calc_offset(30., 10.),
            26.5
        );
        assert_eq!(strp_length("10%").expect("test").calc_offset(30., 10.), 28.);
    }

    #[test]
    fn test_length_adjust() {
        assert_eq!(strp_length("25%").expect("test").adjust(10.), 2.5);
        assert_eq!(strp_length("-50%").expect("test").adjust(150.), -75.);
        assert_eq!(strp_length("125%").expect("test").adjust(20.), 25.);
        assert_eq!(strp_length("1").expect("test").adjust(23.), 24.);
        assert_eq!(strp_length("-12").expect("test").adjust(123.), 111.);
    }

    #[test]
    fn test_edgespec() {
        assert_eq!(
            "t:10".parse::<EdgeSpec>().expect("test"),
            EdgeSpec::Top(Length::Absolute(10.))
        );
        assert_eq!(
            "r:25%".parse::<EdgeSpec>().expect("test"),
            EdgeSpec::Right(Length::Ratio(0.25))
        );
        assert_eq!(
            "b:10".parse::<EdgeSpec>().expect("test"),
            EdgeSpec::Bottom(Length::Absolute(10.))
        );
        assert_eq!(
            "l:75%".parse::<EdgeSpec>().expect("test"),
            EdgeSpec::Left(Length::Ratio(0.75))
        );
    }

    #[test]
    fn test_locspec() {
        assert_eq!("tl".parse::<LocSpec>().expect("test"), LocSpec::TopLeft);
        assert_eq!("t".parse::<LocSpec>().expect("test"), LocSpec::Top);
        assert_eq!("tr".parse::<LocSpec>().expect("test"), LocSpec::TopRight);
        assert_eq!("r".parse::<LocSpec>().expect("test"), LocSpec::Right);
        assert_eq!("br".parse::<LocSpec>().expect("test"), LocSpec::BottomRight);
        assert_eq!("b".parse::<LocSpec>().expect("test"), LocSpec::Bottom);
        assert_eq!("bl".parse::<LocSpec>().expect("test"), LocSpec::BottomLeft);
        assert_eq!("l".parse::<LocSpec>().expect("test"), LocSpec::Left);
        assert_eq!("c".parse::<LocSpec>().expect("test"), LocSpec::Center);
    }

    #[test]
    fn test_get_point() {
        let bb = BoundingBox::new(10., 10., 20., 20.);
        assert_eq!(bb.get_point("t:2").expect("test"), (12., 10.));
        assert_eq!(bb.get_point("r:25%").expect("test"), (20., 12.5));
        assert_eq!(bb.get_point("b:6").expect("test"), (16., 20.));
        assert_eq!(bb.get_point("l:150%").expect("test"), (10., 25.));
        assert_eq!(bb.get_point("tl").expect("test"), (10., 10.));
        assert_eq!(bb.get_point("t").expect("test"), (15., 10.));
        assert_eq!(bb.get_point("tr").expect("test"), (20., 10.));
        assert_eq!(bb.get_point("r").expect("test"), (20., 15.));
        assert_eq!(bb.get_point("br").expect("test"), (20., 20.));
        assert_eq!(bb.get_point("b").expect("test"), (15., 20.));
        assert_eq!(bb.get_point("bl").expect("test"), (10., 20.));
        assert_eq!(bb.get_point("l").expect("test"), (10., 15.));
        assert_eq!(bb.get_point("c").expect("test"), (15., 15.));
    }
}
