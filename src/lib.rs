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
use std::io::{BufRead, Cursor, Write};
#[cfg(feature = "cli")]
use std::io::{BufReader, IsTerminal, Read};
#[cfg(feature = "cli")]
use std::path::{Path, PathBuf};

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
mod scope;
#[cfg(feature = "server")]
pub mod server;
mod style;
mod transform;
mod types;

pub use config::{ErrorMode, TransformConfig};
pub use errors::{Error, Result};
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
        Box::new(BufReader::new(File::open(input).map_err(Error::Io)?)) as Box<dyn BufRead>
    };

    if output == "-" {
        transform_stream(&mut in_reader, &mut std::io::stdout(), cfg)?;
    } else {
        let temp_output = output_temp_path(output);
        let transform_result = (|| -> Result<()> {
            let mut out_temp = File::create(&temp_output).map_err(Error::Io)?;
            transform_stream(&mut in_reader, &mut out_temp, cfg)?;
            out_temp.flush().map_err(Error::Io)?;
            std::fs::rename(&temp_output, output).map_err(Error::Io)?;
            Ok(())
        })();

        if transform_result.is_err() {
            let _ = std::fs::remove_file(&temp_output);
        }

        transform_result?;
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn output_temp_path(output: &str) -> PathBuf {
    let output = Path::new(output);
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("svgdx-output");
    parent.join(format!("{file_name}.{}.tmp", std::process::id()))
}

#[deprecated(
    note = "Use 'transform_json' for WASM entrypoint, or transform_str/transform_str_default for Rust library use"
)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn transform_string(input: String, add_metadata: bool) -> core::result::Result<String, String> {
    let cfg = TransformConfig {
        add_metadata,
        ..Default::default()
    };
    transform_str(input, &cfg).map_err(|e| e.to_string())
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

// JSON API for editor/WASM use
#[cfg(feature = "json")]
pub mod json_api {
    use super::{Error, Result, TransformConfig, transform_str};
    use serde_derive::{Deserialize, Serialize};
    use std::collections::HashMap;

    pub const JSON_API_VERSION: u32 = 1;

    #[derive(Debug, Deserialize)]
    pub struct TransformRequest {
        pub version: u32,
        pub input: String,
        #[serde(default)]
        pub config: RequestConfig,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct RequestConfig {
        #[serde(default)]
        pub add_metadata: bool,
        #[serde(default)]
        pub vars: HashMap<String, String>,
    }

    impl TryFrom<RequestConfig> for TransformConfig {
        type Error = Error;

        fn try_from(config: RequestConfig) -> Result<Self> {
            let vars = config
                .vars
                .into_iter()
                .map(|(k, v)| Ok((k.parse()?, v)))
                .collect::<Result<HashMap<_, _>>>()?;
            Ok(TransformConfig {
                add_metadata: config.add_metadata,
                vars,
                ..Default::default()
            })
        }
    }

    #[derive(Debug, Serialize)]
    pub struct TransformResponse {
        pub version: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub svg: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub warnings: Vec<String>,
    }

    impl TransformResponse {
        pub fn success(svg: String) -> Self {
            Self {
                version: JSON_API_VERSION,
                svg: Some(svg),
                error: None,
                warnings: vec![],
            }
        }

        pub fn error(message: String) -> Self {
            Self {
                version: JSON_API_VERSION,
                svg: None,
                error: Some(message),
                warnings: vec![],
            }
        }
    }

    /// Transform input using JSON request/response format.
    ///
    /// Takes a JSON string containing `TransformRequest`, returns `TransformResponse`
    pub fn transform_json_impl(input: &str) -> TransformResponse {
        match serde_json::from_str::<TransformRequest>(input) {
            Ok(request) => {
                if request.version != JSON_API_VERSION {
                    TransformResponse::error(format!(
                        "Unsupported API version: {} (expected {})",
                        request.version, JSON_API_VERSION
                    ))
                } else {
                    match request
                        .config
                        .try_into()
                        .and_then(|cfg| transform_str(request.input, &cfg))
                    {
                        Ok(svg) => TransformResponse::success(svg),
                        Err(e) => TransformResponse::error(e.to_string()),
                    }
                }
            }
            Err(e) => TransformResponse::error(format!("Invalid JSON request: {e}")),
        }
    }
}

/// Transform input using JSON request/response format
///
/// Takes a JSON string containing a request object, returns JSON string response.
///
/// **Request format**:
/// `{"version": 1, "input": "...", "config": {"add_metadata": bool, "vars": {"var1": "value1", ...}}}`
///
/// **Success response**:
/// `{"version": 1, "svg": "...", "warnings": []}`
///
/// **Error response**:
/// `{"version": 1, "error": "..."}`
// #[cfg(all(feature = "json", target_arch = "wasm32"))]
#[cfg(feature = "json")]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn transform_json(input: &str) -> String {
    let result = json_api::transform_json_impl(input);
    serde_json::to_string(&result).expect("Failed to serialize response")
}

#[cfg(all(test, feature = "json"))]
mod json_tests {
    use super::transform_json;

    #[test]
    fn test_json_transform_success() {
        let request = r#"{"version": 1, "input": "<svg><rect wh=\"10\"/></svg>", "config": {}}"#;
        let response = transform_json(request);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["version"], 1);
        assert!(parsed["svg"].as_str().unwrap().contains("<svg"));
        assert!(parsed["error"].is_null());
    }

    #[test]
    fn test_json_transform_error() {
        let request = r#"{"version": 1, "input": "<svg><invalid", "config": {}}"#;
        let response = transform_json(request);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["version"], 1);
        assert!(parsed["svg"].is_null());
        assert!(parsed["error"].as_str().is_some());
    }

    #[test]
    fn test_json_invalid_version() {
        let request = r#"{"version": 999, "input": "<svg/>", "config": {}}"#;
        let response = transform_json(request);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["version"], 1);
        assert!(
            parsed["error"]
                .as_str()
                .unwrap()
                .contains("Unsupported API version")
        );
    }

    #[test]
    fn test_json_invalid_request() {
        let request = r#"not valid json"#;
        let response = transform_json(request);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["version"], 1);
        assert!(
            parsed["error"]
                .as_str()
                .unwrap()
                .contains("Invalid JSON request")
        );
    }

    #[test]
    fn test_json_vars() {
        let request = r#"{"version": 1, "input": "<text text='${greeting} ${name}'/>", "config": {"vars": {"greeting": "howdy", "name": "world"}}}"#;
        let response = transform_json(request);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(parsed["version"], 1);
        assert!(
            parsed["svg"]
                .as_str()
                .unwrap()
                .contains(">howdy world</text>")
        );
        assert!(parsed["error"].is_null());
    }
}
