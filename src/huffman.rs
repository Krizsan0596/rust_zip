struct Leaf {
    data: u8,
    frequency: u64,
}

struct Branch {
    left: u64,
    right: u64,
}

pub enum Node {
    Leaf,
    Branch,
}

pub struct Tree {
    pub root: Option<usize>,
    pub nodes: Vec<Node>,
}

impl Tree {
    pub fn new() -> Self {
        Tree {
            root: None,
            nodes: Vec::new(),
        }
    }

    pub fn add_leaf(&mut self, value: u8) {
        let idx = self.nodes.len();
        self.nodes.push(Node::Leaf(value));
    }

    pub fn sort_nodes(&mut self) {
        self.nodes.sort_unstable_by(|a, b| b.frequency.cmp(&a.frequency));
    }
}
