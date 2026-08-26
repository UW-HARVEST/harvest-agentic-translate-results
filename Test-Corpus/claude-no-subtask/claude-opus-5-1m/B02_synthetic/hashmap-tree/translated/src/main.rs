// Rust translation of the C tree/hashmap test driver.
// Produces byte-identical output to the original C program.

const HASHMAP_INITIAL_CAPACITY: usize = 16;
const HASHMAP_LOAD_FACTOR: f64 = 0.75;

const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;

type TreeId = u64;

#[derive(Clone)]
struct HashmapEntry<V: Clone> {
    key: TreeId,
    value: Option<V>,
    occupied: bool,
    deleted: bool,
}

impl<V: Clone> HashmapEntry<V> {
    fn empty() -> Self {
        Self {
            key: 0,
            value: None,
            occupied: false,
            deleted: false,
        }
    }
}

struct Hashmap<V: Clone> {
    entries: Vec<HashmapEntry<V>>,
    capacity: usize,
    size: usize,
    deleted_count: usize,
}

fn hash_function(key: TreeId) -> u64 {
    // FNV-1a hash, mirroring the C version which iterates over the bytes
    // of the key as stored in memory (little-endian on x86_64 Linux).
    let mut hash: u64 = 14695981039346656037u64;
    let bytes = key.to_le_bytes();
    for i in 0..bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

impl<V: Clone> Hashmap<V> {
    fn new() -> Self {
        Self {
            entries: vec![HashmapEntry::empty(); HASHMAP_INITIAL_CAPACITY],
            capacity: HASHMAP_INITIAL_CAPACITY,
            size: 0,
            deleted_count: 0,
        }
    }

    fn should_resize(&self) -> bool {
        let load = (self.size + self.deleted_count) as f64 / self.capacity as f64;
        load > HASHMAP_LOAD_FACTOR
    }

    fn resize(&mut self) -> i32 {
        let old_capacity = self.capacity;
        let new_capacity = old_capacity * 2;
        let old_entries = std::mem::replace(
            &mut self.entries,
            vec![HashmapEntry::empty(); new_capacity],
        );
        self.capacity = new_capacity;
        self.size = 0;
        self.deleted_count = 0;

        for i in 0..old_capacity {
            if old_entries[i].occupied && !old_entries[i].deleted {
                if let Some(val) = &old_entries[i].value {
                    self.put(old_entries[i].key, val.clone());
                }
            }
        }
        0
    }

    fn put(&mut self, key: TreeId, value: V) -> i32 {
        if self.should_resize() {
            if self.resize() != 0 {
                return -1;
            }
        }

        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        let mut probe = 0usize;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;
            if !self.entries[current].occupied {
                self.entries[current].key = key;
                self.entries[current].value = Some(value);
                self.entries[current].occupied = true;
                self.entries[current].deleted = false;
                self.size += 1;
                return 0;
            } else if self.entries[current].deleted {
                self.entries[current].key = key;
                self.entries[current].value = Some(value);
                self.entries[current].deleted = false;
                self.size += 1;
                self.deleted_count -= 1;
                return 0;
            } else if self.entries[current].key == key {
                self.entries[current].value = Some(value);
                return 0;
            }
            probe += 1;
        }
        -1
    }

    fn get(&self, key: TreeId) -> Option<&V> {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        let mut probe = 0usize;
        while probe < self.capacity {
            let current = (index + probe) % self.capacity;
            if !self.entries[current].occupied {
                return None;
            }
            if !self.entries[current].deleted && self.entries[current].key == key {
                return self.entries[current].value.as_ref();
            }
            probe += 1;
        }
        None
    }

    fn get_mut(&mut self, key: TreeId) -> Option<&mut V> {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        let mut probe = 0usize;
        while probe < self.capacity {
            let current = (index + probe) % self.capacity;
            if !self.entries[current].occupied {
                return None;
            }
            if !self.entries[current].deleted && self.entries[current].key == key {
                return self.entries[current].value.as_mut();
            }
            probe += 1;
        }
        None
    }

    fn remove(&mut self, key: TreeId) -> Option<V> {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        let mut probe = 0usize;
        while probe < self.capacity {
            let current = (index + probe) % self.capacity;
            if !self.entries[current].occupied {
                return None;
            }
            if !self.entries[current].deleted && self.entries[current].key == key {
                let val = self.entries[current].value.take();
                self.entries[current].deleted = true;
                self.size -= 1;
                self.deleted_count += 1;
                return val;
            }
            probe += 1;
        }
        None
    }

    fn contains(&self, key: TreeId) -> bool {
        self.get(key).is_some()
    }

    fn size(&self) -> usize {
        self.size
    }
}

