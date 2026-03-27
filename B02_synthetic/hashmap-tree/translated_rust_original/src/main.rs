// ============================================================
// Hashmap (open-addressing, FNV-1a, linear probing)
// ============================================================

const HASHMAP_INITIAL_CAPACITY: usize = 16;
const HASHMAP_LOAD_FACTOR: f64 = 0.75;

type TreeId = u64;

#[derive(Clone)]
struct HashmapEntry {
    key: TreeId,
    value: usize, // index into a value store
    occupied: bool,
    deleted: bool,
}

impl Default for HashmapEntry {
    fn default() -> Self {
        Self { key: 0, value: 0, occupied: false, deleted: false }
    }
}

struct Hashmap {
    entries: Vec<HashmapEntry>,
    capacity: usize,
    size: usize,
    deleted_count: usize,
}

fn hash_function(key: TreeId) -> u64 {
    let bytes = key.to_ne_bytes();
    let mut hash: u64 = 14695981039346656037;
    for &b in &bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

impl Hashmap {
    fn new() -> Self {
        Self {
            entries: vec![HashmapEntry::default(); HASHMAP_INITIAL_CAPACITY],
            capacity: HASHMAP_INITIAL_CAPACITY,
            size: 0,
            deleted_count: 0,
        }
    }

    fn should_resize(&self) -> bool {
        let load = (self.size + self.deleted_count) as f64 / self.capacity as f64;
        load > HASHMAP_LOAD_FACTOR
    }

    fn resize(&mut self) {
        let old_entries = std::mem::take(&mut self.entries);
        let old_capacity = self.capacity;
        self.capacity *= 2;
        self.entries = vec![HashmapEntry::default(); self.capacity];
        self.size = 0;
        self.deleted_count = 0;
        for i in 0..old_capacity {
            if old_entries[i].occupied && !old_entries[i].deleted {
                self.put(old_entries[i].key, old_entries[i].value);
            }
        }
    }

    fn put(&mut self, key: TreeId, value: usize) -> i32 {
        if self.should_resize() {
            self.resize();
        }
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        for probe in 0..self.capacity {
            let current = (index + probe) % self.capacity;
            if !self.entries[current].occupied {
                self.entries[current].key = key;
                self.entries[current].value = value;
                self.entries[current].occupied = true;
                self.entries[current].deleted = false;
                self.size += 1;
                return 0;
            } else if self.entries[current].deleted {
                self.entries[current].key = key;
                self.entries[current].value = value;
                self.entries[current].deleted = false;
                self.size += 1;
                self.deleted_count -= 1;
                return 0;
            } else if self.entries[current].key == key {
                self.entries[current].value = value;
                return 0;
            }
        }
        -1
    }

    fn get(&self, key: TreeId) -> Option<usize> {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        for probe in 0..self.capacity {
            let current = (index + probe) % self.capacity;
            if !self.entries[current].occupied {
                return None;
            }
            if !self.entries[current].deleted && self.entries[current].key == key {
                return Some(self.entries[current].value);
            }
        }
        None
    }

    fn remove(&mut self, key: TreeId) -> Option<usize> {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        for probe in 0..self.capacity {
            let current = (index + probe) % self.capacity;
            if !self.entries[current].occupied {
                return None;
            }
            if !self.entries[current].deleted && self.entries[current].key == key {
                let value = self.entries[current].value;
                self.entries[current].deleted = true;
                self.size -= 1;
                self.deleted_count += 1;
                return Some(value);
            }
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

// ============================================================
// Tree
// ============================================================

const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;

struct TreeNode {
    id: TreeId,
    parent_id: TreeId,
    child_ids: [TreeId; MAX_CHILDREN],
    child_count: i32,
    data: String,
}

struct Tree {
    node_map: Hashmap,
    nodes: Vec<Option<TreeNode>>, // node storage; index = value stored in hashmap
    root_id: TreeId,
    has_root: bool,
    node_count: usize,
    free_slots: Vec<usize>,
}

impl Tree {
    fn new() -> Self {
        Self {
            node_map: Hashmap::new(),
            nodes: Vec::new(),
            root_id: 0,
            has_root: false,
            node_count: 0,
            free_slots: Vec::new(),
        }
    }

    fn alloc_node(&mut self, node: TreeNode) -> usize {
        if let Some(idx) = self.free_slots.pop() {
            self.nodes[idx] = Some(node);
            idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Some(node));
            idx
        }
    }

    fn free_node(&mut self, idx: usize) {
        self.nodes[idx] = None;
        self.free_slots.push(idx);
    }

    fn get_node(&self, id: TreeId) -> Option<&TreeNode> {
        self.node_map.get(id).and_then(|idx| self.nodes[idx].as_ref())
    }

    fn get_node_mut(&mut self, id: TreeId) -> Option<&mut TreeNode> {
        self.node_map.get(id).and_then(|idx| self.nodes[idx].as_mut())
    }

    fn contains(&self, id: TreeId) -> bool {
        self.node_map.contains(id)
    }

    fn size(&self) -> usize {
        self.node_count
    }

    fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: &str) -> i32 {
        if self.contains(id) {
            eprint!("Error: Node with ID {} already exists\n", id);
            return -1;
        }

        let mut node = TreeNode {
            id,
            parent_id,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: String::new(),
        };

        // strncpy behavior: copy up to MAX_DATA_LENGTH-1 chars
        let truncated: String = data.chars().take(MAX_DATA_LENGTH - 1).collect();
        node.data = truncated;

        if !self.has_root {
            self.root_id = id;
            self.has_root = true;
            node.parent_id = 0;
        } else {
            // Check parent exists and has room
            let parent_child_count = match self.get_node(parent_id) {
                Some(p) => p.child_count,
                None => {
                    eprint!("Error: Parent node {} not found\n", parent_id);
                    return -1;
                }
            };
            if parent_child_count >= MAX_CHILDREN as i32 {
                eprint!("Error: Parent has maximum children\n");
                return -1;
            }
            // Add child id to parent
            let parent = self.get_node_mut(parent_id).unwrap();
            parent.child_ids[parent.child_count as usize] = id;
            parent.child_count += 1;
        }

        let idx = self.alloc_node(node);
        if self.node_map.put(id, idx) != 0 {
            eprint!("Error: Failed to add node to hashmap\n");
            self.free_node(idx);
            return -1;
        }

        self.node_count += 1;
        0
    }

    fn remove_subtree(&mut self, id: TreeId) {
        // Collect child ids first to avoid borrow issues
        let child_ids: Vec<TreeId> = match self.get_node(id) {
            Some(node) => {
                let mut v = Vec::new();
                for i in 0..node.child_count as usize {
                    v.push(node.child_ids[i]);
                }
                v
            }
            None => return,
        };

        for cid in child_ids {
            self.remove_subtree(cid);
        }

        if let Some(idx) = self.node_map.remove(id) {
            self.free_node(idx);
            self.node_count -= 1;
        }
    }

    fn remove_node(&mut self, id: TreeId) -> i32 {
        if self.get_node(id).is_none() {
            eprint!("Error: Node {} not found\n", id);
            return -1;
        }

        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = false;
            self.root_id = 0;
            return 0;
        }

        let parent_id = self.get_node(id).unwrap().parent_id;

        // Remove from parent's child list
        if let Some(parent) = self.get_node_mut(parent_id) {
            let mut i = 0;
            while i < parent.child_count as usize {
                if parent.child_ids[i] == id {
                    let mut j = i;
                    while j < (parent.child_count - 1) as usize {
                        parent.child_ids[j] = parent.child_ids[j + 1];
                        j += 1;
                    }
                    parent.child_count -= 1;
                    break;
                }
                i += 1;
            }
        }

        self.remove_subtree(id);
        0
    }

    fn print_helper(&self, id: TreeId, depth: i32) {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return,
        };

        for _ in 0..depth {
            print!("  ");
        }
        print!("[{}] {}\n", node.id, node.data);

        for i in 0..node.child_count as usize {
            self.print_helper(node.child_ids[i], depth + 1);
        }
    }

    fn print(&self) {
        if !self.has_root {
            print!("(empty tree)\n");
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
            let node = match self.get_node(current_id) {
                Some(n) => n,
                None => return -1,
            };
            current_id = node.parent_id;
            depth += 1;
        }
        depth
    }

    fn get_height(&self, id: TreeId) -> i32 {
        let node = match self.get_node(id) {
            Some(n) => n,
            None => return -1,
        };
        if node.child_count == 0 {
            return 0;
        }
        let mut max_height = 0;
        for i in 0..node.child_count as usize {
            let ch = self.get_height(node.child_ids[i]);
            if ch > max_height {
                max_height = ch;
            }
        }
        max_height + 1
    }

    fn count_descendants(&self, id: TreeId) -> i32 {
        let node = match self.get_node(id) {
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

    fn find_path(&self, id: TreeId, path: &mut [TreeId], max_length: usize) -> i32 {
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
            let node = match self.get_node(current_id) {
                Some(n) => n,
                None => return -1,
            };
            current_id = node.parent_id;
        }

        if length > max_length {
            length = max_length;
        }

        for i in 0..length {
            path[i] = temp_path[length - 1 - i];
        }

        length as i32
    }
}

// ============================================================
// Tests — exact C output reproduction
// ============================================================

// The C code uses a simple value store where hashmap values are pointers.
// For the hashmap-only tests we replicate that with a Vec<i32> as a value pool.

fn test_hashmap_basic() {
    print!("\n=== Testing Hashmap Basic Operations ===\n");

    let mut map = Hashmap::new();
    assert!(map.size() == 0);

    // val indices: 0=42, 1=100, 2=200
    let mut vals: Vec<i32> = vec![42, 100, 200];
    assert!(map.put(1, 0) == 0);
    assert!(map.put(2, 1) == 0);
    assert!(map.put(3, 2) == 0);
    assert!(map.size() == 3);

    assert!(vals[map.get(1).unwrap()] == 42);
    assert!(vals[map.get(2).unwrap()] == 100);
    assert!(vals[map.get(3).unwrap()] == 200);

    // Update key 1 -> val4=500
    vals.push(500); // index 3
    assert!(map.put(1, 3) == 0);
    assert!(map.size() == 3);
    assert!(vals[map.get(1).unwrap()] == 500);

    // Remove key 2
    let removed = map.remove(2);
    assert!(removed == Some(1)); // index 1 -> &val2
    assert!(map.size() == 2);
    assert!(map.get(2).is_none());

    // Contains
    assert!(map.contains(1));
    assert!(!map.contains(2));
    assert!(map.contains(3));

    print!("\u{2713} PASS: test_hashmap_basic\n");
}

fn test_hashmap_collisions() {
    print!("\n=== Testing Hashmap Collisions ===\n");

    let mut map = Hashmap::new();

    let mut values: Vec<i32> = Vec::new();
    for i in 0..100 {
        values.push(i * 10);
        assert!(map.put(i as u64, i as usize) == 0);
    }
    assert!(map.size() == 100);

    for i in 0..100 {
        let val = map.get(i as u64);
        assert!(val.is_some());
        assert!(values[val.unwrap()] == i * 10);
    }

    print!("\u{2713} PASS: test_hashmap_collisions\n");
}

fn test_tree_creation() {
    print!("\n=== Testing Tree Creation ===\n");

    let tree = Tree::new();
    assert!(tree.size() == 0);
    assert!(!tree.has_root);

    print!("\u{2713} PASS: test_tree_creation\n");
}

fn test_tree_add_root() {
    print!("\n=== Testing Tree Add Root ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.size() == 1);
    assert!(tree.has_root);
    assert!(tree.root_id == 1);

    let root = tree.get_node(1).unwrap();
    assert!(root.id == 1);
    assert!(root.data == "root");
    assert!(root.child_count == 0);

    print!("\u{2713} PASS: test_tree_add_root\n");
}

fn test_tree_add_children() {
    print!("\n=== Testing Tree Add Children ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 1, "child2") == 0);
    assert!(tree.add_node(4, 1, "child3") == 0);
    assert!(tree.size() == 4);

    let root = tree.get_node(1).unwrap();
    assert!(root.child_count == 3);
    assert!(root.child_ids[0] == 2);
    assert!(root.child_ids[1] == 3);
    assert!(root.child_ids[2] == 4);

    print!("\u{2713} PASS: test_tree_add_children\n");
}

fn test_tree_deep_hierarchy() {
    print!("\n=== Testing Tree Deep Hierarchy ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "level0") == 0);
    assert!(tree.add_node(2, 1, "level1") == 0);
    assert!(tree.add_node(3, 2, "level2") == 0);
    assert!(tree.add_node(4, 3, "level3") == 0);
    assert!(tree.add_node(5, 4, "level4") == 0);
    assert!(tree.size() == 5);

    assert!(tree.get_depth(1) == 0);
    assert!(tree.get_depth(2) == 1);
    assert!(tree.get_depth(3) == 2);
    assert!(tree.get_depth(4) == 3);
    assert!(tree.get_depth(5) == 4);

    assert!(tree.get_height(1) == 4);
    assert!(tree.get_height(2) == 3);
    assert!(tree.get_height(5) == 0);

    print!("\u{2713} PASS: test_tree_deep_hierarchy\n");
}

fn test_tree_remove_leaf() {
    print!("\n=== Testing Tree Remove Leaf ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 1, "child2") == 0);
    assert!(tree.size() == 3);

    assert!(tree.remove_node(3) == 0);
    assert!(tree.size() == 2);
    assert!(!tree.contains(3));

    let root = tree.get_node(1).unwrap();
    assert!(root.child_count == 1);
    assert!(root.child_ids[0] == 2);

    print!("\u{2713} PASS: test_tree_remove_leaf\n");
}

fn test_tree_remove_subtree() {
    print!("\n=== Testing Tree Remove Subtree ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 2, "grandchild1") == 0);
    assert!(tree.add_node(4, 2, "grandchild2") == 0);
    assert!(tree.add_node(5, 1, "child2") == 0);
    assert!(tree.size() == 5);

    assert!(tree.remove_node(2) == 0);
    assert!(tree.size() == 2);
    assert!(!tree.contains(2));
    assert!(!tree.contains(3));
    assert!(!tree.contains(4));
    assert!(tree.contains(1));
    assert!(tree.contains(5));

    print!("\u{2713} PASS: test_tree_remove_subtree\n");
}

fn test_tree_remove_root() {
    print!("\n=== Testing Tree Remove Root ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 1, "child2") == 0);
    assert!(tree.size() == 3);

    assert!(tree.remove_node(1) == 0);
    assert!(tree.size() == 0);
    assert!(!tree.has_root);

    print!("\u{2713} PASS: test_tree_remove_root\n");
}

fn test_tree_count_descendants() {
    print!("\n=== Testing Tree Count Descendants ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 2, "grandchild1") == 0);
    assert!(tree.add_node(4, 2, "grandchild2") == 0);
    assert!(tree.add_node(5, 1, "child2") == 0);

    assert!(tree.count_descendants(1) == 4);
    assert!(tree.count_descendants(2) == 2);
    assert!(tree.count_descendants(3) == 0);
    assert!(tree.count_descendants(5) == 0);

    print!("\u{2713} PASS: test_tree_count_descendants\n");
}

fn test_tree_find_path() {
    print!("\n=== Testing Tree Find Path ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child") == 0);
    assert!(tree.add_node(3, 2, "grandchild") == 0);

    let mut path = [0u64; 10];

    let length = tree.find_path(3, &mut path, 10);
    assert!(length == 3);
    assert!(path[0] == 1);
    assert!(path[1] == 2);
    assert!(path[2] == 3);

    let length = tree.find_path(1, &mut path, 10);
    assert!(length == 1);
    assert!(path[0] == 1);

    print!("\u{2713} PASS: test_tree_find_path\n");
}

fn test_tree_duplicate_id() {
    print!("\n=== Testing Tree Duplicate ID ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child") == 0);

    // Try to add duplicate
    assert!(tree.add_node(2, 1, "duplicate") != 0);
    assert!(tree.size() == 2);

    print!("\u{2713} PASS: test_tree_duplicate_id\n");
}

fn test_tree_max_children() {
    print!("\n=== Testing Tree Max Children ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);

    for i in 0..MAX_CHILDREN as u64 {
        assert!(tree.add_node(i + 2, 1, "child") == 0);
    }

    // Try to add one more (should fail)
    assert!(tree.add_node(MAX_CHILDREN as u64 + 2, 1, "overflow") != 0);
    assert!(tree.size() == MAX_CHILDREN + 1);

    print!("\u{2713} PASS: test_tree_max_children\n");
}

fn test_tree_complex_structure() {
    print!("\n=== Testing Tree Complex Structure ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 1, "child2") == 0);
    assert!(tree.add_node(4, 1, "child3") == 0);
    assert!(tree.add_node(5, 2, "gc1") == 0);
    assert!(tree.add_node(6, 2, "gc2") == 0);
    assert!(tree.add_node(7, 3, "gc3") == 0);
    assert!(tree.add_node(8, 4, "gc4") == 0);
    assert!(tree.add_node(9, 4, "gc5") == 0);
    assert!(tree.add_node(10, 7, "ggc1") == 0);

    assert!(tree.size() == 10);
    assert!(tree.get_height(1) == 3);
    assert!(tree.count_descendants(1) == 9);
    assert!(tree.count_descendants(2) == 2);
    assert!(tree.count_descendants(7) == 1);

    tree.print();

    print!("\u{2713} PASS: test_tree_complex_structure\n");
}

fn main() {
    print!("\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}\n");
    print!("\u{2551}  TREE WITH HASHMAP ID MAPPING TESTS   \u{2551}\n");
    print!("\u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}\n");

    // Hashmap tests
    test_hashmap_basic();
    test_hashmap_collisions();

    // Tree creation tests
    test_tree_creation();
    test_tree_add_root();
    test_tree_add_children();

    // Tree structure tests
    test_tree_deep_hierarchy();
    test_tree_complex_structure();

    // Tree removal tests
    test_tree_remove_leaf();
    test_tree_remove_subtree();
    test_tree_remove_root();

    // Tree query tests
    test_tree_count_descendants();
    test_tree_find_path();

    // Error handling tests
    test_tree_duplicate_id();
    test_tree_max_children();

    print!("\n");
    print!("========================================\n");
    print!("  All tests passed successfully!\n");
    print!("========================================\n");
}
