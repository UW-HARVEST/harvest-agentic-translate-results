use crate::hashmap::{HashMap, TreeId};

const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;

pub struct TreeNode {
    pub id: TreeId,
    pub parent_id: TreeId,
    pub child_ids: [TreeId; MAX_CHILDREN],
    pub child_count: usize,
    pub data: String,
}

impl TreeNode {
    fn new(id: TreeId, parent_id: TreeId, data: &str) -> Self {
        let mut data_str = data.to_string();
        if data_str.len() >= MAX_DATA_LENGTH {
            data_str.truncate(MAX_DATA_LENGTH - 1);
        }
        
        Self {
            id,
            parent_id,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: data_str,
        }
    }
}

pub struct Tree {
    node_map: HashMap<Box<TreeNode>>,
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
    
    pub fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: &str) -> Result<(), ()> {
        if self.node_map.contains(id) {
            eprintln!("Error: Node with ID {} already exists", id);
            return Err(());
        }
        
        let mut node = Box::new(TreeNode::new(id, parent_id, data));
        
        if !self.has_root {
            self.root_id = id;
            self.has_root = true;
            node.parent_id = 0;
        } else {
            let parent = self.node_map.get_mut(parent_id).ok_or_else(|| {
                eprintln!("Error: Parent node {} not found", parent_id);
            })?;
            
            if parent.child_count >= MAX_CHILDREN {
                eprintln!("Error: Parent has maximum children");
                return Err(());
            }
            
            parent.child_ids[parent.child_count] = id;
            parent.child_count += 1;
        }
        
        self.node_map.put(id, node);
        self.node_count += 1;
        Ok(())
    }
    
    fn remove_subtree(&mut self, id: TreeId) {
        if let Some(node) = self.node_map.get(id) {
            let child_count = node.child_count;
            let child_ids: Vec<TreeId> = node.child_ids[..child_count].to_vec();
            
            for child_id in child_ids {
                self.remove_subtree(child_id);
            }
        }
        
        if let Some(removed) = self.node_map.remove(id) {
            drop(removed);
            self.node_count -= 1;
        }
    }
    
    pub fn remove_node(&mut self, id: TreeId) -> Result<(), ()> {
        let node = self.node_map.get(id).ok_or_else(|| {
            eprintln!("Error: Node {} not found", id);
        })?;
        
        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = false;
            self.root_id = 0;
            return Ok(());
        }
        
        let parent_id = node.parent_id;
        
        if let Some(parent) = self.node_map.get_mut(parent_id) {
            for i in 0..parent.child_count {
                if parent.child_ids[i] == id {
                    for j in i..parent.child_count - 1 {
                        parent.child_ids[j] = parent.child_ids[j + 1];
                    }
                    parent.child_count -= 1;
                    break;
                }
            }
        }
        
        self.remove_subtree(id);
        Ok(())
    }
    
    pub fn get_node(&self, id: TreeId) -> Option<&TreeNode> {
        self.node_map.get(id).map(|b| b.as_ref())
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
    
    pub fn root_id(&self) -> Option<TreeId> {
        if self.has_root {
            Some(self.root_id)
        } else {
            None
        }
    }
    
    fn print_helper(&self, id: TreeId, depth: usize) {
        if let Some(node) = self.get_node(id) {
            for _ in 0..depth {
                print!("  ");
            }
            println!("[{}] {}", node.id, node.data);
            
            for i in 0..node.child_count {
                self.print_helper(node.child_ids[i], depth + 1);
            }
        }
    }
    
    pub fn print(&self) {
        if !self.has_root {
            println!("(empty tree)");
            return;
        }
        
        self.print_helper(self.root_id, 0);
    }
    
    pub fn get_depth(&self, id: TreeId) -> Option<i32> {
        if !self.contains(id) {
            return None;
        }
        
        let mut depth = 0;
        let mut current_id = id;
        
        while current_id != self.root_id {
            let node = self.get_node(current_id)?;
            current_id = node.parent_id;
            depth += 1;
        }
        
        Some(depth)
    }
    
    pub fn get_height(&self, id: TreeId) -> Option<i32> {
        let node = self.get_node(id)?;
        
        if node.child_count == 0 {
            return Some(0);
        }
        
        let mut max_height = 0;
        for i in 0..node.child_count {
            if let Some(child_height) = self.get_height(node.child_ids[i]) {
                if child_height > max_height {
                    max_height = child_height;
                }
            }
        }
        
        Some(max_height + 1)
    }
    
    pub fn count_descendants(&self, id: TreeId) -> Option<i32> {
        let node = self.get_node(id)?;
        
        let mut count = 0;
        for i in 0..node.child_count {
            count += 1;
            if let Some(descendants) = self.count_descendants(node.child_ids[i]) {
                count += descendants;
            }
        }
        
        Some(count)
    }
    
    pub fn find_path(&self, id: TreeId, max_length: usize) -> Vec<TreeId> {
        if !self.contains(id) {
            return Vec::new();
        }
        
        let mut temp_path: Vec<TreeId> = Vec::new();
        let mut current_id = id;
        
        loop {
            temp_path.push(current_id);
            
            if current_id == self.root_id {
                break;
            }
            
            if let Some(node) = self.get_node(current_id) {
                current_id = node.parent_id;
            } else {
                break;
            }
        }
        
        temp_path.reverse();
        
        if temp_path.len() > max_length {
            temp_path.truncate(max_length);
        }
        
        temp_path
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}
