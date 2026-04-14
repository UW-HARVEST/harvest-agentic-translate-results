mod hashmap;
mod tree;

use crate::hashmap::*;
use crate::tree::*;
use std::ffi::c_void;

fn expect(cond: bool) {
    assert!(cond);
}

fn test_hashmap_basic() {
    println!("\n=== Testing Hashmap Basic Operations ===");

    let mut map = hashmap_create();
    expect(hashmap_size(&map) == 0);

    let val1 = Box::into_raw(Box::new(42_i32));
    let val2 = Box::into_raw(Box::new(100_i32));
    let val3 = Box::into_raw(Box::new(200_i32));

    expect(hashmap_put(&mut map, 1, val1 as *mut c_void) == 0);
    expect(hashmap_put(&mut map, 2, val2 as *mut c_void) == 0);
    expect(hashmap_put(&mut map, 3, val3 as *mut c_void) == 0);
    expect(hashmap_size(&map) == 3);

    unsafe {
        expect(*(hashmap_get(&map, 1) as *mut i32) == 42);
        expect(*(hashmap_get(&map, 2) as *mut i32) == 100);
        expect(*(hashmap_get(&map, 3) as *mut i32) == 200);
    }

    let val4 = Box::into_raw(Box::new(500_i32));
    expect(hashmap_put(&mut map, 1, val4 as *mut c_void) == 0);
    expect(hashmap_size(&map) == 3);
    unsafe {
        expect(*(hashmap_get(&map, 1) as *mut i32) == 500);
    }

    let removed = hashmap_remove(&mut map, 2);
    expect(removed == val2 as *mut c_void);
    expect(hashmap_size(&map) == 2);
    expect(hashmap_get(&map, 2).is_null());

    expect(hashmap_contains(&map, 1) == 1);
    expect(hashmap_contains(&map, 2) == 0);
    expect(hashmap_contains(&map, 3) == 1);

    unsafe {
        drop(Box::from_raw(val1));
        drop(Box::from_raw(val3));
        drop(Box::from_raw(val4));
        drop(Box::from_raw(val2));
    }
    hashmap_destroy(map);
    println!("✓ PASS: {}", stringify!(test_hashmap_basic));
}

fn test_hashmap_collisions() {
    println!("\n=== Testing Hashmap Collisions ===");

    let mut map = hashmap_create();
    let mut ptrs = Vec::new();

    for i in 0..100_i32 {
        let ptr = Box::into_raw(Box::new(i * 10));
        ptrs.push(ptr);
        expect(hashmap_put(&mut map, i as u64, ptr as *mut c_void) == 0);
    }

    expect(hashmap_size(&map) == 100);

    for i in 0..100_i32 {
        let val = hashmap_get(&map, i as u64) as *mut i32;
        expect(!val.is_null());
        unsafe {
            expect(*val == i * 10);
        }
    }

    for ptr in ptrs {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
    hashmap_destroy(map);
    println!("✓ PASS: {}", stringify!(test_hashmap_collisions));
}

fn test_tree_creation() {
    println!("\n=== Testing Tree Creation ===");

    let mut tree = tree_create();
    expect(tree_size(&tree) == 0);
    expect(tree.has_root == 0);

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_creation));
}

fn test_tree_add_root() {
    println!("\n=== Testing Tree Add Root ===");

    let mut tree = tree_create();

    expect(tree_add_node(&mut tree, 1, 0, Some("root")) == 0);
    expect(tree_size(&tree) == 1);
    expect(tree.has_root == 1);
    expect(tree.root_id == 1);

    let root = tree_get_node(&tree, 1);
    expect(!root.is_null());
    unsafe {
        expect((*root).id == 1);
        expect(node_data_as_str(&*root) == "root");
        expect((*root).child_count == 0);
    }

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_add_root));
}

