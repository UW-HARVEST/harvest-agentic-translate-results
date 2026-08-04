// main.rs - faithful translation of main.c

mod hashmap;
mod tree;

use hashmap::Hashmap;
use std::ffi::c_void;
use tree::Tree;

// Equivalent of TEST_PASS macro
fn test_pass(func_name: &str) {
    println!("\u{2713} PASS: {}", func_name);
}

fn test_hashmap_basic() {
    println!("\n=== Testing Hashmap Basic Operations ===");

    let mut map = Hashmap::new();
    assert!(map.len() == 0);

    // Test put and get
    let val1: i32 = 42;
    let val2: i32 = 100;
    let val3: i32 = 200;
    let p1 = &val1 as *const i32 as *mut c_void;
    let p2 = &val2 as *const i32 as *mut c_void;
    let p3 = &val3 as *const i32 as *mut c_void;

    assert!(map.put(1, p1) == 0);
    assert!(map.put(2, p2) == 0);
    assert!(map.put(3, p3) == 0);
    assert!(map.len() == 3);

    unsafe {
        assert!(*(map.get(1) as *const i32) == 42);
        assert!(*(map.get(2) as *const i32) == 100);
        assert!(*(map.get(3) as *const i32) == 200);
    }

    // Test update
    let val4: i32 = 500;
    let p4 = &val4 as *const i32 as *mut c_void;
    assert!(map.put(1, p4) == 0);
    assert!(map.len() == 3);
    unsafe {
        assert!(*(map.get(1) as *const i32) == 500);
    }

    // Test remove
    let removed = map.remove(2);
    assert!(removed == p2);
    assert!(map.len() == 2);
    assert!(map.get(2).is_null());

    // Test contains
    assert!(map.contains(1));
    assert!(!map.contains(2));
    assert!(map.contains(3));

    // hashmap_destroy: drops via scope
    test_pass("test_hashmap_basic");
}

fn test_hashmap_collisions() {
    println!("\n=== Testing Hashmap Collisions ===");

    let mut map = Hashmap::new();

    // Add many items to force collisions
    let mut values = [0i32; 100];
    for i in 0..100 {
        values[i] = (i as i32) * 10;
    }
    // Take pointers AFTER values fully populated to avoid mutation alias issues
    for i in 0..100 {
        let p = &values[i] as *const i32 as *mut c_void;
        assert!(map.put(i as u64, p) == 0);
    }

    assert!(map.len() == 100);

    for i in 0..100 {
        let v = map.get(i as u64);
        assert!(!v.is_null());
        unsafe {
            assert!(*(v as *const i32) == (i as i32) * 10);
        }
    }

    test_pass("test_hashmap_collisions");
}

fn test_tree_creation() {
    println!("\n=== Testing Tree Creation ===");

    let tree = Tree::new();
    assert!(tree.size() == 0);
    assert!(tree.has_root == 0);

    tree.delete();
    test_pass("test_tree_creation");
}

fn test_tree_add_root() {
    println!("\n=== Testing Tree Add Root ===");

    let mut tree = Tree::new();

    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.size() == 1);
    assert!(tree.has_root == 1);
    assert!(tree.root_id == 1);

    let root_ptr = tree.get_node(1);
    assert!(!root_ptr.is_null());
    unsafe {
        let root = &*root_ptr;
        assert!(root.id == 1);
        assert!(root.data_str() == "root");
        assert!(root.child_count == 0);
    }

    tree.delete();
    test_pass("test_tree_add_root");
}

fn test_tree_add_children() {
    println!("\n=== Testing Tree Add Children ===");

    let mut tree = Tree::new();

    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 1, "child2") == 0);
    assert!(tree.add_node(4, 1, "child3") == 0);

    assert!(tree.size() == 4);

    let root_ptr = tree.get_node(1);
    unsafe {
        let root = &*root_ptr;
        assert!(root.child_count == 3);
        assert!(root.child_ids[0] == 2);
        assert!(root.child_ids[1] == 3);
        assert!(root.child_ids[2] == 4);
    }

    tree.delete();
    test_pass("test_tree_add_children");
}

fn test_tree_deep_hierarchy() {
    println!("\n=== Testing Tree Deep Hierarchy ===");

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

    tree.delete();
    test_pass("test_tree_deep_hierarchy");
}

fn test_tree_remove_leaf() {
    println!("\n=== Testing Tree Remove Leaf ===");

    let mut tree = Tree::new();

    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 1, "child2") == 0);

    assert!(tree.size() == 3);

    assert!(tree.remove_node(3) == 0);
    assert!(tree.size() == 2);
    assert!(!tree.contains(3));

    let root_ptr = tree.get_node(1);
    unsafe {
        let root = &*root_ptr;
        assert!(root.child_count == 1);
        assert!(root.child_ids[0] == 2);
    }

    tree.delete();
    test_pass("test_tree_remove_leaf");
}

fn test_tree_remove_subtree() {
    println!("\n=== Testing Tree Remove Subtree ===");

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

    tree.delete();
    test_pass("test_tree_remove_subtree");
}

fn test_tree_remove_root() {
    println!("\n=== Testing Tree Remove Root ===");

    let mut tree = Tree::new();

    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 1, "child2") == 0);

    assert!(tree.size() == 3);

    assert!(tree.remove_node(1) == 0);
    assert!(tree.size() == 0);
    assert!(tree.has_root == 0);

    tree.delete();
    test_pass("test_tree_remove_root");
}

fn test_tree_count_descendants() {
    println!("\n=== Testing Tree Count Descendants ===");

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

    tree.delete();
    test_pass("test_tree_count_descendants");
}

fn test_tree_find_path() {
    println!("\n=== Testing Tree Find Path ===");

    let mut tree = Tree::new();

    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child") == 0);
    assert!(tree.add_node(3, 2, "grandchild") == 0);

    let mut path = [0u64; 10];
    let length;

    let length1 = tree.find_path(3, &mut path, 10);
    assert!(length1 == 3);
    assert!(path[0] == 1);
    assert!(path[1] == 2);
    assert!(path[2] == 3);

    length = tree.find_path(1, &mut path, 10);
    assert!(length == 1);
    assert!(path[0] == 1);

    tree.delete();
    test_pass("test_tree_find_path");
}

fn test_tree_duplicate_id() {
    println!("\n=== Testing Tree Duplicate ID ===");

    let mut tree = Tree::new();

    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child") == 0);

    assert!(tree.add_node(2, 1, "duplicate") != 0);
    assert!(tree.size() == 2);

    tree.delete();
    test_pass("test_tree_duplicate_id");
}

fn test_tree_max_children() {
    println!("\n=== Testing Tree Max Children ===");

    let mut tree = Tree::new();

    assert!(tree.add_node(1, 0, "root") == 0);

    for i in 0..tree::MAX_CHILDREN {
        assert!(tree.add_node((i + 2) as u64, 1, "child") == 0);
    }

    assert!(tree.add_node((tree::MAX_CHILDREN + 2) as u64, 1, "overflow") != 0);
    assert!(tree.size() == tree::MAX_CHILDREN + 1);

    tree.delete();
    test_pass("test_tree_max_children");
}

fn test_tree_complex_structure() {
    println!("\n=== Testing Tree Complex Structure ===");

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

    tree.delete();
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
