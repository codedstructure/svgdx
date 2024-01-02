//! # svgdx
//!
//! Entry point for `svgdx` when used as a library.
//!
//! `svgdx` is normally run as a command line tool. Support as a library
//! is currently limited, but it is possible to use the key `transform`
//! functionality which converts an input wrapped in a `Reader` to a
//! corresponding `Writer`. This allows use of the tool without needing
//! to install or run `svgdx` as a command line subprocess.

use std::io::{BufRead, Cursor, IsTerminal, Read, Write};

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

mod connector;
mod custom;
mod element;
mod expression;
mod svg_defs;
mod text;
mod transform;
mod types;

use transform::Transformer;

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

/// Read file from `input` ('-' for stdin), process the result,
/// and write to file given by `output` ('-' for stdout).
pub fn transform_file(input: &str, output: &str) -> Result<()> {
    let mut in_reader = if input == "-" {
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
    } else {
        Box::new(BufReader::new(File::open(input)?)) as Box<dyn BufRead>
    };

    if output == "-" {
        svg_transform(&mut in_reader, &mut std::io::stdout())?;
    } else {
        let mut out_temp = NamedTempFile::new()?;
        svg_transform(&mut in_reader, &mut out_temp)?;
        // Copy content rather than rename (by .persist()) since this
        // could cross filesystems; some apps (e.g. eog) also fail to
        // react to 'moved-over' files.
        fs::copy(out_temp.path(), output)?;
    }

    Ok(())
}

pub fn transform_str<T: Into<String>>(input: T) -> Result<String> {
    let input = input.into();

    let mut input = Cursor::new(input);
    let mut output: Vec<u8> = vec![];

    svg_transform(&mut input, &mut output)?;

    Ok(String::from_utf8(output).expect("Non-UTF8 output generated"))
}

/// Transform given file to SVG
#[derive(Parser)]
#[command(author, version, about, long_about=None)] // Read from Cargo.toml
struct Arguments {
    /// File to process ('-' for stdin)
    #[arg(default_value = "-")]
    file: String,

    /// Watch file for changes; update output on change. (FILE must be given)
    #[arg(short, long, requires = "file")]
    watch: bool,

    /// Target output file ('-' for stdout)
    #[arg(short, long, default_value = "-")]
    output: String,

    /// Scale of user-units to mm for root svg element width/height
    #[arg(long, default_value = "2")]
    scale: f32,

    /// Don't add referenced styles automatically
    #[arg(long)]
    no_auto_style: bool,

    /// Add debug info (e.g. input source) to output
    #[arg(long)]
    debug: bool,
}

/// Configuration used by svgdx
pub struct Config {
    /// Path to input file, or '-' for stdin
    pub input_path: String,
    /// Path to output file, or '-' for stdout
    pub output_path: String,
    /// Stay monitoring `input_path` for changes (Requires input_path is not stdin)
    pub watch: bool,
    /// Add debug info (e.g. input source) to output
    pub debug: bool,
    /// Add style & defs entries based on class usage
    pub add_auto_defs: bool,
    /// Overall output image scale (in mm as scale of user units)
    pub scale: f32,
}

impl Config {
    pub(crate) fn new() -> Self {
        Self {
            input_path: String::from("-"),
            output_path: String::from("-"),
            watch: false,
            debug: false,
            add_auto_defs: true,
            scale: 1.0,
        }
    }

    pub(crate) fn from_args(args: Arguments) -> Result<Self> {
        if args.watch && args.file == "-" {
            // Should already be enforced by clap validation
            bail!("A non-stdin file must be provided with -w/--watch argument");
        }
        Ok(Config {
            input_path: args.file,
            output_path: args.output,
            watch: args.watch,
            debug: args.debug,
            add_auto_defs: !args.no_auto_style,
            scale: args.scale,
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
        transform_file(&config.input_path, &config.output_path)?;
    } else if config.input_path != "-" {
        let watch = config.input_path;
        let (tx, rx) = channel();
        let mut watcher =
            new_debouncer(Duration::from_millis(250), tx).expect("Could not create watcher");
        let watch_path = Path::new(&watch);
        watcher
            .watcher()
            .watch(Path::new(&watch), RecursiveMode::NonRecursive)?;
        transform_file(&watch, &config.output_path).unwrap_or_else(|e| {
            eprintln!("transform failed: {e:?}");
        });
        eprintln!("Watching {watch} for changes");
        loop {
            match rx.recv() {
                Ok(Ok(events)) => {
                    for event in events {
                        if event.path.canonicalize().unwrap() == watch_path.canonicalize().unwrap()
                        {
                            eprintln!("{} changed", event.path.to_string_lossy());
                            transform_file(&watch, &config.output_path).unwrap_or_else(|e| {
                                eprintln!("transform failed: {e:?}");
                            });
                        }
                    }
                }
                Ok(Err(e)) => eprintln!("Watch error {e:?}"),
                Err(e) => eprintln!("Channel error: {e:?}"),
            }
        }
    }

    Ok(())
}
