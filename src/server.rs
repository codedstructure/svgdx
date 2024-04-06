use axum::{
    body::Body,
    http::Response,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
#[cfg(debug_assertions)]
use tokio::fs;

use crate::{transform_str, TransformConfig};

async fn transform(input: String) -> impl IntoResponse {
    transform_str(
        input,
        &TransformConfig {
            add_metadata: true,
            ..Default::default()
        },
    )
    .map(|output| {
        Response::builder()
            .header("Content-Type", "image/svg+xml")
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

async fn index() -> Html<String> {
    // If configured as a release build, use include_str! to embed the file.
    #[cfg(not(debug_assertions))]
    let content =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/index.html")).to_string();
    // During development it's useful to have it re-read each request.
    #[cfg(debug_assertions)]
    let content = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/static/index.html"))
        .await
        .unwrap();

    Html(content)
}

async fn script() -> impl IntoResponse {
    // If configured as a release build, use include_str! to embed the file.
    #[cfg(not(debug_assertions))]
    let content = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/static/svgdx-editor.js"
    ))
    .to_string();
    // During development it's useful to have it re-read each request.
    #[cfg(debug_assertions)]
    let content = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/static/svgdx-editor.js"
    ))
    .await
    .unwrap();

    Response::builder()
        .header("Content-Type", "application/javascript")
        .body(Body::from(content))
        .unwrap()
}

pub async fn start_server(listen_addr: Option<&str>) {
    let addr = listen_addr.unwrap_or("127.0.0.1:3003");
    let app = Router::new()
        .route("/", get(index))
        .route("/svgdx-editor.js", get(script))
        .route("/transform", post(transform));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Listening on: http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
