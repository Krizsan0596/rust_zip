use std::fs::File;
use std::io::Read;

const CHUNK_SIZE: usize = 1024 * 1024 * 8; // 8 MB

pub fn open_file(path: &str) -> Result<File, std::io::Error> {
    let file = File::open(path)?;
    Ok(file)
}

pub fn get_chunk(file: &mut File) -> Result<Vec<u8>, std::io::Error> {
    let mut chunk = vec![0u8; CHUNK_SIZE];
    file.read(&mut chunk)?;
    Ok(chunk)
}
