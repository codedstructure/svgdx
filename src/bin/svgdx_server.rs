use std::net::IpAddr;

use svgdx::server;

use clap::Parser;
use tokio::sync::mpsc::channel;

/// Command line arguments
#[derive(Parser)]
#[command(author, version, about="svgdx-server: web server for svgdx", long_about=None)] // Read from Cargo.toml
struct Arguments {
    /// Address to listen on
    #[arg(long, default_value = "127.0.0.1")]
    address: IpAddr,

    /// Port to listen on
    #[arg(short, long, default_value = "3003")]
    port: u16,

    /// Open browser on startup
    #[arg(long)]
    open: bool,
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();
    let address = if args.address.is_ipv6() {
        format!("[{}]:{}", args.address, args.port)
    } else {
        format!("{}:{}", args.address, args.port)
    };

    let mut tx = None;
    if args.open {
        let (ch_tx, mut rx) = channel(1);
        tx = Some(ch_tx);
        let address = address.clone();
        // spawn tokio task to open browser
        tokio::spawn(async move {
            // Wait for server to start listening
            if rx.recv().await.is_some() {
                // webbrowser is quite heavyweight, but avoids needing to deal with
                // a bunch of command-injection issues when using e.g. `xdg-open`.
                webbrowser::open(&format!("http://{}", address))
                    .unwrap_or_else(|e| eprintln!("Failed to open browser: {}", e));
            }
        });
    }
    server::start_server(Some(&address), tx).await;
}
