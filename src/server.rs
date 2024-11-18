use axum::{
    body::Body,
    extract::Path,
    http::Response,
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use crate::{transform_str, TransformConfig};

// Content-Security-Policy - allow inline CSS used for the generated SVG images,
// but otherwise restrict to same-origin resources.
// Includes 'wasm-unsafe-eval' for consistency even though is doing server-side
// transforms rather than in-browser.
const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'";

async fn transform(input: String) -> impl IntoResponse {
    transform_str(
        input,
        &TransformConfig {
            add_metadata: true,
            ..Default::default()
        },
    )
    .and_then(|output| {
        if output.is_empty() {
            // Can't build a valid image/svg+xml response from empty string.
            Err(anyhow::Error::msg("Empty response"))
        } else {
            Ok(output)
        }
    })
    .map(|output| {
        Response::builder()
            .header("Content-Type", "image/svg+xml")
            .header("Content-Security-Policy", CSP)
            .body(Body::from(output))
            .unwrap()
    })
    .map_err(|e| {
        // TODO: make the error more informative, e.g. by returning a JSON object
        // including line number(s) of failed elements.
        Response::builder()
            .status(400)
            .header("Content-Type", "text/plain")
            .body(Body::from(format!("Error: {}", e)))
            .unwrap()
    })
}

macro_rules! include_or_read {
    ($path:expr, $mime:expr) => {{
        // If configured as a release build, use include_bytes! to embed the file.
        #[cfg(not(debug_assertions))]
        let content =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/editor/", $path)).as_ref();
        // During development it's useful to have it re-read each request.
        #[cfg(debug_assertions)]
        let content = tokio::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/editor/", $path))
            .await
            .unwrap();

        Response::builder()
            .header("Content-Type", $mime)
            .header("Content-Security-Policy", CSP)
            .body(Body::from(content))
            .unwrap()
    }};
}

macro_rules! include_js {
    ($path:expr) => {{
        include_or_read!($path, "application/javascript")
    }};
}

macro_rules! include_css {
    ($path:expr) => {{
        include_or_read!($path, "text/css")
    }};
}

macro_rules! include_html {
    ($path:expr) => {{
        include_or_read!($path, "text/html")
    }};
}

macro_rules! include_ico {
    ($path:expr) => {{
        include_or_read!($path, "image/x-icon")
    }};
}

async fn index() -> impl IntoResponse {
    include_html!("index.html")
}

async fn favicon() -> impl IntoResponse {
    include_ico!("favicon.ico")
}

// Note svgdx-server injects a different bootstrap script (-server.js) vs the bootstrap
// picked up by a static file server (such as `python3 -m http.server`). This is to ensure
// transform requests come to the server rather than being handled by the browser WASM code.
async fn bootstrap() -> impl IntoResponse {
    include_js!("svgdx-bootstrap-server.js")
}

async fn static_file(Path(path): Path<String>) -> impl IntoResponse {
    match path.as_str() {
        "svgdx-editor.js" => {
            include_js!("static/svgdx-editor.js")
        }
        "svgdx-editor.css" => {
            include_css!("static/svgdx-editor.css")
        }
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
        _ => Response::builder()
            .status(404)
            .header("Content-Type", "text/plain")
            .body(Body::from("File not found"))
            .unwrap(),
    }
}

pub async fn start_server(listen_addr: Option<&str>) {
    let addr = listen_addr.unwrap_or("127.0.0.1:3003");
    let app = Router::new()
        .route("/", get(index))
        .route("/favicon.ico", get(favicon))
        .route("/static/*path", get(static_file))
        .route("/svgdx-bootstrap.js", get(bootstrap))
        .route("/api/transform", post(transform));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Listening on: http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
