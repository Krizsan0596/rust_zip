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

    pub fn get_node(&mut self, idx: usize) -> &mut Node {
        return &mut self.nodes[idx];
    }

    pub fn add_leaf(&mut self, value: u8) {
        if let Some(node) = self.nodes.iter_mut().find(|node| {
            match node {
                Node::Leaf(leaf) => leaf.data == value,
                _ => false,
            }
        }) {
            if let Node::Leaf(leaf) = node {
                leaf.frequency += 1;
            }
        }
        else {
            self.nodes.push(Node::Leaf(Leaf {
                data: value,
                frequency: 1,
            }));
        }
    }

    pub fn sort_nodes(&mut self) {
        self.nodes.sort_unstable_by(|a, b| a.frequency().cmp(&b.frequency()));
    }

    pub fn construct_tree(&mut self) -> Result<(), std::io::Error> {
        if self.nodes.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Input file is empty."));
        }
        if self.nodes.len() == 1 {
            self.root = Some(0);
            return Ok(());
        }

        let leaf_count:usize = self.nodes.len();
        self.nodes.reserve(self.nodes.len() - 1); // Huffman tree with n leaves has 2n-1 nodes.

        
        let mut current_leaf: usize = 0;
        let mut current_branch: usize = leaf_count;

        let next_node = |leaf: &mut usize, branch: &mut usize, nodes: &[Node]| -> usize {
                if *leaf < leaf_count
                    && (*branch == nodes.len() || nodes[*leaf].frequency() <= nodes[*branch].frequency())
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

        for _ in 0..leaf_count-1 {
            let left = next_node(&mut current_leaf, &mut current_branch, &self.nodes);
            let right = next_node(&mut current_leaf, &mut current_branch, &self.nodes);

            let freq: u64 = self.nodes[left].frequency() + self.nodes[right].frequency();
            self.nodes.push(Node::Branch(Branch::new(freq, left as u64, right as u64)));
            self.root = Some(self.nodes.len() - 1);
        }

        Ok(())
    }

    fn check_cache(&self, leaf: u8) -> Option<&String> {
        if let Some(res) = &self.cache[leaf as usize] {
            return Some(res);
        }
        else {
            return None;
        }
    }

    pub fn find_leaf(&self, data: u8, root: Option<usize>) -> Option<String> { //Returns inverted
                                                                               //path
        let root: usize = root.unwrap_or(*self.root.as_ref().unwrap());
        if root == *self.root.as_ref().unwrap() && let Some(path) = self.check_cache(data) {
            return Some(path.clone());
        }

        return match &self.nodes[root] {
            Node::Leaf(leaf) => {
                if leaf.data == data {
                    Some(String::new())
                }
                else {
                    None
                }
            }
            Node::Branch(branch) => {
                if let Some(mut x) = self.find_leaf(data, Some(branch.left as usize)) {
                    x.push('0');
                    Some(x)
                }
                else if let Some(mut x) = self.find_leaf(data, Some(branch.right as usize)) {
                    x.push('1');
                    Some(x)
                }
                else {
                    None
                }
            }
        };
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
                }
            }
        }
    }
}
