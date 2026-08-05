use crate::config::{TransformArgs, common_usage, take_value};
use crate::errors::{Error, Result};
use std::io::IsTerminal;

pub const NO_INPUT_STDIN_TERMINAL: &str = r"Not defaulting '--input' when stdin is a terminal.

Use '-h' or '--help' for usage.";

pub fn usage(program_name: &str) -> String {
    let common_usage = common_usage();
    format!(
        r#"
Usage:
  {program_name} [OPTIONS]

Options:
  -i, --input <INPUT>           Input file ('-' for stdin) ['-']
  -o, --output <OUTPUT>         Target output file ('-' for stdout) ['-']
  -h, --help                    Show this help
  -V, --version                 Display program version

Note:
  INPUT only defaults to stdin when not a terminal; use explicit `-i -` to read
  from stdin on a terminal.

Transform Options:
{common_usage}
"#
    )
}

pub struct Args {
    pub input: String,
    pub output: String,
    pub config: TransformArgs,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            input: "-".to_string(),
            output: "-".to_string(),
            config: TransformArgs::default(),
        }
    }
}

pub enum CliAction {
    // -h or --help
    Help,
    // svgdx -V or --version
    Version,
    // stdin is terminal and we have no INPUT arg
    ImplicitStdinTerminal,
    // normal usage
    Run(Args),
}

pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<CliAction> {
    let mut args = args.into_iter().peekable();
    let _ = args.next(); // skip argv[0]

    let mut parsed = Args::default();
    let mut input_value = None;

    while let Some(arg) = args.next() {
        // Support --flag=value style by splitting on the first '='
        let (key, embedded): (String, Option<String>) = match arg.split_once('=') {
            Some((k, v)) if k.starts_with('-') => (k.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };

        match key.as_str() {
            "-h" | "--help" => {
                return Ok(CliAction::Help);
            }
            "-V" | "--version" => {
                return Ok(CliAction::Version);
            }
            "-o" | "--output" => {
                parsed.output = take_value(&key, embedded, &mut args)?;
            }
            "-i" | "--input" => {
                input_value = Some(take_value(&key, embedded, &mut args)?);
            }
            _ => {
                if !parsed.config.handle_arg(&key, embedded, &mut args)? {
                    return Err(Error::Cli(format!("unknown argument: '{key}'")));
                }
            }
        }
    }

    match input_value {
        Some(v) => parsed.input = v,
        None => {
            // Default to stdin, but only if not a terminal
            if std::io::stdin().is_terminal() {
                return Ok(CliAction::ImplicitStdinTerminal);
            }
        }
    }

    Ok(CliAction::Run(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        let config = parse_args(vec!["svgdx".to_string(), "--help".to_string()]);
        assert!(matches!(config, Ok(CliAction::Help)));
    }
}
