use std::net::IpAddr;

use crate::config::{TransformArgs, common_usage, parse_value, take_value};
use crate::{Error, Result};

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

    /// Redirect /docs/ requests to this URL
    pub docs_redirect_url: Option<String>,

    /// Open browser on startup
    pub open: bool,

    /// Common transform options
    pub config: TransformArgs,
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
            docs_redirect_url: None,
            open: DEFAULT_OPEN,
            config: TransformArgs::default(),
        }
    }
}

pub fn usage(program_name: &str) -> String {
    let common_usage = common_usage();
    format!(
        r#"
Usage:
  {program_name} [OPTIONS]

Options:
      --address <ADDRESS>       Address to listen on ['{DEFAULT_ADDRESS}']
  -p, --port <PORT>             Port to listen on [{DEFAULT_PORT}]
      --docs-redirect-url <URL> Redirect /docs/ requests to this URL
      --open                    Open browser on startup
  -h, --help                    Show this help
  -V, --version                 Display program version

Transform Options:
{common_usage}
"#
    )
}

pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<CliAction> {
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
            "--docs-redirect-url" => {
                parsed.docs_redirect_url = Some(take_value(&key, embedded, &mut args)?)
            }
            "--open" => parsed.open = true,
            _ => {
                if !parsed.config.handle_arg(&key, embedded, &mut args)? {
                    return Err(Error::Cli(format!("unknown argument: '{key}'")));
                }
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
        assert!(matches!(
            parse_args(vec!["svgdx-server".to_string(), "--help".to_string()]),
            Ok(CliAction::Help)
        ));

        match parse_args(vec![
            "svgdx-server".to_string(),
            "--address=::1".to_string(),
            "-p".to_string(),
            "4000".to_string(),
            "--docs-redirect-url=http://127.0.0.1:3000/".to_string(),
            "--open".to_string(),
        ]) {
            Ok(CliAction::Run(args)) => {
                assert_eq!(args.address, "::1".parse::<IpAddr>().unwrap());
                assert_eq!(args.port, 4000);
                assert_eq!(
                    args.docs_redirect_url.as_deref(),
                    Some("http://127.0.0.1:3000/")
                );
                assert!(args.open);
            }
            other => panic!("unexpected parse result: {other:?}"),
        }
    }
}
