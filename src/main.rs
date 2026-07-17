mod util;
use util::{ArgError, Config, print_usage, process_args};

mod file;
use file::{BitReader, BitWriter, HuffmanFile, create_output, get_chunk, open_file, write_chunk};
use std::fs::File;

mod huffman;
use huffman::Tree;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name: String = args[0].clone();
    let opts: Config = match process_args(args) {
        Ok(opts) => opts,
        Err(e) => match e {
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
                eprintln!(
                    "Error: Conflicting modes specified (cannot compress and decompress at the same time)"
                );
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
        },
    };

    let mut input_file: File = match open_file(&opts.input_file) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error opening file '{}': {}", opts.input_file, e);
            std::process::exit(1);
        }
    };

    let chunk: Vec<u8> = match get_chunk(&mut input_file) {
        Ok(chunk) => chunk,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", opts.input_file, e);
            std::process::exit(1);
        }
    };

    if opts.compress {
        let mut output_file: File = match create_output(&opts.output_file) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Error creating file '{}': {}", opts.output_file, e);
                std::process::exit(1);
            }
        };

        let mut tree: Tree = Tree::new();

        for byte in &chunk {
            tree.add_leaf(*byte);
        }

        tree.sort_nodes();
        if let Err(e) = tree.construct_tree() {
            eprintln!("Error while constructing Huffman tree: {}", e);
            std::process::exit(1);
        }

        let mut buffer = Vec::new();

        let mut writer = BitWriter::new(&mut buffer);
        for byte in &chunk {
            let bits: String = match tree.find_leaf(*byte, None) {
                Some(bits) => bits.chars().rev().collect(),
                None => {
                    eprintln!("Error: missing Huffman code for byte 0x{:02x}", byte);
                    std::process::exit(1);
                }
            };
            writer.push(&bits);
        }

        let bit_count = (writer.buffer.len() * 8 + writer.bit_count as usize) as u64;

        writer.flush();

        let h_file = HuffmanFile::new(&tree, &buffer, bit_count);

        let mut output: Vec<u8> = Vec::new();
        h_file.write(&mut output);

        if let Err(e) = write_chunk(&mut output_file, &output) {
            eprintln!("Error writing file '{}': {}", opts.output_file, e);
            std::process::exit(1);
        }
    }

    if opts.decompress {
        let mut buffer: Vec<u8> = Vec::new();

        let (leaves, data_len) = {
            let h_file = match HuffmanFile::read(chunk, &mut buffer) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Error reading compressed file: {}", e);
                    std::process::exit(1);
                }
            };

            (h_file.leaves, h_file.data_len)
        };

        let mut reader = BitReader::new(&buffer, data_len);

        let mut tree = Tree::import(leaves);
        if let Err(e) = tree.construct_tree() {
            eprintln!("Error while constructing Huffman tree: {}", e);
            std::process::exit(1);
        }

        let mut output: Vec<u8> = Vec::new();

        while let Some(byte) = tree.get_next_leaf(&mut reader) {
            output.push(byte);
        }

        let mut output_file: File = match create_output(&opts.output_file) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Error creating file '{}': {}", opts.output_file, e);
                std::process::exit(1);
            }
        };

        if let Err(e) = write_chunk(&mut output_file, &output) {
            eprintln!("Error writing file '{}': {}", opts.output_file, e);
            std::process::exit(1);
        }
    }
}
