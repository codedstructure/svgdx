//! Constants used throughout svgdx

/// Separates an ElRef from a relative position ('relpos') spec, e.g. `#abc|h`
pub const RELPOS_SEP: char = '|';
/// Separates an ElRef from a location spec, e.g. `#abc@tl`
pub const LOCSPEC_SEP: char = '@';
/// Separates an ElRef from a scalar spec, e.g. `#abc~x2`
pub const SCALARSPEC_SEP: char = '~';
/// Separates an edge-based locspec from a length value, e.g. `#abc@t:25%`
pub const EDGESPEC_SEP: char = ':';

/// ElRef referencing the previous element
pub const ELREF_PREVIOUS: char = '^';
/// ElRef referencing the next element
pub const ELREF_NEXT: char = '+';
/// ElRef referencing element with the given id, e.g. `#abc`
pub const ELREF_ID_PREFIX: char = '#';

/// Initial character of a variable reference, e.g. `$var`
pub const VAR_PREFIX: char = '$';
/// Opening char for braced variable references, e.g. `${var}`
pub const VAR_OPEN_BRACE: char = '{';
/// Closing char for braced variable references, e.g. `${var}`
pub const VAR_END_BRACE: char = '}';

/// Start of an attribute expression to be evaluated e.g. `{{ 2 + 2 }}`
pub const EXPR_START: &str = "{{";
/// End of an attribute expression to be evaluated e.g. `{{ 2 + 2 }}`
pub const EXPR_END: &str = "}}";

pub const DEFAULT_FONT_SIZE: f32 = 3.0;
pub const DEFAULT_FONT_FAMILY: &str = "sans-serif";
pub const DEFAULT_BACKGROUND: &str = "default";
pub const DEFAULT_RNG_SEED: u64 = 0;
pub const DEFAULT_SCALE: f32 = 1.0;
pub const DEFAULT_BORDER: u16 = 5;
pub const DEFAULT_LOOP_LIMIT: u32 = 1000;
pub const DEFAULT_VAR_LIMIT: u32 = 1024;
pub const DEFAULT_DEPTH_LIMIT: u32 = 100;
pub const DEFAULT_PATH_REPEAT_LIMIT: u32 = 10000;
