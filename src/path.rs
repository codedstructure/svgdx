use crate::element::SvgElement;
use crate::position::BoundingBox;
use anyhow::{bail, Context, Result};

struct PathParser {
    data: Vec<char>,
    index: usize,
    position: Option<(f32, f32)>,
    start_pos: Option<(f32, f32)>,
    command: Option<char>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl PathParser {
    fn new(data: &str) -> Self {
        PathParser {
            data: data.chars().collect(),
            index: 0,
            position: None,
            start_pos: None,
            command: None,
            min_x: 0.,
            min_y: 0.,
            max_x: 0.,
            max_y: 0.,
        }
    }

    fn at_end(&self) -> bool {
        self.index >= self.data.len()
    }

    fn check_not_end(&self) -> Result<()> {
        if self.at_end() {
            bail!("Ran out of data!");
        }
        Ok(())
    }

    fn skip_whitespace(&mut self) {
        // SVG definition of whitespace is 0x20, 0x9, 0xA, 0xD. Rust's is_ascii_whitespace()
        // also includes 0xC, but is close enough and convenient.
        while self.index < self.data.len() && self.data[self.index].is_ascii_whitespace() {
            self.index += 1;
        }
    }

    fn skip_wsp_comma(&mut self) {
        self.skip_whitespace();
        if self.index < self.data.len() && self.data[self.index] == ',' {
            self.index += 1;
            self.skip_whitespace();
        }
    }

    fn read_number(&mut self) -> Result<f32> {
        self.check_not_end()?;
        let mut s = String::new();
        while let Some(&ch) = self.data.get(self.index) {
            if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                s.push(ch);
                self.index += 1;
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
        self.check_not_end()?;
        if "MmLlHhVvZzCcSsQqTtAa".contains(self.data[self.index]) {
            let command = self.data[self.index];
            self.index += 1;
            self.skip_wsp_comma();
            Ok(command)
        } else {
            bail!("Invalid path command")
        }
    }

    fn maybe_command(&mut self) -> Option<char> {
        let orig_idx = self.index;
        let command = self.read_command();
        if command.is_err() {
            self.index = orig_idx;
            None
        } else {
            command.ok()
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
        if self.command.is_none() {
            self.command = Some(self.read_command()?);
        } else if let Some(command) = self.maybe_command() {
            // "The command letter can be eliminated on subsequent commands if the same
            // command is used multiple times in a row (e.g., you can drop the second
            // "L" in "M 100 200 L 200 100 L -100 -200" and use "M 100 200 L 200 100
            // -100 -200" instead)."
            self.command = Some(command);
        }

        match self.command.expect("Command should be already set") {
            'M' | 'L' | 'T' => {
                // "(x y)+"
                let xy = self.read_coord()?;
                self.update_position(xy);
            }
            'm' | 'l' | 't' => {
                let (dx, dy) = self.read_coord()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx + dx, cpy + dy));
            }
            'H' => {
                let new_x = self.read_number()?;
                let (_, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((new_x, cpy));
            }
            'h' => {
                let dx = self.read_number()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx + dx, cpy));
            }
            'V' => {
                let new_y = self.read_number()?;
                let (cpx, _) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx, new_y));
            }
            'v' => {
                let dy = self.read_number()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx, cpy + dy));
            }
            'Z' | 'z' => {
                self.update_position(
                    self.start_pos
                        .context("Cannot 'z' without start position")?,
                );
            }
            'C' => {
                let _cp1 = self.read_coord()?; // control point 1
                let _cp2 = self.read_coord()?; // control point 2
                let xy = self.read_coord()?;
                self.update_position(xy);
            }
            'c' => {
                let _cp1 = self.read_coord()?; // control point 1
                let _cp2 = self.read_coord()?; // control point 2
                let (dx, dy) = self.read_coord()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx + dx, cpy + dy));
            }
            'S' | 'Q' => {
                // S: "(x2 y2 x y)+"
                // Q: "(x1 y1 x y)+"
                // Both 'S' and 'Q' have the same format - a single control point followed by the
                // target point - so deal with both together. They are semantically different though...
                let _cp = self.read_coord()?; // control point
                let xy = self.read_coord()?;
                self.update_position(xy);
            }
            's' | 'q' => {
                // s: "(x2 y2 x y)+"
                // q: "(x1 y1 x y)+"
                let _cp = self.read_coord()?; // control point
                let (dx, dy) = self.read_coord()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx + dx, cpy + dy));
            }
            'A' => {
                // "(rx ry x-axis-rotation large-arc-flag sweep-flag x y)+"
                let _rxy = self.read_coord()?;
                let _xar = self.read_number()?;
                let _laf = self.read_number()?;
                let _sf = self.read_number()?;
                let xy = self.read_coord()?;
                self.update_position(xy);
            }
            'a' => {
                // "(rx ry x-axis-rotation large-arc-flag sweep-flag x y)+"
                let _rxy = self.read_coord()?;
                let _xar = self.read_number()?;
                let _laf = self.read_number()?;
                let _sf = self.read_number()?;
                let (dx, dy) = self.read_coord()?;
                let (cpx, cpy) = self.position.unwrap_or((0., 0.));
                self.update_position((cpx + dx, cpy + dy));
            }
            _ => bail!("Unknown path data instruction"),
        }
        Ok(())
    }

    fn evaluate(&mut self) -> Result<()> {
        self.skip_whitespace();
        while !self.at_end() {
            self.process_instruction()?;
        }
        Ok(())
    }
}

