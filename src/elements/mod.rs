mod bearing;
mod connector;
mod containers;
mod corner_route;
mod element;
mod layout;
mod line_offset;
mod loops;
mod markdown;
mod path;
mod reuse;
mod special;
mod text;

use bearing::process_path_bearing;
use connector::{is_connector, ConnectionType, Connector};
use containers::{Container, GroupElement};
use loops::{ForElement, LoopElement};
use reuse::ReuseElement;
use special::{ConfigElement, DefaultsElement, IfElement, SpecsElement, VarElement};
use text::process_text_attr;

pub use element::SvgElement;
pub use layout::is_layout_element;
