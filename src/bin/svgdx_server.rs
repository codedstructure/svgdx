#[tokio::main]
async fn main() {
    svgdx::server::start_server(None).await;
}
