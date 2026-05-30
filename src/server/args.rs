use std::net::IpAddr;

pub const DEFAULT_ADDRESS: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 3003;
pub const DEFAULT_OPEN: bool = false;

/// Command line arguments
#[derive(Debug)]
pub struct Args {
    /// Address to listen on
    pub address: IpAddr,

    /// Port to listen on
    pub port: u16,

    /// Open browser on startup
    pub open: bool,
}

#[derive(Debug)]
pub enum CliAction {
    Help,
    Version,
    Run(Args),
}

impl Default for Args {
    fn default() -> Self {
        Self {
            address: DEFAULT_ADDRESS
                .parse()
                .expect("default address should be valid"),
            port: DEFAULT_PORT,
            open: DEFAULT_OPEN,
        }
    }
}

pub fn usage(program_name: &str) -> String {
    format!(
        r#"
Usage:
  {program_name} [OPTIONS]

Options:
      --address <ADDRESS>  Address to listen on ['{DEFAULT_ADDRESS}']
  -p, --port <PORT>        Port to listen on [{DEFAULT_PORT}]
      --open               Open browser on startup
  -h, --help               Show this help
  -V, --version            Display program version
"#
    )
}

fn take_value(
    flag: &str,
    embedded: Option<String>,
    args: &mut impl Iterator<Item = String>,
) -> std::result::Result<String, String> {
    embedded
        .or_else(|| args.next())
        .ok_or_else(|| format!("'{flag}' requires a value"))
}

fn parse_value<T>(
    flag: &str,
    embedded: Option<String>,
    args: &mut impl Iterator<Item = String>,
) -> std::result::Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = take_value(flag, embedded, args)?;
    value.parse().map_err(|e| format!("'{flag}': {e}"))
}

pub fn parse_args(
    args: impl IntoIterator<Item = String>,
) -> std::result::Result<CliAction, String> {
    let mut args = args.into_iter();
    let _ = args.next();

    let mut parsed = Args::default();

    while let Some(arg) = args.next() {
        let (key, embedded) = match arg.split_once('=') {
            Some((k, v)) if k.starts_with('-') => (k.to_string(), Some(v.to_string())),
            _ => (arg, None),
        };

        match key.as_str() {
            "-h" | "--help" => return Ok(CliAction::Help),
            "-V" | "--version" => return Ok(CliAction::Version),
            "--address" => parsed.address = parse_value(&key, embedded, &mut args)?,
            "-p" | "--port" => parsed.port = parse_value(&key, embedded, &mut args)?,
            "--open" => parsed.open = true,
            _ => return Err(format!("unknown argument: '{key}'")),
        }
    }

    Ok(CliAction::Run(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        assert!(matches!(
            parse_args(vec!["svgdx-server".to_string(), "--help".to_string()]),
            Ok(CliAction::Help)
        ));

        match parse_args(vec![
            "svgdx-server".to_string(),
            "--address=::1".to_string(),
            "-p".to_string(),
            "4000".to_string(),
            "--open".to_string(),
        ]) {
            Ok(CliAction::Run(args)) => {
                assert_eq!(args.address, "::1".parse::<IpAddr>().unwrap());
                assert_eq!(args.port, 4000);
                assert!(args.open);
            }
            other => panic!("unexpected parse result: {other:?}"),
        }
    }
}
