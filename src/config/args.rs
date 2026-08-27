use std::fs;
use std::str::FromStr;

use crate::errors::{Error, Result};
use crate::{AutoStyleMode, ErrorMode, ThemeType, TransformConfig, VarName};

use crate::constants::{
    DEFAULT_BACKGROUND, DEFAULT_BORDER, DEFAULT_DEPTH_LIMIT, DEFAULT_FONT_FAMILY,
    DEFAULT_FONT_SIZE, DEFAULT_LOOP_LIMIT, DEFAULT_PATH_REPEAT_LIMIT, DEFAULT_RNG_SEED,
    DEFAULT_SCALE, DEFAULT_VAR_LIMIT,
};

#[derive(Debug, PartialEq)]
pub struct TransformArgs {
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
    pub vars: Vec<VarSpec>,
    pub include_files: Vec<String>,
}

impl Default for TransformArgs {
    fn default() -> Self {
        Self {
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
            include_files: vec![],
        }
    }
}

impl TryFrom<TransformArgs> for TransformConfig {
    type Error = Error;

    fn try_from(args: TransformArgs) -> Result<Self> {
        let mut config = Self {
            debug: args.debug,
            scale: args.scale,
            border: args.border,
            auto_style_mode: args.auto_style_mode,
            background: args.background,
            seed: args.seed,
            add_metadata: args.add_metadata,
            loop_limit: args.loop_limit,
            var_limit: args.var_limit,
            depth_limit: args.depth_limit,
            path_repeat_limit: args.path_repeat_limit,
            font_size: args.font_size,
            font_family: args.font_family,
            theme: args.theme,
            svg_style: args.svg_style,
            error_mode: args.error_mode,
            vars: args.vars.into_iter().map(|v| (v.key, v.value)).collect(),
            library_sources: vec![],
        };

        for file in &args.include_files {
            let content = fs::read_to_string(file)
                .map_err(|e| Error::Document(format!("read {file:?}: {e}")))?;
            config.load_library(&content)?;
        }

        Ok(config)
    }
}

pub fn common_usage() -> String {
    let default_theme = ThemeType::default().to_string();
    let themes_str = ThemeType::variants().join(", ");

    let default_error_mode = ErrorMode::default().to_string();
    let error_modes_str = ErrorMode::variants().join(", ");

    let default_auto_style_mode = AutoStyleMode::default().to_string();
    let auto_style_modes_str = AutoStyleMode::variants().join(", ");

    format!(
        r#"      --debug                   Add debug info (e.g. input source) to output
      --scale <SCALE>           User-units per mm for root SVG element [{DEFAULT_SCALE}]
      --border <BORDER>         Border width around image in user-units [{DEFAULT_BORDER}]
      --auto-style-mode <MODE>  Auto-style mode ['{default_auto_style_mode}']
                                ({auto_style_modes_str})
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
                                ({themes_str})
      --svg-style <STYLE>       Optional style to apply to SVG root element
      --error-mode <MODE>       Error handling mode ['{default_error_mode}']
                                ({error_modes_str})
  -D, --var <KEY=VALUE>         Variable key=value pairs (may be repeated)
      --include <FILE>          Include library file (may be repeated)"#
    )
}

impl TransformArgs {
    pub fn handle_arg(
        &mut self,
        key: &str,
        embedded: Option<String>,
        args: &mut impl Iterator<Item = String>,
    ) -> Result<bool> {
        match key {
            "--debug" => self.debug = true,
            "--scale" => self.scale = parse_value(key, embedded, args)?,
            "--border" => self.border = parse_value(key, embedded, args)?,
            "--auto-style-mode" => {
                let v = take_value(key, embedded, args)?;
                self.auto_style_mode = v.parse()?;
            }
            "--background" => {
                self.background = take_value(key, embedded, args)?;
            }
            "--seed" => self.seed = parse_value(key, embedded, args)?,
            "--add-metadata" => self.add_metadata = true,
            "--loop-limit" => self.loop_limit = parse_value(key, embedded, args)?,
            "--var-limit" => self.var_limit = parse_value(key, embedded, args)?,
            "--depth-limit" => self.depth_limit = parse_value(key, embedded, args)?,
            "--path-repeat-limit" => {
                self.path_repeat_limit = parse_value(key, embedded, args)?;
            }
            "--font-size" => self.font_size = parse_value(key, embedded, args)?,
            "--font-family" => {
                self.font_family = take_value(key, embedded, args)?;
            }
            "--theme" => {
                let v = take_value(key, embedded, args)?;
                self.theme = v.parse()?;
            }
            "--svg-style" => {
                self.svg_style = Some(take_value(key, embedded, args)?);
            }
            "--error-mode" => {
                let v = take_value(key, embedded, args)?;
                self.error_mode = v.parse()?;
            }
            "-D" | "--var" => {
                self.vars.push(take_value(key, embedded, args)?.parse()?);
            }
            "--include" => {
                self.include_files.push(take_value(key, embedded, args)?);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

pub fn parse_kv_arg(arg: &str) -> (String, Option<String>) {
    // Support --flag=value style by splitting on the first '='
    match arg.split_once('=') {
        Some((k, v)) if k.starts_with('-') => (k.to_string(), Some(v.to_string())),
        _ => (arg.to_string(), None),
    }
}

pub fn take_value(
    flag: &str,
    embedded: Option<String>,
    args: &mut impl Iterator<Item = String>,
) -> Result<String> {
    embedded
        .or_else(|| args.next())
        .ok_or_else(|| Error::Cli(format!("'{flag}' requires a value")))
}

pub fn parse_value<T>(
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VarSpec {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_spec_parse() {
        let s = "THING=ONE";
        let var_spec: VarSpec = s.parse().unwrap();
        assert_eq!(var_spec.key.to_string(), "THING");
        assert_eq!(var_spec.value, "ONE");

        let s = "THINGONE";
        let result: Result<VarSpec> = s.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_arg_parse() {
        for (args, expected) in &[
            (
                "--debug --theme fine --add-metadata",
                TransformArgs {
                    debug: true,
                    theme: ThemeType::Fine,
                    add_metadata: true,
                    ..Default::default()
                },
            ),
            (
                "--depth-limit=10 --var-limit 100",
                TransformArgs {
                    depth_limit: 10,
                    var_limit: 100,
                    ..Default::default()
                },
            ),
            (
                "-D thing=one -D other=two",
                TransformArgs {
                    vars: vec![
                        VarSpec {
                            key: "thing".parse().unwrap(),
                            value: "one".to_string(),
                        },
                        VarSpec {
                            key: "other".parse().unwrap(),
                            value: "two".to_string(),
                        },
                    ],
                    ..Default::default()
                },
            ),
        ] {
            let mut args = args.split_whitespace().map(String::from);
            let mut ta = TransformArgs::default();
            while let Some(arg) = args.next() {
                let (key, embedded) = parse_kv_arg(&arg);
                ta.handle_arg(&key, embedded, &mut args).unwrap();
            }

            assert_eq!(ta, *expected);
        }
    }
}
