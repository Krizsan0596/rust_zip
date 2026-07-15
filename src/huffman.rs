use crate::file::BitReader;

struct Leaf {
    frequency: u64,
    data: u8,
}

struct Branch {
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

enum Node {
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
    nodes: Vec<Node>,
    cache: Box<[Option<String>; 256]>,
}

impl Tree {
    pub fn new() -> Self {
        Tree {
            root: None,
            nodes: Vec::new(),
            cache: vec![None; 256].into_boxed_slice().try_into().unwrap(),
        }
    }

    // pub fn get_node(&mut self, idx: usize) -> &mut Node {
    //     &mut self.nodes[idx]
    // }

    pub fn add_leaf(&mut self, value: u8) {
        if let Some(node) = self.nodes.iter_mut().find(|node| match node {
            Node::Leaf(leaf) => leaf.data == value,
            _ => false,
        }) {
            if let Node::Leaf(leaf) = node {
                leaf.frequency += 1;
            }
        } else {
            self.nodes.push(Node::Leaf(Leaf {
                data: value,
                frequency: 1,
            }));
        }
    }

    pub fn sort_nodes(&mut self) {
        self.nodes.sort_unstable_by_key(|x| x.frequency());
    }

    pub fn construct_tree(&mut self) -> Result<(), std::io::Error> {
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

        let next_node = |leaf: &mut usize, branch: &mut usize, nodes: &[Node]| -> usize {
            if *leaf < leaf_count
                && (*branch == nodes.len()
                    || nodes[*leaf].frequency() <= nodes[*branch].frequency())
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

            let freq: u64 = self.nodes[left].frequency() + self.nodes[right].frequency();
            self.nodes
                .push(Node::Branch(Branch::new(freq, left as u64, right as u64)));
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
            Node::Leaf(leaf) => {
                if leaf.data == data {
                    Some(String::new())
                } else {
                    None
                }
            }
            Node::Branch(branch) => {
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
        }
    }

    pub fn get_next_leaf<'a>(&self, reader: &mut BitReader<'a>) -> Option<u8> {
        let mut root: &Node = &self.nodes[*self.root.as_ref().unwrap()];
        loop {
            root = match root {
                Node::Leaf(leaf) => return Some(leaf.data),
                Node::Branch(branch) => match reader.read_bit() {
                    Some(true) => &self.nodes[branch.right as usize],
                    Some(false) => &self.nodes[branch.left as usize],
                    None => return None,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_new() {
        let tree = Tree::new();
        assert!(tree.root.is_none());
        assert!(tree.nodes.is_empty());
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
        
        // Add a new leaf
        tree.add_leaf(b'A');
        assert_eq!(tree.nodes.len(), 1);
        match &tree.nodes[0] {
            Node::Leaf(leaf) => {
                assert_eq!(leaf.data, b'A');
                assert_eq!(leaf.frequency, 1);
            }
            _ => panic!("Expected a leaf node"),
        }

        // Add the same leaf again (increment frequency)
        tree.add_leaf(b'A');
        assert_eq!(tree.nodes.len(), 1);
        match &tree.nodes[0] {
            Node::Leaf(leaf) => {
                assert_eq!(leaf.data, b'A');
                assert_eq!(leaf.frequency, 2);
            }
            _ => panic!("Expected a leaf node"),
        }

        // Add a different leaf
        tree.add_leaf(b'B');
        assert_eq!(tree.nodes.len(), 2);
        match &tree.nodes[1] {
            Node::Leaf(leaf) => {
                assert_eq!(leaf.data, b'B');
                assert_eq!(leaf.frequency, 1);
            }
            _ => panic!("Expected a leaf node"),
        }
    }

    #[test]
    fn test_sort_nodes() {
        let mut tree = Tree::new();
        
        // Add leaves such that they are not sorted by frequency
        tree.add_leaf(b'A');
        tree.add_leaf(b'A');
        tree.add_leaf(b'A'); // 'A' frequency = 3

        tree.add_leaf(b'B'); // 'B' frequency = 1

        tree.add_leaf(b'C');
        tree.add_leaf(b'C'); // 'C' frequency = 2

        // Sort them
        tree.sort_nodes();

        // After sorting: B (1), C (2), A (3)
        assert_eq!(tree.nodes.len(), 3);
        
        let get_leaf_data_and_freq = |node: &Node| -> (u8, u64) {
            match node {
                Node::Leaf(leaf) => (leaf.data, leaf.frequency),
                _ => panic!("Expected leaf"),
            }
        };

        assert_eq!(get_leaf_data_and_freq(&tree.nodes[0]), (b'B', 1));
        assert_eq!(get_leaf_data_and_freq(&tree.nodes[1]), (b'C', 2));
        assert_eq!(get_leaf_data_and_freq(&tree.nodes[2]), (b'A', 3));
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

        // Test find_leaf on single-node tree
        assert_eq!(tree.find_leaf(b'X', None), Some(String::new()));
        assert_eq!(tree.find_leaf(b'Y', None), None);
    }

    #[test]
    fn test_construct_tree_multiple() {
        let mut tree = Tree::new();
        
        // Frequencies: A = 3, B = 2, C = 1
        tree.add_leaf(b'A');
        tree.add_leaf(b'A');
        tree.add_leaf(b'A');

        tree.add_leaf(b'B');
        tree.add_leaf(b'B');

        tree.add_leaf(b'C');

        tree.sort_nodes(); // [C(1), B(2), A(3)]
        assert!(tree.construct_tree().is_ok());

        // We expect:
        // root = Some(4)
        // nodes[0] = Leaf C (freq 1)
        // nodes[1] = Leaf B (freq 2)
        // nodes[2] = Leaf A (freq 3)
        // nodes[3] = Branch (freq 3, left = 0, right = 1) -> combines C and B
        // nodes[4] = Branch (freq 6, left = 2, right = 3) -> combines A and nodes[3]
        
        assert_eq!(tree.root, Some(4));
        assert_eq!(tree.nodes.len(), 5);

        // Verify leaf positions
        match &tree.nodes[0] {
            Node::Leaf(leaf) => assert_eq!(leaf.data, b'C'),
            _ => panic!(),
        }
        match &tree.nodes[1] {
            Node::Leaf(leaf) => assert_eq!(leaf.data, b'B'),
            _ => panic!(),
        }
        match &tree.nodes[2] {
            Node::Leaf(leaf) => assert_eq!(leaf.data, b'A'),
            _ => panic!(),
        }

        // Verify Branch 3
        match &tree.nodes[3] {
            Node::Branch(branch) => {
                assert_eq!(branch.frequency, 3);
                assert_eq!(branch.left, 0);
                assert_eq!(branch.right, 1);
            }
            _ => panic!(),
        }

        // Verify Branch 4
        match &tree.nodes[4] {
            Node::Branch(branch) => {
                assert_eq!(branch.frequency, 6);
                assert_eq!(branch.left, 2);
                assert_eq!(branch.right, 3);
            }
            _ => panic!(),
        }

        // Find leaf paths (inverted)
        // Path to A: left (0) from root. Inverted: "0"
        assert_eq!(tree.find_leaf(b'A', None), Some("0".to_string()));
        // Path to C: right (1) then left (0). Inverted: "01"
        assert_eq!(tree.find_leaf(b'C', None), Some("01".to_string()));
        // Path to B: right (1) then right (1). Inverted: "11"
        assert_eq!(tree.find_leaf(b'B', None), Some("11".to_string()));
        
        // Find non-existent leaf
        assert_eq!(tree.find_leaf(b'D', None), None);
    }

    #[test]
    fn test_get_next_leaf() {
        let mut tree = Tree::new();
        tree.add_leaf(b'A');
        tree.add_leaf(b'A');
        tree.add_leaf(b'A');

        tree.add_leaf(b'B');
        tree.add_leaf(b'B');

        tree.add_leaf(b'C');

        tree.sort_nodes();
        tree.construct_tree().unwrap();

        // A is "0", C is "01" (inverted), B is "11" (inverted)
        // The real path to:
        // A is '0'
        // C is '10' (root -> right -> left)
        // B is '11' (root -> right -> right)
        
        // Let's create a bitstream:
        // 'A' (0), 'B' (11), 'C' (10), 'A' (0)
        // Bits: 0 11 10 0
        // Padding with 0s to make a byte: 0111 0000 = 112 (0x70)
        let buffer = vec![0x70];
        let mut reader = BitReader::new(&buffer);

        assert_eq!(tree.get_next_leaf(&mut reader), Some(b'A'));
        assert_eq!(tree.get_next_leaf(&mut reader), Some(b'B'));
        assert_eq!(tree.get_next_leaf(&mut reader), Some(b'C'));
        assert_eq!(tree.get_next_leaf(&mut reader), Some(b'A'));
    }

    #[test]
    fn test_get_next_leaf_incomplete() {
        let mut tree = Tree::new();
        tree.add_leaf(b'A');
        tree.add_leaf(b'A');
        tree.add_leaf(b'A');

        tree.add_leaf(b'B');
        tree.add_leaf(b'B');

        tree.add_leaf(b'C');

        tree.sort_nodes();
        tree.construct_tree().unwrap();

        // If the reader has no bytes, reading a bit will return None immediately.
        let buffer = Vec::new();
        let mut reader = BitReader::new(&buffer);
        
        assert_eq!(tree.get_next_leaf(&mut reader), None);
    }
}
