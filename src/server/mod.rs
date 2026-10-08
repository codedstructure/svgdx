use std::sync::Arc;

use tokio::sync::mpsc::channel;

use crate::VERSION;
use crate::builtin::stdlib_source;

mod app;
mod args;
mod http;

pub use args::{Args, CliAction, parse_args, usage};

impl Args {
    fn socket_addr(&self) -> String {
        if self.address.is_ipv6() {
            format!("[{}]:{}", self.address, self.port)
        } else {
            format!("{}:{}", self.address, self.port)
        }
    }
}

pub async fn run(config: CliAction, program_name: &str) {
    match config {
        CliAction::Help => {
            println!("{}", usage(program_name));
        }
        CliAction::OutputStdlib => {
            print!("{}", stdlib_source());
        }
        CliAction::Version => {
            println!("{program_name} v{VERSION}");
        }
        CliAction::Run(args) => {
            let address = args.socket_addr();
            let mut tx = None;
            if args.open {
                let (ch_tx, mut rx) = channel(1);
                tx = Some(ch_tx);
                let address = address.clone();
                tokio::spawn(async move {
                    if rx.recv().await.is_some() {
                        open::that(format!("http://{address}"))
                            .unwrap_or_else(|e| eprintln!("Failed to open browser: {e}"));
                    }
                });
            }
            http::start_server(
                Some(&address),
                tx,
                args.docs_redirect_url.clone(),
                Arc::new(args.config.try_into().unwrap_or_else(|e| {
                    eprintln!("Failed to create transform config: {e}");
                    std::process::exit(1);
                })),
            )
            .await;
        }
    }
}
