//! Simple linear and radial gradient elements
//!
//! linearGradient or radialGradient elements may be given a 'stops' attribute,
//! which is converted into additional `stop` child elements.
//!
//! The `stops` attribute format is:
//!
//!  `offset1 color1 [opacity1]; offset2 color2 [opacity2]; ...`
//!
//! which expands to:
//!
//!  `stop offset="offset1" stop-color="color1" [stop-opacity="opacity1"]/>`
//!
//! For linearGradient, the gradient 'line' is defined by (x1,y1) to (x2,y2).
//!
//! Additional 'dir' and 'length' attrs allow polar specification of this:
//!
//! * dir: angle in degrees (0 = left to right, 90 = top to bottom, etc)
//! * length: length of gradient line; default is to extend to unit square edge
//!
//! if dir and/or length are provided, either (x1,y1) or (x2,y2) may be omitted,
//! and will be computed accordingly; if both are provided, both points must be
//! specified explicitly.

use super::SvgElement;
use crate::context::TransformerContext;
use crate::errors::Result;
use crate::events::{OutputEvent, OutputList};
use crate::geometry::{strp_length, BoundingBox, Length};
use crate::transform::{process_events, EventGen};
use crate::types::{attr_split, fstr, split_compound_attr, strp};
use crate::Error;

#[derive(Debug, PartialEq)]
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

fn stop_elements(stops: &str) -> Result<Vec<SvgElement>> {
    let mut new_inner = Vec::new();
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
    Ok(new_inner)
}

pub struct LinearGradient<'a>(pub &'a SvgElement);

impl EventGen for LinearGradient<'_> {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let mut new_el = self.0.clone();
        new_el.eval_attributes(context)?;

        let new_inner = if let Some(stops) = new_el.pop_attr("stops") {
            stop_elements(&stops)?
        } else {
            Vec::new()
        };

        // push variables onto the stack
        context.push_element(self.0);

        // expand any provided compound attributes
        if let Some(xy1) = new_el.pop_attr("xy1") {
            let (x1, y1) = split_compound_attr(&xy1);
            new_el.set_default_attr("x1", &x1);
            new_el.set_default_attr("y1", &y1);
        }
        let origin: Option<(f32, f32)> = {
            let x1 = new_el.get_attr("x1").map(strp_length).transpose()?;
            let y1 = new_el.get_attr("y1").map(strp_length).transpose()?;
            match (x1, y1) {
                (Some(x1), Some(y1)) => Some((x1.evaluate(1.), y1.evaluate(1.))),
                _ => None,
            }
        };
        if let Some(xy2) = new_el.pop_attr("xy2") {
            let (x2, y2) = split_compound_attr(&xy2);
            new_el.set_default_attr("x2", &x2);
            new_el.set_default_attr("y2", &y2);
        }
        let endpoint: Option<(f32, f32)> = {
            let x2 = new_el.get_attr("x2").map(strp_length).transpose()?;
            let y2 = new_el.get_attr("y2").map(strp_length).transpose()?;
            match (x2, y2) {
                (Some(x2), Some(y2)) => Some((x2.evaluate(1.), y2.evaluate(1.))),
                _ => None,
            }
        };

        let length = new_el
            .pop_attr("length")
            .map(|length| {
                strp_length(&length)
                    .map_err(|e| Error::InvalidValue(format!("invalid length: {e}"), length))
            })
            .transpose()?;

        let dir = new_el
            .pop_attr("dir")
            .map(|dir| {
                {
                    // normalize angle to 0-360 range
                    strp(&dir).map(|a| {
                        let a = a % 360.0;
                        if a < 0.0 {
                            a + 360.0
                        } else {
                            a
                        }
                    })
                }
                .map_err(|e| Error::InvalidValue(format!("invalid dir: {e}"), dir))
            })
            .transpose()?;

        match (origin, endpoint, length, dir) {
            (Some(_), Some(_), None, None) | (None, None, None, None) => {
                // no-op; either have all coords or nothing
            }
            (Some(_), None, maybe_len, maybe_dir)
            | (None, Some(_), maybe_len, maybe_dir)
            | (None, None, maybe_len, maybe_dir) => {
                let dir = maybe_dir.unwrap_or(0.0);

                // Determine fixed coord
                let fixed = if let Some(xy1) = origin {
                    xy1
                } else if let Some(xy2) = endpoint {
                    xy2
                } else {
                    // Select corner based on angle quadrant
                    match dir {
                        d if (0.0..=90.0).contains(&d) => (0.0, 0.0),
                        d if (90.0..180.0).contains(&d) => (1.0, 0.0),
                        d if (180.0..=270.0).contains(&d) => (1.0, 1.0),
                        _ => (0.0, 1.0),
                    }
                };

                let (x_fixed, y_fixed) = fixed;
                let rad = dir.to_radians();

                // Calculate the other endpoint
                let (x_calc, y_calc) = if let Some(length) = maybe_len {
                    let icept = intercept_unit_square(fixed, dir);
                    let dist = ((icept.0 - x_fixed).powi(2) + (icept.1 - y_fixed).powi(2)).sqrt();
                    (
                        x_fixed + length.evaluate(dist) * rad.cos(),
                        y_fixed + length.evaluate(dist) * rad.sin(),
                    )
                } else {
                    intercept_unit_square(fixed, dir)
                };

                // Set the appropriate missing coordinate pair(s)
                if origin.is_none() && endpoint.is_none() {
                    // No coordinates provided - set all four
                    new_el.set_default_attr("x1", &fstr(x_fixed));
                    new_el.set_default_attr("y1", &fstr(y_fixed));
                    new_el.set_default_attr("x2", &fstr(x_calc));
                    new_el.set_default_attr("y2", &fstr(y_calc));
                } else if endpoint.is_none() {
                    // Have origin, set endpoint
                    new_el.set_default_attr("x2", &fstr(x_calc));
                    new_el.set_default_attr("y2", &fstr(y_calc));
                } else {
                    // Have endpoint, set origin
                    new_el.set_default_attr("x1", &fstr(x_calc));
                    new_el.set_default_attr("y1", &fstr(y_calc));
                }
            }
            (Some(_), Some(_), Some(_), _) | (Some(_), Some(_), _, Some(_)) => {
                // origin + endpoint together with either length or dir is over-constrained
                return Err(Error::InvalidValue(
                    "Over-constrained linearGradient definition".into(),
                    self.0.to_string(),
                ));
            }
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

// given an origin (assumed inside unit square) and direction (in degrees),
// return the intersection point of the corresponding ray with the unit square
fn intercept_unit_square(origin: (f32, f32), dir: f32) -> (f32, f32) {
    let rad = dir.to_radians();
    let dx = rad.cos();
    let dy = rad.sin();
    let (ox, oy) = origin;

    let mut t = f32::INFINITY;

    if dx > 1e-6 {
        t = t.min((1.0 - ox) / dx);
    } else if dx < -1e-6 {
        t = t.min(-ox / dx);
    }

    if dy > 1e-6 {
        t = t.min((1.0 - oy) / dy);
    } else if dy < -1e-6 {
        t = t.min(-oy / dy);
    }

    (ox + t * dx, oy + t * dy)
}

pub struct RadialGradient<'a>(pub &'a SvgElement);

