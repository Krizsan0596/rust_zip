mod util;
use util::{Config, process_args, print_usage};

mod file;
use file::{open_file, get_chunk};
use std::fs::File;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name: String = args[0].clone();
    let opts: Config = process_args(args);

    let mut file: File = match open_file(&opts.input_file) {
        Ok(file) => file,
        Err(_e) => {
            eprintln!("Input file does not exist!");
            print_usage(&name);
            std::process::exit(1);
        }
    };

    let chunk: Vec<u8> = get_chunk(&mut file);
}
