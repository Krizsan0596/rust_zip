use crate::file::{BitWriter, CHUNK_SIZE, get_chunk};
use crate::huffman::Tree;
use std::fs::File;
use std::io::Error;
use std::sync::Mutex;
use std::thread;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgError {
    Help,
    MissingOutputArg,
    ConflictingModes,
    NoModeSpecified,
    MissingInput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub input_file: String,
    pub output_file: String,
    pub compress: bool,
    pub decompress: bool,
}

pub struct SharedReader<'a> {
    file: &'a mut File,
    index: usize,
}

fn get_subtree(chunk: &[u8]) -> Tree {
    let mut res = Tree::new();

    for byte in chunk {
        res.add_leaf(*byte);
    }

    res
}

fn compress_chunk(chunk: &[u8], tree: &Tree) -> (Vec<u8>, u64) {
    let mut res = Vec::new();
    let mut writer = BitWriter::new(&mut res);
    for byte in chunk {
        let bits = tree.find_leaf(*byte);
        writer.push(bits.unwrap());
    }
    let bit_count = (writer.buffer.len() * 8 + writer.bit_count as usize) as u64;
    writer.flush();
    (res, bit_count)
}

pub fn parallel_frequency_count(file: &mut File, max_threads: usize) -> Result<Tree, Error> {
    let file_size = file.metadata()?.len();
    let chunk_count = if file_size == 0 {
        0
    } else {
        file_size.div_ceil(CHUNK_SIZE as u64) as usize
    };
    let threads = max_threads.min(chunk_count);

    if threads <= 1 {
        let mut chunk = Vec::new();
        match get_chunk(file, &mut chunk) {
            Ok(0) => return Ok(Tree::new()),
            Ok(_) => return Ok(get_subtree(&chunk)),
            Err(e) => return Err(e),
        }
    }

    let shared = Mutex::new(SharedReader { file, index: 0 });
    let results: Mutex<Vec<(usize, Tree)>> = Mutex::new(Vec::new());
    let error: Mutex<Option<Error>> = Mutex::new(None);

    thread::scope(|s| {
        for _ in 0..threads {
            let shared = &shared;
            let results = &results;
            let error = &error;

            s.spawn(move || {
                let mut chunk = Vec::new();

                loop {
                    if error.lock().unwrap().is_some() {
                        return;
                    }
                    let idx = {
                        let mut guard = shared.lock().unwrap();
                        match get_chunk(guard.file, &mut chunk) {
                            Ok(0) => return,
                            Ok(_) => (),
                            Err(e) => {
                                *error.lock().unwrap() = Some(e);
                                return;
                            }
                        };
                        let idx = guard.index;
                        guard.index += 1;
                        idx
                    };

                    let res = get_subtree(&chunk);
                    results.lock().unwrap().push((idx, res));
                }
            });
        }
    });

    if let Some(e) = error.into_inner().unwrap() {
        return Err(e);
    }

    let results = results.into_inner().unwrap();
    Ok(Tree::merge(results.into_iter().map(|(_, x)| x).collect()))
}

pub fn parallel_compression(
    file: &mut File,
    tree: &Tree,
    max_threads: usize,
) -> Result<(Vec<u8>, u64), Error> {
    let file_size = file.metadata()?.len();
    let chunk_count = if file_size == 0 {
        0
    } else {
        file_size.div_ceil(CHUNK_SIZE as u64) as usize
    };
    let threads = max_threads.min(chunk_count);

    if threads <= 1 {
        let mut chunk = Vec::new();
        match get_chunk(file, &mut chunk) {
            Ok(0) => return Ok((Vec::new(), 0)),
            Ok(_) => return Ok(compress_chunk(&chunk, tree)),
            Err(e) => return Err(e),
        }
    }

    type ChunkData = (Vec<u8>, u64);

    let shared = Mutex::new(SharedReader { file, index: 0 });
    let results: Mutex<Vec<(usize, ChunkData)>> = Mutex::new(Vec::new());
    let error: Mutex<Option<Error>> = Mutex::new(None);

    thread::scope(|s| {
        for _ in 0..threads {
            let shared = &shared;
            let results = &results;
            let error = &error;

            s.spawn(move || {
                let mut chunk = Vec::new();

                loop {
                    if error.lock().unwrap().is_some() {
                        return;
                    }
                    let idx = {
                        let mut guard = shared.lock().unwrap();
                        match get_chunk(guard.file, &mut chunk) {
                            Ok(0) => return,
                            Ok(_) => (),
                            Err(e) => {
                                *error.lock().unwrap() = Some(e);
                                return;
                            }
                        };
                        let idx = guard.index;
                        guard.index += 1;
                        idx
                    };

                    let res = compress_chunk(&chunk, tree);
                    results.lock().unwrap().push((idx, res));
                }
            });
        }
    });

    if let Some(e) = error.into_inner().unwrap() {
        return Err(e);
    }

    let mut results = results.into_inner().unwrap();
    results.sort_unstable_by_key(|x| x.0);
    let results: Vec<(Vec<u8>, u64)> = results.into_iter().map(|(_, v)| v).collect();

    let result = merge_bit_streams(&results);
    Ok(result)
}

