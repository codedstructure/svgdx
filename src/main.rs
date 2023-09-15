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

    if path_exists(&args.input_file_path) == false {
        panic!("File does not exist");
    }

    println!("{}", args.input_file_path);

    let mut t = Transformer::new(&args.input_file_path);
    let _ = t.transform();
}
