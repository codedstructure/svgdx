use svgdx::cli::{parse_args, run};
use svgdx::{Error, Result, VERSION_FULL};

fn main() -> Result<()> {
    match parse_args(std::env::args()).and_then(run) {
        Err(Error::Cli(msg)) => {
            eprintln!("Error: {msg}");
            eprintln!();
            eprintln!("{VERSION_FULL}");
            eprintln!(" '-h' or '--help' for usage");
            std::process::exit(2);
        }
        x => x,
    }
}
