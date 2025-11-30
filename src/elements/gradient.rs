// simplify linearGradient and radialGradient elements

use crate::context::TransformerContext;
use crate::errors::Result;
use crate::events::{OutputEvent, OutputList};
use crate::geometry::{BoundingBox, Length};
use crate::transform::{process_events, EventGen};
use crate::types::attr_split;
use crate::Error;

use super::SvgElement;

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
                let stop_el = SvgElement::new(
                    "stop",
                    &[
                        ("offset".into(), stop.offset.to_string()),
                        ("stop-color".into(), stop.colour),
                    ],
                );
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
}

struct GradStopList {
    stops: Vec<GradStop>,
}

impl std::str::FromStr for GradStopList {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut stops = Vec::new();
        for part in s.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let mut pieces = attr_split(part);
            let offset_str = pieces
                .next()
                .ok_or_else(|| "Missing offset in stop definition".to_string())?;
            let colour = pieces
                .next()
                .ok_or_else(|| "Missing colour in stop definition".to_string())?;
            let offset: Length = offset_str
                .parse()
                .map_err(|e| format!("Invalid stop offset '{}': {}", offset_str, e))?;
            stops.push(GradStop { offset, colour });
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

    let (x1, y1, x2, y2) = if angle >= 0.0 && angle <= 90.0 {
        // Quadrant 1: anchor at (0,0), project to edge of unit square
        let x = rad.cos();
        let y = rad.sin();
        // Scale to reach edge: either x=1 or y=1
        let scale = if x > y { 1.0 / x } else { 1.0 / y };
        (0.0, 0.0, x * scale, y * scale)
    } else if angle > 90.0 && angle <= 180.0 {
        // Quadrant 2: anchor at (1,0), project to edge
        let x = rad.cos();
        let y = rad.sin();
        // Scale to reach edge: either x=0 (left) or y=1 (bottom)
        let scale = if x.abs() > y { 1.0 / x.abs() } else { 1.0 / y };
        (1.0, 0.0, 1.0 + x * scale, y * scale)
    } else if angle > 180.0 && angle < 270.0 {
        // Quadrant 3: anchor at (1,1), project to edge
        let x = rad.cos();
        let y = rad.sin();
        // Scale to reach edge: either x=0 (left) or y=0 (top)
        let scale = if x.abs() > y.abs() {
            1.0 / x.abs()
        } else {
            1.0 / y.abs()
        };
        (1.0, 1.0, 1.0 + x * scale, 1.0 + y * scale)
    } else {
        // Quadrant 4 (>=270 and <360): anchor at (0,1), project to edge
        let x = rad.cos();
        let y = rad.sin();
        // Scale to reach edge: either x=1 (right) or y=0 (top)
        let scale = if x > y.abs() { 1.0 / x } else { 1.0 / y.abs() };
        (0.0, 1.0, x * scale, 1.0 + y * scale)
    };

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
                let stop_el = SvgElement::new(
                    "stop",
                    &[
                        ("offset".into(), stop.offset.to_string()),
                        ("stop-color".into(), stop.colour),
                    ],
                );
                new_inner.push(stop_el);
            }
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
