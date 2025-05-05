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

use super::path::PathSyntax;
use crate::errors::{Result, SvgdxError};
use crate::types::fstr;

pub struct BearingPathSyntax {
    data: Vec<char>,
    index: usize,
}

impl BearingPathSyntax {
    pub fn new(data: &str) -> Self {
        Self {
            data: data.chars().collect(),
            index: 0,
        }
    }
}

impl PathSyntax for BearingPathSyntax {
    fn at_command(&self) -> Result<bool> {
        self.check_not_end()?;
        let c = self
            .current()
            .ok_or(SvgdxError::ParseError("No data".to_string()))?;
        // Adds 'B' and 'b' to the set of SVG commands.
        Ok("MmBbLlHhVvZzCcSsQqTtAa".contains(c))
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

struct PathBearing {
    tokens: BearingPathSyntax,
    output: String,
    bearing: f32,
    command: Option<char>,
}

impl PathBearing {
    fn new(data: &str) -> Self {
        PathBearing {
            tokens: BearingPathSyntax::new(data),
            output: String::new(),
            bearing: 0.,
            command: None,
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

        let cmd = self.command.expect("Command should be already set");
        match cmd {
            'B' => {
                // Bearing command
                let bearing = self.tokens.read_number()?;
                self.bearing = bearing;
            }
            'b' => {
                // Relative bearing command
                let bearing = self.tokens.read_number()?;
                self.bearing += bearing;
            }
            'm' | 'l' if self.bearing != 0. => {
                let (dx, dy) = self.tokens.read_coord()?;
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
                let offset = self.tokens.read_number()?;
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
                while !self.tokens.at_end() {
                    if self.tokens.at_command()? {
                        break;
                    }
                    self.output.push(self.tokens.current().unwrap());
                    self.tokens.advance();
                }
            }
        }
        Ok(())
    }

    fn evaluate(&mut self) -> Result<&String> {
        self.tokens.skip_whitespace();
        while !self.tokens.at_end() {
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
        assert_eq!(
            process_path_bearing(input).unwrap(),
            "M0 0 h10 l0 10l-10 0l0 -10"
        );
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
