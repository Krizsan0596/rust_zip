use std::fs::File;
use std::path::Path;
use std::io::{self, Read, Write};

const CHUNK_SIZE: usize = 1024 * 1024 * 8; // 8 MB

pub fn open_file(path: &str) -> Result<File, io::Error> {
    let file = File::open(path)?;
    Ok(file)
}

pub fn get_chunk(file: &mut File) -> Result<Vec<u8>, io::Error> {
    let mut chunk = vec![0u8; CHUNK_SIZE];
    file.read(&mut chunk)?;
    Ok(chunk)
}

pub fn create_output(path: &str) -> Result<File, io::Error> {
    if !Path::new(path).exists() {
        let file: File = File::create(path)?;
        return Ok(file);
    }
    
    print!("Output file '{}' already exists. Overwrite? [y/N]>", path);
    io::stdout().flush()?;
    let mut ans = String::new();
    io::stdin().read_line(&mut ans)?;
    
    if ans.trim().eq_ignore_ascii_case("y") {
        let file: File = File::create(path)?;
        return Ok(file);
    }
    return Err(io::Error::new(io::ErrorKind::AlreadyExists, "File already exists, and user declined overwrite."));
}

pub fn write_chunk(file: &mut File, chunk: &[u8]) -> Result<(), io::Error> {
    file.write_all(chunk)?;
    Ok(())
}
