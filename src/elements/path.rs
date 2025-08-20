use super::SvgElement;
use crate::errors::{Result, SvgdxError};
use crate::geometry::BoundingBox;
use crate::types::{attr_split, fstr, strp};

struct PathParser {
    tokens: SvgPathSyntax,
    position: Option<(f32, f32)>,
    start_pos: Option<(f32, f32)>,
    command: Option<char>,
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
            .ok_or(SvgdxError::ParseError("No data".to_string()))?;
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
            Err(SvgdxError::ParseError("Ran out of data!".to_string()))
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

    fn read_number(&mut self) -> Result<f32> {
        self.check_not_end()?;
        let mut s = String::new();
        while let Some(ch) = self.current() {
            if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                s.push(ch);
                self.advance();
            } else {
                break;
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
            Err(SvgdxError::InvalidData("Invalid path command".to_string()))
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

        match self.command.expect("Command should be already set") {
            'M' | 'L' | 'T' => {
                // "(x y)+"
                let xy = self.tokens.read_coord()?;
                self.update_position(xy);
            }
            'm' | 'l' | 't' => {
                let (dx, dy) = self.tokens.read_coord()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx + dx, cpy + dy));
            }
            'H' => {
                let new_x = self.tokens.read_number()?;
                let (_, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((new_x, cpy));
            }
            'h' => {
                let dx = self.tokens.read_number()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx + dx, cpy));
            }
            'V' => {
                let new_y = self.tokens.read_number()?;
                let (cpx, _) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx, new_y));
            }
            'v' => {
                let dy = self.tokens.read_number()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx, cpy + dy));
            }
            'Z' | 'z' => {
                self.update_position(self.start_pos.ok_or_else(|| {
                    SvgdxError::InvalidData("Cannot 'z' without start position".to_owned())
                })?);
            }
            'C' => {
                let _cp1 = self.tokens.read_coord()?; // control point 1
                let _cp2 = self.tokens.read_coord()?; // control point 2
                let xy = self.tokens.read_coord()?;
                self.update_position(xy);
            }
            'c' => {
                let _cp1 = self.tokens.read_coord()?; // control point 1
                let _cp2 = self.tokens.read_coord()?; // control point 2
                let (dx, dy) = self.tokens.read_coord()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx + dx, cpy + dy));
            }
            'S' | 'Q' => {
                // S: "(x2 y2 x y)+"
                // Q: "(x1 y1 x y)+"
                // Both 'S' and 'Q' have the same format - a single control point followed by the
                // target point - so deal with both together. They are semantically different though...
                let _cp = self.tokens.read_coord()?; // control point
                let xy = self.tokens.read_coord()?;
                self.update_position(xy);
            }
            's' | 'q' => {
                // s: "(x2 y2 x y)+"
                // q: "(x1 y1 x y)+"
                let _cp = self.tokens.read_coord()?; // control point
                let (dx, dy) = self.tokens.read_coord()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx + dx, cpy + dy));
            }
            'A' => {
                // "(rx ry x-axis-rotation large-arc-flag sweep-flag x y)+"
                let _rxy = self.tokens.read_coord()?;
                let _xar = self.tokens.read_number()?;
                let _laf = self.tokens.read_number()?;
                let _sf = self.tokens.read_number()?;
                let xy = self.tokens.read_coord()?;
                self.update_position(xy);
            }
            'a' => {
                // "(rx ry x-axis-rotation large-arc-flag sweep-flag x y)+"
                let _rxy = self.tokens.read_coord()?;
                let _xar = self.tokens.read_number()?;
                let _laf = self.tokens.read_number()?;
                let _sf = self.tokens.read_number()?;
                let (dx, dy) = self.tokens.read_coord()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx + dx, cpy + dy));
            }
            _ => Err(SvgdxError::InvalidData(
                "Unknown path data instruction".to_string(),
            ))?,
        }
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

pub fn path_bbox(element: &SvgElement) -> Result<Option<BoundingBox>> {
    if let Some(path_data) = element.get_attr("d") {
        let mut pp = PathParser::new(path_data);
        pp.evaluate()?;
        Ok(pp.get_bbox())
    } else {
        Ok(None)
    }
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

        let l1 = dx1.hypot(dy1);
        let l2 = dx2.hypot(dy2);

        // calculate where curve starts
        pos.0 += dx1 - dx1 * radii[i] / l1;
        pos.1 += dy1 - dy1 * radii[i] / l1;

        // move there
        result.push(format!("L {} {}", fstr(pos.0), fstr(pos.1)));

        let mut new_pos = p1;

        // calculate where curve ends
        new_pos.0 += dx2 * radii[i] / l2;
        new_pos.1 += dy2 * radii[i] / l2;

        // using the dot product normalised
        // negated as one line goes into point
        // and one line goes out
        let cos = -(dx1 * dx2 + dy1 * dy2) as f64 / (l1 * l2) as f64;
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

pub fn points_to_path(element: &SvgElement) -> Result<SvgElement> {
    let mut points: Vec<(f32, f32)>;
    let max_radius;
    if let (Some(r), Some(p)) = (
        element.get_attr("corner-radius"),
        element.get_attr("points"),
    ) {
        if let Ok(r) = strp(r) {
            max_radius = r;
            let floats: Vec<f32> = attr_split(p).filter_map(|a| strp(&a).ok()).collect();
            if floats.len() % 2 == 0 {
                points = floats.chunks(2).map(|a| (a[0], a[1])).collect();
            } else {
                return Err(SvgdxError::ParseError(
                    "odd number of values in points".to_string(),
                ));
            }
        } else {
            return Err(SvgdxError::ParseError(
                "corner radius is not a float".to_string(),
            ));
        }
    } else {
        return Err(SvgdxError::InternalLogicError(
            "calling points to path without checking if has points and corner-radius".to_string(),
        ));
    }

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
    }

    #[test]
    fn test_ps_coord() {
        let mut ps = SvgPathSyntax::new("123 456");
        assert_eq!(ps.read_coord().unwrap(), (123., 456.));

        let mut ps = SvgPathSyntax::new("123,456");
        assert_eq!(ps.read_coord().unwrap(), (123., 456.));

        let mut ps = SvgPathSyntax::new("123 ,   456");
        assert_eq!(ps.read_coord().unwrap(), (123., 456.));
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
