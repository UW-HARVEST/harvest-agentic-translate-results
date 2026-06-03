// main.rs - Rust translation of main.c
mod hashmap;
mod tree;

use hashmap::{
    hashmap_contains, hashmap_create, hashmap_destroy, hashmap_get, hashmap_put, hashmap_remove,
    hashmap_size, TreeId,
};
use std::ffi::c_void;
use tree::{
    tree_add_node, tree_contains, tree_count_descendants, tree_create, tree_delete,
    tree_find_path, tree_get_depth, tree_get_height, tree_get_node, tree_print, tree_size,
    tree_remove_node, MAX_CHILDREN,
};

macro_rules! test_pass {
    ($name:expr) => {
        println!("\u{2713} PASS: {}", $name);
    };
}

fn test_hashmap_basic() {
    println!("\n=== Testing Hashmap Basic Operations ===");

    let mut map = hashmap_create();
    assert_eq!(hashmap_size(&map), 0);

    let mut val1: i32 = 42;
    let mut val2: i32 = 100;
    let mut val3: i32 = 200;
    assert_eq!(hashmap_put(&mut map, 1, &mut val1 as *mut i32 as *mut c_void), 0);
    assert_eq!(hashmap_put(&mut map, 2, &mut val2 as *mut i32 as *mut c_void), 0);
    assert_eq!(hashmap_put(&mut map, 3, &mut val3 as *mut i32 as *mut c_void), 0);
    assert_eq!(hashmap_size(&map), 3);

    unsafe {
        assert_eq!(*(hashmap_get(&map, 1) as *const i32), 42);
        assert_eq!(*(hashmap_get(&map, 2) as *const i32), 100);
        assert_eq!(*(hashmap_get(&map, 3) as *const i32), 200);
    }

    let mut val4: i32 = 500;
    assert_eq!(hashmap_put(&mut map, 1, &mut val4 as *mut i32 as *mut c_void), 0);
    assert_eq!(hashmap_size(&map), 3);
    unsafe {
        assert_eq!(*(hashmap_get(&map, 1) as *const i32), 500);
    }

    let removed = hashmap_remove(&mut map, 2);
    assert_eq!(removed, &mut val2 as *mut i32 as *mut c_void);
    assert_eq!(hashmap_size(&map), 2);
    assert!(hashmap_get(&map, 2).is_null());

    assert!(hashmap_contains(&map, 1));
    assert!(!hashmap_contains(&map, 2));
    assert!(hashmap_contains(&map, 3));

    hashmap_destroy(map);
    test_pass!("test_hashmap_basic");
}

fn test_hashmap_collisions() {
    println!("\n=== Testing Hashmap Collisions ===");

    let mut map = hashmap_create();

    let mut values: Vec<i32> = (0..100).map(|i| i * 10).collect();
    for i in 0..100 {
        let key = i as TreeId;
        assert_eq!(
            hashmap_put(&mut map, key, &mut values[i] as *mut i32 as *mut c_void),
            0
        );
    }
    assert_eq!(hashmap_size(&map), 100);

    for i in 0..100 {
        let key = i as TreeId;
        let val_ptr = hashmap_get(&map, key) as *const i32;
        assert!(!val_ptr.is_null());
        unsafe {
            assert_eq!(*val_ptr, (i as i32) * 10);
        }
    }

    hashmap_destroy(map);
    test_pass!("test_hashmap_collisions");
}

fn test_tree_creation() {
    println!("\n=== Testing Tree Creation ===");

    let tree = tree_create();
    assert_eq!(tree_size(&tree), 0);
    assert!(!tree.has_root);

    tree_delete(tree);
    test_pass!("test_tree_creation");
}

fn test_tree_add_root() {
    println!("\n=== Testing Tree Add Root ===");

    let mut tree = tree_create();

    assert_eq!(tree_add_node(&mut tree, 1, 0, Some("root")), 0);
    assert_eq!(tree_size(&tree), 1);
    assert!(tree.has_root);
    assert_eq!(tree.root_id, 1);

    let root_ptr = tree_get_node(&tree, 1);
    assert!(!root_ptr.is_null());
    unsafe {
        let root = &*root_ptr;
        assert_eq!(root.id, 1);
        assert_eq!(root.data_str(), "root");
        assert_eq!(root.child_count, 0);
    }

    tree_delete(tree);
    test_pass!("test_tree_add_root");
}

