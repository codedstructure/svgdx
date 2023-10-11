use clap::Parser;
use std::{
    fs::{self, File},
    io::{BufReader, Write},
};

use svgd::Transformer;

fn path_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    input_file_path: String,

    #[arg(short, long)]
    output_file_path: Option<String>,
}

fn main() {
    let args = Args::parse();
    let input_file_path: String = args.input_file_path;
    let output_file_path: Option<String> = args.output_file_path;

    if !path_exists(&input_file_path) {
        panic!("File does not exist");
    }

    let mut out_writer = match output_file_path {
        Some(x) => {
            let path = std::path::Path::new(&x);
            Box::new(File::create(path).unwrap()) as Box<dyn Write>
        }
        None => Box::new(std::io::stdout()) as Box<dyn Write>,
    };

    let mut in_reader = Box::new(BufReader::new(File::open(input_file_path).unwrap()));

    let mut t = Transformer::new();
    let _ = t.transform(&mut in_reader, &mut out_writer);
}
