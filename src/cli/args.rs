use crate::errors::{Error, Result};
use crate::{AutoStyleMode, ErrorMode, ThemeType};
use std::io::IsTerminal;
use std::str::FromStr;

use crate::constants::*;

pub const NO_INPUT_STDIN_TERMINAL: &str = r"Not defaulting '--input' when stdin is a terminal.

Use '-h' or '--help' for usage.";

pub fn usage() -> String {
    let default_theme = ThemeType::default().to_string();
    let default_error_mode = ErrorMode::default().to_string();
    let default_auto_style_mode = AutoStyleMode::default().to_string();
    format!(
        r#"
Usage:
  svgdx [OPTIONS]

Options:
  -i, --input <INPUT>           Input file ('-' for stdin) ['-']
  -o, --output <OUTPUT>         Target output file ('-' for stdout) ['-']
      --debug                   Add debug info (e.g. input source) to output
      --scale <SCALE>           User-units per mm for root SVG element [{DEFAULT_SCALE}]
      --border <BORDER>         Border width around image in user-units [{DEFAULT_BORDER}]
      --auto-style-mode <MODE>  Auto-style mode: none, inline, css ['{default_auto_style_mode}']
      --background <COLOUR>     Default background colour ['{DEFAULT_BACKGROUND}']
      --seed <SEED>             Seed for RNG functions [{DEFAULT_RNG_SEED}]
      --add-metadata            Include metadata in output
      --loop-limit <N>          Limit on loop element iterations [{DEFAULT_LOOP_LIMIT}]
      --var-limit <N>           Limit on length of variable values [{DEFAULT_VAR_LIMIT}]
      --depth-limit <N>         Recursion depth limit [{DEFAULT_DEPTH_LIMIT}]
      --path-repeat-limit <N>   Path repeat expansion limit [{DEFAULT_PATH_REPEAT_LIMIT}]
      --font-size <SIZE>        Default font-size in user-units [{DEFAULT_FONT_SIZE}]
      --font-family <FAMILY>    Default font-family for text ['{DEFAULT_FONT_FAMILY}']
      --theme <THEME>           Select theme to apply ['{default_theme}']
      --svg-style <STYLE>       Optional style to apply to SVG root element
      --error-mode <MODE>       Error handling: strict, warn, ignore ['{default_error_mode}']
  -D, --var <KEY=VALUE>         Variable key=value pairs (may be repeated)
  -h, --help                    Show this help
  -V, --version                 Display program version

Notes:
  INPUT only defaults to stdin when not a terminal; use explicit `-i -` to read
  from stdin on a terminal.
"#
    )
}

pub struct Args {
    pub input: String,
    pub output: String,
    pub debug: bool,
    pub scale: f32,
    pub border: u16,
    pub auto_style_mode: AutoStyleMode,
    pub background: String,
    pub seed: u64,
    pub add_metadata: bool,
    pub loop_limit: u32,
    pub var_limit: u32,
    pub depth_limit: u32,
    pub path_repeat_limit: u32,
    pub font_size: f32,
    pub font_family: String,
    pub theme: ThemeType,
    pub svg_style: Option<String>,
    pub error_mode: ErrorMode,
    pub vars: Vec<String>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            input: "-".to_string(),
            output: "-".to_string(),
            debug: false,
            scale: DEFAULT_SCALE,
            border: DEFAULT_BORDER,
            auto_style_mode: AutoStyleMode::default(),
            background: DEFAULT_BACKGROUND.to_string(),
            seed: DEFAULT_RNG_SEED,
            add_metadata: false,
            loop_limit: DEFAULT_LOOP_LIMIT,
            var_limit: DEFAULT_VAR_LIMIT,
            depth_limit: DEFAULT_DEPTH_LIMIT,
            path_repeat_limit: DEFAULT_PATH_REPEAT_LIMIT,
            font_size: DEFAULT_FONT_SIZE,
            font_family: DEFAULT_FONT_FAMILY.to_string(),
            theme: ThemeType::default(),
            svg_style: None,
            error_mode: ErrorMode::default(),
            vars: vec![],
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

fn take_value(
    flag: &str,
    embedded: Option<String>,
    args: &mut impl Iterator<Item = String>,
) -> Result<String> {
    embedded
        .or_else(|| args.next())
        .ok_or_else(|| Error::Cli(format!("'{flag}' requires a value")))
}

fn parse_value<T>(
    flag: &str,
    embedded: Option<String>,
    args: &mut impl Iterator<Item = String>,
) -> Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let v = take_value(flag, embedded, args)?;
    v.parse().map_err(|e| Error::Cli(format!("'{flag}': {e}")))
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
            "--debug" => parsed.debug = true,
            "--scale" => parsed.scale = parse_value(&key, embedded, &mut args)?,
            "--border" => parsed.border = parse_value(&key, embedded, &mut args)?,
            "--auto-style-mode" => {
                let v = take_value(&key, embedded, &mut args)?;
                parsed.auto_style_mode = v.parse()?;
            }
            "--background" => {
                parsed.background = take_value(&key, embedded, &mut args)?;
            }
            "--seed" => parsed.seed = parse_value(&key, embedded, &mut args)?,
            "--add-metadata" => parsed.add_metadata = true,
            "--loop-limit" => parsed.loop_limit = parse_value(&key, embedded, &mut args)?,
            "--var-limit" => parsed.var_limit = parse_value(&key, embedded, &mut args)?,
            "--depth-limit" => parsed.depth_limit = parse_value(&key, embedded, &mut args)?,
            "--path-repeat-limit" => {
                parsed.path_repeat_limit = parse_value(&key, embedded, &mut args)?;
            }
            "--font-size" => parsed.font_size = parse_value(&key, embedded, &mut args)?,
            "--font-family" => {
                parsed.font_family = take_value(&key, embedded, &mut args)?;
            }
            "--theme" => {
                let v = take_value(&key, embedded, &mut args)?;
                parsed.theme = v.parse()?;
            }
            "--svg-style" => {
                parsed.svg_style = Some(take_value(&key, embedded, &mut args)?);
            }
            "--error-mode" => {
                let v = take_value(&key, embedded, &mut args)?;
                parsed.error_mode = v.parse()?;
            }
            "-D" | "--var" => {
                parsed.vars.push(take_value(&key, embedded, &mut args)?);
            }
            _ => return Err(Error::Cli(format!("unknown argument: '{key}'"))),
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
