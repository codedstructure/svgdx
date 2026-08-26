#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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

impl RequestConfig {
    pub fn merge_config(&self, server_config: &TransformConfig) -> TransformConfig {
        let mut merged = server_config.clone();
        merged.add_metadata = self.add_metadata;
        for (k, v) in &self.vars {
            if let Ok(var_name) = k.parse() {
                merged.vars.insert(var_name, v.clone());
            }
        }
        merged
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
pub fn transform_json_impl(input: &str, cfg: &TransformConfig) -> TransformResponse {
    match serde_json::from_str::<TransformRequest>(input) {
        Ok(request) => {
            if request.version != JSON_API_VERSION {
                TransformResponse::error(format!(
                    "Unsupported API version: {} (expected {})",
                    request.version, JSON_API_VERSION
                ))
            } else {
                let cfg = request.config.merge_config(cfg);
                match transform_str(request.input, &cfg) {
                    Ok(svg) => TransformResponse::success(svg),
                    Err(e) => TransformResponse::error(e.to_string()),
                }
            }
        }
        Err(e) => TransformResponse::error(format!("Invalid JSON request: {e}")),
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
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn transform_json(input: &str) -> String {
    let cfg = TransformConfig::default();
    let result = transform_json_impl(input, &cfg);
    serde_json::to_string(&result).expect("Failed to serialize response")
}

// note this is not (currently) exposed to WASM; it's intended to support
// svgdx-server's transform config options.
pub fn transform_json_with_config(input: &str, cfg: &TransformConfig) -> String {
    let result = transform_json_impl(input, cfg);
    serde_json::to_string(&result).expect("Failed to serialize response")
}

#[cfg(test)]
mod tests {
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
