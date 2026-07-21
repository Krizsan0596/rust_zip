use crate::{file::BitReader, util::LUTEntry};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leaf {
    pub frequency: u64,
    pub data: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Branch {
    frequency: u64,
    left: u64,
    right: u64,
}

impl Branch {
    fn new(frequency: u64, left: u64, right: u64) -> Self {
        Branch {
            frequency,
            left,
            right,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Node {
    Leaf(Leaf),
    Branch(Branch),
}

impl Node {
    fn frequency(&self) -> u64 {
        match self {
            Node::Leaf(leaf) => leaf.frequency,
            Node::Branch(branch) => branch.frequency,
        }
    }
}

pub struct Tree {
    pub root: Option<usize>,
    pub nodes: Vec<Option<Node>>,
    pub cache: Box<[Option<(u32, u8)>; 256]>,
}

impl Tree {
    pub fn new() -> Self {
        Tree {
            root: None,
            nodes: vec![None; 256],
            cache: Box::new([None; 256]),
        }
    }

    pub fn merge(trees: Vec<Tree>) -> Self {
        let mut merged = Tree::new();
        for tree in trees {
            for node_opt in tree.nodes {
                if let Some(Node::Leaf(leaf)) = node_opt {
                    let idx = leaf.data as usize;
                    if let Some(Node::Leaf(ref mut merged_leaf)) = merged.nodes[idx] {
                        merged_leaf.frequency += leaf.frequency;
                    } else {
                        merged.nodes[idx] = Some(Node::Leaf(leaf));
                    }
                }
            }
        }
        merged
    }

    #[allow(dead_code)]
    pub fn add_leaf(&mut self, value: u8) {
        if let Some(Node::Leaf(ref mut leaf)) = self.nodes[value as usize] {
            leaf.frequency += 1;
        } else {
            self.nodes[value as usize] = Some(Node::Leaf(Leaf {
                frequency: 1,
                data: value,
            }));
        }
    }

    pub fn sort_nodes(&mut self) {
        self.nodes.retain(|x| x.is_some());
        self.nodes
            .sort_unstable_by_key(|x| x.as_ref().unwrap().frequency());
    }

    pub fn construct_tree(&mut self) -> Result<(), std::io::Error> {
        self.nodes.retain(|x| x.is_some());
        if self.nodes.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Input file is empty.",
            ));
        }
        if self.nodes.len() == 1 {
            self.root = Some(0);
            return Ok(());
        }

        let leaf_count: usize = self.nodes.len();
        self.nodes.reserve(self.nodes.len() - 1); // Huffman tree with n leaves has 2n-1 nodes.

        let mut current_leaf: usize = 0;
        let mut current_branch: usize = leaf_count;

        let next_node = |leaf: &mut usize, branch: &mut usize, nodes: &[Option<Node>]| -> usize {
            if *leaf < leaf_count
                && (*branch == nodes.len()
                    || nodes[*leaf].as_ref().unwrap().frequency()
                        <= nodes[*branch].as_ref().unwrap().frequency())
            {
                let idx = *leaf;
                *leaf += 1;
                idx
            } else {
                let idx = *branch;
                *branch += 1;
                idx
            }
        };

        for _ in 0..leaf_count - 1 {
            let left = next_node(&mut current_leaf, &mut current_branch, &self.nodes);
            let right = next_node(&mut current_leaf, &mut current_branch, &self.nodes);

            let freq: u64 = self.nodes[left].as_ref().unwrap().frequency()
                + self.nodes[right].as_ref().unwrap().frequency();
            self.nodes.push(Some(Node::Branch(Branch::new(
                freq,
                left as u64,
                right as u64,
            ))));
            self.root = Some(self.nodes.len() - 1);
        }

        Ok(())
    }

    #[allow(dead_code)]
    #[inline]
    pub fn find_leaf(&self, leaf: u8) -> Option<(u32, u8)> {
        self.cache[leaf as usize]
    }

    pub fn populate_cache(&mut self, root: Option<usize>, current_path: Option<(u32, u8)>) {
        let root: usize = root.unwrap_or(*self.root.as_ref().unwrap());
        let current_path = current_path.unwrap_or((0, 0));

        match self.nodes[root] {
            Some(Node::Leaf(leaf)) => self.cache[leaf.data as usize] = Some(current_path),
            Some(Node::Branch(branch)) => {
                let left_path = (current_path.0 << 1, current_path.1 + 1);
                self.populate_cache(Some(branch.left as usize), Some(left_path));
                let right_path = ((current_path.0 << 1) | 1, current_path.1 + 1);
                self.populate_cache(Some(branch.right as usize), Some(right_path));
            }
            None => {
                unreachable!();
            }
        }
    }

    pub fn get_next_leaf<'a>(&self, reader: &mut BitReader<'a>) -> Option<u8> {
        let mut root: &Node = self.nodes[*self.root.as_ref().unwrap()].as_ref().unwrap();
        loop {
            root = match root {
                Node::Leaf(leaf) => return Some(leaf.data),
                Node::Branch(branch) => match reader.read_bit() {
                    Some(true) => self.nodes[branch.right as usize].as_ref().unwrap(),
                    Some(false) => self.nodes[branch.left as usize].as_ref().unwrap(),
                    None => return None,
                },
            }
        }
    }

    pub fn import(from: &[Leaf]) -> Self {
        let nodes: Vec<Option<Node>> = from
            .iter()
            .filter(|leaf| leaf.frequency > 0)
            .map(|leaf| Some(Node::Leaf(*leaf)))
            .collect();
        Tree {
            root: None,
            nodes,
            cache: Box::new([None; 256]),
        }
    }

    pub fn build_lut(&self) -> Vec<LUTEntry> {
        let mut res: Vec<LUTEntry> = (0..256).map(|_| LUTEntry { length: 0, byte: 0 }).collect();
        let mut bytes = vec![0u8; 1];

        for byte in 0..256 {
            bytes[0] = byte as u8;
            let mut reader = BitReader::new(&bytes, 8);
            if let Some(val) = self.get_next_leaf(&mut reader) {
                res[byte] = LUTEntry {
                    length: 8 - reader.bit_count,
                    byte: val,
                };
            }
        }

        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::BitWriter;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_tree_new() {
        let tree = Tree::new();
        assert!(tree.root.is_none());
        assert_eq!(tree.nodes.len(), 256);
        assert!(tree.nodes.iter().all(|x| x.is_none()));
        for i in 0..256 {
            assert!(tree.cache[i].is_none());
        }
    }

    #[test]
    fn test_tree_import() {
        let leaves = vec![
            Leaf {
                frequency: 10,
                data: b'a',
            },
            Leaf {
                frequency: 0,
                data: b'b',
            },
            Leaf {
                frequency: 5,
                data: b'c',
            },
        ];
        let tree = Tree::import(&leaves);
        assert_eq!(
            tree.nodes,
            vec![
                Some(Node::Leaf(Leaf {
                    frequency: 10,
                    data: b'a'
                })),
                Some(Node::Leaf(Leaf {
                    frequency: 5,
                    data: b'c'
                })),
            ]
        );
    }

    #[test]
    fn test_node_frequency() {
        let leaf = Node::Leaf(Leaf {
            data: b'a',
            frequency: 42,
        });
        assert_eq!(leaf.frequency(), 42);

        let branch = Node::Branch(Branch::new(100, 1, 2));
        assert_eq!(branch.frequency(), 100);
    }

    #[test]
    fn test_add_leaf() {
        let mut tree = Tree::new();

        tree.add_leaf(b'A');
        assert_eq!(
            tree.nodes.iter().filter_map(|&x| x).collect::<Vec<_>>(),
            vec![Node::Leaf(Leaf {
                data: b'A',
                frequency: 1
            })]
        );

        tree.add_leaf(b'A');
        assert_eq!(
            tree.nodes.iter().filter_map(|&x| x).collect::<Vec<_>>(),
            vec![Node::Leaf(Leaf {
                data: b'A',
                frequency: 2
            })]
        );

        tree.add_leaf(b'B');
        assert_eq!(
            tree.nodes.iter().filter_map(|&x| x).collect::<Vec<_>>(),
            vec![
                Node::Leaf(Leaf {
                    data: b'A',
                    frequency: 2
                }),
                Node::Leaf(Leaf {
                    data: b'B',
                    frequency: 1
                }),
            ]
        );
    }

    #[test]
    fn test_merge() {
        let mut tree1 = Tree::new();
        tree1.add_leaf(b'A');
        tree1.add_leaf(b'B');

        let mut tree2 = Tree::new();
        tree2.add_leaf(b'A');
        tree2.add_leaf(b'C');

        let merged = Tree::merge(vec![tree1, tree2]);

        assert_eq!(
            merged.nodes.iter().filter_map(|&x| x).collect::<Vec<_>>(),
            vec![
                Node::Leaf(Leaf {
                    data: b'A',
                    frequency: 2
                }),
                Node::Leaf(Leaf {
                    data: b'B',
                    frequency: 1
                }),
                Node::Leaf(Leaf {
                    data: b'C',
                    frequency: 1
                }),
            ]
        );
    }

    #[test]
    fn test_sort_nodes() {
        let mut tree = Tree::new();

        tree.add_leaf(b'A');
        tree.add_leaf(b'A');
        tree.add_leaf(b'A');

        tree.add_leaf(b'B');

        tree.add_leaf(b'C');
        tree.add_leaf(b'C');

        tree.sort_nodes();

        assert_eq!(
            tree.nodes,
            vec![
                Some(Node::Leaf(Leaf {
                    data: b'B',
                    frequency: 1
                })),
                Some(Node::Leaf(Leaf {
                    data: b'C',
                    frequency: 2
                })),
                Some(Node::Leaf(Leaf {
                    data: b'A',
                    frequency: 3
                })),
            ]
        );
    }

    #[test]
    fn test_construct_tree_empty() {
        let mut tree = Tree::new();
        let result = tree.construct_tree();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(err.to_string(), "Input file is empty.");
    }

    #[test]
    fn test_construct_tree_single() {
        let mut tree = Tree::new();
        tree.add_leaf(b'X');
        assert!(tree.construct_tree().is_ok());
        assert_eq!(tree.root, Some(0));
        tree.populate_cache(None, None);

        assert_eq!(tree.find_leaf(b'X'), Some((0, 0)));
        assert_eq!(tree.find_leaf(b'Y'), None);
    }

    fn build_abc_tree() -> Tree {
        let mut tree = Tree::new();
        tree.add_leaf(b'A');
        tree.add_leaf(b'A');
        tree.add_leaf(b'A');
        tree.add_leaf(b'B');
        tree.add_leaf(b'B');
        tree.add_leaf(b'C');
        tree.sort_nodes();
        tree.construct_tree().unwrap();
        tree.populate_cache(None, None);
        tree
    }

    #[test]
    fn test_construct_tree_multiple() {
        let tree = build_abc_tree();

        assert_eq!(tree.find_leaf(b'A'), Some((0, 1)));
        assert_eq!(tree.find_leaf(b'C'), Some((2, 2)));
        assert_eq!(tree.find_leaf(b'B'), Some((3, 2)));

        assert_eq!(tree.find_leaf(b'D'), None);
    }

    #[test]
    fn test_get_next_leaf() {
        let tree = build_abc_tree();

        let buffer = vec![0x70];
        let mut reader = BitReader::new(&buffer, 8);

        assert_eq!(tree.get_next_leaf(&mut reader), Some(b'A'));
        assert_eq!(tree.get_next_leaf(&mut reader), Some(b'B'));
        assert_eq!(tree.get_next_leaf(&mut reader), Some(b'C'));
        assert_eq!(tree.get_next_leaf(&mut reader), Some(b'A'));
    }

    #[test]
    fn test_get_next_leaf_incomplete() {
        let tree = build_abc_tree();

        let buffer = Vec::new();
        let mut reader = BitReader::new(&buffer, 0);

        assert_eq!(tree.get_next_leaf(&mut reader), None);
    }

    #[test]
    fn test_huffman_round_trip() {
        let input_bytes = b"hello world huffman encoding test";
        let mut tree = Tree::new();
        for &byte in input_bytes {
            tree.add_leaf(byte);
        }
        tree.sort_nodes();
        tree.construct_tree().unwrap();
        tree.populate_cache(None, None);

        let mut buffer = Vec::new();
        let bit_count = {
            let mut writer = BitWriter::new(&mut buffer);
            for &byte in input_bytes {
                let bits = tree.find_leaf(byte).unwrap();
                writer.push(bits);
            }
            let count = (writer.buffer.len() * 8 + writer.bit_count as usize) as u64;
            writer.flush();
            count
        };

        let mut reader = BitReader::new(&buffer, bit_count);
        let mut decoded_bytes = Vec::new();

        for _ in 0..input_bytes.len() {
            if let Some(byte) = tree.get_next_leaf(&mut reader) {
                decoded_bytes.push(byte);
            } else {
                break;
            }
        }

        assert_eq!(decoded_bytes, input_bytes);
    }

    #[test]
    fn test_single_symbol_repeated() {
        let input_bytes = b"AAAAAA";
        let mut tree = Tree::new();
        for &byte in input_bytes {
            tree.add_leaf(byte);
        }
        tree.sort_nodes();
        tree.construct_tree().unwrap();
        tree.populate_cache(None, None);

        let bits = tree.find_leaf(b'A').unwrap();
        assert_eq!(bits, (0, 0));

        let mut buffer = Vec::new();
        {
            let mut writer = BitWriter::new(&mut buffer);
            for &_ in input_bytes {
                writer.push(bits);
            }
            writer.flush();
        }

        assert!(buffer.is_empty());
    }

    #[test]
    fn test_leaf_clone_and_copy() {
        let leaf = Leaf {
            data: b'A',
            frequency: 10,
        };
        let cloned_leaf = leaf;
        assert_eq!(cloned_leaf, leaf);

        let copied_leaf = leaf;
        assert_eq!(copied_leaf, leaf);
    }

    #[test]
    fn test_node_clone_and_copy() {
        let leaf = Node::Leaf(Leaf {
            data: b'A',
            frequency: 10,
        });
        let cloned_node = leaf;
        assert_eq!(cloned_node, leaf);

        let copied_node = leaf;
        assert_eq!(copied_node, leaf);

        let branch = Node::Branch(Branch::new(100, 1, 2));
        let cloned_branch = branch;
        assert_eq!(cloned_branch, branch);

        let copied_branch = branch;
        assert_eq!(copied_branch, branch);
    }
}
