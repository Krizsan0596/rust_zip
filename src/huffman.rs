use crate::file::BitReader;

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
    cache: Box<[Option<String>; 256]>,
}

impl Tree {
    pub fn new() -> Self {
        Tree {
            root: None,
            nodes: vec![None; 256],
            cache: vec![None; 256].into_boxed_slice().try_into().unwrap(),
        }
    }

    // pub fn get_node(&mut self, idx: usize) -> &mut Node {
    //     &mut self.nodes[idx]
    // }

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

    fn check_cache(&self, leaf: u8) -> Option<&String> {
        if let Some(res) = &self.cache[leaf as usize] {
            Some(res)
        } else {
            None
        }
    }

    pub fn find_leaf(&self, data: u8, root: Option<usize>) -> Option<String> {
        //Returns inverted
        //path
        let root: usize = root.unwrap_or(*self.root.as_ref().unwrap());
        if root == *self.root.as_ref().unwrap()
            && let Some(path) = self.check_cache(data)
        {
            return Some(path.clone());
        }

        match &self.nodes[root] {
            Some(Node::Leaf(leaf)) => {
                if leaf.data == data {
                    Some(String::new())
                } else {
                    None
                }
            }
            Some(Node::Branch(branch)) => {
                if let Some(mut x) = self.find_leaf(data, Some(branch.left as usize)) {
                    x.push('0');
                    Some(x)
                } else if let Some(mut x) = self.find_leaf(data, Some(branch.right as usize)) {
                    x.push('1');
                    Some(x)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub fn cache_leaf(&mut self, data: &u8, path: &str) {
        if self.cache[*data as usize].is_none() {
            self.cache[*data as usize] = Some(path.chars().rev().collect());
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

    pub fn import(from: Vec<Leaf>) -> Self {
        let nodes: Vec<Option<Node>> = from
            .into_iter()
            .map(|leaf| Some(Node::Leaf(leaf)))
            .collect();
        Tree {
            root: None,
            nodes,
            cache: vec![None; 256].into_boxed_slice().try_into().unwrap(),
        }
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

        assert_eq!(tree.find_leaf(b'X', None), Some(String::new()));
        assert_eq!(tree.find_leaf(b'Y', None), None);
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
        tree
    }

    #[test]
    fn test_construct_tree_multiple() {
        let tree = build_abc_tree();

        assert_eq!(tree.find_leaf(b'A', None), Some("0".to_string()));
        assert_eq!(tree.find_leaf(b'C', None), Some("01".to_string()));
        assert_eq!(tree.find_leaf(b'B', None), Some("11".to_string()));

        assert_eq!(tree.find_leaf(b'D', None), None);
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

        let mut buffer = Vec::new();
        let bit_count = {
            let mut writer = BitWriter::new(&mut buffer);
            for &byte in input_bytes {
                let bits = tree.find_leaf(byte, None).unwrap();
                let reversed_bits: String = bits.chars().rev().collect();
                writer.push(&reversed_bits);
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

        let bits = tree.find_leaf(b'A', None).unwrap();
        assert_eq!(bits, "");

        let mut buffer = Vec::new();
        {
            let mut writer = BitWriter::new(&mut buffer);
            for &_ in input_bytes {
                let reversed_bits: String = bits.chars().rev().collect();
                writer.push(&reversed_bits);
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

    #[test]
    fn test_cache_leaf() {
        let mut tree = build_abc_tree();

        // Before caching, find_leaf returns inverted path for 'C' which is "01"
        assert_eq!(tree.find_leaf(b'C', None), Some("01".to_string()));

        // Cache the correct path "10" for 'C'. It should be stored inverted as "01".
        tree.cache_leaf(&b'C', "10");

        // find_leaf should now hit the cache and return the stored inverted path "01"
        assert_eq!(tree.find_leaf(b'C', None), Some("01".to_string()));
    }
}
