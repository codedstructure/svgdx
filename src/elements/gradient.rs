// simplify linearGradient and radialGradient elements

use super::SvgElement;
use crate::context::TransformerContext;
use crate::errors::Result;
use crate::events::{OutputEvent, OutputList};
use crate::geometry::{BoundingBox, Length};
use crate::transform::{process_events, EventGen};
use crate::types::{attr_split, split_compound_attr, strp};
use crate::Error;

pub struct LinearGradient<'a>(pub &'a SvgElement);

impl EventGen for LinearGradient<'_> {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        // thoughts:
        // if an empty element with 'stops' attr, generate stops from that
        // else process inner elements as normal
        // stops format is "offset1 color1; offset2 color2; ..." which expands to
        // <stop offset="offset1" stop-color="color1" /> etc.
        // direction can be specified with dir="angle" or "x1,y1,x2,y2"
        // default is top to bottom (0,0 to 0,1), equivalent to dir="90"

        // so... what sort of trait derived from SvgElement would be useful here?
        // got the basic get_attr/pop_attr stuff, but need to generate inner events,
        // or maybe just transform the element into an element that contains other
        // synthesised elements?

        let mut new_el = self.0.clone();
        new_el.eval_attributes(context)?;

        let stops = new_el.pop_attr("stops");
        let mut new_inner = Vec::new();
        if let Some(stops) = stops {
            let stop_list: GradStopList = stops
                .parse()
                .map_err(|e| Error::Parse(format!("stops: {e}")))?;
            for stop in stop_list.stops {
                let mut stop_el = SvgElement::new(
                    "stop",
                    &[
                        ("offset".into(), stop.offset.to_string()),
                        ("stop-color".into(), stop.colour),
                    ],
                );
                if let Some(opacity) = stop.opacity {
                    stop_el.set_attr("stop-opacity", &opacity.to_string());
                }
                new_inner.push(stop_el);
            }
        }

        // push variables onto the stack
        context.push_element(self.0);

        if let Some(dir) = new_el.pop_attr("dir") {
            let angle: f32 = dir.parse().map_err(|e| Error::Parse(format!("dir: {e}")))?;
            let (x1, y1, x2, y2) = dir_to_coords(angle)?;
            new_el.set_attr("x1", &x1.to_string());
            new_el.set_attr("y1", &y1.to_string());
            new_el.set_attr("x2", &x2.to_string());
            new_el.set_attr("y2", &y2.to_string());
        }

        let (inner, _) = if let Some(inner_events) = self.0.inner_events(context) {
            // get the inner events / bbox first, as some outer element attrs
            // (e.g. `transform` via rotate) may depend on the bbox.
            let mut inner_events = inner_events.clone();
            inner_events.rebase_under(new_el.order_index.clone());
            process_events(inner_events, context).inspect_err(|_| {
                context.pop_element();
            })?
        } else {
            (OutputList::new(), None)
        };

        // pop variables off the stack
        context.pop_element();

        // Need bbox to provide center of rotation
        new_el.handle_rotation()?;

        let mut events = OutputList::new();
        if self.0.is_empty_element() && new_inner.is_empty() {
            events.push(OutputEvent::Empty(new_el.clone()));
        } else {
            let el_name = new_el.name().to_owned();
            events.push(OutputEvent::Start(new_el.clone()));
            events.extend(inner);
            for el in new_inner {
                events.extend(el.generate_events(context)?.0);
            }
            events.push(OutputEvent::End(el_name));
        }

        context.update_element(&new_el);

        Ok((events, None))
    }
}

struct GradStop {
    offset: Length,
    colour: String,
    opacity: Option<f32>,
}

impl std::str::FromStr for GradStop {
    type Err = Error;

    // format: "offset colour [opacity]"
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut pieces = attr_split(s);
        let offset_str = pieces.next().ok_or_else(|| {
            Error::InvalidValue(
                "Missing offset in stop definition".to_string(),
                s.to_string(),
            )
        })?;
        let colour = pieces.next().ok_or_else(|| {
            Error::InvalidValue(
                "Missing colour in stop definition".to_string(),
                s.to_string(),
            )
        })?;
        let opacity = pieces
            .next()
            .map(|o_str| {
                strp(&o_str).and_then(|o| {
                    (0.0..=1.0).contains(&o).then_some(o).ok_or_else(|| {
                        Error::InvalidValue("Invalid opacity".into(), o_str.to_owned())
                    })
                })
            })
            .transpose()?;
        let offset: Length = offset_str
            .parse()
            .map_err(|_| Error::InvalidValue("Invalid stop offset".into(), offset_str))?;
        Ok(GradStop {
            offset,
            colour,
            opacity,
        })
    }
}