fn test_tree_add_children() {
    println!("\n=== Testing Tree Add Children ===");

    let mut tree = tree_create();

    expect(tree_add_node(&mut tree, 1, 0, Some("root")) == 0);
    expect(tree_add_node(&mut tree, 2, 1, Some("child1")) == 0);
    expect(tree_add_node(&mut tree, 3, 1, Some("child2")) == 0);
    expect(tree_add_node(&mut tree, 4, 1, Some("child3")) == 0);

    expect(tree_size(&tree) == 4);

    let root = tree_get_node(&tree, 1);
    unsafe {
        expect((*root).child_count == 3);
        expect((*root).child_ids[0] == 2);
        expect((*root).child_ids[1] == 3);
        expect((*root).child_ids[2] == 4);
    }

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_add_children));
}

fn test_tree_deep_hierarchy() {
    println!("\n=== Testing Tree Deep Hierarchy ===");

    let mut tree = tree_create();

    expect(tree_add_node(&mut tree, 1, 0, Some("level0")) == 0);
    expect(tree_add_node(&mut tree, 2, 1, Some("level1")) == 0);
    expect(tree_add_node(&mut tree, 3, 2, Some("level2")) == 0);
    expect(tree_add_node(&mut tree, 4, 3, Some("level3")) == 0);
    expect(tree_add_node(&mut tree, 5, 4, Some("level4")) == 0);

    expect(tree_size(&tree) == 5);

    expect(tree_get_depth(&tree, 1) == 0);
    expect(tree_get_depth(&tree, 2) == 1);
    expect(tree_get_depth(&tree, 3) == 2);
    expect(tree_get_depth(&tree, 4) == 3);
    expect(tree_get_depth(&tree, 5) == 4);

    expect(tree_get_height(&tree, 1) == 4);
    expect(tree_get_height(&tree, 2) == 3);
    expect(tree_get_height(&tree, 5) == 0);

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_deep_hierarchy));
}

fn test_tree_remove_leaf() {
    println!("\n=== Testing Tree Remove Leaf ===");

    let mut tree = tree_create();

    expect(tree_add_node(&mut tree, 1, 0, Some("root")) == 0);
    expect(tree_add_node(&mut tree, 2, 1, Some("child1")) == 0);
    expect(tree_add_node(&mut tree, 3, 1, Some("child2")) == 0);

    expect(tree_size(&tree) == 3);

    expect(tree_remove_node(&mut tree, 3) == 0);
    expect(tree_size(&tree) == 2);
    expect(tree_contains(&tree, 3) == 0);

    let root = tree_get_node(&tree, 1);
    unsafe {
        expect((*root).child_count == 1);
        expect((*root).child_ids[0] == 2);
    }

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_remove_leaf));
}

fn test_tree_remove_subtree() {
    println!("\n=== Testing Tree Remove Subtree ===");

    let mut tree = tree_create();

    expect(tree_add_node(&mut tree, 1, 0, Some("root")) == 0);
    expect(tree_add_node(&mut tree, 2, 1, Some("child1")) == 0);
    expect(tree_add_node(&mut tree, 3, 2, Some("grandchild1")) == 0);
    expect(tree_add_node(&mut tree, 4, 2, Some("grandchild2")) == 0);
    expect(tree_add_node(&mut tree, 5, 1, Some("child2")) == 0);

    expect(tree_size(&tree) == 5);

    expect(tree_remove_node(&mut tree, 2) == 0);
    expect(tree_size(&tree) == 2);
    expect(tree_contains(&tree, 2) == 0);
    expect(tree_contains(&tree, 3) == 0);
    expect(tree_contains(&tree, 4) == 0);
    expect(tree_contains(&tree, 1) == 1);
    expect(tree_contains(&tree, 5) == 1);

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_remove_subtree));
}

fn test_tree_remove_root() {
    println!("\n=== Testing Tree Remove Root ===");

    let mut tree = tree_create();

    expect(tree_add_node(&mut tree, 1, 0, Some("root")) == 0);
    expect(tree_add_node(&mut tree, 2, 1, Some("child1")) == 0);
    expect(tree_add_node(&mut tree, 3, 1, Some("child2")) == 0);

    expect(tree_size(&tree) == 3);

    expect(tree_remove_node(&mut tree, 1) == 0);
    expect(tree_size(&tree) == 0);
    expect(tree.has_root == 0);

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_remove_root));
}

