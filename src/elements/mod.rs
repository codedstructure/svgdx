mod bearing;
mod connector;
mod containers;
mod element;
mod loops;
mod path;
mod reuse;
mod special;
mod text;

use bearing::process_path_bearing;
use connector::{ConnectionType, Connector};
use containers::{Container, GroupElement};
use loops::{ForElement, LoopElement};
use path::path_bbox;
use reuse::ReuseElement;
use special::{ConfigElement, DefaultsElement, IfElement, SpecsElement, VarElement};
use text::process_text_attr;

pub use element::SvgElement;
