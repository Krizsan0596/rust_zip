use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use crate::huffman::{Leaf, Node, Tree};

const CHUNK_SIZE: usize = 1024 * 1024 * 8; // 8 MB
const MAGIC_NUMBER: [u8; 4] = *b"ZIP1";

pub fn open_file(path: &str) -> Result<File, io::Error> {
    let file = File::open(path)?;
    Ok(file)
}

pub fn get_chunk(file: &mut File, chunk: &mut Vec<u8>) -> Result<usize, io::Error> {
    chunk.clear();

    let n = Read::by_ref(file).take(CHUNK_SIZE as u64).read_to_end(chunk)?;
    Ok(n)
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

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "File already exists, and user declined overwrite.",
    ))
}

pub fn write_chunk(file: &mut File, chunk: &[u8]) -> Result<(), io::Error> {
    file.write_all(chunk)?;
    Ok(())
}

pub struct BitWriter<'a> {
    pub buffer: &'a mut Vec<u8>,
    byte: u8,
    pub bit_count: u8,
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
    bits_read: u64,
    total_bits: u64,
}

impl<'a> BitReader<'a> {
    pub fn new(input: &'a Vec<u8>, total_bits: u64) -> Self {
        BitReader {
            buffer: input,
            byte: 0,
            bit_count: 0,
            cursor: 0,
            bits_read: 0,
            total_bits,
        }
    }

    pub fn read_bit(&mut self) -> Option<bool> {
        if self.bits_read == self.total_bits {
            return None;
        }
        if self.bit_count == 0 {
            if self.cursor == self.buffer.len() {
                return None; // should panic?
            }
            self.byte = self.buffer[self.cursor];
            self.cursor += 1;
        }

        let bit = (self.byte & (1 << (7 - self.bit_count))) != 0;

        self.bit_count += 1;
        self.bits_read += 1;
        if self.bit_count == 8 {
            self.bit_count = 0;
        }

        Some(bit)
    }

    // pub fn read_bits(&mut self, count: u8) -> Option<String> {
    //     let mut out: String = String::new();
    //
    //     for _ in 0..count {
    //         match self.read_bit() {
    //             Some(false) => out.push('0'),
    //             Some(true) => out.push('1'),
    //             None => return None,
    //         }
    //     }
    //
    //     Some(out)
    // }
}

#[derive(Debug, PartialEq)]
pub struct HuffmanFile<'a> {
    magic_number: [u8; 4],
    leaf_count: u8,
    pub leaves: Vec<Leaf>,
    pub data_len: u64, // in bits
    compressed_data: &'a Vec<u8>,
}

impl<'a> HuffmanFile<'a> {
    pub fn new(tree: &Tree, data: &'a Vec<u8>, data_len: u64) -> Self {
        let mut new = HuffmanFile {
            magic_number: MAGIC_NUMBER,
            leaf_count: (tree.nodes.len().div_ceil(2) - 1) as u8,
            leaves: Vec::new(),
            data_len,
            compressed_data: data,
        };

        new.leaves
            .reserve(new.leaf_count as usize + 1 - new.leaves.len());

        for idx in 0..=new.leaf_count {
            if let Some(Node::Leaf(leaf)) = tree.nodes[idx as usize] {
                new.leaves.push(leaf);
            }
        }

        new
    }

    pub fn write(&self, to: &mut Vec<u8>) {
        to.extend_from_slice(&self.magic_number);

        to.push(self.leaf_count);

        for leaf in &self.leaves {
            to.extend_from_slice(&leaf.frequency.to_be_bytes());
            to.push(leaf.data);
        }

        to.extend_from_slice(&self.data_len.to_be_bytes());

        to.extend_from_slice(self.compressed_data);
    }

