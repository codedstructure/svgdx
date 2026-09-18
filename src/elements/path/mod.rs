mod arc;
mod bezier;
mod command;
mod convert;
mod lines;
mod parser;
mod repeat;
mod sample;
mod state;
mod syntax;
#[cfg(test)]
mod test_path;
mod types;

use super::SvgElement;

pub use convert::points_to_path;
pub use parser::{get_point_along_path, process_path_data};
pub use repeat::process_path_repeat;
use syntax::PathSyntax;
use types::Vec2;
