use std::sync::Arc;

use http::header::{CONTENT_SECURITY_POLICY, CONTENT_TYPE, LOCATION};
use http::{Method, Response, StatusCode};

use super::http::HttpRequest;
use crate::json::{TransformResponse, reformat_json_impl, transform_json_impl};
use crate::{TransformConfig, VERSION};

pub(crate) type AppResponse = Response<Vec<u8>>;

// Content-Security-Policy - allow inline CSS used for the generated SVG images,
// but otherwise restrict to same-origin resources.
// Includes 'wasm-unsafe-eval' for consistency even though is doing server-side
// transforms rather than in-browser.
// img-src requires `blob:` scheme for clipboard copy.
const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' blob:; frame-ancestors 'none'";

pub(crate) async fn route_request(
    request: HttpRequest,
    config: Arc<TransformConfig>,
    docs_redirect_url: Option<String>,
) -> AppResponse {
    match request.path.as_str() {
        "/" => {
            if request.method == Method::GET {
                index().await
            } else {
                method_not_allowed()
            }
        }
        "/favicon.ico" => {
            if request.method == Method::GET {
                favicon().await
            } else {
                method_not_allowed()
            }
        }
        "/svgdx-bootstrap.js" => {
            if request.method == Method::GET {
                bootstrap().await
            } else {
                method_not_allowed()
            }
        }
        "/api/transform_json" => {
            if request.method == Method::POST {
                transform_json(request, &config)
            } else {
                method_not_allowed()
            }
        }
        "/api/reformat_json" => {
            if request.method == Method::POST {
                reformat_json(request, &config)
            } else {
                method_not_allowed()
            }
        }
        "/docs/" => {
            if let Some(url) = docs_redirect_url {
                if request.method == Method::GET {
                    temporary_redirect(&url)
                } else {
                    method_not_allowed()
                }
            } else {
                not_found()
            }
        }
        _ if request.path.starts_with("/static/") => {
            if request.method == Method::GET {
                static_file(request.path.trim_start_matches("/static/")).await
            } else {
                method_not_allowed()
            }
        }
        _ => not_found(),
    }
}

fn transform_json(request: HttpRequest, config: &TransformConfig) -> AppResponse {
    let input = match body_to_string(request.body) {
        Ok(input) => input,
        Err(err) => return bad_request(err),
    };

    transform_json_handler(input, config)
}

fn reformat_json(request: HttpRequest, config: &TransformConfig) -> AppResponse {
    let input = match body_to_string(request.body) {
        Ok(input) => input,
        Err(err) => return bad_request(err),
    };

    reformat_json_handler(input, config)
}

fn body_to_string(body: Vec<u8>) -> Result<String, String> {
    String::from_utf8(body).map_err(|_| "Request body must be UTF-8".into())
}

fn transform_json_handler(input: String, config: &TransformConfig) -> AppResponse {
    let response: TransformResponse = transform_json_impl(&input, config);
    json_response(response)
}

fn reformat_json_handler(input: String, _config: &TransformConfig) -> AppResponse {
    let response: TransformResponse = reformat_json_impl(&input);
    json_response(response)
}

fn json_response(response: TransformResponse) -> AppResponse {
    let is_error = response.error.is_some();
    let body = match serde_json::to_vec(&response) {
        Ok(body) => body,
        Err(e) => {
            return respond(
                format!("Error serializing JSON response: {e}"),
                "text/plain",
                StatusCode::INTERNAL_SERVER_ERROR,
                false,
            );
        }
    };
    let status = if is_error {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::OK
    };

    respond(body, "application/json", status, true)
}

fn bad_request(err: String) -> AppResponse {
    respond(err, "text/plain", StatusCode::BAD_REQUEST, false)
}

// return 200 OK with given body / mime-type
fn respond_ok(content: impl Into<Vec<u8>>, mime: &str) -> AppResponse {
    respond(content, mime, StatusCode::OK, true)
}

fn respond(
    content: impl Into<Vec<u8>>,
    mime: &str,
    status: StatusCode,
    include_csp: bool,
) -> AppResponse {
    let mut response = Response::new(content.into());
    *response.status_mut() = status;
    let headers = response.headers_mut();
    let mime_value = http::HeaderValue::from_str(mime)
        .unwrap_or_else(|_| http::HeaderValue::from_static("text/plain"));
    headers.insert(CONTENT_TYPE, mime_value);
    if include_csp {
        headers.insert(CONTENT_SECURITY_POLICY, http::HeaderValue::from_static(CSP));
    }
    response
}

fn method_not_allowed() -> AppResponse {
    respond(
        "Method Not Allowed",
        "text/plain",
        StatusCode::METHOD_NOT_ALLOWED,
        false,
    )
}

