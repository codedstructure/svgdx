mod args;
use std::path::Path;

use crate::{Error, Result, TransformConfig, VERSION, transform_file};

pub use args::{Args, CliAction, NO_INPUT_STDIN_TERMINAL, parse_args, usage};

/// Top-level configuration used by the `svgdx` command-line process.
///
/// This is typically derived from command line arguments and passed to `run()`.
///
/// 'front-end' program settings (e.g. input/output filenames, whether to continually
/// process input on change, etc) are stored directly in this struct. Per-transform
/// ('back-end') settings are stored in the embedded `TransformConfig` struct.
#[derive(Clone)]
pub struct Config {
    /// Path to input file, or '-' for stdin
    pub input_path: String,
    /// Path to output file, or '-' for stdout
    pub output_path: String,
    /// transform config options
    pub transform: TransformConfig,
}

impl Args {
    pub fn into_config(self) -> Result<Config> {
        if self.input != "-" && self.output != "-" {
            // Arguably creating this struct shouldn't do any IO, but this is a
            // deliberate UX safety restriction on the CLI which is worth keeping
            // as high-level as possible to keep the lower level API cleaner.
            let in_path = Path::new(&self.input);
            let out_path = Path::new(&self.output);
            if out_path.exists() && out_path.canonicalize()? == in_path.canonicalize()? {
                return Err(Error::Document(
                    "Output path must not refer to the same file as the input file.".into(),
                ));
            }
        }
        Ok(Config {
            input_path: self.input,
            output_path: self.output,
            transform: self.config.try_into()?,
        })
    }
}

pub fn run(config: CliAction, program_name: &str) -> Result<()> {
    match config {
        CliAction::Help => {
            println!("{}", usage(program_name));
        }
        CliAction::ImplicitStdinTerminal => {
            println!("{program_name} v{VERSION}");
            println!();
            println!("{NO_INPUT_STDIN_TERMINAL}");
        }
        CliAction::Version => {
            println!("{program_name} v{VERSION}");
        }
        CliAction::Run(args) => {
            let config = args.into_config()?;
            transform_file(&config.input_path, &config.output_path, &config.transform)?;
        }
    }

    Ok(())
}
