#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoundingBox {
    Unknown,
    BBox(f32, f32, f32, f32),
}

impl BoundingBox {
    pub fn new() -> Self {
        BoundingBox::Unknown
    }

    pub fn extend(&mut self, other: &BoundingBox) -> &Self {
        *self = match (&self, other) {
            (Self::BBox(x1, y1, x2, y2), Self::BBox(ox1, oy1, ox2, oy2)) => {
                Self::BBox(x1.min(*ox1), y1.min(*oy1), x2.max(*ox2), y2.max(*oy2))
            }
            (Self::BBox(_, _, _, _), Self::Unknown) => *self,
            (Self::Unknown, Self::BBox(_, _, _, _)) => *other,
            (Self::Unknown, Self::Unknown) => Self::Unknown,
        };
        self
    }

    /// dilate the bounding box by the given absolute amount in each direction
    pub fn expand(&mut self, amount: f32) -> &Self {
        if let BoundingBox::BBox(x1, y1, x2, y2) = self {
            let new = BoundingBox::BBox(*x1 - amount, *y1 - amount, *x2 + amount, *y2 + amount);
            *self = new;
        }
        self
    }

    pub fn width(&self) -> Option<f32> {
        if let Self::BBox(x1, _, x2, _) = self {
            Some(x2 - x1)
        } else {
            None
        }
    }

    pub fn height(&self) -> Option<f32> {
        if let Self::BBox(_, y1, _, y2) = self {
            Some(y2 - y1)
        } else {
            None
        }
    }

    /// Scale the bounding box by the given amount with origin at the center
    pub fn scale(&mut self, amount: f32) -> &Self {
        if let BoundingBox::BBox(x1, y1, x2, y2) = &self {
            let dx_by_2 = (self.width().unwrap() * amount - self.width().unwrap()) / 2.;
            let dy_by_2 = (self.height().unwrap() * amount - self.height().unwrap()) / 2.;
            *self = BoundingBox::BBox(*x1 - dx_by_2, *y1 - dy_by_2, *x2 + dx_by_2, *y2 + dy_by_2);
        }
        self
    }
}

#[test]
fn test_bbox() {
    let mut bb = BoundingBox::new();
    bb.extend(&BoundingBox::BBox(10., 0., 10., 10.));
    bb.extend(&BoundingBox::BBox(20., 10., 30., 15.));
    bb.extend(&BoundingBox::BBox(25., 20., 25., 30.));
    assert_eq!(bb, BoundingBox::BBox(10., 0., 30., 30.));

    bb.expand(10.);
    assert_eq!(bb, BoundingBox::BBox(0., -10., 40., 40.));

    bb.scale(1.1);
    assert_eq!(bb, BoundingBox::BBox(-2., -12.5, 42., 42.5));
}
