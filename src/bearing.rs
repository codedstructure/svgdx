//! Support for 'bearing' commands in SVG paths.
//!
//! See https://www.w3.org/TR/2016/CR-SVG2-20160915/paths.html#PathDataBearingCommands
//! for details.
//!
//! The bearing commands have sadly been removed from SVG2 due to lack of implementations,
//! but svgdx brings them back to life by converting them into standard SVG path commands.
//!
//! There are two new path commands to set the bearing:
//! - `B` sets the absolute bearing in degrees.
//! - `b` is the relative version, adjusting the bearing by the given amount in degrees.
//!
//! The current bearing is initially 0 degrees (positive x-axis) and is used to adjust
//! the direction of relative `l`, `m`, `h`, and `v` commands, where the 'x' coordinate is
//! aligned with the bearing direction and the 'y' coordinate is perpendicular to it.

use crate::errors::{Result, SvgdxError};
use crate::types::fstr;

struct PathBearing {
    data: Vec<char>,
    output: String,
    index: usize,
    bearing: f32,
    command: Option<char>,
}

impl PathBearing {
    fn new(data: &str) -> Self {
        PathBearing {
            data: data.chars().collect(),
            output: String::new(),
            index: 0,
            bearing: 0.,
            command: None,
        }
    }

    fn at_end(&self) -> bool {
        self.index >= self.data.len()
    }

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
        while !self.at_end() && self.data[self.index].is_ascii_whitespace() {
            self.index += 1;
        }
    }

    fn skip_wsp_comma(&mut self) {
        self.skip_whitespace();
        if !self.at_end() && self.data[self.index] == ',' {
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

    fn at_command(&self) -> Result<bool> {
        self.check_not_end()?;
        Ok("MmBbLlHhVvZzCcSsQqTtAa".contains(self.data[self.index]))
    }

    fn read_command(&mut self) -> Result<char> {
        if self.at_command()? {
            let command = self.data[self.index];
            self.index += 1;
            self.skip_wsp_comma();
            Ok(command)
        } else {
            Err(SvgdxError::InvalidData("Invalid path command".to_string()))
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

        let cmd = self.command.expect("Command should be already set");
        match cmd {
            'B' => {
                // Bearing command
                let bearing = self.read_number()?;
                self.bearing = bearing;
            }
            'b' => {
                // Relative bearing command
                let bearing = self.read_number()?;
                self.bearing += bearing;
            }
            'm' | 'l' if self.bearing != 0. => {
                let (dx, dy) = self.read_coord()?;
                let cosb = self.bearing.to_radians().cos();
                let sinb = self.bearing.to_radians().sin();
                // "When a relative l command is used, the end point of the line is
                // (cpx + x cos cb + y sin cb, cpy + x sin cb + y cos cb)."
                let bdx = fstr(dx * cosb + dy * sinb);
                let bdy = fstr(dx * sinb + dy * cosb);
                // emit same command with new native coordinates
                self.output.push(cmd);
                self.output.push_str(&format!("{bdx} {bdy}"));
            }
            'h' | 'v' if self.bearing != 0. => {
                let offset = self.read_number()?;
                // "When a relative h command is used, the end point of the line is (cpx + x cos cb, cpy + x sin cb)."
                // "When a relative v command is used, the end point of the line is (cpx + y sin cb, cpy + y cos cb)."
                let bdx = fstr(offset * self.bearing.to_radians().cos());
                let bdy = fstr(offset * self.bearing.to_radians().sin());
                // convert to l command in native coordinates
                self.output.push('l');
                if cmd == 'h' {
                    self.output.push_str(&format!("{bdx} {bdy}"));
                } else {
                    self.output.push_str(&format!("{bdy} {bdx}"));
                }
            }
            _ => {
                // copy to output
                self.output.push(cmd);
                while !self.at_end() {
                    if self.at_command()? {
                        break;
                    }
                    self.output.push(self.data[self.index]);
                    self.index += 1;
                }
            }
        }
        Ok(())
    }

    fn evaluate(&mut self) -> Result<&String> {
        self.skip_whitespace();
        while !self.at_end() {
            self.process_instruction()?;
        }
        Ok(&self.output)
    }
}

pub fn process_path_bearing(data: &str) -> Result<String> {
    let mut pp = PathBearing::new(data);
    pp.evaluate()?;
    Ok(pp.output)
}

#[cfg(test)]
mod tests {
    use assertables::{assert_contains, assert_not_contains};

    use super::*;

    #[test]
    fn test_path_bearing() {
        let mut pp = PathBearing::new("M0 0");
        pp.evaluate().unwrap();
        assert_eq!(pp.bearing, 0.);

        let mut pp = PathBearing::new("M0 0b-90");
        pp.evaluate().unwrap();
        assert_eq!(pp.bearing, -90.);

        let mut pp = PathBearing::new("M0 0b60b30");
        pp.evaluate().unwrap();
        assert_eq!(pp.bearing, 90.);

        let mut pp = PathBearing::new("M0 0B123");
        pp.evaluate().unwrap();
        assert_eq!(pp.bearing, 123.);
    }

    #[test]
    fn test_path_bearing_hv() {
        let input = "M0 0B90h10";
        assert_eq!(process_path_bearing(input).unwrap(), "M0 0l0 10");

        let input = "M0 0 h10 b90 h10 b90 h10 b90 h10";
        assert_eq!(process_path_bearing(input).unwrap(), "M0 0 h10 l0 10l-10 0l0 -10");
    }

    #[test]
    fn test_path_bearing_line() {
        let input = "M0 0B90l10 0";
        assert_eq!(process_path_bearing(input).unwrap(), "M0 0l0 10");

        let input = "M0 0 B45 l 10 0";
        assert_eq!(process_path_bearing(input).unwrap(), "M0 0 l7.071 7.071");
    }

    #[test]
    fn test_path_bearing_spec_example() {
        // Check the example from the (obsolete) SVG2 spec version can be processed.
        let input = r#"M 150,10
           B 36 h 47
           b 72 h 47
           b 72 h 47
           b 72 h 47 z"#;
        let output = process_path_bearing(input).unwrap();
        assert_not_contains!(output, "B");
        assert_not_contains!(output, "b");
        assert_contains!(output, "l");
    }
}