impl EventGen for RadialGradient<'_> {
    fn generate_events(
        &self,
        context: &mut TransformerContext,
    ) -> Result<(OutputList, Option<BoundingBox>)> {
        let mut new_el = self.0.clone();
        new_el.eval_attributes(context)?;

        let new_inner = if let Some(stops) = new_el.pop_attr("stops") {
            stop_elements(&stops)?
        } else {
            Vec::new()
        };

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
    fn test_gradstoplist_parse() {
        let input = "0% red; 50% green; 100% blue";
        let stop_list: GradStopList = input.parse().unwrap();
        assert_eq!(stop_list.stops.len(), 3);
        assert_eq!(
            stop_list.stops,
            [
                GradStop {
                    offset: Length::Ratio(0.0),
                    colour: "red".into(),
                    opacity: None,
                },
                GradStop {
                    offset: Length::Ratio(0.5),
                    colour: "green".into(),
                    opacity: None,
                },
                GradStop {
                    offset: Length::Ratio(1.0),
                    colour: "blue".into(),
                    opacity: None,
                }
            ]
        );

        let input = "0 red 1; 1 blue 0.5";
        let stop_list: GradStopList = input.parse().unwrap();
        assert_eq!(stop_list.stops.len(), 2);
        assert_eq!(
            stop_list.stops,
            [
                GradStop {
                    offset: Length::Absolute(0.0),
                    colour: "red".into(),
                    opacity: Some(1.0),
                },
                GradStop {
                    offset: Length::Absolute(1.0),
                    colour: "blue".into(),
                    opacity: Some(0.5),
                }
            ]
        );
    }

    #[test]
    fn test_intercept_unit_square() {
        let (x, y) = intercept_unit_square((0.5, 0.5), 0.);
        assert_eq_approx!(x, 1.0);
        assert_eq_approx!(y, 0.5);

        let (x, y) = intercept_unit_square((0.5, 0.5), 90.);
        assert_eq_approx!(x, 0.5);
        assert_eq_approx!(y, 1.0);

        let (x, y) = intercept_unit_square((0.5, 0.5), -45.);
        assert_eq_approx!(x, 1.0);
        assert_eq_approx!(y, 0.0);

        let (x, y) = intercept_unit_square((0.5, 0.5), 45.);
        assert_eq_approx!(x, 1.0);
        assert_eq_approx!(y, 1.0);

        let (x, y) = intercept_unit_square((0.5, 0.5), 135.);
        assert_eq_approx!(x, 0.0);
        assert_eq_approx!(y, 1.0);

        let (x, y) = intercept_unit_square((0.5, 0.5), 225.);
        assert_eq_approx!(x, 0.0);
        assert_eq_approx!(y, 0.0);

        let (x, y) = intercept_unit_square((0.5, 0.5), 315.);
        assert_eq_approx!(x, 1.0);
        assert_eq_approx!(y, 0.0);

        // non-centre origin
        let (x, y) = intercept_unit_square((0.2, 0.2), 45.);
        assert_eq_approx!(x, 1.0);
        assert_eq_approx!(y, 1.0);

        let (x, y) = intercept_unit_square((0.25, 0.5), 45.);
        assert_eq_approx!(x, 0.75);
        assert_eq_approx!(y, 1.0);

        let (x, y) = intercept_unit_square((1., 0.5), 180.);
        assert_eq_approx!(x, 0.0);
        assert_eq_approx!(y, 0.5);
    }
}