struct GradStopList {
    stops: Vec<GradStop>,
}

impl std::str::FromStr for GradStopList {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut stops = Vec::new();
        for part in s.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            stops.push(part.parse()?)
        }
        Ok(GradStopList { stops })
    }
}

// convert 'dir=angle' to x1,y1,x2,y2
fn dir_to_coords(angle: f32) -> Result<(Length, Length, Length, Length)> {
    let rad = angle.to_radians();
    // SVG defaults are (0%, 0%) to (100%, 0%), i.e. left to right; this corresponds to dir="0".
    // generated coords are in %age terms, but should be normalised to fit in a unit square,
    // so a 45% angle becomes (0,0) to (1,1).
    // the quadrant should be considered, e.g. between 0 and 90, start at (0,0) (this includes
    // the default settings of x1/y1/x2/y2 being 0/0/100%/0).
    // Between 90 and 180, start at (1,0), betwee n 180 and 270 start at (1,1), and
    // between 270 and 360 start at (0,1).

    // Normalize angle to 0-360 range
    let angle = angle % 360.0;
    let angle = if angle < 0.0 { angle + 360.0 } else { angle };

    const ANGLE_EPSILON: f32 = 0.05; // degrees

    // special-case cardinal directions
    if (angle - 0.0).abs() < ANGLE_EPSILON {
        return Ok((
            Length::Ratio(0.0),
            Length::Ratio(0.0),
            Length::Ratio(1.0),
            Length::Ratio(0.0),
        ));
    } else if (angle - 90.0).abs() < ANGLE_EPSILON {
        return Ok((
            Length::Ratio(0.0),
            Length::Ratio(0.0),
            Length::Ratio(0.0),
            Length::Ratio(1.0),
        ));
    } else if (angle - 180.0).abs() < ANGLE_EPSILON {
        return Ok((
            Length::Ratio(1.0),
            Length::Ratio(0.0),
            Length::Ratio(0.0),
            Length::Ratio(0.0),
        ));
    } else if (angle - 270.0).abs() < ANGLE_EPSILON {
        return Ok((
            Length::Ratio(0.0),
            Length::Ratio(1.0),
            Length::Ratio(0.0),
            Length::Ratio(0.0),
        ));
    }

    // Ray-box intersection from center
    let dx = rad.cos();
    let dy = rad.sin();

    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;

    // Intersect with vertical edges (x=0 and x=1)
    if dx.abs() > 1e-6 {
        let t1 = (0.0 - 0.5) / dx;
        let t2 = (1.0 - 0.5) / dx;
        t_min = t_min.max(t1.min(t2));
        t_max = t_max.min(t1.max(t2));
    }

    // Intersect with horizontal edges (y=0 and y=1)
    if dy.abs() > 1e-6 {
        let t1 = (0.0 - 0.5) / dy;
        let t2 = (1.0 - 0.5) / dy;
        t_min = t_min.max(t1.min(t2));
        t_max = t_max.min(t1.max(t2));
    }

    let x1 = 0.5 + t_min * dx;
    let y1 = 0.5 + t_min * dy;
    let x2 = 0.5 + t_max * dx;
    let y2 = 0.5 + t_max * dy;

    Ok((
        Length::Ratio(x1),
        Length::Ratio(y1),
        Length::Ratio(x2),
        Length::Ratio(y2),
    ))
}

pub struct RadialGradient<'a>(pub &'a SvgElement);

