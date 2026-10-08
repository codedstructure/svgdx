use svgdx::VERSION;
use svgdx::server::{parse_args, run};

const BIN_NAME: &str = env!("CARGO_BIN_NAME");

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| {
            eprintln!("Failed to build tokio runtime: {e}");
            std::process::exit(1);
        });

    match parse_args(std::env::args()) {
        Ok(config) => runtime.block_on(run(config, BIN_NAME)),
        Err(msg) => {
            eprintln!("Error: {msg}");
            eprintln!();
            eprintln!("{BIN_NAME} v{VERSION}");
            eprintln!(" '-h' or '--help' for usage");
            std::process::exit(2);
        }
    }
}
