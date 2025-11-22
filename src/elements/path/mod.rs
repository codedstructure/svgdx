mod bbox;
mod bearing;
mod convert;
mod repeat;
mod syntax;

use super::SvgElement;

pub use bbox::path_bbox;
pub use bearing::process_path_bearing;
pub use convert::points_to_path;
pub use repeat::process_path_repeat;
pub use syntax::PathSyntax;