fn test_tree_count_descendants() {
    println!("\n=== Testing Tree Count Descendants ===");

    let mut tree = tree_create();

    expect(tree_add_node(&mut tree, 1, 0, Some("root")) == 0);
    expect(tree_add_node(&mut tree, 2, 1, Some("child1")) == 0);
    expect(tree_add_node(&mut tree, 3, 2, Some("grandchild1")) == 0);
    expect(tree_add_node(&mut tree, 4, 2, Some("grandchild2")) == 0);
    expect(tree_add_node(&mut tree, 5, 1, Some("child2")) == 0);

    expect(tree_count_descendants(&tree, 1) == 4);
    expect(tree_count_descendants(&tree, 2) == 2);
    expect(tree_count_descendants(&tree, 3) == 0);
    expect(tree_count_descendants(&tree, 5) == 0);

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_count_descendants));
}

fn test_tree_find_path() {
    println!("\n=== Testing Tree Find Path ===");

    let mut tree = tree_create();

    expect(tree_add_node(&mut tree, 1, 0, Some("root")) == 0);
    expect(tree_add_node(&mut tree, 2, 1, Some("child")) == 0);
    expect(tree_add_node(&mut tree, 3, 2, Some("grandchild")) == 0);

    let mut path = [0_u64; 10];
    let mut length;

    length = tree_find_path(&tree, 3, &mut path, 10);
    expect(length == 3);
    expect(path[0] == 1);
    expect(path[1] == 2);
    expect(path[2] == 3);

    length = tree_find_path(&tree, 1, &mut path, 10);
    expect(length == 1);
    expect(path[0] == 1);

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_find_path));
}

fn test_tree_duplicate_id() {
    println!("\n=== Testing Tree Duplicate ID ===");

    let mut tree = tree_create();

    expect(tree_add_node(&mut tree, 1, 0, Some("root")) == 0);
    expect(tree_add_node(&mut tree, 2, 1, Some("child")) == 0);

    expect(tree_add_node(&mut tree, 2, 1, Some("duplicate")) != 0);
    expect(tree_size(&tree) == 2);

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_duplicate_id));
}

fn test_tree_max_children() {
    println!("\n=== Testing Tree Max Children ===");

    let mut tree = tree_create();

    expect(tree_add_node(&mut tree, 1, 0, Some("root")) == 0);

    for i in 0..MAX_CHILDREN {
        expect(tree_add_node(&mut tree, i as u64 + 2, 1, Some("child")) == 0);
    }

    expect(tree_add_node(&mut tree, MAX_CHILDREN as u64 + 2, 1, Some("overflow")) != 0);
    expect(tree_size(&tree) == MAX_CHILDREN + 1);

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_max_children));
}

fn test_tree_complex_structure() {
    println!("\n=== Testing Tree Complex Structure ===");

    let mut tree = tree_create();

    expect(tree_add_node(&mut tree, 1, 0, Some("root")) == 0);
    expect(tree_add_node(&mut tree, 2, 1, Some("child1")) == 0);
    expect(tree_add_node(&mut tree, 3, 1, Some("child2")) == 0);
    expect(tree_add_node(&mut tree, 4, 1, Some("child3")) == 0);
    expect(tree_add_node(&mut tree, 5, 2, Some("gc1")) == 0);
    expect(tree_add_node(&mut tree, 6, 2, Some("gc2")) == 0);
    expect(tree_add_node(&mut tree, 7, 3, Some("gc3")) == 0);
    expect(tree_add_node(&mut tree, 8, 4, Some("gc4")) == 0);
    expect(tree_add_node(&mut tree, 9, 4, Some("gc5")) == 0);
    expect(tree_add_node(&mut tree, 10, 7, Some("ggc1")) == 0);

    expect(tree_size(&tree) == 10);
    expect(tree_get_height(&tree, 1) == 3);
    expect(tree_count_descendants(&tree, 1) == 9);
    expect(tree_count_descendants(&tree, 2) == 2);
    expect(tree_count_descendants(&tree, 7) == 1);

    tree_print(&tree);

    tree_delete(&mut tree);
    println!("✓ PASS: {}", stringify!(test_tree_complex_structure));
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
