//! ## svgdx - create SVG diagrams easily
//!
//! `svgdx` is normally run as a command line tool, taking an input file and processing
//! it into an SVG output file.
//!
//! ## Library use
//!
//! Support as a library is primarily to allow other front-ends to convert svgdx
//! documents to SVG without having to call `svgdx` as a command-line subprocess.
//!
//! A `TransformConfig` object should be created as appropriate to configure the
//! transform process, and the appropriate `transform_*` function called passing
//! this and appropriate input / output parameters as required.
//!
//! Errors in processing are handled via `svgdx::Result`; currently these are mainly
//! useful in providing basic error messages suitable for end-users.
//!
//! ## Example
//!
//! ```
//! let cfg = svgdx::TransformConfig::default();
//!
//! let input = r#"<rect wh="50" text="Hello!"/>"#;
//! let output = svgdx::transform_str(input, &cfg).unwrap();
//!
//! println!("{output}");
//! ```

use themes::ThemeType;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;


use std::io::{BufRead, Cursor, Write};


// #[cfg(feature = "cli")]
// pub mod cli;
mod colours;
mod connector;
mod context;
mod element;
pub mod errors;
mod events;
mod expression;
mod functions;
mod loop_el;
mod path;
mod position;
mod reuse;
// #[cfg(feature = "server")]
// pub mod server;
mod text;
pub mod themes;
mod transform;
mod transform_attr;
mod types;

pub use errors::{Result, SvgdxError};
use transform::Transformer;

// Allow users of this as a library to easily retrieve the version of svgdx being used
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Settings to configure a single transformation.
///
/// Note the settings here are specific to a single transformation; alternate front-ends
/// may use this directly rather than `Config` which wraps this struct when `svgdx` is
/// run as a command-line program.
#[derive(Clone, Debug)]
pub struct TransformConfig {
    /// Add debug info (e.g. input source) to output
    pub debug: bool,
    /// Overall output image scale (in mm as scale of user units)
    pub scale: f32,
    /// Border width (user-units, default 5)
    pub border: u16,
    /// Add style & defs entries based on class usage
    pub add_auto_styles: bool,
    /// Background colour (default "default" - use theme default or none)
    pub background: String, // TODO: sanitize this with a `Colour: FromStr + Display` type
    /// Random seed
    pub seed: u64,
    /// Maximum loop iterations
    pub loop_limit: u32,
    /// Max length of variable
    pub var_limit: u32,
    /// Maximum depth of recursion
    pub depth_limit: u32,
    /// Add source metadata to output
    pub add_metadata: bool,
    /// Default font-size (in user-units)
    pub font_size: f32,
    /// Default font-family
    pub font_family: String,
    /// Theme to use (default "default")
    pub theme: ThemeType,
    /// Make styles local to this document
    pub use_local_styles: bool,
    /// Optional style to apply to SVG root element
    pub svg_style: Option<String>,
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self {
            debug: false,
            scale: 1.0,
            border: 5,
            add_auto_styles: true,
            background: "default".to_owned(),
            seed: 0,
            loop_limit: 1000,
            var_limit: 1024,
            depth_limit: 100,
            add_metadata: false,
            font_size: 3.0,
            font_family: "sans-serif".to_owned(),
            theme: ThemeType::default(),
            use_local_styles: false,
            svg_style: None,
        }
    }
}

/// Reads from the `reader` stream, processes document, and writes to `writer`.
///
/// Note the entire stream may be read before any converted data is written to `writer`.
///
/// The transform can be modified by providing a suitable `TransformConfig` value.
pub fn transform_stream(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    config: &TransformConfig,
) -> Result<()> {
    let mut t = Transformer::from_config(config);
    t.transform(reader, writer)
}

/// Transform `input` provided as a string, returning the result as a string.
///
/// The transform can be modified by providing a suitable `TransformConfig` value.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn transform_string(input: String, add_metadata: bool) -> core::result::Result<String, String> {
    let cfg = TransformConfig {
        add_metadata,
        ..Default::default()
    };
    transform_str(input, &cfg).map_err(|e| e.to_string())
}

pub fn transform_str<T: Into<String>>(input: T, cfg: &TransformConfig) -> Result<String> {
    let input = input.into();

    let mut input = Cursor::new(input);
    let mut output: Vec<u8> = vec![];

    transform_stream(&mut input, &mut output, cfg)?;

    Ok(String::from_utf8(output).expect("Non-UTF8 output generated"))
}

/// Transform the provided `input` string using default config, returning the result string.
///
/// Uses default `TransformConfig` settings.
pub fn transform_str_default<T: Into<String>>(input: T) -> Result<String> {
    transform_str(input, &TransformConfig::default())
}
