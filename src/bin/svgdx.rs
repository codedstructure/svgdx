use svgdx::cli::{USAGE, parse_args, run};
use svgdx::{Error, Result};

fn main() -> Result<()> {
    match parse_args(std::env::args()).and_then(run) {
        Err(Error::Cli(msg)) => {
            eprintln!("Error: {msg}");
            eprintln!();
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
        x => x,
    }
}
