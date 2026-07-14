mod util;
use util::{Config, process_args, print_usage, ArgError};

mod file;
use file::{open_file, get_chunk, create_output, write_chunk};
use std::fs::File;

mod huffman;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name: String = args[0].clone();
    let opts: Config = match process_args(args) {
        Ok(opts) => opts,
        Err(e) => {
            match e {
                ArgError::Help => {
                    print_usage(&name);
                    std::process::exit(0);
                }
                ArgError::MissingOutputArg => {
                    eprintln!("Error: -o option requires an argument");
                    print_usage(&name);
                    std::process::exit(1);
                }
                ArgError::ConflictingModes => {
                    eprintln!("Error: Conflicting modes specified (cannot compress and decompress at the same time)");
                    print_usage(&name);
                    std::process::exit(1);
                }
                ArgError::NoModeSpecified => {
                    eprintln!("Error: No mode specified (must specify either -c or -x)");
                    print_usage(&name);
                    std::process::exit(1);
                }
                ArgError::MissingInput => {
                    eprintln!("Error: Missing input file");
                    print_usage(&name);
                    std::process::exit(1);
                }
            }
        }
    };

    let mut input_file: File = match open_file(&opts.input_file) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error opening file '{}': {}", opts.input_file, e);
            std::process::exit(1);
        }
    };

    let mut output_file: File = match create_output(&opts.output_file){
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error creating file '{}': {}", opts.output_file, e);
            std::process::exit(1);
        }
    };

    let chunk: Vec<u8> = match get_chunk(&mut input_file) {
        Ok(chunk) => chunk,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

}
