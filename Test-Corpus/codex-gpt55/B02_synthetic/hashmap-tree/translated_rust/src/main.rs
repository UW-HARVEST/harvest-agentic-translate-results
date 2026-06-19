const HASHMAP_INITIAL_CAPACITY: usize = 16;
const HASHMAP_LOAD_FACTOR: f64 = 0.75;
const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;

type TreeId = u64;

#[derive(Clone)]
struct HashMapEntry<V> {
    key: TreeId,
    value: Option<V>,
    occupied: bool,
    deleted: bool,
}

impl<V> Default for HashMapEntry<V> {
    fn default() -> Self {
        Self {
            key: 0,
            value: None,
            occupied: false,
            deleted: false,
        }
    }
}

struct HashMapC<V> {
    entries: Vec<HashMapEntry<V>>,
    capacity: usize,
    size: usize,
    deleted_count: usize,
}

impl<V> HashMapC<V> {
    fn create() -> Self {
        let mut entries = Vec::with_capacity(HASHMAP_INITIAL_CAPACITY);
        entries.resize_with(HASHMAP_INITIAL_CAPACITY, HashMapEntry::default);

        Self {
            entries,
            capacity: HASHMAP_INITIAL_CAPACITY,
            size: 0,
            deleted_count: 0,
        }
    }

    fn hash_function(key: TreeId) -> u64 {
        let mut hash = 14695981039346656037_u64;

        for byte in key.to_ne_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(1099511628211_u64);
        }

