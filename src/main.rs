use clap::Parser;
use std::fs;

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

    let mut t = Transformer::new(&input_file_path, &output_file_path);
    let _ = t.transform();
}
