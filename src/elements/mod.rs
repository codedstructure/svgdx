mod connector;
mod containers;
mod element;
mod gradient;
mod layout;
mod line_offset;
mod loops;
mod markdown;
mod path;
mod reuse;
mod special;
mod text;

use connector::{ConnectorType, is_connector};
use containers::{Container, GroupElement};
use gradient::{LinearGradient, RadialGradient};
use loops::{ForElement, LoopElement};
use reuse::ReuseElement;
use special::{ConfigElement, DefaultsElement, IfElement, SpecsElement, VarElement};
use text::process_text_attr;

pub use element::SvgElement;
pub use layout::is_layout_element;
