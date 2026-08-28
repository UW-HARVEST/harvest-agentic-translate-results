use crate::hashmap::HashMap;

pub const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;

pub struct TreeNode {
    pub id: u64,
    pub parent_id: u64,
    pub child_ids: Vec<u64>,
    pub data: Vec<u8>,
}

pub struct Tree {
    nodes: HashMap<TreeNode>,
    pub root_id: u64,
    pub has_root: bool,
    node_count: usize,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root_id: 0,
            has_root: false,
            node_count: 0,
        }
    }

    pub fn add_node(&mut self, id: u64, parent_id: u64, data: Option<&[u8]>) -> i32 {
        if self.contains(id) {
            eprintln!("Error: Node with ID {id} already exists");
            return -1;
        }

        let mut node = TreeNode {
            id,
            parent_id,
            child_ids: Vec::new(),
            data: data.unwrap_or_default()
                [..data.unwrap_or_default().len().min(MAX_DATA_LENGTH - 1)]
                .to_vec(),
        };

        if !self.has_root {
            self.root_id = id;
            self.has_root = true;
            node.parent_id = 0;
        } else {
            let Some(parent) = self.nodes.get_mut(parent_id) else {
                eprintln!("Error: Parent node {parent_id} not found");
                return -1;
            };

            if parent.child_ids.len() >= MAX_CHILDREN {
                eprintln!("Error: Parent has maximum children");
                return -1;
            }

            parent.child_ids.push(id);
        }

        if self.nodes.put(id, node) != 0 {
            eprintln!("Error: Failed to add node to hashmap");
            return -1;
        }

        self.node_count += 1;
        0
    }

    fn remove_subtree(&mut self, id: u64) -> i32 {
        let Some(node) = self.nodes.get(id) else {
            return -1;
        };
        let child_ids = node.child_ids.clone();

        for child_id in child_ids {
            self.remove_subtree(child_id);
        }

        if self.nodes.remove(id).is_some() {
            self.node_count -= 1;
        }
        0
    }

    pub fn remove_node(&mut self, id: u64) -> i32 {
        let Some(node) = self.nodes.get(id) else {
            eprintln!("Error: Node {id} not found");
            return -1;
        };
        let parent_id = node.parent_id;

        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = false;
            self.root_id = 0;
            return 0;
        }

        if let Some(parent) = self.nodes.get_mut(parent_id) {
            if let Some(index) = parent.child_ids.iter().position(|child| *child == id) {
                parent.child_ids.remove(index);
            }
        }

        self.remove_subtree(id);
        0
    }

    pub fn get_node(&self, id: u64) -> Option<&TreeNode> {
        self.nodes.get(id)
    }

    pub fn contains(&self, id: u64) -> bool {
        self.get_node(id).is_some()
    }

    pub fn len(&self) -> usize {
        self.node_count
    }

    fn print_node(&self, id: u64, depth: usize) {
        let Some(node) = self.get_node(id) else {
            return;
        };

        let data = String::from_utf8_lossy(&node.data);
        println!("{}[{}] {}", "  ".repeat(depth), node.id, data);
        for child_id in &node.child_ids {
            self.print_node(*child_id, depth + 1);
        }
    }

    pub fn print(&self) {
        if !self.has_root {
            println!("(empty tree)");
            return;
        }
        self.print_node(self.root_id, 0);
    }

    pub fn depth(&self, id: u64) -> i32 {
        if !self.contains(id) {
            return -1;
        }

        let mut depth = 0;
        let mut current_id = id;
        while current_id != self.root_id {
            let Some(node) = self.get_node(current_id) else {
                return -1;
            };
            current_id = node.parent_id;
            depth += 1;
        }
        depth
    }

    pub fn height(&self, id: u64) -> i32 {
        let Some(node) = self.get_node(id) else {
            return -1;
        };
        if node.child_ids.is_empty() {
            return 0;
        }

        let mut max_height = 0;
        for child_id in &node.child_ids {
            let child_height = self.height(*child_id);
            if child_height > max_height {
                max_height = child_height;
            }
        }
        max_height + 1
    }

    pub fn count_descendants(&self, id: u64) -> i32 {
        let Some(node) = self.get_node(id) else {
            return -1;
        };

        let mut count = 0;
        for child_id in &node.child_ids {
            count += 1;
            count += self.count_descendants(*child_id);
        }
        count
    }

    pub fn find_path(&self, id: u64, path: &mut [u64], max_length: i32) -> i32 {
        if !self.contains(id) {
            return -1;
        }

        let mut temp_path = [0_u64; 1000];
        let mut length = 0_usize;
        let mut current_id = id;

        while length < temp_path.len() {
            temp_path[length] = current_id;
            length += 1;

            if current_id == self.root_id {
                break;
            }
            let Some(node) = self.get_node(current_id) else {
                return -1;
            };
            current_id = node.parent_id;
        }

        if length as i32 > max_length {
            length = max_length as usize;
        }
        for (index, destination) in path.iter_mut().take(length).enumerate() {
            *destination = temp_path[length - 1 - index];
        }
        length as i32
    }
}