#[derive(Clone)]
struct TreeNode {
    id: TreeId,
    parent_id: TreeId,
    child_ids: [TreeId; MAX_CHILDREN],
    child_count: i32,
    data: [u8; MAX_DATA_LENGTH],
}

impl TreeNode {
    fn new(id: TreeId, parent_id: TreeId, data: &str) -> Self {
        let mut node_data = [0u8; MAX_DATA_LENGTH];
        let bytes = data.as_bytes();
        let len = bytes.len().min(MAX_DATA_LENGTH - 1);
        node_data[..len].copy_from_slice(&bytes[..len]);
        Self {
            id,
            parent_id,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: node_data,
        }
    }

    fn data_str(&self) -> &str {
        let null_pos = self
            .data
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(MAX_DATA_LENGTH);
        std::str::from_utf8(&self.data[..null_pos]).unwrap_or("")
    }
}

struct Tree {
    node_map: Hashmap<TreeNode>,
    root_id: TreeId,
    has_root: bool,
    node_count: usize,
}

impl Tree {
    fn new() -> Self {
        Self {
            node_map: Hashmap::new(),
            root_id: 0,
            has_root: false,
            node_count: 0,
        }
    }

    fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: &str) -> i32 {
        // Check if node already exists
        if self.contains(id) {
            eprintln!("Error: Node with ID {} already exists", id);
            return -1;
        }

        let mut node = TreeNode::new(id, parent_id, data);

        if !self.has_root {
            self.root_id = id;
            self.has_root = true;
            node.parent_id = 0;
        } else {
            let parent = match self.node_map.get_mut(parent_id) {
                Some(p) => p,
                None => {
                    eprintln!("Error: Parent node {} not found", parent_id);
                    return -1;
                }
            };
            if parent.child_count as usize >= MAX_CHILDREN {
                eprintln!("Error: Parent has maximum children");
                return -1;
            }
            parent.child_ids[parent.child_count as usize] = id;
            parent.child_count += 1;
        }

        if self.node_map.put(id, node) != 0 {
            eprintln!("Error: Failed to add node to hashmap");
            return -1;
        }

        self.node_count += 1;
        0
    }

    fn remove_subtree(&mut self, id: TreeId) -> i32 {
        let children: Vec<TreeId> = match self.node_map.get(id) {
            Some(node) => node.child_ids[..node.child_count as usize].to_vec(),
            None => return -1,
        };

        for child in children {
            self.remove_subtree(child);
        }

        if self.node_map.remove(id).is_some() {
            self.node_count -= 1;
        }
        0
    }

    fn remove_node(&mut self, id: TreeId) -> i32 {
        let parent_id = match self.node_map.get(id) {
            Some(node) => node.parent_id,
            None => {
                eprintln!("Error: Node {} not found", id);
                return -1;
            }
        };

        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = false;
            self.root_id = 0;
            return 0;
        }

        if let Some(parent) = self.node_map.get_mut(parent_id) {
            let mut idx_to_remove: Option<usize> = None;
            for i in 0..parent.child_count as usize {
                if parent.child_ids[i] == id {
                    idx_to_remove = Some(i);
                    break;
                }
            }
            if let Some(i) = idx_to_remove {
                for j in i..(parent.child_count as usize - 1) {
                    parent.child_ids[j] = parent.child_ids[j + 1];
                }
                parent.child_count -= 1;
            }
        }

        self.remove_subtree(id);
        0
    }

    fn get_node(&self, id: TreeId) -> Option<&TreeNode> {
        self.node_map.get(id)
    }

    fn contains(&self, id: TreeId) -> bool {
        self.node_map.contains(id)
    }

    fn size(&self) -> usize {
        self.node_count
    }

    fn print_helper(&self, id: TreeId, depth: i32) {
        let node = match self.node_map.get(id) {
            Some(n) => n,
            None => return,
        };

        for _ in 0..depth {
            print!("  ");
        }

        println!("[{}] {}", node.id, node.data_str());

        for i in 0..node.child_count as usize {
            self.print_helper(node.child_ids[i], depth + 1);
        }
    }

    fn print(&self) {
        if !self.has_root {
            println!("(empty tree)");
            return;
        }
        self.print_helper(self.root_id, 0);
    }

    fn get_depth(&self, id: TreeId) -> i32 {
        if !self.contains(id) {
            return -1;
        }
        let mut depth = 0;
        let mut current_id = id;
        while current_id != self.root_id {
            let node = match self.node_map.get(current_id) {
                Some(n) => n,
                None => return -1,
            };
            current_id = node.parent_id;
            depth += 1;
        }
        depth
    }

    fn get_height(&self, id: TreeId) -> i32 {
        let node = match self.node_map.get(id) {
            Some(n) => n,
            None => return -1,
        };
        if node.child_count == 0 {
            return 0;
        }
        let mut max_height = 0;
        for i in 0..node.child_count as usize {
            let child_height = self.get_height(node.child_ids[i]);
            if child_height > max_height {
                max_height = child_height;
            }
        }
        max_height + 1
    }

    fn count_descendants(&self, id: TreeId) -> i32 {
        let node = match self.node_map.get(id) {
            Some(n) => n,
            None => return -1,
        };
        let mut count = 0;
        for i in 0..node.child_count as usize {
            count += 1;
            count += self.count_descendants(node.child_ids[i]);
        }
        count
    }

    #[allow(dead_code)]
    fn find_path(&self, id: TreeId, path: &mut [TreeId]) -> i32 {
        if !self.contains(id) {
            return -1;
        }
        let mut temp_path = [0u64; 1000];
        let mut length = 0usize;
        let mut current_id = id;
        while length < 1000 {
            temp_path[length] = current_id;
            length += 1;
            if current_id == self.root_id {
                break;
            }
            let node = match self.node_map.get(current_id) {
                Some(n) => n,
                None => return -1,
            };
            current_id = node.parent_id;
        }
        if length > path.len() {
            length = path.len();
        }
        for i in 0..length {
            path[i] = temp_path[length - 1 - i];
        }
        length as i32
    }
}

