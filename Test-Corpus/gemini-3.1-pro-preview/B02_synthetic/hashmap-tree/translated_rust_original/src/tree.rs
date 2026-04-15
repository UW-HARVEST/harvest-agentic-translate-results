use crate::hashmap::{HashMap, TreeId};

pub const MAX_CHILDREN: usize = 32;
pub const MAX_DATA_LENGTH: usize = 256;

#[derive(Clone)]
pub struct TreeNode {
    pub id: TreeId,
    pub parent_id: TreeId,
    pub child_ids: Vec<TreeId>,
    pub data: String,
}

pub struct Tree {
    node_map: HashMap<TreeNode>,
    root_id: TreeId,
    has_root: bool,
    node_count: usize,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            node_map: HashMap::new(),
            root_id: 0,
            has_root: false,
            node_count: 0,
        }
    }

    pub fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: &str) -> Result<(), &'static str> {
        if self.contains(id) {
            eprintln!("Error: Node with ID {} already exists", id);
            return Err("Node already exists");
        }

        let mut node = TreeNode {
            id,
            parent_id,
            child_ids: Vec::new(),
            data: data.to_string(),
        };

        if !self.has_root {
            self.root_id = id;
            self.has_root = true;
            node.parent_id = 0;
        } else {
            let parent = self.node_map.get_mut(parent_id).ok_or_else(|| {
                eprintln!("Error: Parent node {} not found", parent_id);
                "Parent node not found"
            })?;
            
            if parent.child_ids.len() >= MAX_CHILDREN {
                eprintln!("Error: Parent has maximum children");
                return Err("Parent has maximum children");
            }
            
            parent.child_ids.push(id);
        }

        self.node_map.put(id, node).unwrap();
        self.node_count += 1;
        Ok(())
    }

    pub fn remove_node(&mut self, id: TreeId) -> Result<(), &'static str> {
        if !self.contains(id) {
            eprintln!("Error: Node {} not found", id);
            return Err("Node not found");
        }

        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = false;
            self.root_id = 0;
            return Ok(());
        }

        let parent_id = self.node_map.get(id).unwrap().parent_id;
        if let Some(parent) = self.node_map.get_mut(parent_id) {
            parent.child_ids.retain(|&child_id| child_id != id);
        }

        self.remove_subtree(id);
        Ok(())
    }

    fn remove_subtree(&mut self, id: TreeId) {
        let child_ids = if let Some(node) = self.node_map.get(id) {
            node.child_ids.clone()
        } else {
            return;
        };

        for child_id in child_ids {
            self.remove_subtree(child_id);
        }

        if self.node_map.remove(id).is_some() {
            self.node_count -= 1;
        }
    }

    pub fn get_node(&self, id: TreeId) -> Option<&TreeNode> {
        self.node_map.get(id)
    }

    pub fn contains(&self, id: TreeId) -> bool {
        self.node_map.contains(id)
    }

    pub fn size(&self) -> usize {
        self.node_count
    }

    pub fn has_root(&self) -> bool {
        self.has_root
    }

    pub fn root_id(&self) -> TreeId {
        self.root_id
    }

    pub fn print(&self) {
        if !self.has_root {
            println!("(empty tree)");
            return;
        }
        self.print_helper(self.root_id, 0);
    }

    fn print_helper(&self, id: TreeId, depth: usize) {
        if let Some(node) = self.get_node(id) {
            for _ in 0..depth {
                print!("  ");
            }
            println!("[{}] {}", node.id, node.data);
            for &child_id in &node.child_ids {
                self.print_helper(child_id, depth + 1);
            }
        }
    }

    pub fn get_depth(&self, id: TreeId) -> Option<usize> {
        if !self.contains(id) {
            return None;
        }

        let mut depth = 0;
        let mut current_id = id;

        while current_id != self.root_id {
            if let Some(node) = self.get_node(current_id) {
                current_id = node.parent_id;
                depth += 1;
            } else {
                return None;
            }
        }

        Some(depth)
    }

    pub fn get_height(&self, id: TreeId) -> Option<usize> {
        let node = self.get_node(id)?;
        if node.child_ids.is_empty() {
            return Some(0);
        }

        let mut max_height = 0;
        for &child_id in &node.child_ids {
            if let Some(child_height) = self.get_height(child_id) {
                if child_height > max_height {
                    max_height = child_height;
                }
            }
        }

        Some(max_height + 1)
    }

    pub fn count_descendants(&self, id: TreeId) -> Option<usize> {
        let node = self.get_node(id)?;
        let mut count = 0;
        for &child_id in &node.child_ids {
            count += 1;
            if let Some(descendants) = self.count_descendants(child_id) {
                count += descendants;
            }
        }
        Some(count)
    }

    pub fn find_path(&self, id: TreeId, path: &mut [TreeId]) -> Option<usize> {
        if !self.contains(id) {
            return None;
        }

        let mut temp_path = Vec::new();
        let mut current_id = id;

        while temp_path.len() < 1000 {
            temp_path.push(current_id);

            if current_id == self.root_id {
                break;
            }

            if let Some(node) = self.get_node(current_id) {
                current_id = node.parent_id;
            } else {
                return None;
            }
        }

        let mut length = temp_path.len();
        if length > path.len() {
            length = path.len();
        }
        
        for i in 0..length {
            path[i] = temp_path[length - 1 - i];
        }

        Some(length)
    }
}
