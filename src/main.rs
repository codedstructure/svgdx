use clap::{Args, Parser};
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use std::{
    fs::{self, File},
    io::{BufReader, Write},
    path::Path,
    sync::mpsc::channel,
    time::Duration,
};

use svgd::Transformer;

fn path_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

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

fn transform(input: &str, output: Option<String>) {
    if !path_exists(input) {
        panic!("File does not exist");
    }
    let mut out_writer = match output {
        Some(x) => {
            let path = std::path::Path::new(&x);
            Box::new(File::create(path).unwrap()) as Box<dyn Write>
        }
        None => Box::new(std::io::stdout()) as Box<dyn Write>,
    };

    let mut t = Transformer::new();
    let mut in_reader = Box::new(BufReader::new(File::open(input).unwrap()));
    let _ = t.transform(&mut in_reader, &mut out_writer);
}

fn main() {
    let args = Arguments::parse();
    let input_type = args.input_type;

    if let Some(input) = input_type.input {
        transform(&input, args.output.clone());
    } else if let Some(watch) = input_type.watch {
        let (tx, rx) = channel();
        let mut watcher =
            new_debouncer(Duration::from_millis(250), tx).expect("Could not create watcher");
        let watch_path = Path::new(&watch);
        watcher
            .watcher()
            .watch(Path::new(&watch), RecursiveMode::NonRecursive)
            .unwrap_or_else(|_| panic!("Could not establish watch on {}", watch));
        eprintln!("Watching {} for changes", watch);
        loop {
            match rx.recv() {
                Ok(Ok(events)) => {
                    for event in events {
                        if event.path.canonicalize().unwrap() == watch_path.canonicalize().unwrap()
                        {
                            eprintln!("{} changed", event.path.to_string_lossy());
                            transform(&watch, args.output.clone());
                        }
                    }
                }
                Ok(Err(e)) => eprintln!("Watch error {:?}", e),
                Err(e) => eprintln!("Channel error: {:?}", e),
            }
        }
    }
}
