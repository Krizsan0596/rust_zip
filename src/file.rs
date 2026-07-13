use std::fs::File;
use std::io::{Read, Write, ErrorKind};

const CHUNK_SIZE: usize = 1024 * 1024 * 8; // 8 MB

pub fn open_file(path: &str) -> Result<File, std::io::Error> {
    match File::open(path) {
        Ok(file) => Ok(file),
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            let mut file = File::create(path)?;
            Ok(file)
        }
        Err(e) => Err(e)
    }
}

pub fn get_chunk(file: &mut File) -> Result<Vec<u8>, std::io::Error> {
    let mut chunk = vec![0u8; CHUNK_SIZE];
    file.read(&mut chunk)?;
    Ok(chunk)
}

pub fn write_chunk(file: &mut File, chunk: &[u8]) -> Result<(), std::io::Error> {
    file.write_all(chunk)?;
    Ok(())
}