// ----- Test functions -----

macro_rules! test_pass {
    ($name:expr) => {
        println!("✓ PASS: {}", $name);
    };
}

fn test_hashmap_basic() {
    println!();
    println!("=== Testing Hashmap Basic Operations ===");

    let mut map: Hashmap<i32> = Hashmap::new();
    assert_eq!(map.size(), 0);

    let val1 = 42i32;
    let val2 = 100i32;
    let val3 = 200i32;
    assert_eq!(map.put(1, val1), 0);
    assert_eq!(map.put(2, val2), 0);
    assert_eq!(map.put(3, val3), 0);
    assert_eq!(map.size(), 3);

    assert_eq!(*map.get(1).unwrap(), 42);
    assert_eq!(*map.get(2).unwrap(), 100);
    assert_eq!(*map.get(3).unwrap(), 200);

    let val4 = 500i32;
    assert_eq!(map.put(1, val4), 0);
    assert_eq!(map.size(), 3);
    assert_eq!(*map.get(1).unwrap(), 500);

    let removed = map.remove(2);
    assert_eq!(removed, Some(100));
    assert_eq!(map.size(), 2);
    assert!(map.get(2).is_none());

    assert_eq!(map.contains(1), true);
    assert_eq!(map.contains(2), false);
    assert_eq!(map.contains(3), true);

    test_pass!("test_hashmap_basic");
}