fn temporary_redirect(url: &str) -> AppResponse {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = StatusCode::TEMPORARY_REDIRECT;
    let location = match http::HeaderValue::from_str(url) {
        Ok(value) => value,
        Err(e) => {
            return respond(
                format!("Invalid redirect URL: {e}"),
                "text/plain",
                StatusCode::INTERNAL_SERVER_ERROR,
                false,
            );
        }
    };
    response.headers_mut().insert(LOCATION, location);
    response
}

macro_rules! include_or_read {
    ($path:expr, $mime:expr) => {{
        #[cfg(not(debug_assertions))]
        let content =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/editor/", $path)).to_vec();
        #[cfg(debug_assertions)]
        let content =
            match tokio::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/editor/", $path)).await {
                Ok(content) => content,
                Err(e) => {
                    return respond(
                        format!("Failed to load static asset '{}': {e}", $path),
                        "text/plain",
                        StatusCode::INTERNAL_SERVER_ERROR,
                        false,
                    );
                }
            };
        respond_ok(content, $mime)
    }};
}

macro_rules! include_js {
    ($path:expr) => {{ include_or_read!($path, "application/javascript") }};
}

macro_rules! include_css {
    ($path:expr) => {{ include_or_read!($path, "text/css") }};
}

macro_rules! include_html {
    ($path:expr) => {{ include_or_read!($path, "text/html") }};
}

macro_rules! include_ico {
    ($path:expr) => {{ include_or_read!($path, "image/x-icon") }};
}

async fn index() -> AppResponse {
    include_html!("index.html")
}

async fn favicon() -> AppResponse {
    include_ico!("favicon.ico")
}

async fn bootstrap() -> AppResponse {
    // server mode bootstrap; hardcoded here rather than loaded
    // from file to allow version string to be injected.
    let content = format!(
        r#"
console.log("svgdx: using server transform");
window.svgdx_use_server = true;
window.svgdx_version_label = "svgdx v{VERSION}";
window.dispatchEvent(new Event('svgdx-ready'));
"#
    );
    respond_ok(content, "application/javascript")
}

async fn static_file(path: &str) -> AppResponse {
    match path {
        "main.js" => include_js!("static/main.js"),
        "svgdx-editor.css" => include_css!("static/svgdx-editor.css"),
        "modules/config.js" => include_js!("static/modules/config.js"),
        "modules/storage.js" => include_js!("static/modules/storage.js"),
        "modules/dom.js" => include_js!("static/modules/dom.js"),
        "modules/editor-adapter.js" => include_js!("static/modules/editor-adapter.js"),
        "modules/transform.js" => include_js!("static/modules/transform.js"),
        "modules/reformat-action.js" => include_js!("static/modules/reformat-action.js"),
        "modules/tabs.js" => include_js!("static/modules/tabs.js"),
        "modules/layout.js" => include_js!("static/modules/layout.js"),
        "modules/viewport.js" => include_js!("static/modules/viewport.js"),
        "modules/splitter.js" => include_js!("static/modules/splitter.js"),
        "modules/statusbar.js" => include_js!("static/modules/statusbar.js"),
        "modules/clipboard.js" => include_js!("static/modules/clipboard.js"),
        "modules/svg-text.js" => include_js!("static/modules/svg-text.js"),
        "modules/popup-menus.js" => include_js!("static/modules/popup-menus.js"),
        "modules/toolbar.js" => include_js!("static/modules/toolbar.js"),
        "modules/slider.js" => include_js!("static/modules/slider.js"),
        "vendor/cm5/codemirror.min.css" => {
            include_css!("static/vendor/cm5/codemirror.min.css")
        }
        "vendor/cm5/codemirror.min.js" => include_js!("static/vendor/cm5/codemirror.min.js"),
        "vendor/cm5/mode/xml/xml.min.js" => include_js!("static/vendor/cm5/mode/xml/xml.min.js"),
        "vendor/cm5/addon/fold/xml-fold.min.js" => {
            include_js!("static/vendor/cm5/addon/fold/xml-fold.min.js")
        }
        "vendor/cm5/addon/fold/foldcode.js" => {
            include_js!("static/vendor/cm5/addon/fold/foldcode.js")
        }
        "vendor/cm5/addon/fold/foldgutter.js" => {
            include_js!("static/vendor/cm5/addon/fold/foldgutter.js")
        }
        "vendor/cm5/addon/fold/foldgutter.min.css" => {
            include_css!("static/vendor/cm5/addon/fold/foldgutter.min.css")
        }
        "vendor/cm5/addon/display/autorefresh.min.js" => {
            include_js!("static/vendor/cm5/addon/display/autorefresh.min.js")
        }
        _ => respond("File not found", "text/plain", StatusCode::NOT_FOUND, false),
    }
}

fn not_found() -> AppResponse {
    respond(
        "404 Not Found\n",
        "text/plain",
        StatusCode::NOT_FOUND,
        false,
    )
}
