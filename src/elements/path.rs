use super::SvgElement;
use crate::errors::{Error, Result};
use crate::geometry::BoundingBox;
use crate::types::{attr_split, fstr, strp};

struct PathParser {
    tokens: SvgPathSyntax,
    position: Option<(f32, f32)>,
    start_pos: Option<(f32, f32)>,
    command: Option<char>,
    // previous second control point (if any) for evaluating 'S' and 's'
    cubic_cp2: Option<(f32, f32)>,
    // previous control point (if any) for evaluating 'T' and 't'
    quadratic_cp: Option<(f32, f32)>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

pub struct SvgPathSyntax {
    data: Vec<char>,
    index: usize,
}

impl SvgPathSyntax {
    pub fn new(data: &str) -> Self {
        Self {
            data: data.chars().collect(),
            index: 0,
        }
    }
}

impl PathSyntax for SvgPathSyntax {
    fn at_command(&self) -> Result<bool> {
        self.check_not_end()?;
        let c = self
            .current()
            .ok_or_else(|| Error::Parse("no data".to_string()))?;
        Ok("MmLlHhVvZzCcSsQqTtAa".contains(c))
    }

    fn current(&self) -> Option<char> {
        self.data.get(self.index).copied()
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn at_end(&self) -> bool {
        self.index >= self.data.len()
    }
}

pub trait PathSyntax {
    fn at_command(&self) -> Result<bool>;
    fn current(&self) -> Option<char>;
    fn advance(&mut self);
    fn at_end(&self) -> bool;

    fn check_not_end(&self) -> Result<()> {
        if self.at_end() {
            Err(Error::Parse("ran out of data!".to_string()))
        } else {
            Ok(())
        }
    }

    fn skip_whitespace(&mut self) {
        // SVG definition of whitespace is 0x20, 0x9, 0xA, 0xD. Rust's is_ascii_whitespace()
        // also includes 0xC, but is close enough and convenient.
        while !self.at_end() && self.current().unwrap().is_ascii_whitespace() {
            self.advance();
        }
    }

    fn skip_wsp_comma(&mut self) {
        self.skip_whitespace();
        if !self.at_end() && self.current().unwrap() == ',' {
            self.advance();
            self.skip_whitespace();
        }
    }

    fn read_flag(&mut self) -> Result<u32> {
        self.check_not_end()?;
        // per the grammar for `a`/`A`, could have '00' etc for
        // the two adjacent flags...
        let res = match self.current().unwrap() {
            '0' => 0,
            '1' => 1,
            _ => {
                return Err(Error::InvalidValue(
                    "flag".to_string(),
                    self.current().unwrap().to_string(),
                ))
            }
        };
        self.advance();
        self.skip_wsp_comma();
        Ok(res)
    }

    fn read_number(&mut self) -> Result<f32> {
        self.check_not_end()?;
        let mut mult = 1.;
        match self.current().unwrap() {
            '-' => {
                mult = -1.;
                self.advance();
            }
            '+' => {
                self.advance();
            }
            _ => {}
        };
        Ok(mult * self.read_non_negative()?)
    }

    fn read_non_negative(&mut self) -> Result<f32> {
        self.check_not_end()?;
        let mut s = String::new();
        let mut dot_valid = true;
        let mut exp_valid = true;
        while let Some(ch) = self.current() {
            match ch {
                '0'..='9' => {
                    s.push(ch);
                    self.advance();
                }
                '.' if dot_valid => {
                    s.push(ch);
                    self.advance();
                    dot_valid = false;
                }
                'e' | 'E' if exp_valid && s.ends_with(|c: char| c.is_ascii_digit()) => {
                    s.push(ch);
                    self.advance();
                    // include sign character if present
                    if self.current() == Some('-') || self.current() == Some('+') {
                        s.push(self.current().unwrap());
                        self.advance();
                    }
                    exp_valid = false;
                    dot_valid = false;
                }
                _ => break,
            }
        }
        self.skip_wsp_comma();
        Ok(s.parse()?)
    }

    fn read_coord(&mut self) -> Result<(f32, f32)> {
        let x = self.read_number()?;
        self.skip_wsp_comma();
        let y = self.read_number()?;
        self.skip_wsp_comma();
        Ok((x, y))
    }

