// Rust translation of the C tree/hashmap test driver.
//
// The C program is a self-contained test driver that prints fixed output to
// stdout (test progress, tree_print) and a couple of error messages to stderr
// for the intentional failure tests. The translation aims to produce
// byte-identical output for both streams.

use std::collections::HashMap;

const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;

type TreeId = u64;

// ---------------------------------------------------------------------------
// Tree data structures
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct TreeNode {
    id: TreeId,
    parent_id: TreeId,
    child_ids: Vec<TreeId>,
    data: String,
}

struct Tree {
    nodes: HashMap<TreeId, TreeNode>,
    root_id: TreeId,
    has_root: bool,
    node_count: usize,
}

impl Tree {
    fn new() -> Self {
        Tree {
            nodes: HashMap::new(),
            root_id: 0,
            has_root: false,
            node_count: 0,
        }
    }

    fn contains(&self, id: TreeId) -> bool {
        self.nodes.contains_key(&id)
    }

    fn size(&self) -> usize {
        self.node_count
    }

    fn get(&self, id: TreeId) -> Option<&TreeNode> {
        self.nodes.get(&id)
    }

    /// Mirrors the C strncpy(node->data, data, MAX_DATA_LENGTH - 1) followed by
    /// forced NUL termination at position MAX_DATA_LENGTH - 1. We model the C
    /// "data" buffer as a String containing only the bytes up to (but not
    /// including) the NUL terminator.
    fn make_data(data: &str) -> String {
        let bytes = data.as_bytes();
        let max = MAX_DATA_LENGTH - 1;
        if bytes.len() <= max {
            data.to_string()
        } else {
            // Truncate at byte boundary; the test data is ASCII so this is safe.
            String::from_utf8_lossy(&bytes[..max]).into_owned()
        }
    }

    fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: &str) -> i32 {
        if self.contains(id) {
            eprintln!("Error: Node with ID {} already exists", id);
            return -1;
        }

        let truncated = Self::make_data(data);

        if !self.has_root {
            // First node becomes the root; its parent_id is forced to 0.
            let node = TreeNode {
                id,
                parent_id: 0,
                child_ids: Vec::new(),
                data: truncated,
            };
            self.root_id = id;
            self.has_root = true;
            self.nodes.insert(id, node);
        } else {
            // Mirror the C order of validation: parent existence check first,
            // then capacity, then insertion.
            let parent_exists = self.nodes.contains_key(&parent_id);
            if !parent_exists {
                eprintln!("Error: Parent node {} not found", parent_id);
                return -1;
            }
            {
                let parent = self.nodes.get_mut(&parent_id).unwrap();
                if parent.child_ids.len() >= MAX_CHILDREN {
                    eprintln!("Error: Parent has maximum children");
                    return -1;
                }
                parent.child_ids.push(id);
            }
            let node = TreeNode {
                id,
                parent_id,
                child_ids: Vec::new(),
                data: truncated,
            };
            self.nodes.insert(id, node);
        }

        self.node_count += 1;
        0
    }

    fn remove_subtree(&mut self, id: TreeId) {
        // Snapshot children to avoid borrowing issues during recursion.
        let children = match self.nodes.get(&id) {
            Some(n) => n.child_ids.clone(),
            None => return,
        };
        for child in children {
            self.remove_subtree(child);
        }
        if self.nodes.remove(&id).is_some() {
            self.node_count -= 1;
        }
    }

    fn remove_node(&mut self, id: TreeId) -> i32 {
        if !self.contains(id) {
            eprintln!("Error: Node {} not found", id);
            return -1;
        }

        // Removing the root empties the tree.
        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = false;
            self.root_id = 0;
            return 0;
        }

        // Detach this node from its parent's child list.
        let parent_id = self.nodes.get(&id).unwrap().parent_id;
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            if let Some(pos) = parent.child_ids.iter().position(|&c| c == id) {
                parent.child_ids.remove(pos);
            }
        }

        self.remove_subtree(id);
        0
    }

    fn get_depth(&self, id: TreeId) -> i32 {
        if !self.contains(id) {
            return -1;
        }
        let mut depth = 0i32;
        let mut current_id = id;
        while current_id != self.root_id {
            let node = match self.nodes.get(&current_id) {
                Some(n) => n,
                None => return -1,
            };
            current_id = node.parent_id;
            depth += 1;
        }
        depth
    }

    fn get_height(&self, id: TreeId) -> i32 {
        let node = match self.nodes.get(&id) {
            Some(n) => n,
            None => return -1,
        };
        if node.child_ids.is_empty() {
            return 0;
        }
        let mut max_height = 0i32;
        let children = node.child_ids.clone();
        for child in children {
            let h = self.get_height(child);
            if h > max_height {
                max_height = h;
            }
        }
        max_height + 1
    }

    fn count_descendants(&self, id: TreeId) -> i32 {
        let node = match self.nodes.get(&id) {
            Some(n) => n,
            None => return -1,
        };
        let mut count = 0i32;
        let children = node.child_ids.clone();
        for child in children {
            count += 1;
            count += self.count_descendants(child);
        }
        count
    }

    fn find_path(&self, id: TreeId, path: &mut [TreeId], max_length: i32) -> i32 {
        if !self.contains(id) {
            return -1;
        }
        let mut temp_path: Vec<TreeId> = Vec::with_capacity(1000);
        let mut current_id = id;
        loop {
            if temp_path.len() >= 1000 {
                break;
            }
            temp_path.push(current_id);
            if current_id == self.root_id {
                break;
            }
            let node = match self.nodes.get(&current_id) {
                Some(n) => n,
                None => return -1,
            };
            current_id = node.parent_id;
        }

        let mut length = temp_path.len() as i32;
        if length > max_length {
            length = max_length;
        }
        for i in 0..length as usize {
            path[i] = temp_path[temp_path.len() - 1 - i];
        }
        length
    }

    fn print(&self) {
        if !self.has_root {
            println!("(empty tree)");
            return;
        }
        self.print_helper(self.root_id, 0);
    }

    fn print_helper(&self, id: TreeId, depth: i32) {
        let node = match self.nodes.get(&id) {
            Some(n) => n,
            None => return,
        };
        for _ in 0..depth {
            print!("  ");
        }
        println!("[{}] {}", node.id, node.data);
        let children = node.child_ids.clone();
        for child in children {
            self.print_helper(child, depth + 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Test functions (mirror the C TEST_PASS macro behaviour)
// ---------------------------------------------------------------------------

fn test_pass(name: &str) {
    println!("\u{2713} PASS: {}", name);
}

fn test_hashmap_basic() {
    println!();
    println!("=== Testing Hashmap Basic Operations ===");

    let mut map: HashMap<TreeId, i32> = HashMap::new();
    assert_eq!(map.len(), 0);

    let val1 = 42i32;
    let val2 = 100i32;
    let val3 = 200i32;
    map.insert(1, val1);
    map.insert(2, val2);
    map.insert(3, val3);
    assert_eq!(map.len(), 3);

    assert_eq!(*map.get(&1).unwrap(), 42);
    assert_eq!(*map.get(&2).unwrap(), 100);
    assert_eq!(*map.get(&3).unwrap(), 200);

    let val4 = 500i32;
    map.insert(1, val4);
    assert_eq!(map.len(), 3);
    assert_eq!(*map.get(&1).unwrap(), 500);

    let removed = map.remove(&2);
    assert_eq!(removed, Some(val2));
    assert_eq!(map.len(), 2);
    assert!(map.get(&2).is_none());

    assert!(map.contains_key(&1));
    assert!(!map.contains_key(&2));
    assert!(map.contains_key(&3));

    test_pass("test_hashmap_basic");
}

fn test_hashmap_collisions() {
    println!();
    println!("=== Testing Hashmap Collisions ===");

    let mut map: HashMap<TreeId, i32> = HashMap::new();
    let mut values = [0i32; 100];
    for i in 0..100 {
        values[i] = (i as i32) * 10;
        map.insert(i as TreeId, values[i]);
    }

    assert_eq!(map.len(), 100);

    for i in 0..100 {
        let v = map.get(&(i as TreeId));
        assert!(v.is_some());
        assert_eq!(*v.unwrap(), (i as i32) * 10);
    }

    test_pass("test_hashmap_collisions");
}

fn test_tree_creation() {
    println!();
    println!("=== Testing Tree Creation ===");

    let tree = Tree::new();
    assert_eq!(tree.size(), 0);
    assert!(!tree.has_root);

    test_pass("test_tree_creation");
}

fn test_tree_add_root() {
    println!();
    println!("=== Testing Tree Add Root ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.size(), 1);
    assert!(tree.has_root);
    assert_eq!(tree.root_id, 1);

    let root = tree.get(1).unwrap();
    assert_eq!(root.id, 1);
    assert_eq!(root.data, "root");
    assert_eq!(root.child_ids.len(), 0);

    test_pass("test_tree_add_root");
}

fn test_tree_add_children() {
    println!();
    println!("=== Testing Tree Add Children ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.add_node(2, 1, "child1"), 0);
    assert_eq!(tree.add_node(3, 1, "child2"), 0);
    assert_eq!(tree.add_node(4, 1, "child3"), 0);

    assert_eq!(tree.size(), 4);

    let root = tree.get(1).unwrap();
    assert_eq!(root.child_ids.len(), 3);
    assert_eq!(root.child_ids[0], 2);
    assert_eq!(root.child_ids[1], 3);
    assert_eq!(root.child_ids[2], 4);

    test_pass("test_tree_add_children");
}

fn test_tree_deep_hierarchy() {
    println!();
    println!("=== Testing Tree Deep Hierarchy ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "level0"), 0);
    assert_eq!(tree.add_node(2, 1, "level1"), 0);
    assert_eq!(tree.add_node(3, 2, "level2"), 0);
    assert_eq!(tree.add_node(4, 3, "level3"), 0);
    assert_eq!(tree.add_node(5, 4, "level4"), 0);

    assert_eq!(tree.size(), 5);

    assert_eq!(tree.get_depth(1), 0);
    assert_eq!(tree.get_depth(2), 1);
    assert_eq!(tree.get_depth(3), 2);
    assert_eq!(tree.get_depth(4), 3);
    assert_eq!(tree.get_depth(5), 4);

    assert_eq!(tree.get_height(1), 4);
    assert_eq!(tree.get_height(2), 3);
    assert_eq!(tree.get_height(5), 0);

    test_pass("test_tree_deep_hierarchy");
}

fn test_tree_remove_leaf() {
    println!();
    println!("=== Testing Tree Remove Leaf ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.add_node(2, 1, "child1"), 0);
    assert_eq!(tree.add_node(3, 1, "child2"), 0);

    assert_eq!(tree.size(), 3);

    assert_eq!(tree.remove_node(3), 0);
    assert_eq!(tree.size(), 2);
    assert!(!tree.contains(3));

    let root = tree.get(1).unwrap();
    assert_eq!(root.child_ids.len(), 1);
    assert_eq!(root.child_ids[0], 2);

    test_pass("test_tree_remove_leaf");
}

fn test_tree_remove_subtree_test() {
    println!();
    println!("=== Testing Tree Remove Subtree ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.add_node(2, 1, "child1"), 0);
    assert_eq!(tree.add_node(3, 2, "grandchild1"), 0);
    assert_eq!(tree.add_node(4, 2, "grandchild2"), 0);
    assert_eq!(tree.add_node(5, 1, "child2"), 0);

    assert_eq!(tree.size(), 5);

    assert_eq!(tree.remove_node(2), 0);
    assert_eq!(tree.size(), 2);
    assert!(!tree.contains(2));
    assert!(!tree.contains(3));
    assert!(!tree.contains(4));
    assert!(tree.contains(1));
    assert!(tree.contains(5));

    test_pass("test_tree_remove_subtree");
}

fn test_tree_remove_root() {
    println!();
    println!("=== Testing Tree Remove Root ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.add_node(2, 1, "child1"), 0);
    assert_eq!(tree.add_node(3, 1, "child2"), 0);

    assert_eq!(tree.size(), 3);

    assert_eq!(tree.remove_node(1), 0);
    assert_eq!(tree.size(), 0);
    assert!(!tree.has_root);

    test_pass("test_tree_remove_root");
}

fn test_tree_count_descendants() {
    println!();
    println!("=== Testing Tree Count Descendants ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.add_node(2, 1, "child1"), 0);
    assert_eq!(tree.add_node(3, 2, "grandchild1"), 0);
    assert_eq!(tree.add_node(4, 2, "grandchild2"), 0);
    assert_eq!(tree.add_node(5, 1, "child2"), 0);

    assert_eq!(tree.count_descendants(1), 4);
    assert_eq!(tree.count_descendants(2), 2);
    assert_eq!(tree.count_descendants(3), 0);
    assert_eq!(tree.count_descendants(5), 0);

    test_pass("test_tree_count_descendants");
}

fn test_tree_find_path() {
    println!();
    println!("=== Testing Tree Find Path ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.add_node(2, 1, "child"), 0);
    assert_eq!(tree.add_node(3, 2, "grandchild"), 0);

    let mut path = [0u64; 10];

    let length = tree.find_path(3, &mut path, 10);
    assert_eq!(length, 3);
    assert_eq!(path[0], 1);
    assert_eq!(path[1], 2);
    assert_eq!(path[2], 3);

    let length = tree.find_path(1, &mut path, 10);
    assert_eq!(length, 1);
    assert_eq!(path[0], 1);

    test_pass("test_tree_find_path");
}

fn test_tree_duplicate_id() {
    println!();
    println!("=== Testing Tree Duplicate ID ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.add_node(2, 1, "child"), 0);

    assert_ne!(tree.add_node(2, 1, "duplicate"), 0);
    assert_eq!(tree.size(), 2);

    test_pass("test_tree_duplicate_id");
}

fn test_tree_max_children() {
    println!();
    println!("=== Testing Tree Max Children ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);

    for i in 0..MAX_CHILDREN {
        assert_eq!(tree.add_node((i + 2) as TreeId, 1, "child"), 0);
    }

    assert_ne!(tree.add_node((MAX_CHILDREN + 2) as TreeId, 1, "overflow"), 0);
    assert_eq!(tree.size(), MAX_CHILDREN + 1);

    test_pass("test_tree_max_children");
}

fn test_tree_complex_structure() {
    println!();
    println!("=== Testing Tree Complex Structure ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.add_node(2, 1, "child1"), 0);
    assert_eq!(tree.add_node(3, 1, "child2"), 0);
    assert_eq!(tree.add_node(4, 1, "child3"), 0);
    assert_eq!(tree.add_node(5, 2, "gc1"), 0);
    assert_eq!(tree.add_node(6, 2, "gc2"), 0);
    assert_eq!(tree.add_node(7, 3, "gc3"), 0);
    assert_eq!(tree.add_node(8, 4, "gc4"), 0);
    assert_eq!(tree.add_node(9, 4, "gc5"), 0);
    assert_eq!(tree.add_node(10, 7, "ggc1"), 0);

    assert_eq!(tree.size(), 10);
    assert_eq!(tree.get_height(1), 3);
    assert_eq!(tree.count_descendants(1), 9);
    assert_eq!(tree.count_descendants(2), 2);
    assert_eq!(tree.count_descendants(7), 1);

    tree.print();

    test_pass("test_tree_complex_structure");
}

fn main() {
    let bar = "\u{2550}".repeat(40);
    println!("\u{2554}{}\u{2557}", bar);
    println!("\u{2551}  TREE WITH HASHMAP ID MAPPING TESTS   \u{2551}");
    println!("\u{255A}{}\u{255D}", bar);

    test_hashmap_basic();
    test_hashmap_collisions();

    test_tree_creation();
    test_tree_add_root();
    test_tree_add_children();

    test_tree_deep_hierarchy();
    test_tree_complex_structure();

    test_tree_remove_leaf();
    test_tree_remove_subtree_test();
    test_tree_remove_root();

    test_tree_count_descendants();
    test_tree_find_path();

    test_tree_duplicate_id();
    test_tree_max_children();

    println!();
    println!("========================================");
    println!("  All tests passed successfully!");
    println!("========================================");
}