        hash
    }

    fn should_resize(&self) -> bool {
        let load = (self.size + self.deleted_count) as f64 / self.capacity as f64;
        load > HASHMAP_LOAD_FACTOR
    }

    fn resize(&mut self) -> i32 {
        let old_capacity = self.capacity;
        let mut old_entries = Vec::new();
        std::mem::swap(&mut old_entries, &mut self.entries);

        self.capacity *= 2;
        self.entries = Vec::with_capacity(self.capacity);
        self.entries
            .resize_with(self.capacity, HashMapEntry::default);
        self.size = 0;
        self.deleted_count = 0;

        for entry in old_entries.into_iter().take(old_capacity) {
            if entry.occupied && !entry.deleted {
                if let Some(value) = entry.value {
                    self.put(entry.key, value);
                }
            }
        }

        0
    }

    fn put(&mut self, key: TreeId, value: V) -> i32 {
        if self.should_resize() && self.resize() != 0 {
            return -1;
        }

        let hash = Self::hash_function(key);
        let index = hash as usize % self.capacity;
        let mut probe = 0;
        let mut pending = Some(value);

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if !self.entries[current].occupied {
                self.entries[current].key = key;
                self.entries[current].value = pending.take();
                self.entries[current].occupied = true;
                self.entries[current].deleted = false;
                self.size += 1;
                return 0;
            } else if self.entries[current].deleted {
                self.entries[current].key = key;
                self.entries[current].value = pending.take();
                self.entries[current].deleted = false;
                self.size += 1;
                self.deleted_count -= 1;
                return 0;
            } else if self.entries[current].key == key {
                self.entries[current].value = pending.take();
                return 0;
            }

            probe += 1;
        }

        -1
    }

    fn get(&self, key: TreeId) -> Option<&V> {
        let hash = Self::hash_function(key);
        let index = hash as usize % self.capacity;
        let mut probe = 0;

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
        let hash = Self::hash_function(key);
        let index = hash as usize % self.capacity;
        let mut found = None;

        for probe in 0..self.capacity {
            let current = (index + probe) % self.capacity;

            if !self.entries[current].occupied {
                break;
            }

            if !self.entries[current].deleted && self.entries[current].key == key {
                found = Some(current);
                break;
            }
        }

        found.and_then(|idx| self.entries[idx].value.as_mut())
    }

    fn remove(&mut self, key: TreeId) -> Option<V> {
        let hash = Self::hash_function(key);
        let index = hash as usize % self.capacity;
        let mut probe = 0;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if !self.entries[current].occupied {
                return None;
            }

            if !self.entries[current].deleted && self.entries[current].key == key {
                let value = self.entries[current].value.take();
                self.entries[current].deleted = true;
                self.size -= 1;
                self.deleted_count += 1;
                return value;
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
    child_count: usize,
    data: String,
}

impl TreeNode {
    fn new(id: TreeId, parent_id: TreeId, data: Option<&str>) -> Self {
        let mut bytes = match data {
            Some(text) => text.as_bytes().to_vec(),
            None => Vec::new(),
        };
        if bytes.len() >= MAX_DATA_LENGTH {
            bytes.truncate(MAX_DATA_LENGTH - 1);
        }

        Self {
            id,
            parent_id,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: String::from_utf8_lossy(&bytes).into_owned(),
        }
    }
}

struct Tree {
    node_map: HashMapC<TreeNode>,
    root_id: TreeId,
    has_root: bool,
    node_count: usize,
}

impl Tree {
    fn create() -> Self {
        Self {
            node_map: HashMapC::create(),
            root_id: 0,
            has_root: false,
            node_count: 0,
        }
    }

    fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: Option<&str>) -> i32 {
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
            let Some(parent) = self.get_node_mut(parent_id) else {
                eprintln!("Error: Parent node {} not found", parent_id);
                return -1;
            };

            if parent.child_count >= MAX_CHILDREN {
                eprintln!("Error: Parent has maximum children");
                return -1;
            }

            parent.child_ids[parent.child_count] = id;
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
        let Some(node) = self.get_node(id).cloned() else {
            return -1;
        };

        for i in 0..node.child_count {
            self.remove_subtree(node.child_ids[i]);
        }

        if self.node_map.remove(id).is_some() {
            self.node_count -= 1;
        }

        0
    }

    fn remove_node(&mut self, id: TreeId) -> i32 {
        let Some(node) = self.get_node(id).cloned() else {
            eprintln!("Error: Node {} not found", id);
            return -1;
        };

        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = false;
            self.root_id = 0;
            return 0;
        }

        if let Some(parent) = self.get_node_mut(node.parent_id) {
            for i in 0..parent.child_count {
                if parent.child_ids[i] == id {
                    for j in i..(parent.child_count - 1) {
                        parent.child_ids[j] = parent.child_ids[j + 1];
                    }
                    parent.child_count -= 1;
                    break;
                }
            }
        }

        self.remove_subtree(id);
        0
    }

    fn get_node(&self, id: TreeId) -> Option<&TreeNode> {
        self.node_map.get(id)
    }

    fn get_node_mut(&mut self, id: TreeId) -> Option<&mut TreeNode> {
        self.node_map.get_mut(id)
    }

    fn contains(&self, id: TreeId) -> bool {
        self.get_node(id).is_some()
    }

    fn size(&self) -> usize {
        self.node_count
    }

    fn print(&self) {
        if !self.has_root {
            println!("(empty tree)");
            return;
        }

        self.print_helper(self.root_id, 0);
    }

    fn print_helper(&self, id: TreeId, depth: i32) {
        let Some(node) = self.get_node(id) else {
            return;
        };

        for _ in 0..depth {
            print!("  ");
        }

        println!("[{}] {}", node.id, node.data);

        for i in 0..node.child_count {
            self.print_helper(node.child_ids[i], depth + 1);
        }
    }

    fn get_depth(&self, id: TreeId) -> i32 {
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

    fn get_height(&self, id: TreeId) -> i32 {
        let Some(node) = self.get_node(id) else {
            return -1;
        };

        if node.child_count == 0 {
            return 0;
        }

        let mut max_height = 0;
        for i in 0..node.child_count {
            let child_height = self.get_height(node.child_ids[i]);
            if child_height > max_height {
                max_height = child_height;
            }
        }

        max_height + 1
    }

    fn count_descendants(&self, id: TreeId) -> i32 {
        let Some(node) = self.get_node(id) else {
            return -1;
        };

        let mut count = 0;
        for i in 0..node.child_count {
            count += 1;
            count += self.count_descendants(node.child_ids[i]);
        }

        count
    }

    fn find_path(&self, id: TreeId, path: &mut [TreeId], max_length: i32) -> i32 {
        if max_length < 0 || !self.contains(id) {
            return -1;
        }

        let mut temp_path = [0_u64; 1000];
        let mut length = 0_usize;
        let mut current_id = id;

        while length < 1000 {
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

        if length > max_length as usize {
            length = max_length as usize;
        }

        for i in 0..length {
            path[i] = temp_path[length - 1 - i];
        }

        length as i32
    }
}

fn test_pass(func: &str) {
    println!("✓ PASS: {}", func);
}

fn test_hashmap_basic() {
    println!("\n=== Testing Hashmap Basic Operations ===");

    let mut map = HashMapC::create();
    assert_eq!(map.size(), 0);

    assert_eq!(map.put(1, 42), 0);
    assert_eq!(map.put(2, 100), 0);
    assert_eq!(map.put(3, 200), 0);
    assert_eq!(map.size(), 3);

    assert_eq!(*map.get(1).unwrap(), 42);
    assert_eq!(*map.get(2).unwrap(), 100);
    assert_eq!(*map.get(3).unwrap(), 200);

    assert_eq!(map.put(1, 500), 0);
    assert_eq!(map.size(), 3);
    assert_eq!(*map.get(1).unwrap(), 500);

    let removed = map.remove(2);
    assert_eq!(removed, Some(100));
    assert_eq!(map.size(), 2);
    assert!(map.get(2).is_none());

    assert!(map.contains(1));
    assert!(!map.contains(2));
    assert!(map.contains(3));

    test_pass("test_hashmap_basic");
}

fn test_hashmap_collisions() {
    println!("\n=== Testing Hashmap Collisions ===");

    let mut map = HashMapC::create();

    for i in 0..100 {
        assert_eq!(map.put(i, i * 10), 0);
    }

    assert_eq!(map.size(), 100);

    for i in 0..100 {
        let val = map.get(i).unwrap();
        assert_eq!(*val, i * 10);
    }

    test_pass("test_hashmap_collisions");
}

fn test_tree_creation() {
    println!("\n=== Testing Tree Creation ===");

    let tree = Tree::create();
    assert_eq!(tree.size(), 0);
    assert!(!tree.has_root);

    test_pass("test_tree_creation");
}

fn test_tree_add_root() {
    println!("\n=== Testing Tree Add Root ===");

    let mut tree = Tree::create();

    assert_eq!(tree.add_node(1, 0, Some("root")), 0);
    assert_eq!(tree.size(), 1);
    assert!(tree.has_root);
    assert_eq!(tree.root_id, 1);

    let root = tree.get_node(1).unwrap();
    assert_eq!(root.id, 1);
    assert_eq!(root.data, "root");
    assert_eq!(root.child_count, 0);

    test_pass("test_tree_add_root");
}

fn test_tree_add_children() {
    println!("\n=== Testing Tree Add Children ===");

    let mut tree = Tree::create();

    assert_eq!(tree.add_node(1, 0, Some("root")), 0);
    assert_eq!(tree.add_node(2, 1, Some("child1")), 0);
    assert_eq!(tree.add_node(3, 1, Some("child2")), 0);
    assert_eq!(tree.add_node(4, 1, Some("child3")), 0);

    assert_eq!(tree.size(), 4);

    let root = tree.get_node(1).unwrap();
    assert_eq!(root.child_count, 3);
    assert_eq!(root.child_ids[0], 2);
    assert_eq!(root.child_ids[1], 3);
    assert_eq!(root.child_ids[2], 4);

    test_pass("test_tree_add_children");
}

fn test_tree_deep_hierarchy() {
    println!("\n=== Testing Tree Deep Hierarchy ===");

    let mut tree = Tree::create();

    assert_eq!(tree.add_node(1, 0, Some("level0")), 0);
    assert_eq!(tree.add_node(2, 1, Some("level1")), 0);
    assert_eq!(tree.add_node(3, 2, Some("level2")), 0);
    assert_eq!(tree.add_node(4, 3, Some("level3")), 0);
    assert_eq!(tree.add_node(5, 4, Some("level4")), 0);

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
    println!("\n=== Testing Tree Remove Leaf ===");

    let mut tree = Tree::create();

    assert_eq!(tree.add_node(1, 0, Some("root")), 0);
    assert_eq!(tree.add_node(2, 1, Some("child1")), 0);
    assert_eq!(tree.add_node(3, 1, Some("child2")), 0);

    assert_eq!(tree.size(), 3);

    assert_eq!(tree.remove_node(3), 0);
    assert_eq!(tree.size(), 2);
    assert!(!tree.contains(3));

    let root = tree.get_node(1).unwrap();
    assert_eq!(root.child_count, 1);
    assert_eq!(root.child_ids[0], 2);

    test_pass("test_tree_remove_leaf");
}

fn test_tree_remove_subtree() {
    println!("\n=== Testing Tree Remove Subtree ===");

    let mut tree = Tree::create();

    assert_eq!(tree.add_node(1, 0, Some("root")), 0);
    assert_eq!(tree.add_node(2, 1, Some("child1")), 0);
    assert_eq!(tree.add_node(3, 2, Some("grandchild1")), 0);
    assert_eq!(tree.add_node(4, 2, Some("grandchild2")), 0);
    assert_eq!(tree.add_node(5, 1, Some("child2")), 0);

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
    println!("\n=== Testing Tree Remove Root ===");

    let mut tree = Tree::create();

    assert_eq!(tree.add_node(1, 0, Some("root")), 0);
    assert_eq!(tree.add_node(2, 1, Some("child1")), 0);
    assert_eq!(tree.add_node(3, 1, Some("child2")), 0);

    assert_eq!(tree.size(), 3);

    assert_eq!(tree.remove_node(1), 0);
    assert_eq!(tree.size(), 0);
    assert!(!tree.has_root);

    test_pass("test_tree_remove_root");
}

fn test_tree_count_descendants() {
    println!("\n=== Testing Tree Count Descendants ===");

    let mut tree = Tree::create();

    assert_eq!(tree.add_node(1, 0, Some("root")), 0);
    assert_eq!(tree.add_node(2, 1, Some("child1")), 0);
    assert_eq!(tree.add_node(3, 2, Some("grandchild1")), 0);
    assert_eq!(tree.add_node(4, 2, Some("grandchild2")), 0);
    assert_eq!(tree.add_node(5, 1, Some("child2")), 0);

    assert_eq!(tree.count_descendants(1), 4);
    assert_eq!(tree.count_descendants(2), 2);
    assert_eq!(tree.count_descendants(3), 0);
    assert_eq!(tree.count_descendants(5), 0);

    test_pass("test_tree_count_descendants");
}

fn test_tree_find_path() {
    println!("\n=== Testing Tree Find Path ===");

    let mut tree = Tree::create();

    assert_eq!(tree.add_node(1, 0, Some("root")), 0);
    assert_eq!(tree.add_node(2, 1, Some("child")), 0);
    assert_eq!(tree.add_node(3, 2, Some("grandchild")), 0);

    let mut path = [0_u64; 10];

    let mut length = tree.find_path(3, &mut path, 10);
    assert_eq!(length, 3);
    assert_eq!(path[0], 1);
    assert_eq!(path[1], 2);
    assert_eq!(path[2], 3);

    length = tree.find_path(1, &mut path, 10);
    assert_eq!(length, 1);
    assert_eq!(path[0], 1);

    test_pass("test_tree_find_path");
}

fn test_tree_duplicate_id() {
    println!("\n=== Testing Tree Duplicate ID ===");

    let mut tree = Tree::create();

    assert_eq!(tree.add_node(1, 0, Some("root")), 0);
    assert_eq!(tree.add_node(2, 1, Some("child")), 0);

    assert_ne!(tree.add_node(2, 1, Some("duplicate")), 0);
    assert_eq!(tree.size(), 2);

    test_pass("test_tree_duplicate_id");
}

fn test_tree_max_children() {
    println!("\n=== Testing Tree Max Children ===");

    let mut tree = Tree::create();

    assert_eq!(tree.add_node(1, 0, Some("root")), 0);

    for i in 0..MAX_CHILDREN {
        assert_eq!(tree.add_node((i + 2) as u64, 1, Some("child")), 0);
    }

    assert_ne!(
        tree.add_node((MAX_CHILDREN + 2) as u64, 1, Some("overflow")),
        0
    );
    assert_eq!(tree.size(), MAX_CHILDREN + 1);

    test_pass("test_tree_max_children");
}

fn test_tree_complex_structure() {
    println!("\n=== Testing Tree Complex Structure ===");

    let mut tree = Tree::create();

    assert_eq!(tree.add_node(1, 0, Some("root")), 0);
    assert_eq!(tree.add_node(2, 1, Some("child1")), 0);
    assert_eq!(tree.add_node(3, 1, Some("child2")), 0);
    assert_eq!(tree.add_node(4, 1, Some("child3")), 0);
    assert_eq!(tree.add_node(5, 2, Some("gc1")), 0);
    assert_eq!(tree.add_node(6, 2, Some("gc2")), 0);
    assert_eq!(tree.add_node(7, 3, Some("gc3")), 0);
    assert_eq!(tree.add_node(8, 4, Some("gc4")), 0);
    assert_eq!(tree.add_node(9, 4, Some("gc5")), 0);
    assert_eq!(tree.add_node(10, 7, Some("ggc1")), 0);

    assert_eq!(tree.size(), 10);
    assert_eq!(tree.get_height(1), 3);
    assert_eq!(tree.count_descendants(1), 9);
    assert_eq!(tree.count_descendants(2), 2);
    assert_eq!(tree.count_descendants(7), 1);

    tree.print();

    test_pass("test_tree_complex_structure");
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
