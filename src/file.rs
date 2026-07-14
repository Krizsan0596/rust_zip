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
    let n = file.read(&mut chunk)?;
    chunk.truncate(n);
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

struct BitWriter<'a> {
    buffer: &'a mut Vec<u8>,
    byte: u8,
    bit_count: u8
}

impl<'a> BitWriter<'a> {
    pub fn new(dest: &'a mut Vec<u8>) -> Self {
        BitWriter { buffer: dest, byte: 0, bit_count: 0 }
    }
    pub fn push(&mut self, bits: &String) {
        for char in bits.chars() {
            if char == '0' {
                self.byte = self.byte << 1;
            }
            else if char == '1' {
                self.byte = (self.byte << 1) | 1;
            }
            self.bit_count += 1;
            
            if self.bit_count >= 8 {
                self.buffer.push(self.byte);
                self.byte = 0;
            }
        }
    }
}