fn test_tree_add_children() {
    println!("\n=== Testing Tree Add Children ===");

    let mut tree = tree_create();

    assert_eq!(tree_add_node(&mut tree, 1, 0, Some("root")), 0);
    assert_eq!(tree_add_node(&mut tree, 2, 1, Some("child1")), 0);
    assert_eq!(tree_add_node(&mut tree, 3, 1, Some("child2")), 0);
    assert_eq!(tree_add_node(&mut tree, 4, 1, Some("child3")), 0);

    assert_eq!(tree_size(&tree), 4);

    let root_ptr = tree_get_node(&tree, 1);
    unsafe {
        let root = &*root_ptr;
        assert_eq!(root.child_count, 3);
        assert_eq!(root.child_ids[0], 2);
        assert_eq!(root.child_ids[1], 3);
        assert_eq!(root.child_ids[2], 4);
    }

    tree_delete(tree);
    test_pass!("test_tree_add_children");
}

fn test_tree_deep_hierarchy() {
    println!("\n=== Testing Tree Deep Hierarchy ===");

    let mut tree = tree_create();

    assert_eq!(tree_add_node(&mut tree, 1, 0, Some("level0")), 0);
    assert_eq!(tree_add_node(&mut tree, 2, 1, Some("level1")), 0);
    assert_eq!(tree_add_node(&mut tree, 3, 2, Some("level2")), 0);
    assert_eq!(tree_add_node(&mut tree, 4, 3, Some("level3")), 0);
    assert_eq!(tree_add_node(&mut tree, 5, 4, Some("level4")), 0);

    assert_eq!(tree_size(&tree), 5);

    assert_eq!(tree_get_depth(&tree, 1), 0);
    assert_eq!(tree_get_depth(&tree, 2), 1);
    assert_eq!(tree_get_depth(&tree, 3), 2);
    assert_eq!(tree_get_depth(&tree, 4), 3);
    assert_eq!(tree_get_depth(&tree, 5), 4);

    assert_eq!(tree_get_height(&tree, 1), 4);
    assert_eq!(tree_get_height(&tree, 2), 3);
    assert_eq!(tree_get_height(&tree, 5), 0);

    tree_delete(tree);
    test_pass!("test_tree_deep_hierarchy");
}

fn test_tree_remove_leaf() {
    println!("\n=== Testing Tree Remove Leaf ===");

    let mut tree = tree_create();

    assert_eq!(tree_add_node(&mut tree, 1, 0, Some("root")), 0);
    assert_eq!(tree_add_node(&mut tree, 2, 1, Some("child1")), 0);
    assert_eq!(tree_add_node(&mut tree, 3, 1, Some("child2")), 0);
    assert_eq!(tree_size(&tree), 3);

    assert_eq!(tree_remove_node(&mut tree, 3), 0);
    assert_eq!(tree_size(&tree), 2);
    assert!(!tree_contains(&tree, 3));

    let root_ptr = tree_get_node(&tree, 1);
    unsafe {
        let root = &*root_ptr;
        assert_eq!(root.child_count, 1);
        assert_eq!(root.child_ids[0], 2);
    }

    tree_delete(tree);
    test_pass!("test_tree_remove_leaf");
}

fn test_tree_remove_subtree() {
    println!("\n=== Testing Tree Remove Subtree ===");

    let mut tree = tree_create();

    assert_eq!(tree_add_node(&mut tree, 1, 0, Some("root")), 0);
    assert_eq!(tree_add_node(&mut tree, 2, 1, Some("child1")), 0);
    assert_eq!(tree_add_node(&mut tree, 3, 2, Some("grandchild1")), 0);
    assert_eq!(tree_add_node(&mut tree, 4, 2, Some("grandchild2")), 0);
    assert_eq!(tree_add_node(&mut tree, 5, 1, Some("child2")), 0);
    assert_eq!(tree_size(&tree), 5);

    assert_eq!(tree_remove_node(&mut tree, 2), 0);
    assert_eq!(tree_size(&tree), 2);
    assert!(!tree_contains(&tree, 2));
    assert!(!tree_contains(&tree, 3));
    assert!(!tree_contains(&tree, 4));
    assert!(tree_contains(&tree, 1));
    assert!(tree_contains(&tree, 5));

    tree_delete(tree);
    test_pass!("test_tree_remove_subtree");
}

fn test_tree_remove_root() {
    println!("\n=== Testing Tree Remove Root ===");

    let mut tree = tree_create();

    assert_eq!(tree_add_node(&mut tree, 1, 0, Some("root")), 0);
    assert_eq!(tree_add_node(&mut tree, 2, 1, Some("child1")), 0);
    assert_eq!(tree_add_node(&mut tree, 3, 1, Some("child2")), 0);
    assert_eq!(tree_size(&tree), 3);

    assert_eq!(tree_remove_node(&mut tree, 1), 0);
    assert_eq!(tree_size(&tree), 0);
    assert!(!tree.has_root);

    tree_delete(tree);
    test_pass!("test_tree_remove_root");
}

