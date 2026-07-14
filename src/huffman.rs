struct Leaf {
    data: u8,
    frequency: u64,
}

struct Branch {
    left: u64,
    right: u64,
}

enum Node {
    Leaf(Leaf),
    Branch(Branch),
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
        self.nodes.sort_unstable_by(|a, b| match (a, b) {
            (Node::Leaf(a), Node::Leaf(b)) => b.frequency.cmp(&a.frequency),
            _ => std::cmp::Ordering::Equal,
        });
    }
}