impl EventGen for RadialGradient<'_> {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let mut new_el = self.0.clone();
        new_el.eval_attributes(context)?;

        let stops = new_el.pop_attr("stops");
        let mut new_inner = Vec::new();
        if let Some(stops) = stops {
            let stop_list: GradStopList = stops
                .parse()
                .map_err(|e| Error::Parse(format!("stops: {e}")))?;
            for stop in stop_list.stops {
                let mut stop_el = SvgElement::new(
                    "stop",
                    &[
                        ("offset".into(), stop.offset.to_string()),
                        ("stop-color".into(), stop.colour),
                    ],
                );
                if let Some(opacity) = stop.opacity {
                    stop_el.set_attr("stop-opacity", &opacity.to_string());
                }
                new_inner.push(stop_el);
            }
        }

        // outermost circle is defined by cx,cy,r, and corresponding to a
        // stop at offset 1.0; defaults to 50%/50%/50%
        if let Some(cxy) = new_el.pop_attr("cxy") {
            let (cx, cy) = split_compound_attr(&cxy);
            new_el.set_default_attr("cx", &cx);
            new_el.set_default_attr("cy", &cy);
        }

        // focal point of radial gradient; corresponds to stop at offset 0
        // and defaults to cx,cy
        if let Some(cxy) = new_el.pop_attr("fxy") {
            let (cx, cy) = split_compound_attr(&cxy);
            new_el.set_default_attr("fx", &cx);
            new_el.set_default_attr("fy", &cy);
        }

        // push variables onto the stack
        context.push_element(self.0);

        let (inner, _) = if let Some(inner_events) = self.0.inner_events(context) {
            // get the inner events / bbox first, as some outer element attrs
            // (e.g. `transform` via rotate) may depend on the bbox.
            let mut inner_events = inner_events.clone();
            inner_events.rebase_under(new_el.order_index.clone());
            process_events(inner_events, context).inspect_err(|_| {
                context.pop_element();
            })?
        } else {
            (OutputList::new(), None)
        };

        // pop variables off the stack
        context.pop_element();

        // Need bbox to provide center of rotation
        new_el.handle_rotation()?;

        let mut events = OutputList::new();
        if self.0.is_empty_element() && new_inner.is_empty() {
            events.push(OutputEvent::Empty(new_el.clone()));
        } else {
            let el_name = new_el.name().to_owned();
            events.push(OutputEvent::Start(new_el.clone()));
            events.extend(inner);
            for el in new_inner {
                events.extend(el.generate_events(context)?.0);
            }
            events.push(OutputEvent::End(el_name));
        }

        context.update_element(&new_el);

        Ok((events, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_eq_approx {
        ($left:expr, $right:expr) => {
            let left_val = $left;
            let right_val = $right;
            assert!(
                (left_val - right_val).abs() < 0.001,
                "assertion failed: `(left ≈ right)`\n  left: `{left_val:?}`,\n right: `{right_val:?}`",
            );
        };
    }

    #[test]
    fn test_dir_to_coords() {
        let (x1, y1, x2, y2) = dir_to_coords(0.).unwrap();
        // SVG defaults are (0%, 0%) to (100%, 0%)
        assert_eq_approx!(x1.ratio().unwrap(), 0.0);
        assert_eq_approx!(y1.ratio().unwrap(), 0.0);
        assert_eq_approx!(x2.ratio().unwrap(), 1.0);
        assert_eq_approx!(y2.ratio().unwrap(), 0.0);

        let (x1, y1, x2, y2) = dir_to_coords(90.).unwrap();
        assert_eq_approx!(x1.ratio().unwrap(), 0.0);
        assert_eq_approx!(y1.ratio().unwrap(), 0.0);
        assert_eq_approx!(x2.ratio().unwrap(), 0.0);
        assert_eq_approx!(y2.ratio().unwrap(), 1.0);

        let (x1, y1, x2, y2) = dir_to_coords(45.).unwrap();
        assert_eq_approx!(x1.ratio().unwrap(), 0.0);
        assert_eq_approx!(y1.ratio().unwrap(), 0.0);
        assert_eq_approx!(x2.ratio().unwrap(), 1.0);
        assert_eq_approx!(y2.ratio().unwrap(), 1.0);

        let (x1, y1, x2, y2) = dir_to_coords(135.).unwrap();
        assert_eq_approx!(x1.ratio().unwrap(), 1.0);
        assert_eq_approx!(y1.ratio().unwrap(), 0.0);
        assert_eq_approx!(x2.ratio().unwrap(), 0.0);
        assert_eq_approx!(y2.ratio().unwrap(), 1.0);

        // 180
        let (x1, y1, x2, y2) = dir_to_coords(180.).unwrap();
        assert_eq_approx!(x1.ratio().unwrap(), 1.0);
        assert_eq_approx!(y1.ratio().unwrap(), 0.0);
        assert_eq_approx!(x2.ratio().unwrap(), 0.0);
        assert_eq_approx!(y2.ratio().unwrap(), 0.0);

        // 270
        let (x1, y1, x2, y2) = dir_to_coords(270.).unwrap();
        assert_eq_approx!(x1.ratio().unwrap(), 0.0);
        assert_eq_approx!(y1.ratio().unwrap(), 1.0);
        assert_eq_approx!(x2.ratio().unwrap(), 0.0);
        assert_eq_approx!(y2.ratio().unwrap(), 0.0);
    }
}
