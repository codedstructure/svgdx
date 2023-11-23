use anyhow::Result;
use clap::{Args, Parser};
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

use svgd::svg_transform;

/// Transform given file to SVG
#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Arguments {
    #[command(flatten)]
    input_type: InputArgs,

    /// target output file; omit for stdout
    #[arg(short, long)]
    output: Option<String>,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
struct InputArgs {
    /// input file to read
    #[arg(short, long, group = "input-type")]
    input: Option<String>,

    /// file to watch for changes
    #[arg(short, long, group = "input-type")]
    watch: Option<String>,
}

fn transform(input: &str, output: Option<String>) -> Result<()> {
    let mut in_reader = Box::new(BufReader::new(File::open(input)?));

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

fn main() -> Result<()> {
    let args = Arguments::parse();
    let input_type = args.input_type;

    if let Some(input) = input_type.input {
        transform(&input, args.output.clone())?;
    } else if let Some(watch) = input_type.watch {
        let (tx, rx) = channel();
        let mut watcher =
            new_debouncer(Duration::from_millis(250), tx).expect("Could not create watcher");
        let watch_path = Path::new(&watch);
        watcher
            .watcher()
            .watch(Path::new(&watch), RecursiveMode::NonRecursive)?;
        transform(&watch, args.output.clone()).unwrap_or_else(|e| {
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
                            transform(&watch, args.output.clone()).unwrap_or_else(|e| {
                                eprintln!("transform failed: {e:?}");
                            });
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
