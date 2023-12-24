//! # svgdx
//!
//! Entry point for `svgdx` when used as a library.
//!
//! `svgdx` is normally run as a command line tool. Support as a library
//! is currently limited, but it is possible to use the key `transform`
//! functionality which converts an input wrapped in a `Reader` to a
//! corresponding `Writer`. This allows use of the tool without needing
//! to install or run `svgdx` as a command line subprocess.

use std::{
    io::{BufRead, Cursor, IsTerminal, Read, Write},
    num::ParseFloatError,
};

use anyhow::{bail, Result};
use clap::Parser;
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use std::{
    fs::{self, File},
    io::BufReader,
    path::Path,
    sync::mpsc::channel,
    time::Duration,
};
use tempfile::NamedTempFile;

mod transform;
mod types;

pub(crate) use transform::Transformer;
mod connector;
mod custom;
mod element;
mod expression;
mod svg_defs;
mod text;

/// The main entry point once any command line or other initialisation
/// has been completed.
///
/// Reads from the `reader` stream, performs processing and conversion
/// as required, and writes to the `writer`. Note the entire stream may
/// be read before any converted data is written to `writer`.
pub fn svg_transform(reader: &mut dyn BufRead, writer: &mut dyn Write) -> Result<()> {
    let mut t = Transformer::new();
    t.transform(reader, writer)
}

/// Return a 'minimal' representation of the given number
fn fstr(x: f32) -> String {
    if x == (x as u32) as f32 {
        return (x as u32).to_string();
    }
    let result = format!("{x:.3}");
    if result.contains('.') {
        result.trim_end_matches('0').trim_end_matches('.').into()
    } else {
        result
    }
}

/// Parse a string to an f32
fn strp(s: &str) -> Result<f32> {
    s.parse().map_err(|e: ParseFloatError| e.into())
}

/// Returns iterator over whitespace-or-comma separated values
fn attr_split(input: &str) -> impl Iterator<Item = String> + '_ {
    input
        .split_whitespace()
        .flat_map(|v| v.split(','))
        .map(|v| v.to_string())
}

/// Returns iterator *cycling* over whitespace-or-comma separated values
fn attr_split_cycle(input: &str) -> impl Iterator<Item = String> + '_ {
    let x: Vec<String> = attr_split(input).collect();
    x.into_iter().cycle()
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
enum Length {
    Absolute(f32),
    Ratio(f32),
}

impl Default for Length {
    fn default() -> Self {
        Self::Absolute(0.)
    }
}

impl Length {
    #[allow(dead_code)]
    const fn ratio(&self) -> Option<f32> {
        if let Self::Ratio(result) = self {
            Some(*result)
        } else {
            None
        }
    }

    const fn absolute(&self) -> Option<f32> {
        if let Self::Absolute(result) = self {
            Some(*result)
        } else {
            None
        }
    }

    /// Given a single value, update it (scale or addition) from
    /// the current Length value
    fn adjust(&self, value: f32) -> f32 {
        match self {
            Self::Absolute(abs) => value + abs,
            Self::Ratio(ratio) => value * ratio,
        }
    }

    /// Given a range, return a value (typically) in the range
    /// where a positive Absolute is 'from start', a negative Absolute
    /// is 'backwards from end' and Ratios scale as 0%=start, 100%=end
    /// but ratio values are not limited to 0..100 at either end.
    fn calc_offset(&self, start: f32, end: f32) -> f32 {
        match self {
            Self::Absolute(abs) => {
                let mult = if end < start { -1. } else { 1. };
                if abs < &0. {
                    // '+' here since abs is negative and
                    // we're going 'back' from the end.
                    end + abs * mult
                } else {
                    start + abs * mult
                }
            }
            Self::Ratio(ratio) => start + (end - start) * ratio,
        }
    }
}

/// Parse a ratio (float or %age) to an f32
/// Note this deliberately does not clamp to 0..1
fn strp_length(s: &str) -> Result<Length> {
    if let Some(s) = s.strip_suffix('%') {
        Ok(Length::Ratio(strp(s)? * 0.01))
    } else {
        Ok(Length::Absolute(strp(s)?))
    }
}

/// Transform given file to SVG
#[derive(Parser)]
#[command(author, version, about, long_about=None)] // Read from Cargo.toml
struct Arguments {
    /// file to process (defaults to stdin)
    file: Option<String>,

    /// watch file for changes; update output on change. (FILE must be given)
    #[arg(short, long, requires = "file")]
    watch: bool,

    /// target output file; omit for stdout
    #[arg(short, long)]
    output: Option<String>,
}

fn transform(input: Option<String>, output: Option<String>) -> Result<()> {
    let mut in_reader = match input {
        None => {
            let mut stdin = std::io::stdin().lock();
            if stdin.is_terminal() {
                // This is unpleasant; at least on Mac, a single Ctrl-D is not otherwise
                // enough to signal end-of-input, even when given at the start of a line.
                // Work around this by reading entire input, then wrapping in a Cursor to
                // provide a buffered reader.
                // It would be nice to improve this.
                let mut buf = Vec::new();
                stdin.read_to_end(&mut buf).unwrap();
                Box::new(BufReader::new(Cursor::new(buf))) as Box<dyn BufRead>
            } else {
                Box::new(stdin) as Box<dyn BufRead>
            }
        }
        Some(x) => Box::new(BufReader::new(File::open(x)?)) as Box<dyn BufRead>,
    };

    match output {
        Some(x) => {
            let mut out_temp = NamedTempFile::new()?;
            svg_transform(&mut in_reader, &mut out_temp)?;
            // Copy content rather than rename (by .persist()) since this
            // could cross filesystems; some apps (e.g. eog) also fail to
            // react to 'moved-over' files.
            fs::copy(out_temp.path(), x)?;
        }
        None => {
            svg_transform(&mut in_reader, &mut std::io::stdout())?;
        }
    }

    Ok(())
}

