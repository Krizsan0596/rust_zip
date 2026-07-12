use std::fs::File;
use std::io::Read;

const CHUNK_SIZE: usize = 1024 * 1024 * 8; // 8 MB

pub fn open_file(path: &str) -> Result<File, std::io::Error> {
    let file = File::open(path)?;
    Ok(file)
}

pub fn get_chunk(file: &mut File) -> Vec<u8> {
    let mut chunk = vec![0u8; CHUNK_SIZE];
    let bytes_read = match file.read(&mut chunk) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };
    return chunk;
}
