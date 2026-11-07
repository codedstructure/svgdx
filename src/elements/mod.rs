mod connector;
mod containers;
mod element;
mod gradient;
mod layout;
mod line_offset;
mod loops;
mod path;
mod pipeline;
mod preprocess;
mod reuse;
mod special;
mod text;

use connector::{ConnectorType, is_connector};
use containers::{Container, GroupElement};
use gradient::GradientElement;
use loops::{ForElement, IfElement, LoopElement};
use reuse::ReuseElement;
use special::{ConfigElement, DefaultsElement, SpecsElement, VarElement};
use text::process_text_attr;

pub use element::SvgElement;
pub use layout::is_layout_element;
pub use pipeline::element_pipeline;
