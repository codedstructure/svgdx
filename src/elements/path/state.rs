use super::Vec2;
use super::syntax::{PathSyntax, SvgPathSyntax};
use crate::errors::Result;
use crate::geometry::BoundingBox;

#[derive(Clone, Copy)]
pub(super) struct PathState {
    // current position, updated as commands are processed
    position: Option<Vec2>,
    // location to return to for 'Z'/'z' commands
    subpath_start: Option<Vec2>,
    // current command being processed; most commands take multiple parameter
    // sets without repeating the command character
    command: Option<char>,
    // previous second control point (if any) for evaluating 'S' and 's'
    previous_cubic_cp2: Option<Vec2>,
    // previous control point (if any) for evaluating 'T' and 't'
    previous_quadratic_cp: Option<Vec2>,
    // active bearing for relative m/l/h/v commands; None == 0 degrees
    bearing: Option<f32>,
    // distance along the path so far, for line-offset
    elapsed_distance: f32,
    // extrema, updated as path is processed
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl PathState {
    pub fn new() -> Self {
        Self {
            position: None,
            subpath_start: None,
            command: None,
            previous_cubic_cp2: None,
            previous_quadratic_cp: None,
            bearing: None,
            elapsed_distance: 0.,
            min_x: 0.,
            min_y: 0.,
            max_x: 0.,
            max_y: 0.,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn current_position(&self) -> Vec2 {
        self.position.unwrap_or_default()
    }

    pub fn position(&self) -> Option<Vec2> {
        self.position
    }

    pub fn subpath_start(&self) -> Option<Vec2> {
        self.subpath_start
    }

    pub fn previous_cubic_cp2(&self) -> Option<Vec2> {
        self.previous_cubic_cp2
    }

    pub fn previous_quadratic_cp(&self) -> Option<Vec2> {
        self.previous_quadratic_cp
    }

    pub fn bearing(&self) -> Option<f32> {
        self.bearing
    }

    pub fn set_bearing(&mut self, bearing: f32) {
        self.bearing = Some(bearing);
    }

    pub fn apply_bearing(&self, delta: Vec2) -> Vec2 {
        if let Some(bearing) = self.bearing
            && bearing != 0.
        {
            let (sinb, cosb) = bearing.to_radians().sin_cos();
            return Vec2::new(
                delta.x * cosb + delta.y * sinb,
                delta.x * sinb + delta.y * cosb,
            );
        }
        delta
    }

    pub fn set_previous_control_points(
        &mut self,
        cubic_cp2: Option<Vec2>,
        quadratic_cp: Option<Vec2>,
    ) {
        self.previous_cubic_cp2 = cubic_cp2;
        self.previous_quadratic_cp = quadratic_cp;
    }

    fn extend_extrema(&mut self, pos: Vec2) {
        let (x, y) = (pos.x, pos.y);
        if self.position.is_none() {
            self.min_x = x;
            self.min_y = y;
            self.max_x = x;
            self.max_y = y;
        } else {
            self.min_x = self.min_x.min(x);
            self.min_y = self.min_y.min(y);
            self.max_x = self.max_x.max(x);
            self.max_y = self.max_y.max(y);
        }
    }

    pub fn extend_subpath(&mut self, pos: Vec2) {
        self.extend_extrema(pos);

        let old = self.position.unwrap_or(pos);
        self.elapsed_distance += pos.distance(old);

        self.position = Some(pos);
    }

    pub fn extend_curve<I>(&mut self, end: Vec2, extrema: I, length: f32)
    where
        I: IntoIterator<Item = Vec2>,
    {
        for point in extrema {
            self.extend_extrema(point);
        }
        self.extend_extrema(end);
        self.elapsed_distance += length;
        self.position = Some(end);
    }

    pub fn new_subpath(&mut self, pos: Vec2) {
        self.extend_extrema(pos);
        // note we don't add to elapsed_distance here; instantly jump to the new position.
        self.position = Some(pos);
        self.subpath_start = Some(pos);
    }

    pub fn clear_command(&mut self) {
        self.command = None;
    }

    pub fn length_so_far(&self) -> f32 {
        self.elapsed_distance
    }

    pub fn get_bbox(&self) -> Option<BoundingBox> {
        if self.position.is_some() {
            Some(BoundingBox::new(
                self.min_x, self.min_y, self.max_x, self.max_y,
            ))
        } else {
            None // we've never called extend_subpath()
        }
    }

    pub fn read_instruction_command(&mut self, tokens: &mut SvgPathSyntax) -> Result<char> {
        if self.command.is_none() || tokens.at_command()? {
            // "The command letter can be eliminated on subsequent commands if the same
            // command is used multiple times in a row (e.g., you can drop the second
            // "L" in "M 100 200 L 200 100 L -100 -200" and use "M 100 200 L 200 100
            // -100 -200" instead)."
            self.command = Some(tokens.read_command()?);
        } else {
            // this will only happen for subsequent values to an existing command
            match self.command {
                // "If a moveto is followed by multiple pairs of coordinates,
                // the subsequent pairs are treated as implicit lineto commands."
                Some('m') => {
                    self.command = Some('l');
                }
                Some('M') => {
                    self.command = Some('L');
                }
                _ => {}
            }
        }

        Ok(self.command.expect("Command should be already set"))
    }
}