/// Configuration used by svgdx
pub struct Config {
    input: Option<String>,
    output: Option<String>,
    watch: bool,
}

impl Config {
    pub(crate) fn from_args(args: Arguments) -> Result<Self> {
        if args.watch && args.file.is_none() {
            // Should already be enforced by clap validation
            bail!("Filename must be provided with -w/--watch argument");
        }
        Ok(Config {
            input: args.file,
            output: args.output,
            watch: args.watch,
        })
    }

    /// Create a `Config` object set up given a command line string.
    ///
    /// The string is parsed using `shlex::split()`, so values containing
    /// spaces or quotes should be quoted or escaped appropriately.
    pub fn from_cmdline(args: &str) -> Result<Self> {
        let args = shlex::split(args).unwrap_or_default();
        let args = Arguments::try_parse_from(args.iter())?;
        Self::from_args(args)
    }
}

/// Create a `Config` object from arguments given to this process
pub fn get_config() -> Result<Config> {
    let args = Arguments::parse();
    Config::from_args(args)
}

/// Start running svgdx with a given `Config`
pub fn run(config: Config) -> Result<()> {
    if !config.watch {
        transform(config.input.clone(), config.output.clone())?;
    } else if let Some(watch) = config.input {
        let (tx, rx) = channel();
        let mut watcher =
            new_debouncer(Duration::from_millis(250), tx).expect("Could not create watcher");
        let watch_path = Path::new(&watch);
        watcher
            .watcher()
            .watch(Path::new(&watch), RecursiveMode::NonRecursive)?;
        transform(Some(watch.clone()), config.output.clone()).unwrap_or_else(|e| {
            eprintln!("transform failed: {e:?}");
        });
        eprintln!("Watching {} for changes", watch);
        loop {
            match rx.recv() {
                Ok(Ok(events)) => {
                    for event in events {
                        if event.path.canonicalize().unwrap() == watch_path.canonicalize().unwrap()
                        {
                            eprintln!("{} changed", event.path.to_string_lossy());
                            transform(Some(watch.clone()), config.output.clone()).unwrap_or_else(
                                |e| {
                                    eprintln!("transform failed: {e:?}");
                                },
                            );
                        }
                    }
                }
                Ok(Err(e)) => eprintln!("Watch error {:?}", e),
                Err(e) => eprintln!("Channel error: {:?}", e),
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fstr() {
        assert_eq!(fstr(1.0), "1");
        assert_eq!(fstr(-100.0), "-100");
        assert_eq!(fstr(1.2345678), "1.235");
        assert_eq!(fstr(-1.2345678), "-1.235");
        assert_eq!(fstr(91.0004), "91");
        // Large-ish integers (up to 24 bit mantissa) should be fine
        assert_eq!(fstr(12345678.0), "12345678");
        assert_eq!(fstr(12340000.0), "12340000");
    }

    #[test]
    fn test_strp() {
        assert_eq!(strp("1").ok(), Some(1.));
        assert_eq!(strp("100").ok(), Some(100.));
        assert_eq!(strp("-100").ok(), Some(-100.));
        assert_eq!(strp("-0.00123").ok(), Some(-0.00123));
        assert_eq!(strp("1234567.8").ok(), Some(1234567.8));
    }

    #[test]
    fn test_strp_length() {
        assert_eq!(strp_length("1").ok(), Some(Length::Absolute(1.)));
        assert_eq!(strp_length("123").ok(), Some(Length::Absolute(123.)));
        assert_eq!(strp_length("-0.0123").ok(), Some(Length::Absolute(-0.0123)));
        assert_eq!(strp_length("0.5%").ok(), Some(Length::Ratio(0.005)));
        assert_eq!(strp_length("150%").ok(), Some(Length::Ratio(1.5)));
        assert_eq!(strp_length("1.2.3").ok(), None);
        assert_eq!(strp_length("a").ok(), None);
        assert_eq!(strp_length("a%").ok(), None);
    }

    #[test]
    fn test_length_calc_offset() {
        assert_eq!(strp_length("25%").expect("test").calc_offset(10., 50.), 20.);
        assert_eq!(
            strp_length("50%").expect("test").calc_offset(-10., -9.),
            -9.5
        );
        assert_eq!(
            strp_length("200%").expect("test").calc_offset(10., 50.),
            90.
        );
        assert_eq!(
            strp_length("-3.5").expect("test").calc_offset(10., 50.),
            46.5
        );
        assert_eq!(
            strp_length("3.5").expect("test").calc_offset(-10., 90.),
            -6.5
        );
    }

    #[test]
    fn test_length_adjust() {
        assert_eq!(strp_length("25%").expect("test").adjust(10.), 2.5);
        assert_eq!(strp_length("-50%").expect("test").adjust(150.), -75.);
        assert_eq!(strp_length("125%").expect("test").adjust(20.), 25.);
        assert_eq!(strp_length("1").expect("test").adjust(23.), 24.);
        assert_eq!(strp_length("-12").expect("test").adjust(123.), 111.);
    }
}
