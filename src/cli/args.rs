use crate::errors::{Error, Result};
use crate::{AutoStyleMode, ErrorMode, ThemeType};
use std::str::FromStr;

pub const USAGE: &str = concat!(
    "svgdx ",
    env!("CARGO_PKG_VERSION"),
    r#" - create SVG diagrams easily

Usage: svgdx [OPTIONS] [FILE]

Arguments:
  [FILE]  File to process ('-' for stdin) [default: -]

Options:
  -o, --output <FILE>              Target output file ('-' for stdout)
            [default: '-', i.e. stdout]
      --debug                      Add debug info (e.g. input source) to output
      --scale <SCALE>              Scale of user-units to mm for root svg element
            [default: 1.0]
      --border <BORDER>            Border width around image in user-units
            [default: 5]
      --auto-style-mode <MODE>     Auto-style mode: none, inline, css
            [default: 'css']
      --background <COLOUR>        Default background colour
            [default: 'default']
      --seed <SEED>                Seed for RNG functions
            [default: 0]
      --add-metadata               Include metadata in output
      --loop-limit <N>             Limit on loop element iterations
            [default: 1000]
      --var-limit <N>              Limit on length of variable values
            [default: 1024]
      --depth-limit <N>            Recursion depth limit
            [default: 100]
      --path-repeat-limit <N>      Path repeat expansion limit
            [default: 10000]
      --font-size <SIZE>           Default font-size in user-units
            [default: 3.0]
      --font-family <FAMILY>       Default font-family for text elements
            [default: 'sans-serif']
      --theme <THEME>              Theme: default, bold, fine, glass, light, dark
            [default: 'default']
      --svg-style <STYLE>          Optional style to apply to SVG root element
      --error-mode <MODE>          Error handling: strict, warn, ignore
            [default: 'strict']
  -D, --var <KEY=VALUE>            Variable key=value pairs (may be repeated)
  -h, --help                       Show this help
  -V, --version                    Display program version
"#
);

pub struct Args {
    pub file: String,
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
            file: "-".to_string(),
            output: "-".to_string(),
            debug: false,
            scale: 1.0,
            border: 5,
            auto_style_mode: AutoStyleMode::default(),
            background: "default".to_string(),
            seed: 0,
            add_metadata: false,
            loop_limit: 1000,
            var_limit: 1024,
            depth_limit: 100,
            path_repeat_limit: 10000,
            font_size: 3.0,
            font_family: "sans-serif".to_string(),
            theme: ThemeType::default(),
            svg_style: None,
            error_mode: ErrorMode::default(),
            vars: vec![],
        }
    }
}

pub enum CliAction {
    Help,
    Version,
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

pub fn parse_args(args: impl Iterator<Item = String>) -> Result<CliAction> {
    let mut args = args.peekable();
    let _ = args.next(); // skip argv[0]

    let mut parsed = Args::default();
    let mut got_file = false;

    while let Some(arg) = args.next() {
        // Support --flag=value style by splitting on the first '='
        let (key, embedded): (String, Option<String>) = match arg.split_once('=') {
            Some((k, v)) if k.starts_with('-') => (k.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };

        match key.as_str() {
            "--" => {
                // End of options; consume the next arg as the positional file
                if let Some(file) = args.next() {
                    parsed.file = file;
                }
                break;
            }
            "-h" | "--help" => {
                return Ok(CliAction::Help);
            }
            "-V" | "--version" => {
                return Ok(CliAction::Version);
            }
            "-o" | "--output" => {
                parsed.output = take_value(&key, embedded, &mut args)?;
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
            _ if !key.starts_with('-') && !got_file => {
                parsed.file = key;
                got_file = true;
            }
            _ => return Err(Error::Cli(format!("unknown argument: '{key}'"))),
        }
    }

    Ok(CliAction::Run(parsed))
}