    fn read_command(&mut self) -> Result<char> {
        if self.at_command()? {
            let command = self.current().unwrap();
            self.advance();
            self.skip_wsp_comma();
            Ok(command)
        } else {
            Err(Error::InvalidValue(
                "invalid path command".to_string(),
                self.current().map(|c| c.to_string()).unwrap_or_default(),
            ))
        }
    }
}

impl PathParser {
    fn new(data: &str) -> Self {
        PathParser {
            tokens: SvgPathSyntax::new(data),
            position: None,
            start_pos: None,
            command: None,
            cubic_cp2: None,
            quadratic_cp: None,
            min_x: 0.,
            min_y: 0.,
            max_x: 0.,
            max_y: 0.,
        }
    }

    fn update_position(&mut self, pos: (f32, f32)) {
        let old_pos = self.position;
        self.position = Some(pos);
        if self.start_pos.is_none() {
            self.start_pos = self.position;
        }
        if old_pos.is_none() {
            self.min_x = pos.0;
            self.min_y = pos.1;
            self.max_x = pos.0;
            self.max_y = pos.1;
        } else {
            self.min_x = self.min_x.min(pos.0);
            self.min_y = self.min_y.min(pos.1);
            self.max_x = self.max_x.max(pos.0);
            self.max_y = self.max_y.max(pos.1);
        }
    }

    fn get_bbox(&self) -> Option<BoundingBox> {
        if self.start_pos.is_some() {
            Some(BoundingBox::new(
                self.min_x, self.min_y, self.max_x, self.max_y,
            ))
        } else {
            None
        }
    }

    fn process_instruction(&mut self) -> Result<()> {
        if self.command.is_none() || self.tokens.at_command()? {
            // "The command letter can be eliminated on subsequent commands if the same
            // command is used multiple times in a row (e.g., you can drop the second
            // "L" in "M 100 200 L 200 100 L -100 -200" and use "M 100 200 L 200 100
            // -100 -200" instead)."
            self.command = Some(self.tokens.read_command()?);
        }

        let mut cubic_cp2: Option<(f32, f32)> = None;
        let mut quadratic_cp: Option<(f32, f32)> = None;

        match self.command.expect("Command should be already set") {
            'M' | 'L' => {
                // "(x y)+"
                let xy = self.tokens.read_coord()?;
                self.update_position(xy);
            }
            'm' | 'l' => {
                let (dx, dy) = self.tokens.read_coord()?;
                let (px, py) = self.position.unwrap_or((0., 0.));
                self.update_position((px + dx, py + dy));
            }
            'H' => {
                let new_x = self.tokens.read_number()?;
                let (_, py) = self.position.unwrap_or((0., 0.));
                self.update_position((new_x, py));
            }
            'h' => {
                let dx = self.tokens.read_number()?;
                let (px, py) = self.position.unwrap_or((0., 0.));
                self.update_position((px + dx, py));
            }
            'V' => {
                let new_y = self.tokens.read_number()?;
                let (px, _) = self.position.unwrap_or((0., 0.));
                self.update_position((px, new_y));
            }
            'v' => {
                let dy = self.tokens.read_number()?;
                let (px, py) = self.position.unwrap_or((0., 0.));
                self.update_position((px, py + dy));
            }
            'Z' | 'z' => {
                self.update_position(self.start_pos.unwrap_or((0., 0.)));
            }
            'C' => {
                let cp1 = self.tokens.read_coord()?; // control point 1
                let cp2 = self.tokens.read_coord()?; // control point 2
                let end = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));

                cubic_cp2 = Some(cp2);
                let extrema = cubic_extrema(start, cp1, cp2, end);
                for point in extrema {
                    self.update_position(point);
                }
                self.update_position(end);
            }
            'c' => {
                let cp1 = self.tokens.read_coord()?; // control point 1
                let cp2 = self.tokens.read_coord()?; // control point 2
                let (dx, dy) = self.tokens.read_coord()?;
                let (px, py) = self.position.unwrap_or((0., 0.));

                let start = (px, py);
                let end = (px + dx, py + dy);
                let cp1 = (px + cp1.0, py + cp1.1);
                let cp2 = (px + cp2.0, py + cp2.1);

                cubic_cp2 = Some(cp2);
                let extrema = cubic_extrema(start, cp1, cp2, end);
                for point in extrema {
                    self.update_position(point);
                }
                self.update_position(end);
            }
            'S' => {
                // S: "(x2 y2 x y)+"
                let cp2 = self.tokens.read_coord()?;
                let end = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));

                // "The first control point is assumed to be the reflection of the second
                //  control point on the previous command relative to the current point.
                //  If there is no previous command or if the previous command was not an
                //  C, c, S or s, assume the first control point is coincident with the
                //  current point."
                let cp1 = if let Some((prev_cp2x, prev_cp2y)) = self.cubic_cp2 {
                    let (px, py) = start;
                    (2. * px - prev_cp2x, 2. * py - prev_cp2y)
                } else {
                    start
                };

