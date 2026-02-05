mod connector;
mod containers;
mod elbow_connector;
mod element;
mod gradient;
mod layout;
mod line_offset;
mod loops;
mod path;
mod reuse;
mod special;
mod text;

use connector::{is_connector, ConnectorType};
use containers::{Container, GroupElement};
use gradient::{LinearGradient, RadialGradient};
use loops::{ForElement, LoopElement};
use reuse::ReuseElement;
use special::{ConfigElement, DefaultsElement, IfElement, SpecsElement, VarElement};
use text::process_text_attr;

pub use element::SvgElement;
pub use layout::is_layout_element;
