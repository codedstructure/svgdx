use clap::Parser;

use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use std::{path::Path, sync::mpsc::channel, time::Duration};

use crate::errors::{Error, Result};
use crate::style::ThemeType;
use crate::{transform_file, AutoStyleMode, TransformConfig};

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
    #[deprecated(note = "use `auto-style-mode` instead")]
    no_auto_styles: bool,

    /// Make styles local to this document using CSS nesting.
    ///
    /// Useful when embedding multiple SVGs in a single document,
    /// especially if they use different themes.
    /// May not work in all SVG applications.
    #[arg(long)]
    #[deprecated(note = "use `auto-style-mode` instead")]
    use_local_styles: bool,

    /// Auto-style mode to use
    #[arg(long, default_value = "css")]
    auto_style_mode: AutoStyleMode,

    /// Default background colour if auto-styles are active
    #[arg(long, default_value = "default")]
    background: String,

    /// Seed for RNG functions, default 0
    #[arg(long, default_value = "0")]
    seed: u64,

    /// Include metadata in output
    #[arg(long)]
    add_metadata: bool,

    /// Limit on number of iterations for loop elements
    ///
    /// This helps prevent infinite loops when rendering `<loop>` elements.
    #[arg(long, default_value = "1000")]
    loop_limit: u32,

    /// Limit on length of variable values
    #[arg(long, default_value = "1024")]
    var_limit: u32,

    /// Recursion depth limit
    #[arg(long, default_value = "100")]
    depth_limit: u32,

    /// Path repeat expansion limit
    #[arg(long, default_value = "10000")]
    path_repeat_limit: u32,

    /// Default font-size (in user-units)
    ///
    /// Text size classes (such as d-text-smaller) are based on this value.
    #[arg(long, default_value = "3.0")]
    font_size: f32,

    /// Default font-family to use for text elements
    #[arg(long, default_value = "sans-serif")]
    font_family: String,

    /// Theme to use
    #[arg(long, default_value = "default")]
    theme: ThemeType,

    /// Optional style to apply to SVG root element
    #[arg(long)]
    svg_style: Option<String>,
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
    fn from_args(args: Arguments) -> Result<Self> {
        if args.watch && args.file == "-" {
            // Should already be enforced by clap validation
            return Err(Error::Cli(
                "A non-stdin file must be provided with -w/--watch argument".into(),
            ));
        }
        if args.file != "-" && args.output != "-" {
            // Arguably creating this struct shouldn't do any IO, but this is a
            // deliberate UX safety restriction on the CLI which is worth keeping
            // as high-level as possible to keep the lower level API cleaner.
            let in_path = Path::new(&args.file);
            let out_path = Path::new(&args.output);
            if out_path.exists()
                && out_path.canonicalize().map_err(Error::from_err)?
                    == in_path.canonicalize().map_err(Error::from_err)?
            {
                return Err(Error::Cli(
                    "Output path must not refer to the same file as the input file.".into(),
                ));
            }
        }
        Ok(Self {
            input_path: args.file,
            output_path: args.output,
            watch: args.watch,
            transform: TransformConfig {
                debug: args.debug,
                scale: args.scale,
                border: args.border,
                #[allow(deprecated)]
                auto_style_mode: if args.no_auto_styles {
                    AutoStyleMode::None
                } else if args.use_local_styles {
                    AutoStyleMode::Inline
                } else {
                    args.auto_style_mode
                },
                #[allow(deprecated)]
                use_local_styles: args.use_local_styles,
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
            },
        })
    }

    /// Create a `Config` object set up given a command line string.
    ///
    /// The string is parsed using `shlex::split()`, so values containing
    /// spaces or quotes should be quoted or escaped appropriately.
    pub fn from_cmdline(args: &str) -> Result<Self> {
        let args = shlex::split(args).unwrap_or_default();
        let args = Arguments::try_parse_from(args.iter()).map_err(Error::from_err)?;
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
            .watch(Path::new(&watch), RecursiveMode::NonRecursive)
            .map_err(Error::from_err)?;
        transform_file(&watch, &config.output_path, &config.transform).unwrap_or_else(|e| {
            eprintln!("transform failed: {e:?}");
        });
        eprintln!("Watching {watch} for changes");
        loop {
            match rx.recv() {
                Ok(Ok(events)) => {
                    for event in events {
                        if event.path.canonicalize().map_err(Error::Io)?
                            == watch_path.canonicalize().map_err(Error::Io)?
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