fn test_tree_count_descendants() {
    println!("\n=== Testing Tree Count Descendants ===");

    let mut tree = tree_create();

    assert_eq!(tree_add_node(&mut tree, 1, 0, Some("root")), 0);
    assert_eq!(tree_add_node(&mut tree, 2, 1, Some("child1")), 0);
    assert_eq!(tree_add_node(&mut tree, 3, 2, Some("grandchild1")), 0);
    assert_eq!(tree_add_node(&mut tree, 4, 2, Some("grandchild2")), 0);
    assert_eq!(tree_add_node(&mut tree, 5, 1, Some("child2")), 0);

    assert_eq!(tree_count_descendants(&tree, 1), 4);
    assert_eq!(tree_count_descendants(&tree, 2), 2);
    assert_eq!(tree_count_descendants(&tree, 3), 0);
    assert_eq!(tree_count_descendants(&tree, 5), 0);

    tree_delete(tree);
    test_pass!("test_tree_count_descendants");
}

fn test_tree_find_path() {
    println!("\n=== Testing Tree Find Path ===");

    let mut tree = tree_create();

    assert_eq!(tree_add_node(&mut tree, 1, 0, Some("root")), 0);
    assert_eq!(tree_add_node(&mut tree, 2, 1, Some("child")), 0);
    assert_eq!(tree_add_node(&mut tree, 3, 2, Some("grandchild")), 0);

    let mut path: [TreeId; 10] = [0; 10];

    let length = tree_find_path(&tree, 3, &mut path, 10);
    assert_eq!(length, 3);
    assert_eq!(path[0], 1);
    assert_eq!(path[1], 2);
    assert_eq!(path[2], 3);

    let length = tree_find_path(&tree, 1, &mut path, 10);
    assert_eq!(length, 1);
    assert_eq!(path[0], 1);

    tree_delete(tree);
    test_pass!("test_tree_find_path");
}

fn test_tree_duplicate_id() {
    println!("\n=== Testing Tree Duplicate ID ===");

    let mut tree = tree_create();

    assert_eq!(tree_add_node(&mut tree, 1, 0, Some("root")), 0);
    assert_eq!(tree_add_node(&mut tree, 2, 1, Some("child")), 0);

    assert_ne!(tree_add_node(&mut tree, 2, 1, Some("duplicate")), 0);
    assert_eq!(tree_size(&tree), 2);

    tree_delete(tree);
    test_pass!("test_tree_duplicate_id");
}

fn test_tree_max_children() {
    println!("\n=== Testing Tree Max Children ===");

    let mut tree = tree_create();

    assert_eq!(tree_add_node(&mut tree, 1, 0, Some("root")), 0);

    for i in 0..MAX_CHILDREN {
        assert_eq!(
            tree_add_node(&mut tree, (i + 2) as TreeId, 1, Some("child")),
            0
        );
    }

    assert_ne!(
        tree_add_node(&mut tree, (MAX_CHILDREN + 2) as TreeId, 1, Some("overflow")),
        0
    );
    assert_eq!(tree_size(&tree), MAX_CHILDREN + 1);

    tree_delete(tree);
    test_pass!("test_tree_max_children");
}

fn test_tree_complex_structure() {
    println!("\n=== Testing Tree Complex Structure ===");

    let mut tree = tree_create();

    assert_eq!(tree_add_node(&mut tree, 1, 0, Some("root")), 0);
    assert_eq!(tree_add_node(&mut tree, 2, 1, Some("child1")), 0);
    assert_eq!(tree_add_node(&mut tree, 3, 1, Some("child2")), 0);
    assert_eq!(tree_add_node(&mut tree, 4, 1, Some("child3")), 0);
    assert_eq!(tree_add_node(&mut tree, 5, 2, Some("gc1")), 0);
    assert_eq!(tree_add_node(&mut tree, 6, 2, Some("gc2")), 0);
    assert_eq!(tree_add_node(&mut tree, 7, 3, Some("gc3")), 0);
    assert_eq!(tree_add_node(&mut tree, 8, 4, Some("gc4")), 0);
    assert_eq!(tree_add_node(&mut tree, 9, 4, Some("gc5")), 0);
    assert_eq!(tree_add_node(&mut tree, 10, 7, Some("ggc1")), 0);

    assert_eq!(tree_size(&tree), 10);
    assert_eq!(tree_get_height(&tree, 1), 3);
    assert_eq!(tree_count_descendants(&tree, 1), 9);
    assert_eq!(tree_count_descendants(&tree, 2), 2);
    assert_eq!(tree_count_descendants(&tree, 7), 1);

    tree_print(&tree);

    tree_delete(tree);
    test_pass!("test_tree_complex_structure");
}

fn main() {
    println!("\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}");
    println!("\u{2551}  TREE WITH HASHMAP ID MAPPING TESTS   \u{2551}");
    println!("\u{255A}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D}");

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