fn test_hashmap_collisions() {
    println!();
    println!("=== Testing Hashmap Collisions ===");

    let mut map: Hashmap<i32> = Hashmap::new();

    let mut values = [0i32; 100];
    for i in 0..100 {
        values[i] = (i as i32) * 10;
        assert_eq!(map.put(i as TreeId, values[i]), 0);
    }

    assert_eq!(map.size(), 100);

    for i in 0..100 {
        let val = map.get(i as TreeId);
        assert!(val.is_some());
        assert_eq!(*val.unwrap(), (i as i32) * 10);
    }

    test_pass!("test_hashmap_collisions");
}

fn test_tree_creation() {
    println!();
    println!("=== Testing Tree Creation ===");

    let tree = Tree::new();
    assert_eq!(tree.size(), 0);
    assert_eq!(tree.has_root, false);

    test_pass!("test_tree_creation");
}

fn test_tree_add_root() {
    println!();
    println!("=== Testing Tree Add Root ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.size(), 1);
    assert_eq!(tree.has_root, true);
    assert_eq!(tree.root_id, 1);

    let root = tree.get_node(1).unwrap();
    assert_eq!(root.id, 1);
    assert_eq!(root.data_str(), "root");
    assert_eq!(root.child_count, 0);

    test_pass!("test_tree_add_root");
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

    let root = tree.get_node(1).unwrap();
    assert_eq!(root.child_count, 3);
    assert_eq!(root.child_ids[0], 2);
    assert_eq!(root.child_ids[1], 3);
    assert_eq!(root.child_ids[2], 4);

    test_pass!("test_tree_add_children");
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

    test_pass!("test_tree_deep_hierarchy");
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
    assert_eq!(tree.contains(3), false);

    let root = tree.get_node(1).unwrap();
    assert_eq!(root.child_count, 1);
    assert_eq!(root.child_ids[0], 2);

    test_pass!("test_tree_remove_leaf");
}

fn test_tree_remove_subtree() {
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
    assert_eq!(tree.contains(2), false);
    assert_eq!(tree.contains(3), false);
    assert_eq!(tree.contains(4), false);
    assert_eq!(tree.contains(1), true);
    assert_eq!(tree.contains(5), true);

    test_pass!("test_tree_remove_subtree");
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
    assert_eq!(tree.has_root, false);

    test_pass!("test_tree_remove_root");
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

    test_pass!("test_tree_count_descendants");
}

fn test_tree_find_path() {
    println!();
    println!("=== Testing Tree Find Path ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.add_node(2, 1, "child"), 0);
    assert_eq!(tree.add_node(3, 2, "grandchild"), 0);

    let mut path = [0u64; 10];

    let length = tree.find_path(3, &mut path);
    assert_eq!(length, 3);
    assert_eq!(path[0], 1);
    assert_eq!(path[1], 2);
    assert_eq!(path[2], 3);

    let length = tree.find_path(1, &mut path);
    assert_eq!(length, 1);
    assert_eq!(path[0], 1);

    test_pass!("test_tree_find_path");
}

fn test_tree_duplicate_id() {
    println!();
    println!("=== Testing Tree Duplicate ID ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);
    assert_eq!(tree.add_node(2, 1, "child"), 0);

    // Try to add duplicate - this should print to stderr and return non-zero
    assert!(tree.add_node(2, 1, "duplicate") != 0);
    assert_eq!(tree.size(), 2);

    test_pass!("test_tree_duplicate_id");
}

fn test_tree_max_children() {
    println!();
    println!("=== Testing Tree Max Children ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, "root"), 0);

    for i in 0..MAX_CHILDREN {
        assert_eq!(tree.add_node((i + 2) as TreeId, 1, "child"), 0);
    }

    // Try to add one more (should fail)
    assert!(tree.add_node((MAX_CHILDREN + 2) as TreeId, 1, "overflow") != 0);
    assert_eq!(tree.size(), MAX_CHILDREN + 1);

    test_pass!("test_tree_max_children");
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

    test_pass!("test_tree_complex_structure");
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║  TREE WITH HASHMAP ID MAPPING TESTS   ║");
    println!("╚════════════════════════════════════════╝");

    test_hashmap_basic();
    test_hashmap_collisions();

    test_tree_creation();
    test_tree_add_root();
    test_tree_add_children();

    test_tree_deep_hierarchy();
    test_tree_complex_structure();

    test_tree_remove_leaf();
    test_tree_remove_subtree();
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