#[cfg_attr(no_inline, inline(never))]
fn merge_bit_streams(streams: &[(Vec<u8>, u64)]) -> (Vec<u8>, u64) {
    let total_bits: u64 = streams.iter().map(|(_, bits)| bits).sum();
    let mut res = Vec::with_capacity((total_bits as usize).div_ceil(8));

    let mut current_byte = 0u8;
    let mut offset = 0u32;

    for (data, bits) in streams {
        if *bits == 0 {
            continue;
        }

        let full_bytes = (bits / 8) as usize;
        let remainder = (bits % 8) as u32;

        if offset == 0 {
            res.extend_from_slice(&data[..full_bytes]);
        } else {
            for item in data.iter().take(full_bytes) {
                res.push(current_byte | (item >> offset));
                current_byte = item << (8 - offset);
            }
        }

        if remainder > 0 {
            let last_byte = data[full_bytes];
            let mask = !((1 << (8 - remainder)) - 1);
            let b = last_byte & mask;

            if offset == 0 {
                current_byte = b;
                offset = remainder;
            } else {
                let combined = current_byte | (b >> offset);
                if offset + remainder >= 8 {
                    res.push(combined);
                    current_byte = b << (8 - offset);
                    offset = offset + remainder - 8;
                } else {
                    current_byte = combined;
                    offset += remainder;
                }
            }
        }
    }

    if offset > 0 {
        res.push(current_byte);
    }

    (res, total_bits)
}

pub fn print_usage(program_name: &str) {
    println!("Usage: {} [-c] [-x] [-o <output>] <input>", program_name);
}

