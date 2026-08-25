use std::io::{self, Write};

use crate::hashmap::HashMap;

pub const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;

pub struct TreeNode {
    pub id: u64,
    parent_id: u64,
    pub child_ids: Vec<u64>,
    pub data: Vec<u8>,
}

pub struct Tree {
    node_map: HashMap<TreeNode>,
    pub root_id: u64,
    pub has_root: bool,
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

    pub fn add_node(&mut self, id: u64, parent_id: u64, data: Option<&[u8]>) -> Result<(), ()> {
        if self.contains(id) {
            eprintln!("Error: Node with ID {id} already exists");
            return Err(());
        }

        let mut node = TreeNode {
            id,
            parent_id,
            child_ids: Vec::new(),
            data: data
                .map(|value| value[..value.len().min(MAX_DATA_LENGTH - 1)].to_vec())
                .unwrap_or_default(),
        };

        if !self.has_root {
            self.root_id = id;
            self.has_root = true;
            node.parent_id = 0;
        } else {
            let Some(parent) = self.node_map.get_mut(parent_id) else {
                eprintln!("Error: Parent node {parent_id} not found");
                return Err(());
            };

            if parent.child_ids.len() >= MAX_CHILDREN {
                eprintln!("Error: Parent has maximum children");
                return Err(());
            }

            parent.child_ids.push(id);
        }

        self.node_map.put(id, node);
        self.node_count += 1;
        Ok(())
    }

    pub fn remove_node(&mut self, id: u64) -> Result<(), ()> {
        let Some(node) = self.get_node(id) else {
            eprintln!("Error: Node {id} not found");
            return Err(());
        };
        let parent_id = node.parent_id;

        if id == self.root_id {
            let _ = self.remove_subtree(id);
            self.has_root = false;
            self.root_id = 0;
            return Ok(());
        }

        if let Some(parent) = self.node_map.get_mut(parent_id) {
            if let Some(index) = parent.child_ids.iter().position(|child_id| *child_id == id) {
                parent.child_ids.remove(index);
            }
        }

        let _ = self.remove_subtree(id);
        Ok(())
    }

    fn remove_subtree(&mut self, id: u64) -> Result<(), ()> {
        let Some(node) = self.get_node(id) else {
            return Err(());
        };
        let child_ids = node.child_ids.clone();

        for child_id in child_ids {
            let _ = self.remove_subtree(child_id);
        }

        if self.node_map.remove(id).is_some() {
            self.node_count -= 1;
        }
        Ok(())
    }

    pub fn get_node(&self, id: u64) -> Option<&TreeNode> {
        self.node_map.get(id)
    }

    pub fn contains(&self, id: u64) -> bool {
        self.get_node(id).is_some()
    }

    pub fn len(&self) -> usize {
        self.node_count
    }

    pub fn print(&self, output: &mut impl Write) -> io::Result<()> {
        if !self.has_root {
            writeln!(output, "(empty tree)")?;
            return Ok(());
        }
        self.print_node(self.root_id, 0, output)
    }

    fn print_node(&self, id: u64, depth: usize, output: &mut impl Write) -> io::Result<()> {
        let Some(node) = self.get_node(id) else {
            return Ok(());
        };

        for _ in 0..depth {
            write!(output, "  ")?;
        }
        write!(output, "[{}] ", node.id)?;
        output.write_all(&node.data)?;
        writeln!(output)?;

        for child_id in &node.child_ids {
            self.print_node(*child_id, depth + 1, output)?;
        }
        Ok(())
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

        let mut temp_path = Vec::with_capacity(1000);
        let mut current_id = id;
        while temp_path.len() < 1000 {
            temp_path.push(current_id);
            if current_id == self.root_id {
                break;
            }

            let Some(node) = self.get_node(current_id) else {
                return -1;
            };
            current_id = node.parent_id;
        }

        let mut length = temp_path.len() as i32;
        if length > max_length {
            length = max_length;
        }

        for index in 0..length.max(0) as usize {
            path[index] = temp_path[length as usize - 1 - index];
        }
        length
    }
}
