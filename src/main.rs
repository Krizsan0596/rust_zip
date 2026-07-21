mod util;
use util::{
    ArgError, Config, parallel_compression, parallel_frequency_count, print_usage, process_args,
};

mod file;
use file::{BitReader, HuffmanFile, create_output, get_chunk, open_file, write_chunk};
use std::fs::File;
use std::io::{BufWriter, Seek, Write};

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
            ArgError::MissingThreadsArg => {
                eprintln!("Error: -t option requires an argument");
                print_usage(&name);
                std::process::exit(1);
            }
            ArgError::InvalidThreadsArg => {
                eprintln!("Error: -t option requires a valid positive integer");
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

    if opts.compress {
        let mut output_file: File = match create_output(&opts.output_file) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Error creating file '{}': {}", opts.output_file, e);
                std::process::exit(1);
            }
        };

        let mut tree = match parallel_frequency_count(&mut input_file, opts.max_threads) {
            Ok(tree) => tree,
            Err(e) => {
                eprintln!("Error reading file '{}': {}", opts.input_file, e);
                std::process::exit(1);
            }
        };
        tree.nodes.retain(|x| x.is_some());

        tree.sort_nodes();
        if let Err(e) = tree.construct_tree() {
            eprintln!("Error while constructing Huffman tree: {}", e);
            std::process::exit(1);
        }
        tree.populate_cache(None, None);

        if let Err(e) = input_file.seek(std::io::SeekFrom::Start(0)) {
            eprintln!("Error seeking input file '{}': {}", opts.input_file, e);
            std::process::exit(1);
        }

        let (buffer, bit_count) =
            match parallel_compression(&mut input_file, &tree, opts.max_threads) {
                Ok(res) => res,
                Err(e) => {
                    eprintln!("Error while compressing file: '{}': {}", opts.input_file, e);
                    std::process::exit(1);
                }
            };

        let h_file = HuffmanFile::new(&tree, &buffer, bit_count);

        let mut output: Vec<u8> = Vec::new();
        h_file.write(&mut output);

        if let Err(e) = write_chunk(&mut output_file, &output) {
            eprintln!("Error writing file '{}': {}", opts.output_file, e);
            std::process::exit(1);
        }
    }

    if opts.decompress {
        let mut compressed_file_data = Vec::new();
        let mut chunk = Vec::new();

        loop {
            match get_chunk(&mut input_file, &mut chunk) {
                Ok(0) => break,
                Ok(_) => {
                    compressed_file_data.extend_from_slice(&chunk);
                }
                Err(e) => {
                    eprintln!("Error reading file '{}': {}", opts.input_file, e);
                    std::process::exit(1);
                }
            }
        }

        let mut buffer: Vec<u8> = Vec::new();

        let (leaves, data_len) = {
            let h_file = match HuffmanFile::read(&compressed_file_data, &mut buffer) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Error reading compressed file: {}", e);
                    std::process::exit(1);
                }
            };

            (h_file.leaves, h_file.data_len)
        };

        let mut reader = BitReader::new(&buffer, data_len);

        let mut tree = Tree::import(&leaves);
        if let Err(e) = tree.construct_tree() {
            eprintln!("Error while constructing Huffman tree: {}", e);
            std::process::exit(1);
        }

        let lut = tree.build_lut();

        let output_file: File = match create_output(&opts.output_file) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Error creating file '{}': {}", opts.output_file, e);
                std::process::exit(1);
            }
        };

        let mut writer = BufWriter::new(output_file);
        let mut buffer = [0u8; 8192];
        let mut count = 0;

        loop {
            if let Some(byte) = reader.peek_byte()
                && let res = &lut[byte as usize]
                && res.length > 0
            {
                buffer[count] = res.byte;
                count += 1;
                if count == buffer.len() {
                    if let Err(e) = writer.write_all(&buffer) {
                        eprintln!("Error writing to file '{}': {}", opts.output_file, e);
                        std::process::exit(1);
                    }
                    count = 0;
                }

                reader.seek(res.length as u64);
                continue;
            }

            match tree.get_next_leaf(&mut reader) {
                Some(byte) => {
                    buffer[count] = byte;
                    count += 1;
                    if count == buffer.len() {
                        if let Err(e) = writer.write_all(&buffer) {
                            eprintln!("Error writing to file '{}': {}", opts.output_file, e);
                            std::process::exit(1);
                        }
                        count = 0;
                    }
                }
                None => break,
            }
        }

        if count > 0
            && let Err(e) = writer.write_all(&buffer[..count])
        {
            eprintln!("Error writing to file '{}': {}", opts.output_file, e);
            std::process::exit(1);
        }

        if let Err(e) = writer.flush() {
            eprintln!("Error flushing output file '{}': {}", opts.output_file, e);
            std::process::exit(1);
        }
    }
}
