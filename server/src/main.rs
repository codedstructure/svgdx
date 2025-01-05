use std::{net::IpAddr, str::FromStr};

mod server;

use clap::Parser;
use tokio::sync::mpsc::channel;

#[derive(Clone, Debug)]
struct Hostname(String);

impl FromStr for Hostname {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let orig = s;
        let mut s = s;
        // must not be empty
        if s.is_empty() {
            return Err("Empty hostname".to_string());
        }
        // Allow leading wildcard
        if s.starts_with("*.") {
            s = &s[2..];
        }
        let parts = s.split('.').collect::<Vec<_>>();
        if parts.is_empty() {
            return Err("Empty hostname".to_string());
        }
        for part in &parts {
            // Each part must alphanumeric + '-', and non-empty
            if part.is_empty() {
                return Err("Empty hostname part".to_string());
            }
            if !part.chars().all(|c| c.is_alphanumeric() || c == '-') {
                return Err(format!("Invalid character in hostname part: {}", part));
            }
        }
        Ok(Self(orig.to_string()))
    }
}

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

    /// Additional allowed image sources for CSP.
    #[arg(long, number_of_values = 1)]
    img_src: Vec<Hostname>,
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
