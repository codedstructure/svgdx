use svgdx::VERSION;
use svgdx::server::{parse_args, run};

const BIN_NAME: &str = env!("CARGO_BIN_NAME");

#[tokio::main]
async fn main() {
    match parse_args(std::env::args()) {
        Ok(config) => run(config, BIN_NAME).await,
        Err(msg) => {
            eprintln!("Error: {msg}");
            eprintln!();
            eprintln!("{BIN_NAME} v{VERSION}");
            eprintln!(" '-h' or '--help' for usage");
            std::process::exit(2);
        }
    }
}
