mod args;
use std::{path::Path, str::FromStr};

use crate::{Error, Result, TransformConfig, VERSION_FULL, VarName, transform_file};

pub use args::{Args, CliAction, NO_INPUT_STDIN_TERMINAL, parse_args, usage};

#[derive(Clone, Debug)]
struct VarSpec {
    pub key: VarName,
    pub value: String,
}

impl FromStr for VarSpec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let (key, value) = s
            .split_once('=')
            .ok_or_else(|| Error::Cli(format!("Missing '=' in '--var {s}'")))?;

        let key = key.parse()?;

        Ok(VarSpec {
            key,
            value: value.to_string(),
        })
    }
}

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
            if out_path.exists()
                && out_path.canonicalize().map_err(Error::from_err)?
                    == in_path.canonicalize().map_err(Error::from_err)?
            {
                return Err(Error::Document(
                    "Output path must not refer to the same file as the input file.".into(),
                ));
            }
        }
        let vars = self
            .vars
            .iter()
            .map(|s| s.parse())
            .collect::<Result<Vec<VarSpec>>>()?;
        Ok(Config {
            input_path: self.input,
            output_path: self.output,
            transform: TransformConfig {
                debug: self.debug,
                scale: self.scale,
                border: self.border,
                auto_style_mode: self.auto_style_mode,
                background: self.background,
                seed: self.seed,
                add_metadata: self.add_metadata,
                loop_limit: self.loop_limit,
                var_limit: self.var_limit,
                depth_limit: self.depth_limit,
                path_repeat_limit: self.path_repeat_limit,
                font_size: self.font_size,
                font_family: self.font_family,
                theme: self.theme,
                svg_style: self.svg_style,
                error_mode: self.error_mode,
                vars: vars.into_iter().map(|v| (v.key, v.value)).collect(),
            },
        })
    }
}

pub fn run(config: CliAction) -> Result<()> {
    match config {
        CliAction::Help => {
            println!("{}", usage());
        }
        CliAction::ImplicitStdinTerminal => {
            println!("{VERSION_FULL}");
            println!();
            println!("{NO_INPUT_STDIN_TERMINAL}");
        }
        CliAction::Version => {
            println!("{VERSION_FULL}");
        }
        CliAction::Run(args) => {
            let config = args.into_config()?;
            transform_file(&config.input_path, &config.output_path, &config.transform)?;
        }
    }

    Ok(())
}
