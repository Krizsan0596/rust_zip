use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

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
    return Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "File already exists, and user declined overwrite.",
    ));
}

pub fn write_chunk(file: &mut File, chunk: &[u8]) -> Result<(), io::Error> {
    file.write_all(chunk)?;
    Ok(())
}

pub struct BitWriter<'a> {
    buffer: &'a mut Vec<u8>,
    byte: u8,
    bit_count: u8,
}

impl<'a> BitWriter<'a> {
    pub fn new(dest: &'a mut Vec<u8>) -> Self {
        BitWriter {
            buffer: dest,
            byte: 0,
            bit_count: 0,
        }
    }

    pub fn push(&mut self, bits: &str) {
        for b in bits.bytes() {
            match b {
                b'0' => self.byte <<= 1,
                b'1' => self.byte = (self.byte << 1) | 1,
                _ => continue,
            }

            self.bit_count += 1;
            if self.bit_count == 8 {
                self.buffer.push(self.byte);
                self.byte = 0;
                self.bit_count = 0;
            }
        }
    }

    pub fn flush(&mut self) {
        if self.bit_count == 0 {
            return;
        }

        self.byte <<= 8 - self.bit_count;
        self.buffer.push(self.byte);
        self.byte = 0;
        self.bit_count = 0;
    }
}

pub struct BitReader<'a> {
    buffer: &'a Vec<u8>,
    byte: u8,
    bit_count: u8,
    cursor: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(input: &'a Vec<u8>) -> Self {
        BitReader {
            buffer: input,
            byte: 0,
            bit_count: 0,
            cursor: 0,
        }
    }

    pub fn read_bit(&mut self) -> Option<bool> {
        if self.bit_count == 0 {
            if self.cursor == self.buffer.len() {
                return None;
            }
            self.byte = self.buffer[self.cursor];
            self.cursor += 1;
        }

        let bit = (self.byte & (1 << (7 - self.bit_count))) != 0;

        self.bit_count += 1;
        if self.bit_count == 8 {
            self.bit_count = 0;
        }

        Some(bit)
    }

    pub fn read_bits(&mut self, count: u8) -> Option<String> {
        let mut out: String = String::new();

        for _ in 0..count {
            match self.read_bit() {
                Some(false) => out.push('0'),
                Some(true) => out.push('1'),
                None => return None,
            }
        }

        return Some(out);
    }
}