pub fn path_bbox(element: &SvgElement) -> Result<BoundingBox> {
    let path_data = element
        .get_attr("d")
        .context("path element should have 'd' attribute")?;

    let mut pp = PathParser::new(&path_data);
    pp.evaluate()?;
    pp.get_bbox()
        .context("Path element should have a computable boundingbox")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pp_number() {
        let mut pp = PathParser::new("123 4.5  -9.25");
        pp.skip_whitespace();
        assert_eq!(pp.read_number().unwrap(), 123.);
        pp.skip_whitespace();
        assert_eq!(pp.read_number().unwrap(), 4.5);
        pp.skip_whitespace();
        assert_eq!(pp.read_number().unwrap(), -9.25);
    }

    #[test]
    fn test_pp_coord() {
        let mut pp = PathParser::new("123 456");
        assert_eq!(pp.read_coord().unwrap(), (123., 456.));

        let mut pp = PathParser::new("123,456");
        assert_eq!(pp.read_coord().unwrap(), (123., 456.));

        let mut pp = PathParser::new("123 ,   456");
        assert_eq!(pp.read_coord().unwrap(), (123., 456.));
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
        assert!(pp.at_end());

        // There can be multiple coordinates, in which case subsequent ones
        // are implicit 'line-to' coordinates
        let mut pp = PathParser::new("m10 20 100 200");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((110., 220.)));
        assert!(pp.at_end());

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
        assert!(pp.at_end());

        // There can be multiple coordinates, in which case subsequent ones
        // are implicit 'line-to' coordinates
        let mut pp = PathParser::new("l10 20 100 200");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((110., 220.)));
        assert!(pp.at_end());

        //
        // Horizontal lines
        //
        let mut pp = PathParser::new("H 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 0.)));
        assert!(pp.at_end());

        let mut pp = PathParser::new("H 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((30., 0.)));
        assert!(pp.at_end());

        let mut pp = PathParser::new("h 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((10., 0.)));
        assert!(pp.at_end());

        let mut pp = PathParser::new("h 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((120., 0.)));
        assert!(pp.at_end());

        //
        // Vertical lines
        //
        let mut pp = PathParser::new("V 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 10.)));
        assert!(pp.at_end());

        let mut pp = PathParser::new("V 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 30.)));
        assert!(pp.at_end());

        let mut pp = PathParser::new("v 10");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 10.)));
        assert!(pp.at_end());

        let mut pp = PathParser::new("v 10 80 30");
        pp.evaluate().unwrap();
        assert_eq!(pp.position, Some((0., 120.)));
        assert!(pp.at_end());
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
}
