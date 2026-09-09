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

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "cli")]
use std::fs::File;
#[cfg(feature = "cli")]
use std::io::Read;
use std::io::{BufRead, Cursor, Write};
#[cfg(feature = "cli")]
use std::path::{Path, PathBuf};

mod builtin;
#[cfg(feature = "cli")]
#[cfg(not(target_arch = "wasm32"))]
pub mod cli;
mod config;
mod constants;
mod context;
mod document;
mod elements;
mod errors;
mod expr;
mod geometry;
#[cfg(feature = "json")]
mod json;
mod scope;
#[cfg(feature = "server")]
pub mod server;
mod style;
mod transform;
mod types;

pub use config::{ErrorMode, TransformConfig};
use document::InputList;
pub use errors::{Error, Result};
#[cfg(feature = "json")]
pub use json::{
    TransformResponse, reformat_json, reformat_json_with_config, transform_json,
    transform_json_with_config,
};
pub use style::{AutoStyleMode, ThemeType};
use transform::Transformer;
pub use types::VarName;

// Allow users of this as a library to easily retrieve the version of svgdx being used
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// for injecting into svgdx-bootstrap.js
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn version_label() -> String {
    format!("svgdx v{VERSION}")
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

/// Read file from `input` ('-' for stdin), process the result,
/// and write to file given by `output` ('-' for stdout).
///
/// The transform can be modified by providing a suitable `TransformConfig` value.
#[cfg(feature = "cli")]
pub fn transform_file(input: &str, output: &str, cfg: &TransformConfig) -> Result<()> {
    do_io(input, output, |input| transform_str(input, cfg))
}

/// Read file from `input` ('-' for stdin), reformat the result,
/// and write to file given by `output` ('-' for stdout).
#[cfg(feature = "cli")]
pub fn reformat_file(input: &str, output: &str) -> Result<()> {
    do_io(input, output, |input| reformat(input))
}

/// Helper function to handle IO for transforming files & std streams
#[cfg(feature = "cli")]
fn do_io(
    input_name: &str,
    output_name: &str,
    transform: impl Fn(&str) -> Result<String>,
) -> Result<()> {
    let input = if input_name == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        std::fs::read_to_string(input_name)?
    };

    let output = transform(&input)?;

    if output_name == "-" {
        std::io::stdout().write_all(output.as_bytes())?;
    } else {
        let (mut out_temp, temp_name) = output_temp_file(output_name)?;
        if let Err(e) = out_temp
            .write_all(output.as_bytes())
            .and_then(|_| std::fs::rename(&temp_name, output_name))
        {
            let _ = std::fs::remove_file(&temp_name);
            return Err(e.into());
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn output_temp_file(output: &str) -> Result<(File, PathBuf)> {
    let output = Path::new(output);
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("svgdx-output");
    let candidate = parent.join(format!("{file_name}.{}.tmp", std::process::id()));
    Ok((File::create_new(&candidate)?, candidate))
}

/// Transform `input` provided as a string, returning the result as a string.
///
/// The transform can be modified by providing a suitable `TransformConfig` value.
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

/// Reformat the provided XML-like `input` without applying svgdx transforms.
pub fn reformat<T: Into<String>>(input: T) -> Result<String> {
    let input = input.into();
    let mut cursor = Cursor::new(input.as_bytes());
    let output = InputList::from_reformat_reader(&mut cursor)?.reformat();
    let mut bytes = Vec::new();
    output.write_to(&mut bytes)?;
    Ok(String::from_utf8(bytes).expect("Non-UTF8 output generated"))
}

#[cfg(test)]
mod tests {
    use super::reformat;

    #[test]
    fn test_reformat_normalizes_document_spacing() {
        let input = "<svg>\n<g>\n  <rect/>\n\n</g>\n</svg>";
        let output = reformat(input).unwrap();

        assert_eq!(output, "<svg>\n  <g>\n    <rect/>\n\n  </g>\n</svg>");
    }

    #[test]
    fn test_reformat_returns_parse_error() {
        let error = reformat("<svg><g>").unwrap_err().to_string();

        assert!(error.contains("unclosed element <g>"));
    }
}