pub fn process_args(args: Vec<String>) -> Result<Config, ArgError> {
    let mut iter = args.into_iter().skip(1);

    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut compress: bool = false;
    let mut decompress: bool = false;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" => {
                return Err(ArgError::Help);
            }
            "-c" => {
                compress = true;
            }
            "-x" => {
                decompress = true;
            }
            "-o" => {
                if let Some(value) = iter.next() {
                    output = Some(value);
                } else {
                    return Err(ArgError::MissingOutputArg);
                }
            }
            other => {
                input = Some(other.to_string());
            }
        }
    }

    if compress && decompress {
        return Err(ArgError::ConflictingModes);
    }

    if !compress && !decompress {
        return Err(ArgError::NoModeSpecified);
    }

    let input_file = match input {
        Some(file) => file,
        None => {
            return Err(ArgError::MissingInput);
        }
    };

    let output_file = match output {
        Some(file) => file,
        None => {
            return Err(ArgError::MissingOutputArg);
        }
    };

    Ok(Config {
        input_file,
        output_file,
        compress,
        decompress,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_valid_compression() {
        let args = vec![
            "prog".to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "out".to_string(),
            "input".to_string(),
        ];
        assert_eq!(
            process_args(args),
            Ok(Config {
                input_file: "input".to_string(),
                output_file: "out".to_string(),
                compress: true,
                decompress: false,
            })
        );
    }

    #[test]
    fn test_valid_decompression() {
        let args = vec![
            "prog".to_string(),
            "-x".to_string(),
            "-o".to_string(),
            "out".to_string(),
            "input".to_string(),
        ];
        assert_eq!(
            process_args(args),
            Ok(Config {
                input_file: "input".to_string(),
                output_file: "out".to_string(),
                compress: false,
                decompress: true,
            })
        );
    }

    #[test]
    fn test_help() {
        let args = vec!["prog".to_string(), "-h".to_string()];
        assert_eq!(process_args(args), Err(ArgError::Help));
    }

    #[test]
    fn test_missing_output_arg_flag_only() {
        let args = vec!["prog".to_string(), "-c".to_string(), "-o".to_string()];
        assert_eq!(process_args(args), Err(ArgError::MissingOutputArg));
    }

    #[test]
    fn test_missing_output_arg_value() {
        let args = vec![
            "prog".to_string(),
            "-c".to_string(),
            "input".to_string(),
            "-o".to_string(),
        ];
        assert_eq!(process_args(args), Err(ArgError::MissingOutputArg));
    }

    #[test]
    fn test_conflicting_modes() {
        let args = vec![
            "prog".to_string(),
            "-c".to_string(),
            "-x".to_string(),
            "-o".to_string(),
            "out".to_string(),
            "input".to_string(),
        ];
        assert_eq!(process_args(args), Err(ArgError::ConflictingModes));
    }

    #[test]
    fn test_no_mode() {
        let args = vec![
            "prog".to_string(),
            "-o".to_string(),
            "out".to_string(),
            "input".to_string(),
        ];
        assert_eq!(process_args(args), Err(ArgError::NoModeSpecified));
    }

    #[test]
    fn test_missing_input() {
        let args = vec![
            "prog".to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "out".to_string(),
        ];
        assert_eq!(process_args(args), Err(ArgError::MissingInput));
    }

    #[test]
    fn test_repeated_positional_args() {
        let args = vec![
            "prog".to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "out".to_string(),
            "input1".to_string(),
            "input2".to_string(),
        ];
        assert_eq!(
            process_args(args),
            Ok(Config {
                input_file: "input2".to_string(),
                output_file: "out".to_string(),
                compress: true,
                decompress: false,
            })
        );
    }

    #[test]
    fn test_parallel_frequency_count_empty() {
        use std::io::Seek;
        let mut file = tempfile::tempfile().unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();

        let tree = parallel_frequency_count(&mut file, 3).unwrap();
        assert!(tree.nodes.iter().all(|n| n.is_none()));
    }

    #[test]
    fn test_parallel_frequency_count_small() {
        use std::io::{Seek, Write};
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(b"aaaaabbbcc").unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();

        let mut tree = parallel_frequency_count(&mut file, 3).unwrap();
        tree.nodes.retain(|x| x.is_some());

        let mut leaf_counts = std::collections::HashMap::new();
        for node in tree.nodes {
            if let Some(crate::huffman::Node::Leaf(leaf)) = node {
                leaf_counts.insert(leaf.data, leaf.frequency);
            }
        }

        assert_eq!(leaf_counts.get(&b'a'), Some(&5));
        assert_eq!(leaf_counts.get(&b'b'), Some(&3));
        assert_eq!(leaf_counts.get(&b'c'), Some(&2));
    }

    #[test]
    fn test_parallel_frequency_count_large() {
        use std::io::{Seek, Write};
        let mut file = tempfile::tempfile().unwrap();

        // Write 10 MB of data
        // 5 MB of 'A', 3 MB of 'B', 2 MB of 'C'
        let a_data = vec![b'A'; 1024 * 1024 * 5];
        let b_data = vec![b'B'; 1024 * 1024 * 3];
        let c_data = vec![b'C'; 1024 * 1024 * 2];

        file.write_all(&a_data).unwrap();
        file.write_all(&b_data).unwrap();
        file.write_all(&c_data).unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();

        let mut tree = parallel_frequency_count(&mut file, 3).unwrap();
        tree.nodes.retain(|x| x.is_some());

        let mut leaf_counts = std::collections::HashMap::new();
        for node in tree.nodes {
            if let Some(crate::huffman::Node::Leaf(leaf)) = node {
                leaf_counts.insert(leaf.data, leaf.frequency);
            }
        }

        assert_eq!(leaf_counts.get(&b'A'), Some(&(1024 * 1024 * 5)));
        assert_eq!(leaf_counts.get(&b'B'), Some(&(1024 * 1024 * 3)));
        assert_eq!(leaf_counts.get(&b'C'), Some(&(1024 * 1024 * 2)));
    }
}