    pub fn read(from: &[u8], buffer: &'a mut Vec<u8>) -> Result<Self, io::Error> {
        if from.len() < 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File too short to contain magic number",
            ));
        }

        let mut magic_number = [0u8; 4];
        magic_number.copy_from_slice(&from[0..4]);

        if magic_number != MAGIC_NUMBER {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid magic number",
            ));
        }

        let mut cursor: usize = 4;

        if cursor >= from.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Unexpected EOF reading leaf count",
            ));
        }
        let leaf_count: u8 = from[cursor];
        cursor += 1;

        let mut leaves: Vec<Leaf> = Vec::with_capacity(leaf_count as usize + 1);
        for _ in 0..leaf_count as usize + 1 {
            if cursor + 9 > from.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Unexpected EOF reading leaf data",
                ));
            }
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&from[cursor..cursor + 8]);
            let byte = from[cursor + 8];
            cursor += 9;
            leaves.push(Leaf {
                frequency: u64::from_be_bytes(bytes),
                data: byte,
            });
        }

        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&from[cursor..cursor + 8]);
        let data_len = u64::from_be_bytes(bytes);
        cursor += 8;

        buffer.extend_from_slice(&from[cursor..]);

        Ok(Self {
            magic_number,
            leaf_count,
            leaves,
            data_len,
            compressed_data: buffer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_bit_writer_basic_push() {
        let mut buffer = Vec::new();
        let mut writer = BitWriter::new(&mut buffer);
        writer.push("101");
        assert!(writer.buffer.is_empty());
        writer.flush();
        assert_eq!(*writer.buffer, vec![160]);
    }

    #[test]
    fn test_bit_writer_multiple_bytes() {
        let mut buffer = Vec::new();
        let mut writer = BitWriter::new(&mut buffer);

        writer.push("11111111");
        assert_eq!(*writer.buffer, vec![255]);

        writer.push("00000000");
        assert_eq!(*writer.buffer, vec![255, 0]);

        writer.push("1010");
        assert_eq!(*writer.buffer, vec![255, 0]);

        writer.flush();
        // 10100000 == 160
        assert_eq!(*writer.buffer, vec![255, 0, 160]);
    }

    #[test]
    fn test_bit_writer_flush_empty() {
        let mut buffer = Vec::new();
        let mut writer = BitWriter::new(&mut buffer);
        writer.flush();
        assert!(writer.buffer.is_empty());

        writer.push("1");
        writer.flush();
        assert_eq!(*writer.buffer, vec![128]);

        writer.flush();
        assert_eq!(*writer.buffer, vec![128]);
    }

    #[test]
    fn test_bit_reader_basic() {
        let buffer = vec![160]; // 10100000
        let mut reader = BitReader::new(&buffer, 8);

        assert_eq!(reader.read_bit(), Some(true));
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), Some(true));
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), None);
    }

    #[test]
    fn test_bit_reader_empty() {
        let buffer = Vec::new();
        let mut reader = BitReader::new(&buffer, 0);
        assert_eq!(reader.read_bit(), None);
    }

    #[test]
    fn test_bit_reader_multiple_bytes() {
        let buffer = vec![255, 0];
        let mut reader = BitReader::new(&buffer, 16);

        for _ in 0..8 {
            assert_eq!(reader.read_bit(), Some(true));
        }
        for _ in 0..8 {
            assert_eq!(reader.read_bit(), Some(false));
        }
        assert_eq!(reader.read_bit(), None);
    }

    #[test]
    fn test_round_trip() {
        let mut buffer = Vec::new();
        let bit_string = "110110001100101"; // 15 bits
        {
            let mut writer = BitWriter::new(&mut buffer);
            writer.push(bit_string);
            writer.flush();
        }

        let mut reader = BitReader::new(&buffer, 16);
        let mut decoded = String::new();

        for _ in 0..15 {
            match reader.read_bit() {
                Some(true) => decoded.push('1'),
                Some(false) => decoded.push('0'),
                None => break,
            }
        }
        assert_eq!(decoded, bit_string);

        assert_eq!(reader.read_bit(), Some(false));

        assert_eq!(reader.read_bit(), None);
    }

    #[test]
    fn test_open_file_succeeds_and_fails() {
        let res = open_file("non_existent_file_path_123.tmp");
        assert!(res.is_err());

        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_string_lossy();
        std::fs::write(path.as_ref(), b"test data").unwrap();

        let res = open_file(path.as_ref());
        assert!(res.is_ok());
    }

    #[test]
    fn test_get_chunk_exact_bytes() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_string_lossy();
        let test_data = b"hello world";
        std::fs::write(path.as_ref(), test_data).unwrap();

        let mut opened_file = open_file(path.as_ref()).unwrap();
        let mut chunk = Vec::new();
        get_chunk(&mut opened_file, &mut chunk).unwrap();
        assert_eq!(chunk, test_data);
    }

    #[test]
    fn test_get_chunk_truncates_at_eof() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_string_lossy();
        let test_data = vec![b'A'; 100];
        std::fs::write(path.as_ref(), &test_data).unwrap();

        let mut opened_file = open_file(path.as_ref()).unwrap();

        let mut chunk = Vec::new();
        get_chunk(&mut opened_file, &mut chunk).unwrap();
        assert_eq!(chunk, test_data);
        assert_eq!(chunk.len(), 100);

        get_chunk(&mut opened_file, &mut chunk).unwrap();
        assert!(chunk.is_empty());
    }

    #[test]
    fn test_write_chunk_exact_bytes() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_string_lossy();

        let mut opened_file = File::create(path.as_ref()).unwrap();
        let data = b"some random bytes to write";
        write_chunk(&mut opened_file, data).unwrap();
        drop(opened_file);

        let read_data = std::fs::read(path.as_ref()).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_create_output_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_file.tmp");

        let path_str = path.to_string_lossy();
        let _res = create_output(path_str.as_ref());
        assert!(path.exists());
    }

    #[test]
    fn test_huffman_file_validation() {
        let mut tree = Tree::new();
        tree.add_leaf(b'a');
        tree.add_leaf(b'b');
        tree.sort_nodes();
        tree.construct_tree().unwrap();

        let compressed_data = vec![1, 2, 3];
        let h_file = HuffmanFile::new(&tree, &compressed_data, 24);

        let mut written_bytes = Vec::new();
        h_file.write(&mut written_bytes);

        let mut read_buffer = Vec::new();
        let read_res = HuffmanFile::read(&written_bytes, &mut read_buffer);
        assert!(read_res.is_ok());
        let read_file = read_res.unwrap();
        assert_eq!(read_file, h_file);
        assert_eq!(read_buffer, compressed_data);

        let short_bytes = vec![b'Z', b'I', b'P'];
        let mut buf = Vec::new();
        let err_res = HuffmanFile::read(&short_bytes, &mut buf);
        assert!(err_res.is_err());
        let err = err_res.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("too short"));

        let mut wrong_magic = written_bytes.clone();
        wrong_magic[0..4].copy_from_slice(b"ZIP2");
        let mut buf = Vec::new();
        let err_res = HuffmanFile::read(&wrong_magic, &mut buf);
        assert!(err_res.is_err());
        let err = err_res.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Invalid magic number"));
    }
}
