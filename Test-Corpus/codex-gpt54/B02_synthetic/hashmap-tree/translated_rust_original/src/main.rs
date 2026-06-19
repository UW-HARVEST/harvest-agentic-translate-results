use std::io::{self, Write};

const HASHMAP_INITIAL_CAPACITY: usize = 16;
const HASHMAP_LOAD_FACTOR: f64 = 0.75;
const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;

type TreeId = u64;

#[derive(Clone, Copy, Default)]
struct HashmapEntry<T: Copy + Default> {
    key: TreeId,
    value: T,
    occupied: bool,
    deleted: bool,
}

struct Hashmap<T: Copy + Default> {
    entries: Vec<HashmapEntry<T>>,
    capacity: usize,
    size: usize,
    deleted_count: usize,
}

impl<T: Copy + Default> Hashmap<T> {
    fn create() -> Option<Self> {
        let capacity = HASHMAP_INITIAL_CAPACITY;
        let mut entries = Vec::new();
        entries.try_reserve_exact(capacity).ok()?;
        entries.resize(capacity, HashmapEntry::default());
        Some(Self {
            entries,
            capacity,
            size: 0,
            deleted_count: 0,
        })
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
        let old_entries = std::mem::take(&mut self.entries);

        self.capacity *= 2;
        let mut new_entries = Vec::new();
        if new_entries.try_reserve_exact(self.capacity).is_err() {
            self.entries = old_entries;
            self.capacity = old_capacity;
            return -1;
        }
        new_entries.resize(self.capacity, HashmapEntry::default());
        self.entries = new_entries;
        self.size = 0;
        self.deleted_count = 0;

        for entry in old_entries {
            if entry.occupied && !entry.deleted {
                let _ = self.put(entry.key, entry.value);
            }
        }

        0
    }

    fn put(&mut self, key: TreeId, value: T) -> i32 {
        if self.should_resize() && self.resize() != 0 {
            return -1;
        }

        let hash = Self::hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe = 0usize;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;
            let entry = &mut self.entries[current];

            if !entry.occupied {
                entry.key = key;
                entry.value = value;
                entry.occupied = true;
                entry.deleted = false;
                self.size += 1;
                return 0;
            } else if entry.deleted {
                entry.key = key;
                entry.value = value;
                entry.deleted = false;
                self.size += 1;
                self.deleted_count -= 1;
                return 0;
            } else if entry.key == key {
                entry.value = value;
                return 0;
            }

            probe += 1;
        }