                cubic_cp2 = Some(cp2);
                let extrema = cubic_extrema(start, cp1, cp2, end);
                for e in extrema {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            's' => {
                // s: "(x2 y2 x y)+"
                let cp2 = self.tokens.read_coord()?;
                let (dx, dy) = self.tokens.read_coord()?;
                let (px, py) = self.position.unwrap_or((0., 0.));

                let end = (px + dx, py + dy);
                let start = (px, py);

                // "The first control point is assumed to be the reflection of the second
                //  control point on the previous command relative to the current point.
                //  If there is no previous command or if the previous command was not an
                //  C, c, S or s, assume the first control point is coincident with the
                //  current point."
                let cp1 = if let Some((prev_cp2x, prev_cp2y)) = self.cubic_cp2 {
                    (2. * px - prev_cp2x, 2. * py - prev_cp2y)
                } else {
                    start
                };
                let cp2 = (px + cp2.0, py + cp2.1);

                cubic_cp2 = Some(cp2);
                let extrema = cubic_extrema(start, cp1, cp2, end);
                for e in extrema {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            'Q' => {
                // Q: "(x1 y1 x y)+"
                let cp = self.tokens.read_coord()?;
                let end = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));

                quadratic_cp = Some(cp);
                for e in quadratic_extrema(start, cp, end) {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            'q' => {
                // q: "(x1 y1 x y)+"
                let cp = self.tokens.read_coord()?; // control point
                let (dx, dy) = self.tokens.read_coord()?;
                let (px, py) = self.position.unwrap_or((0., 0.));

                let start = (px, py);
                let end = (px + dx, py + dy);
                let cp = (px + cp.0, py + cp.1);

                quadratic_cp = Some(cp);
                for e in quadratic_extrema(start, cp, end) {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            'T' => {
                // "(x y)+"
                let end = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));
                // "The control point is assumed to be the reflection of the control point
                //  on the previous command relative to the current point. (If there is no
                //  previous command or if the previous command was not a Q, q, T or t,
                //  assume the control point is coincident with the current point.)"
                let cp = if let Some((prev_cpx, prev_cpy)) = self.quadratic_cp {
                    let (px, py) = start;
                    (2. * px - prev_cpx, 2. * py - prev_cpy)
                } else {
                    start
                };

                quadratic_cp = Some(cp);
                for e in quadratic_extrema(start, cp, end) {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            't' => {
                // "(x y)+"
                let (dx, dy) = self.tokens.read_coord()?;
                let start = self.position.unwrap_or((0., 0.));
                let (px, py) = start;
                // "The control point is assumed to be the reflection of the control point
                //  on the previous command relative to the current point. (If there is no
                //  previous command or if the previous command was not a Q, q, T or t,
                //  assume the control point is coincident with the current point.)"
                let cp = if let Some((prev_cpx, prev_cpy)) = self.quadratic_cp {
                    (2. * px - prev_cpx, 2. * py - prev_cpy)
                } else {
                    start
                };

                let end = (px + dx, py + dy);

                quadratic_cp = Some(cp);
                for e in quadratic_extrema(start, cp, end) {
                    self.update_position(e);
                }
                self.update_position(end);
            }
            'A' => {
                // "(rx ry x-axis-rotation large-arc-flag sweep-flag x y)+"
                let _rx = self.tokens.read_non_negative()?;
                let _ry = self.tokens.read_non_negative()?;
                let _xar = self.tokens.read_number()?;
                let _laf = self.tokens.read_flag()?;
                let _sf = self.tokens.read_flag()?;
                let xy = self.tokens.read_coord()?;
                self.update_position(xy);
            }
            'a' => {
                // "(rx ry x-axis-rotation large-arc-flag sweep-flag x y)+"
                let _rx = self.tokens.read_non_negative()?;
                let _ry = self.tokens.read_non_negative()?;
                let _xar = self.tokens.read_number()?;
                let _laf = self.tokens.read_flag()?;
                let _sf = self.tokens.read_flag()?;
                let (dx, dy) = self.tokens.read_coord()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx + dx, cpy + dy));
            }
            _ => Err(Error::InvalidValue(
                "path command".to_string(),
                self.command.unwrap_or_default().to_string(),
            ))?,
        }
        self.cubic_cp2 = cubic_cp2;
        self.quadratic_cp = quadratic_cp;
        Ok(())
    }

    fn evaluate(&mut self) -> Result<()> {
        self.tokens.skip_whitespace();
        while !self.tokens.at_end() {
            self.process_instruction()?;
        }
        Ok(())
    }
}

/// Compute the extremal points of a quadratic Bezier
/// by solving for dx/dt = 0, dy/dt = 0
fn quadratic_extrema(start: (f32, f32), cp: (f32, f32), end: (f32, f32)) -> Vec<(f32, f32)> {
    // Evaluate quadratic Bezier for a single coordinate
    fn bezier(t: f32, p0: f32, p1: f32, p2: f32) -> f32 {
        let mt = 1.0 - t;
        mt * mt * p0 + 2.0 * mt * t * p1 + t * t * p2
    }

    // Compute stationary point t for one dimension,
    // if it lies in (0,1) (the range of t for the curve)
    fn stationary_t(p0: f32, p1: f32, p2: f32) -> Option<f32> {
        // B'(t) = 2(1-t)(p1-p0) + 2t(p2-p1)
        // B'(t) == 0  when  t = (p0-p1) / (p0-2p1+p2)
        let denom = p0 - 2.0 * p1 + p2;
        if denom.abs() < 1e-6_f32 {
            None
        } else {
            let t = (p0 - p1) / denom;
            if t > 0.0 && t < 1.0 {
                Some(t)
            } else {
                None
            }
        }
    }

    [
        stationary_t(start.0, cp.0, end.0),
        stationary_t(start.1, cp.1, end.1),
    ]
    .into_iter()
    .flatten()
    .map(|t| {
        (
            bezier(t, start.0, cp.0, end.0),
            bezier(t, start.1, cp.1, end.1),
        )
    })
    .collect()
}

fn cubic_extrema(
    start: (f32, f32),
    cp1: (f32, f32),
    cp2: (f32, f32),
    end: (f32, f32),
) -> Vec<(f32, f32)> {
    fn cubic(t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
        let mt = 1.0 - t;
        mt * mt * mt * p0 + 3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t * p3
    }

    fn stationary_ts(p0: f32, p1: f32, p2: f32, p3: f32) -> Vec<f32> {
        // Derivative of cubic Bezier: B'(t) = 3(1-t)^2 * (p1-p0) + 6(1-t)t(p2-p1) + 3t^2 * (p3-p2)
        // Rearranging to standard form: at^2 + bt + c = 0
        let a = 3.0 * (p3 - 3.0 * p2 + 3.0 * p1 - p0);
        let b = 6.0 * (p2 - 2.0 * p1 + p0);
        let c = 3.0 * (p1 - p0);

        let mut ts = vec![];
        if a.abs() < 1e-6_f32 {
            // Linear case: bt + c = 0
            if b.abs() >= 1e-6_f32 {
                let t = -c / b;
                if t > 0.0 && t < 1.0 {
                    ts.push(t);
                }
            }
        } else {
            // Quadratic case
            let disc = b * b - 4.0 * a * c;
            if disc >= 0.0 {
                let sqrt_disc = disc.sqrt();
                let inv_2a = 1.0 / (2.0 * a);
                let t1 = (-b + sqrt_disc) * inv_2a;
                let t2 = (-b - sqrt_disc) * inv_2a;

                if t1 > 0.0 && t1 < 1.0 {
                    ts.push(t1);
                }
                if t2 > 0.0 && t2 < 1.0 {
                    ts.push(t2);
                }
            }
        }
        ts
    }

    let mut all_t = Vec::new();
    all_t.extend(stationary_ts(start.0, cp1.0, cp2.0, end.0));
    all_t.extend(stationary_ts(start.1, cp1.1, cp2.1, end.1));

    all_t
        .into_iter()
        .map(|t| {
            (
                cubic(t, start.0, cp1.0, cp2.0, end.0),
                cubic(t, start.1, cp1.1, cp2.1, end.1),
            )
        })
        .collect()
}

pub fn path_bbox(element: &SvgElement) -> Result<Option<BoundingBox>> {
    if let Some(path_data) = element.get_attr("d") {
        let mut pp = PathParser::new(path_data);
        pp.evaluate()?;
        Ok(pp.get_bbox())
    } else {
        Ok(None)
    }
}

pub fn points_to_path(element: &SvgElement) -> Result<SvgElement> {
    let (mut points, max_radius) = if let (Some(r), Some(p)) = (
        element.get_attr("corner-radius"),
        element.get_attr("points"),
    ) {
        let floats: Vec<f32> = attr_split(p).filter_map(|a| strp(&a).ok()).collect();
        // chunks_exact to ignore any unpaired final number
        (
            floats
                .chunks_exact(2)
                .map(|a| (a[0], a[1]))
                .collect::<Vec<_>>(),
            strp(r)?,
        )
    } else {
        return Err(Error::InternalLogic(
            "points_to_path() needs points and corner-radius".to_string(),
        ));
    };

    let polygon = element.name() == "polygon";

    let mut result = vec![];
    if points.is_empty() {
        result.push(String::new());
    } else {
        let mut points_no_dupe = vec![];
        let first_item = points[0];
        for p in 0..points.len() {
            if points[p] != points[(p + 1) % points.len()] || (!polygon && p == points.len() - 1) {
                points_no_dupe.push(points[p]);
            }
        }
        points = points_no_dupe;

        if points.len() <= 1 {
            result.push(format!(
                "M {} {} l 0 0",
                fstr(first_item.0),
                fstr(first_item.1)
            ));
        } else {
            result = points_to_path_render(&points, polygon, max_radius);
        }
    }

    // create new element and copy attrs and replace points with path attr
    let mut new_element = SvgElement::new("path", &[]);
    new_element = new_element.with_attrs_from(element);

    new_element.pop_attr("points");
    new_element.set_attr("d", &result.join(" "));

    Ok(new_element)
}

fn points_to_path_render(points: &[(f32, f32)], polygon: bool, max_radius: f32) -> Vec<String> {
    // radii coresponding to each corner
    // may be smaller than max as if 2 adjacent points are too close
    // then their curves would overlap resulting in obvious error
    // decided to limit to half distance to closest neighbour as simple and often good enough
    let mut radii = vec![];
    for i in 1..(points.len() - 1) {
        //           v current point considering
        // x---------x------x
        //  <--d1---> <-d2->
        let mut d1 = (points[i].0 - points[i - 1].0).hypot(points[i].1 - points[i - 1].1);
        let mut d2 = (points[i + 1].0 - points[i].0).hypot(points[i + 1].1 - points[i].1);

        // if it is not a polygon then end points have no radius so dont need to share
        // so dont half
        if i != 1 || polygon {
            d1 /= 2.0;
        }
        if i != points.len() - 2 || polygon {
            d2 /= 2.0;
        }
        let radius = d1.min(d2).min(max_radius);
        radii.push(radius);
    }

    // inited now because may be changed by polygon condition
    let mut pos = points[0];

    // if polygon need to add 2 more corners for each end point and join them up
    if polygon {
        let last = points.len() - 1;
        // v penultimate v last  v first    v second
        // x-------------x-------x----------x
        //  <-----d1----> <--d2-> <---d3--->

        // d1 d2 and d3 are all halved instantly as this is a polygon so nothing special about any point
        let d1 =
            (points[last].0 - points[last - 1].0).hypot(points[last].1 - points[last - 1].1) / 2.0;
        let d2 = (points[0].0 - points[last].0).hypot(points[0].1 - points[last].1) / 2.0;
        let d3 = (points[1].0 - points[0].0).hypot(points[1].1 - points[0].1) / 2.0;
        let radius = d1.min(d2).min(max_radius);
        radii.push(radius);
        let radius = d2.min(d3).min(max_radius);
        radii.push(radius);

        // move by distance of the radius corresponding to first point along d3
        let dx = points[1].0 - pos.0;
        let dy = points[1].1 - pos.1;

        let len = dx.hypot(dy);

        pos.0 += dx * radii[radii.len() - 1] / len;
        pos.1 += dy * radii[radii.len() - 1] / len;
    }

    let mut result = points_to_path_draw_loop(&mut pos, points, radii);
    if !polygon {
        // move to the last point
        pos = points[points.len() - 1];
        result.push(format!("L {} {}", fstr(pos.0), fstr(pos.1)));
    } else {
        // close polygon as even if in same place may look different
        result.push("z".to_string());
    }

    result
}

fn points_to_path_draw_loop(
    pos: &mut (f32, f32),
    points: &[(f32, f32)],
    radii: Vec<f32>,
) -> Vec<String> {
    let mut result = vec![];

    // move to start
    result.push(format!("M {} {}", fstr(pos.0), fstr(pos.1)));

    for i in 0..radii.len() {
        let p1 = points[(i + 1) % points.len()];
        let p2 = points[(i + 2) % points.len()];

        // 1 is from pos to this point
        // 2 is from this point to next point
        let dx1 = p1.0 - pos.0;
        let dy1 = p1.1 - pos.1;
        let dx2 = p2.0 - p1.0;
        let dy2 = p2.1 - p1.1;

        let len1 = dx1.hypot(dy1);
        let len2 = dx2.hypot(dy2);

        // calculate where curve starts
        pos.0 += dx1 - dx1 * radii[i] / len1;
        pos.1 += dy1 - dy1 * radii[i] / len1;

        // move there
        result.push(format!("L {} {}", fstr(pos.0), fstr(pos.1)));

        let mut new_pos = p1;

        // calculate where curve ends
        new_pos.0 += dx2 * radii[i] / len2;
        new_pos.1 += dy2 * radii[i] / len2;

        // using the dot product normalised
        // negated as one line goes into point
        // and one line goes out
        let cos = -(dx1 * dx2 + dy1 * dy2) as f64 / (len1 * len2) as f64;
        // if cos ~~ -1.0 then it is a straight line the corresponding radius is inf or very large
        // to avoid fp errors if corresponding t value > 2000 it is unlikly to work
        // also need to check for greater than 1 due to more fp errors as that does not make sense
        // greater than 1 still semanticly means straight line so same logic is used
        if (cos + 1.0).abs() <= 0.0001 || cos >= 1.0 {
            // for tidyness
            if *pos != new_pos {
                result.push(format!("L {} {}", fstr(new_pos.0), fstr(new_pos.1)));
            }
        } else {
            // this is using a t formulae
            // t = tan(theta/2)
            // cos(theta) = (1-t^2)/(1+t^2)
            // rearange to get t (valid for 0<=theta<pi)
            let t = ((1.0 - cos) / (1.0 + cos)).sqrt();
            // t scales the radius to get used radius
            // draw a kite with 2 corners being right-angles
            // 2 sides are equal to this radius = r
            // the 2 other sides are used radius = a
            // split the kite along diagonal so 2 right-angled triangles
            // it can be seen a = r*t if theta = angle between the 2 rs

            // whether it is going clockwise
            // calculated by taking dotproduct of d1 and (d2 rotated 90deg)
            // equivalent to '2d' cross product
            // the sign of the answer is which way it goes
            let cl = (dx1 * dy2 - dy1 * dx2) > 0.0;

            // the first 0 is rotation and could be any float parsable value
            // the second 0 is whether to do a large arc but for this we never do
            if radii[i] != 0.0 {
                result.push(format!(
                    "a {} {} 0 0 {} {} {}",
                    fstr(radii[i] * t as f32), // radius x
                    fstr(radii[i] * t as f32), // radius y
                    cl as u32,                 // clockwise
                    fstr(new_pos.0 - pos.0),   // dx
                    fstr(new_pos.1 - pos.1),   // dy
                ));
            }
        }

        *pos = new_pos;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ps_number() {
        let mut ps = SvgPathSyntax::new("123 4.5  -9.25");
        ps.skip_whitespace();
        assert_eq!(ps.read_number().unwrap(), 123.);
        ps.skip_whitespace();
        assert_eq!(ps.read_number().unwrap(), 4.5);
        ps.skip_whitespace();
        assert_eq!(ps.read_number().unwrap(), -9.25);

        // should read as little as needed to allow valid parsing,
        // so numbers can be squished together providing the result
        // is unambiguous. See https://www.w3.org/TR/SVG11/paths.html#PathDataBNF
        let mut ps = SvgPathSyntax::new("123-4.5.25+5");
        assert_eq!(ps.read_number().unwrap(), 123.);
        assert_eq!(ps.read_number().unwrap(), -4.5);
        assert_eq!(ps.read_number().unwrap(), 0.25);
        assert_eq!(ps.read_number().unwrap(), 5.);

        // should support exponents
        let mut ps = SvgPathSyntax::new("1e3 -2E-2 +3.5e+2");
        assert_eq!(ps.read_number().unwrap(), 1e3);
        assert_eq!(ps.read_number().unwrap(), -2e-2);
        assert_eq!(ps.read_number().unwrap(), 3.5e+2);
        // ... and without spaces; '1e3.5' is '1e3' followed by '.5'
        let mut ps = SvgPathSyntax::new("1e3.5-2E-2+3.5e+2");
        assert_eq!(ps.read_number().unwrap(), 1e3);
        assert_eq!(ps.read_number().unwrap(), 0.5);
        assert_eq!(ps.read_number().unwrap(), -2e-2);
        assert_eq!(ps.read_number().unwrap(), 3.5e+2);
    }

    #[test]
    fn test_ps_flag() {
        let mut ps = SvgPathSyntax::new("0 1,1 0");
        assert_eq!(ps.read_flag().unwrap(), 0);
        assert_eq!(ps.read_flag().unwrap(), 1);
        assert_eq!(ps.read_flag().unwrap(), 1);
        assert_eq!(ps.read_flag().unwrap(), 0);

        // whitespace is not required around flags
        let mut ps = SvgPathSyntax::new("01");
        assert_eq!(ps.read_flag().unwrap(), 0);
        assert_eq!(ps.read_flag().unwrap(), 1);

        // only '0' and '1' are valid flags
        let mut ps = SvgPathSyntax::new("2");
        assert!(ps.read_flag().is_err());
        let mut ps = SvgPathSyntax::new("1.0");
        // will read the '1' as a flag, leaving '.0'
        assert_eq!(ps.read_flag().unwrap(), 1);
        assert!(ps.read_flag().is_err());
    }

    #[test]
    fn test_ps_coord() {
        let mut ps = SvgPathSyntax::new("123 456");
        assert_eq!(ps.read_coord().unwrap(), (123., 456.));

        let mut ps = SvgPathSyntax::new("123,456");
        assert_eq!(ps.read_coord().unwrap(), (123., 456.));

        let mut ps = SvgPathSyntax::new("123 ,   456");
        assert_eq!(ps.read_coord().unwrap(), (123., 456.));

        // Example from https://www.w3.org/TR/SVG11/paths.html#PathDataBNF
        // 'for the string "M 0.6.5" … the first coordinate will be "0.6" and
        // the second coordinate will be ".5".'
        let mut ps = SvgPathSyntax::new("0.6.5");
        assert_eq!(ps.read_coord().unwrap(), (0.6, 0.5));
    }

    #[test]
    fn test_pp_move() {
        let mut pp = PathParser::new("M10 20");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 20.)));

