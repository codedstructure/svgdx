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