        -1
    }

    fn get(&self, key: TreeId) -> Option<T> {
        let hash = Self::hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe = 0usize;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;
            let entry = self.entries[current];

            if !entry.occupied {
                return None;
            }

            if !entry.deleted && entry.key == key {
                return Some(entry.value);
            }

            probe += 1;
        }

        None
    }

    fn remove(&mut self, key: TreeId) -> Option<T> {
        let hash = Self::hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe = 0usize;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;
            let entry = &mut self.entries[current];

            if !entry.occupied {
                return None;
            }

            if !entry.deleted && entry.key == key {
                let value = entry.value;
                entry.deleted = true;
                self.size -= 1;
                self.deleted_count += 1;
                return Some(value);
            }

            probe += 1;
        }

        None
    }

    fn contains(&self, key: TreeId) -> i32 {
        if self.get(key).is_some() { 1 } else { 0 }
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

struct Tree {
    node_map: Hashmap<usize>,
    root_id: TreeId,
    has_root: i32,
    node_count: usize,
    nodes: Vec<Option<TreeNode>>,
}

impl TreeNode {
    fn new(id: TreeId, parent_id: TreeId, data: Option<&str>) -> Self {
        let mut node = Self {
            id,
            parent_id,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: [0; MAX_DATA_LENGTH],
        };

        if let Some(text) = data {
            let bytes = text.as_bytes();
            let len = bytes.len().min(MAX_DATA_LENGTH - 1);
            node.data[..len].copy_from_slice(&bytes[..len]);
            node.data[MAX_DATA_LENGTH - 1] = 0;
        } else {
            node.data[0] = 0;
        }

        node
    }

    fn data_bytes(&self) -> &[u8] {
        let len = self
            .data
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(MAX_DATA_LENGTH);
        &self.data[..len]
    }
}

impl Tree {
    fn create() -> Option<Self> {
        Some(Self {
            node_map: Hashmap::create()?,
            root_id: 0,
            has_root: 0,
            node_count: 0,
            nodes: Vec::new(),
        })
    }

    fn add_node(&mut self, id: TreeId, parent_id: TreeId, data: Option<&str>) -> i32 {
        if self.contains(id) != 0 {
            eprint!("Error: Node with ID {} already exists\n", id);
            return -1;
        }

        let mut node = TreeNode::new(id, parent_id, data);

        if self.has_root == 0 {
            self.root_id = id;
            self.has_root = 1;
            node.parent_id = 0;
        } else {
            let Some(parent_index) = self.node_map.get(parent_id) else {
                eprint!("Error: Parent node {} not found\n", parent_id);
                return -1;
            };

            let parent = self.nodes[parent_index].as_mut().unwrap();
            if parent.child_count >= MAX_CHILDREN as i32 {
                eprint!("Error: Parent has maximum children\n");
                return -1;
            }

            parent.child_ids[parent.child_count as usize] = id;
            parent.child_count += 1;
        }

        let index = self.nodes.len();
        self.nodes.push(Some(node));
        if self.node_map.put(id, index) != 0 {
            eprint!("Error: Failed to add node to hashmap\n");
            let _ = self.nodes.pop();
            return -1;
        }

        self.node_count += 1;
        0
    }

    fn remove_subtree(&mut self, id: TreeId) -> i32 {
        let Some(index) = self.node_map.get(id) else {
            return -1;
        };

        let child_ids = {
            let node = self.nodes[index].as_ref().unwrap();
            let mut ids = Vec::with_capacity(node.child_count as usize);
            for i in 0..node.child_count as usize {
                ids.push(node.child_ids[i]);
            }
            ids
        };

        for child_id in child_ids {
            self.remove_subtree(child_id);
        }

        if let Some(removed_index) = self.node_map.remove(id) {
            self.nodes[removed_index] = None;
            self.node_count -= 1;
        }

        0
    }

    fn remove_node(&mut self, id: TreeId) -> i32 {
        let Some(index) = self.node_map.get(id) else {
            eprint!("Error: Node {} not found\n", id);
            return -1;
        };

        let parent_id = self.nodes[index].as_ref().unwrap().parent_id;

        if id == self.root_id {
            self.remove_subtree(id);
            self.has_root = 0;
            self.root_id = 0;
            return 0;
        }

        if let Some(parent_index) = self.node_map.get(parent_id) {
            let parent = self.nodes[parent_index].as_mut().unwrap();
            for i in 0..parent.child_count as usize {
                if parent.child_ids[i] == id {
                    for j in i..(parent.child_count as usize - 1) {
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
        let index = self.node_map.get(id)?;
        self.nodes[index].as_ref()
    }

    fn contains(&self, id: TreeId) -> i32 {
        if self.get_node(id).is_some() { 1 } else { 0 }
    }

    fn size(&self) -> usize {
        self.node_count
    }

    fn print(&self) {
        if self.has_root == 0 {
            write_stdout("(empty tree)\n");
            return;
        }

        self.print_helper(self.root_id, 0);
    }

    fn print_helper(&self, id: TreeId, depth: i32) {
        let Some(node) = self.get_node(id) else {
            return;
        };

        for _ in 0..depth {
            write_stdout("  ");
        }
        write_stdout("[");
        write_stdout(&id.to_string());
        write_stdout("] ");
        write_stdout_bytes(node.data_bytes());
        write_stdout("\n");

        for i in 0..node.child_count as usize {
            self.print_helper(node.child_ids[i], depth + 1);
        }
    }

    fn get_depth(&self, id: TreeId) -> i32 {
        if self.contains(id) == 0 {
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
        for i in 0..node.child_count as usize {
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
        for i in 0..node.child_count as usize {
            count += 1;
            count += self.count_descendants(node.child_ids[i]);
        }

        count
    }

    fn find_path(&self, id: TreeId, path: &mut [TreeId], max_length: i32) -> i32 {
        if path.is_empty() && max_length > 0 {
            return -1;
        }
        if self.contains(id) == 0 {
            return -1;
        }

        let mut temp_path = [0_u64; 1000];
        let mut length = 0_i32;
        let mut current_id = id;

        while length < 1000 {
            temp_path[length as usize] = current_id;
            length += 1;

            if current_id == self.root_id {
                break;
            }

            let Some(node) = self.get_node(current_id) else {
                return -1;
            };
            current_id = node.parent_id;
        }

        if length > max_length {
            length = max_length;
        }

        for i in 0..length as usize {
            path[i] = temp_path[length as usize - 1 - i];
        }

        length
    }
}

fn write_stdout(text: &str) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes()).unwrap();
}

fn write_stdout_bytes(bytes: &[u8]) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes).unwrap();
}

fn test_pass(name: &str) {
    write_stdout("✓ PASS: ");
    write_stdout(name);
    write_stdout("\n");
}

fn test_hashmap_basic() {
    write_stdout("\n=== Testing Hashmap Basic Operations ===\n");

    let mut map = Hashmap::<*const i32>::create().unwrap();
    assert!(map.size() == 0);

    let val1 = 42;
    let val2 = 100;
    let val3 = 200;
    assert!(map.put(1, &val1) == 0);
    assert!(map.put(2, &val2) == 0);
    assert!(map.put(3, &val3) == 0);
    assert!(map.size() == 3);

    assert!(unsafe { *map.get(1).unwrap() } == 42);
    assert!(unsafe { *map.get(2).unwrap() } == 100);
    assert!(unsafe { *map.get(3).unwrap() } == 200);

    let val4 = 500;
    assert!(map.put(1, &val4) == 0);
    assert!(map.size() == 3);
    assert!(unsafe { *map.get(1).unwrap() } == 500);

    let removed = map.remove(2).unwrap();
    assert!(std::ptr::eq(removed, &val2));
    assert!(map.size() == 2);
    assert!(map.get(2).is_none());

    assert!(map.contains(1) == 1);
    assert!(map.contains(2) == 0);
    assert!(map.contains(3) == 1);

    test_pass("test_hashmap_basic");
}

fn test_hashmap_collisions() {
    write_stdout("\n=== Testing Hashmap Collisions ===\n");

    let mut map = Hashmap::<*const i32>::create().unwrap();
    let values: Vec<i32> = (0..100).map(|i| i * 10).collect();

    for (i, value) in values.iter().enumerate() {
        assert!(map.put(i as u64, value) == 0);
    }

    assert!(map.size() == 100);

    for (i, expected) in values.iter().enumerate() {
        let value = map.get(i as u64).unwrap();
        assert!(unsafe { *value } == *expected);
    }

    test_pass("test_hashmap_collisions");
}

fn test_tree_creation() {
    write_stdout("\n=== Testing Tree Creation ===\n");

    let tree = Tree::create().unwrap();
    assert!(tree.size() == 0);
    assert!(tree.has_root == 0);

    test_pass("test_tree_creation");
}

fn test_tree_add_root() {
    write_stdout("\n=== Testing Tree Add Root ===\n");

    let mut tree = Tree::create().unwrap();
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.size() == 1);
    assert!(tree.has_root == 1);
    assert!(tree.root_id == 1);

    let root = tree.get_node(1).unwrap();
    assert!(root.id == 1);
    assert!(root.data_bytes() == b"root");
    assert!(root.child_count == 0);

    test_pass("test_tree_add_root");
}

fn test_tree_add_children() {
    write_stdout("\n=== Testing Tree Add Children ===\n");

    let mut tree = Tree::create().unwrap();
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child1")) == 0);
    assert!(tree.add_node(3, 1, Some("child2")) == 0);
    assert!(tree.add_node(4, 1, Some("child3")) == 0);

    assert!(tree.size() == 4);

    let root = tree.get_node(1).unwrap();
    assert!(root.child_count == 3);
    assert!(root.child_ids[0] == 2);
    assert!(root.child_ids[1] == 3);
    assert!(root.child_ids[2] == 4);

    test_pass("test_tree_add_children");
}

fn test_tree_deep_hierarchy() {
    write_stdout("\n=== Testing Tree Deep Hierarchy ===\n");

    let mut tree = Tree::create().unwrap();
    assert!(tree.add_node(1, 0, Some("level0")) == 0);
    assert!(tree.add_node(2, 1, Some("level1")) == 0);
    assert!(tree.add_node(3, 2, Some("level2")) == 0);
    assert!(tree.add_node(4, 3, Some("level3")) == 0);
    assert!(tree.add_node(5, 4, Some("level4")) == 0);

    assert!(tree.size() == 5);

    assert!(tree.get_depth(1) == 0);
    assert!(tree.get_depth(2) == 1);
    assert!(tree.get_depth(3) == 2);
    assert!(tree.get_depth(4) == 3);
    assert!(tree.get_depth(5) == 4);

    assert!(tree.get_height(1) == 4);
    assert!(tree.get_height(2) == 3);
    assert!(tree.get_height(5) == 0);

    test_pass("test_tree_deep_hierarchy");
}

fn test_tree_remove_leaf() {
    write_stdout("\n=== Testing Tree Remove Leaf ===\n");

    let mut tree = Tree::create().unwrap();
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child1")) == 0);
    assert!(tree.add_node(3, 1, Some("child2")) == 0);

    assert!(tree.size() == 3);

    assert!(tree.remove_node(3) == 0);
    assert!(tree.size() == 2);
    assert!(tree.contains(3) == 0);

    let root = tree.get_node(1).unwrap();
    assert!(root.child_count == 1);
    assert!(root.child_ids[0] == 2);

    test_pass("test_tree_remove_leaf");
}

fn test_tree_remove_subtree() {
    write_stdout("\n=== Testing Tree Remove Subtree ===\n");

    let mut tree = Tree::create().unwrap();
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child1")) == 0);
    assert!(tree.add_node(3, 2, Some("grandchild1")) == 0);
    assert!(tree.add_node(4, 2, Some("grandchild2")) == 0);
    assert!(tree.add_node(5, 1, Some("child2")) == 0);

    assert!(tree.size() == 5);

    assert!(tree.remove_node(2) == 0);
    assert!(tree.size() == 2);
    assert!(tree.contains(2) == 0);
    assert!(tree.contains(3) == 0);
    assert!(tree.contains(4) == 0);
    assert!(tree.contains(1) == 1);
    assert!(tree.contains(5) == 1);

    test_pass("test_tree_remove_subtree");
}

fn test_tree_remove_root() {
    write_stdout("\n=== Testing Tree Remove Root ===\n");

    let mut tree = Tree::create().unwrap();
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child1")) == 0);
    assert!(tree.add_node(3, 1, Some("child2")) == 0);

    assert!(tree.size() == 3);

    assert!(tree.remove_node(1) == 0);
    assert!(tree.size() == 0);
    assert!(tree.has_root == 0);

    test_pass("test_tree_remove_root");
}

fn test_tree_count_descendants() {
    write_stdout("\n=== Testing Tree Count Descendants ===\n");

    let mut tree = Tree::create().unwrap();
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child1")) == 0);
    assert!(tree.add_node(3, 2, Some("grandchild1")) == 0);
    assert!(tree.add_node(4, 2, Some("grandchild2")) == 0);
    assert!(tree.add_node(5, 1, Some("child2")) == 0);

    assert!(tree.count_descendants(1) == 4);
    assert!(tree.count_descendants(2) == 2);
    assert!(tree.count_descendants(3) == 0);
    assert!(tree.count_descendants(5) == 0);

    test_pass("test_tree_count_descendants");
}

fn test_tree_find_path() {
    write_stdout("\n=== Testing Tree Find Path ===\n");

    let mut tree = Tree::create().unwrap();
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child")) == 0);
    assert!(tree.add_node(3, 2, Some("grandchild")) == 0);

    let mut path = [0_u64; 10];

    let mut length = tree.find_path(3, &mut path, 10);
    assert!(length == 3);
    assert!(path[0] == 1);
    assert!(path[1] == 2);
    assert!(path[2] == 3);

    length = tree.find_path(1, &mut path, 10);
    assert!(length == 1);
    assert!(path[0] == 1);

    test_pass("test_tree_find_path");
}

fn test_tree_duplicate_id() {
    write_stdout("\n=== Testing Tree Duplicate ID ===\n");

    let mut tree = Tree::create().unwrap();
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child")) == 0);

    assert!(tree.add_node(2, 1, Some("duplicate")) != 0);
    assert!(tree.size() == 2);

    test_pass("test_tree_duplicate_id");
}

fn test_tree_max_children() {
    write_stdout("\n=== Testing Tree Max Children ===\n");

    let mut tree = Tree::create().unwrap();
    assert!(tree.add_node(1, 0, Some("root")) == 0);

    for i in 0..MAX_CHILDREN {
        assert!(tree.add_node(i as u64 + 2, 1, Some("child")) == 0);
    }

    assert!(tree.add_node(MAX_CHILDREN as u64 + 2, 1, Some("overflow")) != 0);
    assert!(tree.size() == MAX_CHILDREN + 1);

    test_pass("test_tree_max_children");
}

fn test_tree_complex_structure() {
    write_stdout("\n=== Testing Tree Complex Structure ===\n");

    let mut tree = Tree::create().unwrap();
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child1")) == 0);
    assert!(tree.add_node(3, 1, Some("child2")) == 0);
    assert!(tree.add_node(4, 1, Some("child3")) == 0);
    assert!(tree.add_node(5, 2, Some("gc1")) == 0);
    assert!(tree.add_node(6, 2, Some("gc2")) == 0);
    assert!(tree.add_node(7, 3, Some("gc3")) == 0);
    assert!(tree.add_node(8, 4, Some("gc4")) == 0);
    assert!(tree.add_node(9, 4, Some("gc5")) == 0);
    assert!(tree.add_node(10, 7, Some("ggc1")) == 0);

    assert!(tree.size() == 10);
    assert!(tree.get_height(1) == 3);
    assert!(tree.count_descendants(1) == 9);
    assert!(tree.count_descendants(2) == 2);
    assert!(tree.count_descendants(7) == 1);

    tree.print();

    test_pass("test_tree_complex_structure");
}

fn main() {
    write_stdout("╔════════════════════════════════════════╗\n");
    write_stdout("║  TREE WITH HASHMAP ID MAPPING TESTS   ║\n");
    write_stdout("╚════════════════════════════════════════╝\n");

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

    write_stdout("\n");
    write_stdout("========================================\n");
    write_stdout("  All tests passed successfully!\n");
    write_stdout("========================================\n");
}
