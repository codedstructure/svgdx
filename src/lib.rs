//! ## svgdx - create SVG diagrams easily
//!
//! `svgdx` is normally run as a command line tool, taking an input file and processing
//! it into an SVG output file.
//!
//! ## Library use
//!
//! Support as a library is primarily to allow other front-ends to convert svgdx
//! documents to SVG without having to call `svgdx` as a command-line subprocess.
//!
//! A `TransformConfig` object should be created as appropriate to configure the
//! transform process, and the appropriate `transform_*` function called passing
//! this and appropriate input / output parameters as required.
//!
//! Errors in processing are handled via `anyhow::Result`; currently these are mainly
//! useful in providing basic error messages suitable for end-users.
//!
//! ## Example
//!
//! ```
//! let cfg = svgdx::TransformConfig::default();
//!
//! let input = r#"<rect wh="50" text="Hello!"/>"#;
//! let output = svgdx::transform_str(input, &cfg).unwrap();
//!
//! println!("{output}");
//! ```

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

/// Settings to configure a single transformation.
///
/// Note the settings here are specific to a single transformation; alternate front-ends
/// may use this directly rather than `Config` which wraps this struct when `svgdx` is
/// run as a command-line program.
#[derive(Clone, Debug)]
pub struct TransformConfig {
    /// Add debug info (e.g. input source) to output
    pub debug: bool,
    /// Overall output image scale (in mm as scale of user units)
    pub scale: f32,
    /// Border width (user-units, default 5)
    pub border: u16,
    /// Add style & defs entries based on class usage
    pub add_auto_defs: bool,
    /// Background colour (default "none")
    pub background: String,
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self {
            debug: false,
            scale: 1.0,
            border: 5,
            add_auto_defs: true,
            background: "none".to_owned(),
        }
    }
}

/// Reads from the `reader` stream, processes document, and writes to `writer`.
///
/// Note the entire stream may be read before any converted data is written to `writer`.
///
/// The transform can be modified by providing a suitable `TransformConfig` value.
pub fn transform_stream(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    config: &TransformConfig,
) -> Result<()> {
    let mut t = Transformer::from_config(config);
    t.transform(reader, writer)
}

/// Read file from `input` ('-' for stdin), process the result,
/// and write to file given by `output` ('-' for stdout).
///
/// The transform can be modified by providing a suitable `TransformConfig` value.
pub fn transform_file(input: &str, output: &str, cfg: &TransformConfig) -> Result<()> {
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
        transform_stream(&mut in_reader, &mut std::io::stdout(), cfg)?;
    } else {
        let mut out_temp = NamedTempFile::new()?;
        transform_stream(&mut in_reader, &mut out_temp, cfg)?;
        // Copy content rather than rename (by .persist()) since this
        // could cross filesystems; some apps (e.g. eog) also fail to
        // react to 'moved-over' files.
        fs::copy(out_temp.path(), output)?;
    }

    Ok(())
}

/// Transform `input` provided as a string, returning the result as a string.
///
/// The transform can be modified by providing a suitable `TransformConfig` value.
pub fn transform_str<T: Into<String>>(input: T, cfg: &TransformConfig) -> Result<String> {
    let input = input.into();

    let mut input = Cursor::new(input);
    let mut output: Vec<u8> = vec![];

    transform_stream(&mut input, &mut output, cfg)?;

    Ok(String::from_utf8(output).expect("Non-UTF8 output generated"))
}

/// Transform the provided `input` string using default config, returning the result string.
///
/// Uses default `TransformConfig` settings.
pub fn transform_str_default<T: Into<String>>(input: T) -> Result<String> {
    transform_str(input, &TransformConfig::default())
}

/// Command line arguments
#[derive(Parser)]
#[command(author, version, about, long_about=None)] // Read from Cargo.toml
struct Arguments {
    /// File to process ('-' for stdin)
    #[arg(default_value = "-")]
    file: String,

    /// Target output file ('-' for stdout)
    #[arg(short, long, default_value = "-")]
    output: String,

    /// Watch file for changes; update output on change. (FILE must be given)
    #[arg(short, long, requires = "file")]
    watch: bool,

    /// Add debug info (e.g. input source) to output
    #[arg(long)]
    debug: bool,

    /// Scale of user-units to mm for root svg element width/height
    #[arg(long, default_value = "1.0")]
    scale: f32,

    /// Border width around image (user-units)
    #[arg(long, default_value = "5")]
    border: u16,

    /// Don't add referenced styles automatically
    #[arg(long)]
    no_auto_style: bool,

    /// Default background colour if auto-styles are active
    #[arg(long, default_value = "none")]
    background: String,
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
    /// Stay monitoring `input_path` for changes (Requires input_path is not stdin)
    pub watch: bool,
    /// transform config options
    pub transform: TransformConfig,
}

impl Config {
    pub(crate) fn from_args(args: Arguments) -> Result<Self> {
        if args.watch && args.file == "-" {
            // Should already be enforced by clap validation
            bail!("A non-stdin file must be provided with -w/--watch argument");
        }
        Ok(Self {
            input_path: args.file,
            output_path: args.output,
            watch: args.watch,
            transform: TransformConfig {
                debug: args.debug,
                scale: args.scale,
                border: args.border,
                add_auto_defs: !args.no_auto_style,
                background: args.background,
            },
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

/// Create a `Config` object from process arguments.
pub fn get_config() -> Result<Config> {
    let args = Arguments::parse();
    Config::from_args(args)
}

/// Run the `svgdx` program with a given `Config`.
pub fn run(config: Config) -> Result<()> {
    if !config.watch {
        transform_file(&config.input_path, &config.output_path, &config.transform)?;
    } else if config.input_path != "-" {
        let watch = config.input_path;
        let (tx, rx) = channel();
        let mut watcher =
            new_debouncer(Duration::from_millis(250), tx).expect("Could not create watcher");
        let watch_path = Path::new(&watch);
        watcher
            .watcher()
            .watch(Path::new(&watch), RecursiveMode::NonRecursive)?;
        transform_file(&watch, &config.output_path, &config.transform).unwrap_or_else(|e| {
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
                            transform_file(&watch, &config.output_path, &config.transform)
                                .unwrap_or_else(|e| {
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
