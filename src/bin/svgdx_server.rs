use std::thread;
use std::time::Duration;

use clap::Parser;
use svgdx::server;

/// Command line arguments
#[derive(Parser)]
#[command(author, version, about="svgdx-server: web server for svgdx", long_about=None)] // Read from Cargo.toml
struct Arguments {
    /// Address to listen on
    #[arg(long, default_value = "127.0.0.1")]
    address: String,

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
    let address = format!("{}:{}", args.address, args.port);
    if args.open {
        let address = address.clone();
        thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            // webbrowser is quite heavyweight, but avoids needing to deal with
            // a bunch of command-injection issues when using e.g. `xdg-open`.
            webbrowser::open(&format!("http://{}", address))
                .unwrap_or_else(|e| eprintln!("Failed to open browser: {}", e));
        });
    }
    server::start_server(Some(&address)).await;
}
