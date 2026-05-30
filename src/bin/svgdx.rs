use svgdx::cli::{parse_args, run};
use svgdx::{Error, Result, VERSION};

const BIN_NAME: &str = env!("CARGO_BIN_NAME");

fn main() -> Result<()> {
    match parse_args(std::env::args()).and_then(|config| run(config, BIN_NAME)) {
        Err(Error::Cli(msg)) => {
            eprintln!("Error: {msg}");
            eprintln!();
            eprintln!("{BIN_NAME} v{VERSION}");
            eprintln!(" '-h' or '--help' for usage");
            std::process::exit(2);
        }
        x => x,
    }
}
