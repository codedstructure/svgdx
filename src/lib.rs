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
//! Errors in processing are handled via `anyhow::Result`; currently these are mainly
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

use std::io::{BufRead, Cursor, IsTerminal, Read, Write};

use anyhow::Result;
use std::{
    fs::{self, File},
    io::BufReader,
};
use tempfile::NamedTempFile;

#[cfg(feature = "cli")]
pub mod cli;
mod connector;
mod element;
mod expression;
mod path;
#[cfg(feature = "server")]
pub mod server;
mod svg_defs;
mod text;
mod transform;
mod types;

use transform::Transformer;

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
    pub add_auto_defs: bool,
    /// Background colour (default "none")
    pub background: String,
    /// Random seed
    pub seed: u64,
    /// Maximum loop iterations
    pub loop_limit: u32,
    /// Add source metadata to output
    pub add_metadata: bool,
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self {
            debug: false,
            scale: 1.0,
            border: 5,
            add_auto_defs: true,
            background: "none".to_owned(),
            seed: 0,
            loop_limit: 1000,
            add_metadata: false,
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

/// Read file from `input` ('-' for stdin), process the result,
/// and write to file given by `output` ('-' for stdout).
///
/// The transform can be modified by providing a suitable `TransformConfig` value.
pub fn transform_file(input: &str, output: &str, cfg: &TransformConfig) -> Result<()> {
    let mut in_reader = if input == "-" {
        let mut stdin = std::io::stdin().lock();
        if stdin.is_terminal() {
            // This is unpleasant; at least on Mac, a single Ctrl-D is not otherwise
            // enough to signal end-of-input, even when given at the start of a line.
            // Work around this by reading entire input, then wrapping in a Cursor to
            // provide a buffered reader.
            // It would be nice to improve this.
            let mut buf = Vec::new();
            stdin
                .read_to_end(&mut buf)
                .expect("stdin should be readable to EOF");
            Box::new(BufReader::new(Cursor::new(buf))) as Box<dyn BufRead>
        } else {
            Box::new(stdin) as Box<dyn BufRead>
        }
    } else {
        Box::new(BufReader::new(File::open(input)?)) as Box<dyn BufRead>
    };

    if output == "-" {
        transform_stream(&mut in_reader, &mut std::io::stdout(), cfg)?;
    } else {
        let mut out_temp = NamedTempFile::new()?;
        transform_stream(&mut in_reader, &mut out_temp, cfg)?;
        // Copy content rather than rename (by .persist()) since this
        // could cross filesystems; some apps (e.g. eog) also fail to
        // react to 'moved-over' files.
        fs::copy(out_temp.path(), output)?;
    }

    Ok(())
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