        // if the first command is 'm' (relative moveto) it is treated
        // as an absolute moveto.
        let mut pp = PathParser::new("m10 20");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 20.)));

        // There can be multiple coordinates, in which case subsequent ones
        // are implicit 'line-to' coordinates
        let mut pp = PathParser::new("M10 20 100 200");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((100., 200.)));
        assert!(pp.tokens.at_end());

        // There can be multiple coordinates, in which case subsequent ones
        // are implicit 'line-to' coordinates
        let mut pp = PathParser::new("m10 20 100 200");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((110., 220.)));
        assert!(pp.tokens.at_end());

        // Example from spec - grammar section.
        let mut pp = PathParser::new("M 0.6.5");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0.6, 0.5)));
        assert!(pp.tokens.at_end());

        //
        // Same again as above, but with lineto (L / l) this time.
        //
        let mut pp = PathParser::new("L10 20");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 20.)));

        // if the first command is 'm' (relative moveto) it is treated
        // as an absolute moveto.
        let mut pp = PathParser::new("l10 20");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 20.)));

        // There can be multiple coordinates, in which case subsequent ones
        // are implicit 'line-to' coordinates
        let mut pp = PathParser::new("L10 20 100 200");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((100., 200.)));
        assert!(pp.tokens.at_end());

        // There can be multiple coordinates, in which case subsequent ones
        // are implicit 'line-to' coordinates
        let mut pp = PathParser::new("l10 20 100 200");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((110., 220.)));
        assert!(pp.tokens.at_end());

        //
        // Horizontal lines
        //
        let mut pp = PathParser::new("H 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 0.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("H 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((30., 0.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("h 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 0.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("h 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((120., 0.)));
        assert!(pp.tokens.at_end());

        //
        // Vertical lines
        //
        let mut pp = PathParser::new("V 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 10.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("V 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 30.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("v 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 10.)));
        assert!(pp.tokens.at_end());

        let mut pp = PathParser::new("v 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 120.)));
        assert!(pp.tokens.at_end());
    }

    #[test]
    fn test_pp_bbox() {
        let mut pp = PathParser::new("M10 20 100 200 200 150");
        pp.evaluate().unwrap();
        assert_eq!(pp.get_bbox(), Some(BoundingBox::new(10., 20., 200., 200.)));

        let mut pp = PathParser::new("M10 20 M100 200 M200 150");
        pp.evaluate().unwrap();
        assert_eq!(pp.get_bbox(), Some(BoundingBox::new(10., 20., 200., 200.)));

        let mut pp = PathParser::new("M10 20 m100 200 m-1000 150");
        pp.evaluate().unwrap();
        assert_eq!(
            pp.get_bbox(),
            Some(BoundingBox::new(-890., 20., 110., 370.))
        );

        let mut pp = PathParser::new("M10 30 Q20 39 30 30 T 50 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.get_bbox(), Some(BoundingBox::new(10., 30., 50., 30.)));

        // Quadratic bezier
        let mut pp = PathParser::new("M 10 30 Q 20 40 30 30 T 50 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.get_bbox(), Some(BoundingBox::new(10., 30., 50., 30.)));
    }

    #[test]
    fn test_points_to_path_squiggle() {
        let squiggle = &[(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (10.0, 5.0)];

        for (cr, expected) in [
            (
                0.0,
                vec![
                    "M 0 0".to_string(),
                    "L 5 0".to_string(),
                    "L 5 5".to_string(),
                    "L 10 5".to_string(),
                ],
            ),
            (
                1.0,
                vec![
                    "M 0 0".to_string(),
                    "L 4 0".to_string(),
                    "a 1 1 0 0 1 1 1".to_string(),
                    "L 5 4".to_string(),
                    "a 1 1 0 0 0 1 1".to_string(),
                    "L 10 5".to_string(),
                ],
            ),
            (
                5.0,
                vec![
                    "M 0 0".to_string(),
                    "L 2.5 0".to_string(),
                    "a 2.5 2.5 0 0 1 2.5 2.5".to_string(),
                    "L 5 2.5".to_string(),
                    "a 2.5 2.5 0 0 0 2.5 2.5".to_string(),
                    "L 10 5".to_string(),
                ],
            ),
            (
                10.0,
                vec![
                    "M 0 0".to_string(),
                    "L 2.5 0".to_string(),
                    "a 2.5 2.5 0 0 1 2.5 2.5".to_string(),
                    "L 5 2.5".to_string(),
                    "a 2.5 2.5 0 0 0 2.5 2.5".to_string(),
                    "L 10 5".to_string(),
                ],
            ),
        ] {
            let v = points_to_path_render(squiggle, false, cr);
            assert_eq!(v, expected);
        }
    }

    #[test]
    fn test_points_to_path_acute() {
        let acute = &[(0.0, 0.0), (5.0, 0.0), (0.0, 5.0)];
        let v = points_to_path_render(acute, false, 0.0);
        assert_eq!(
            v,
            [
                "M 0 0".to_string(),
                "L 5 0".to_string(),
                "L 0 5".to_string(),
            ]
        );
        let v = points_to_path_render(acute, false, 2.0);
        assert_eq!(
            v,
            [
                "M 0 0".to_string(),
                "L 3 0".to_string(),
                "a 0.828 0.828 0 0 1 0.586 1.414".to_string(),
                "L 0 5".to_string(),
            ]
        );
    }

    #[test]
    fn test_points_to_path_obtuse() {
        let obtuse = &[(0.0, 0.0), (5.0, 0.0), (10.0, 5.0)];
        let v = points_to_path_render(obtuse, false, 0.0);
        assert_eq!(
            v,
            [
                "M 0 0".to_string(),
                "L 5 0".to_string(),
                "L 10 5".to_string(),
            ]
        );
        let v = points_to_path_render(obtuse, false, 2.0);
        assert_eq!(
            v,
            [
                "M 0 0".to_string(),
                "L 3 0".to_string(),
                "a 4.828 4.828 0 0 1 3.414 1.414".to_string(),
                "L 10 5".to_string(),
            ]
        );
    }

    #[test]
    fn test_points_to_path_polygon() {
        let square = &[(0.0, 0.0), (7.0, 0.0), (7.0, 7.0), (0.0, 7.0)];
        let v = points_to_path_render(square, true, 0.0);
        assert_eq!(
            v,
            [
                "M 0 0".to_string(),
                "L 7 0".to_string(),
                "L 7 7".to_string(),
                "L 0 7".to_string(),
                "L 0 0".to_string(),
                "z".to_string(),
            ]
        );
        let v = points_to_path_render(square, true, 1.0);
        assert_eq!(
            v,
            [
                "M 1 0".to_string(),
                "L 6 0".to_string(),
                "a 1 1 0 0 1 1 1".to_string(),
                "L 7 6".to_string(),
                "a 1 1 0 0 1 -1 1".to_string(),
                "L 1 7".to_string(),
                "a 1 1 0 0 1 -1 -1".to_string(),
                "L 0 1".to_string(),
                "a 1 1 0 0 1 1 -1".to_string(),
                "z".to_string(),
            ]
        );
    }
}
