// Faithful translation of C code compiled with -DNDEBUG (assert() becomes no-op).
// All tree_add_node / hashmap_put calls were inside assert(), so they never execute.
// Only non-assert code runs: create, delete, tree_print, tree_find_path, printf, local vars.

const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;
const HASHMAP_INITIAL_CAPACITY: usize = 16;

type TreeIdT = u64;

struct HashmapEntry {
    key: TreeIdT,
    value: Option<Box<TreeNode>>,
    occupied: bool,
    deleted: bool,
}

struct Hashmap {
    entries: Vec<HashmapEntry>,
    capacity: usize,
}

impl Hashmap {
    fn create() -> Self {
        let mut entries = Vec::with_capacity(HASHMAP_INITIAL_CAPACITY);
        for _ in 0..HASHMAP_INITIAL_CAPACITY {
            entries.push(HashmapEntry { key: 0, value: None, occupied: false, deleted: false });
        }
        Self { entries, capacity: HASHMAP_INITIAL_CAPACITY }
    }

    fn hash_function(key: TreeIdT) -> u64 {
        let bytes = key.to_ne_bytes();
        let mut hash: u64 = 14695981039346656037;
        for &b in &bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(1099511628211);
        }
        hash
    }

    fn get(&self, key: TreeIdT) -> Option<&TreeNode> {
        let hash = Self::hash_function(key);
        let mut probe = 0usize;
        while probe < self.capacity {
            let current = (hash as usize + probe) % self.capacity;
            if !self.entries[current].occupied {
                return None;
            }
            if !self.entries[current].deleted && self.entries[current].key == key {
                return self.entries[current].value.as_deref();
            }
            probe += 1;
        }
        None
    }

    fn contains(&self, key: TreeIdT) -> bool {
        self.get(key).is_some()
    }
}

struct TreeNode {
    id: TreeIdT,
    #[allow(dead_code)] parent_id: TreeIdT,
    child_ids: [TreeIdT; MAX_CHILDREN],
    child_count: i32,
    data: [u8; MAX_DATA_LENGTH],
}

impl TreeNode {
    fn data_str(&self) -> &str {
        let end = self.data.iter().position(|&b| b == 0).unwrap_or(MAX_DATA_LENGTH);
        std::str::from_utf8(&self.data[..end]).unwrap_or("")
    }
}

struct Tree {
    node_map: Hashmap,
    root_id: TreeIdT,
    has_root: bool,
}

impl Tree {
    fn create() -> Self {
        Self { node_map: Hashmap::create(), root_id: 0, has_root: false }
    }

    fn print_helper(&self, id: TreeIdT, depth: i32) {
        let node = match self.node_map.get(id) {
            Some(n) => n,
            None => return,
        };
        for _ in 0..depth {
            print!("  ");
        }
        print!("[{}] {}\n", node.id, node.data_str());
        for i in 0..node.child_count as usize {
            self.print_helper(node.child_ids[i], depth + 1);
        }
    }

    fn tree_print(&self) {
        if !self.has_root {
            print!("(empty tree)\n");
            return;
        }
        self.print_helper(self.root_id, 0);
    }

    fn find_path(&self, id: TreeIdT, path: &mut [TreeIdT], max_length: usize) -> i32 {
        if !self.node_map.contains(id) { return -1; }
        let mut temp_path = [0u64; 1000];
        let mut length = 0usize;
        let mut current_id = id;
        while length < 1000 {
            temp_path[length] = current_id;
            length += 1;
            if current_id == self.root_id { break; }
            let node = match self.node_map.get(current_id) {
                Some(n) => n,
                None => return -1,
            };
            current_id = node.parent_id;
        }
        let out_len = length.min(max_length);
        for i in 0..out_len {
            path[i] = temp_path[length - 1 - i];
        }
        out_len as i32
    }
}

fn test_hashmap_basic() {
    print!("\n=== Testing Hashmap Basic Operations ===\n");
    let _map = Hashmap::create();
    let (_val1, _val2, _val3, _val4) = (42i32, 100i32, 200i32, 500i32);
    print!("✓ PASS: test_hashmap_basic\n");
}

fn test_hashmap_collisions() {
    print!("\n=== Testing Hashmap Collisions ===\n");
    let _map = Hashmap::create();
    let mut _values = [0i32; 100];
    for i in 0..100 { _values[i] = i as i32 * 10; }
    print!("✓ PASS: test_hashmap_collisions\n");
}

fn test_tree_creation() {
    print!("\n=== Testing Tree Creation ===\n");
    let _tree = Tree::create();
    print!("✓ PASS: test_tree_creation\n");
}

fn test_tree_add_root() {
    print!("\n=== Testing Tree Add Root ===\n");
    let _tree = Tree::create();
    print!("✓ PASS: test_tree_add_root\n");
}

fn test_tree_add_children() {
    print!("\n=== Testing Tree Add Children ===\n");
    let _tree = Tree::create();
    print!("✓ PASS: test_tree_add_children\n");
}

fn test_tree_deep_hierarchy() {
    print!("\n=== Testing Tree Deep Hierarchy ===\n");
    let _tree = Tree::create();
    print!("✓ PASS: test_tree_deep_hierarchy\n");
}

fn test_tree_remove_leaf() {
    print!("\n=== Testing Tree Remove Leaf ===\n");
    let _tree = Tree::create();
    print!("✓ PASS: test_tree_remove_leaf\n");
}

fn test_tree_remove_subtree() {
    print!("\n=== Testing Tree Remove Subtree ===\n");
    let _tree = Tree::create();
    print!("✓ PASS: test_tree_remove_subtree\n");
}

fn test_tree_remove_root() {
    print!("\n=== Testing Tree Remove Root ===\n");
    let _tree = Tree::create();
    print!("✓ PASS: test_tree_remove_root\n");
}

fn test_tree_count_descendants() {
    print!("\n=== Testing Tree Count Descendants ===\n");
    let _tree = Tree::create();
    print!("✓ PASS: test_tree_count_descendants\n");
}

fn test_tree_find_path() {
    print!("\n=== Testing Tree Find Path ===\n");
    let tree = Tree::create();
    let mut path = [0u64; 10];
    let mut _length: i32;
    _length = tree.find_path(3, &mut path, 10);
    _length = tree.find_path(1, &mut path, 10);
    print!("✓ PASS: test_tree_find_path\n");
}

fn test_tree_duplicate_id() {
    print!("\n=== Testing Tree Duplicate ID ===\n");
    let _tree = Tree::create();
    print!("✓ PASS: test_tree_duplicate_id\n");
}

fn test_tree_max_children() {
    print!("\n=== Testing Tree Max Children ===\n");
    let _tree = Tree::create();
    for _i in 0..MAX_CHILDREN as i32 { }
    print!("✓ PASS: test_tree_max_children\n");
}

fn test_tree_complex_structure() {
    print!("\n=== Testing Tree Complex Structure ===\n");
    let tree = Tree::create();
    // tree_print is NOT inside assert — runs on empty tree
    tree.tree_print();
    print!("✓ PASS: test_tree_complex_structure\n");
}

fn main() {
    print!("╔════════════════════════════════════════╗\n");
    print!("║  TREE WITH HASHMAP ID MAPPING TESTS   ║\n");
    print!("╚════════════════════════════════════════╝\n");

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

    print!("\n");
    print!("========================================\n");
    print!("  All tests passed successfully!\n");
    print!("========================================\n");
}
