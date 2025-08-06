use super::types::Size;
use crate::geometry::{LocSpec, ScalarSpec, TrblLength};

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
        use LocSpec::*;
        match ls {
            TopLeft => tl,
            Top => ((self.x1 + self.x2) / 2., self.y1),
            TopRight => tr,
            Right => (self.x2, (self.y1 + self.y2) / 2.),
            BottomRight => br,
            Bottom => ((self.x1 + self.x2) / 2., self.y2),
            BottomLeft => bl,
            Left => (self.x1, (self.y1 + self.y2) / 2.),
            Center => c,
            TopEdge(len) => (len.calc_offset(self.x1, self.x2), self.y1),
            RightEdge(len) => (self.x2, len.calc_offset(self.y1, self.y2)),
            BottomEdge(len) => (len.calc_offset(self.x1, self.x2), self.y2),
            LeftEdge(len) => (self.x1, len.calc_offset(self.y1, self.y2)),
            PureLength(_) => panic!(),
        }
    }

    pub fn xy1(&self) -> (f32, f32) {
        (self.x1, self.y1)
    }

    pub fn xy2(&self) -> (f32, f32) {
        (self.x2, self.y2)
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
            // By convention, radius is maximum of rx/ry
            ScalarSpec::Radius => self
                .scalarspec(ScalarSpec::Rx)
                .max(self.scalarspec(ScalarSpec::Ry)),
            ScalarSpec::Rx => (self.x2 - self.x1).abs() / 2.,
            ScalarSpec::Ry => (self.y2 - self.y1).abs() / 2.,
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

    pub fn size(&self) -> Size {
        Size::new(self.width(), self.height())
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
