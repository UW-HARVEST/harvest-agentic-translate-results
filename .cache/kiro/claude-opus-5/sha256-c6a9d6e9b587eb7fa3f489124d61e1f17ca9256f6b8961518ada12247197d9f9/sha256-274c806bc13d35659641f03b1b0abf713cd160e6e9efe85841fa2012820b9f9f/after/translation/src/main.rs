//! Translation of `c_src/src/main.c`.
//!
//! The C driver takes no input at all; it runs a fixed sequence of self-checks and
//! prints a deterministic report. `assert()` is translated to `assert!()`, which
//! likewise aborts the process on failure.

mod cout;
mod hashmap;
mod tree;

use cout::out_flush;
use hashmap::Hashmap;
use tree::{Tree, MAX_CHILDREN};

/// `#define TEST_PASS printf("✓ PASS: %s\n", __func__)`
macro_rules! test_pass {
    ($func:expr) => {
        c_printf!("✓ PASS: {}\n", $func)
    };
}

fn test_hashmap_basic() {
    c_printf!("\n=== Testing Hashmap Basic Operations ===\n");

    // `hashmap_create` cannot return NULL here, so the NULL assert is vacuous.
    let mut map: Hashmap<usize> = Hashmap::create();
    assert!(map.size() == 0);

    // The C test stores pointers to four stack ints. Arena slots stand in for
    // those addresses so that pointer-identity comparisons still work.
    let values: Vec<i32> = vec![42, 100, 200, 500];
    const VAL1: usize = 0;
    const VAL2: usize = 1;
    const VAL3: usize = 2;
    const VAL4: usize = 3;

    // Test put and get
    assert!(map.put(1, VAL1) == 0);
    assert!(map.put(2, VAL2) == 0);
    assert!(map.put(3, VAL3) == 0);
    assert!(map.size() == 3);

    assert!(values[map.get(1).unwrap()] == 42);
    assert!(values[map.get(2).unwrap()] == 100);
    assert!(values[map.get(3).unwrap()] == 200);

    // Test update
    assert!(map.put(1, VAL4) == 0);
    assert!(map.size() == 3);
    assert!(values[map.get(1).unwrap()] == 500);

    // Test remove
    let removed = map.remove(2);
    assert!(removed == Some(VAL2));
    assert!(map.size() == 2);
    assert!(map.get(2).is_none());

    // Test contains
    assert!(map.contains(1) == 1);
    assert!(map.contains(2) == 0);
    assert!(map.contains(3) == 1);

    // Keep `values` alive for the whole test, mirroring the C stack locals.
    let _ = &values;

    map.destroy();
    test_pass!("test_hashmap_basic");
}

fn test_hashmap_collisions() {
    c_printf!("\n=== Testing Hashmap Collisions ===\n");

    let mut map: Hashmap<usize> = Hashmap::create();

    // Add many items to force collisions
    let mut values = [0i32; 100];
    for i in 0..100 {
        values[i] = (i as i32) * 10;
        assert!(map.put(i as u64, i) == 0);
    }

    assert!(map.size() == 100);

    // Verify all values
    for i in 0..100 {
        let val = map.get(i as u64);
        assert!(val.is_some());
        assert!(values[val.unwrap()] == (i as i32) * 10);
    }

    map.destroy();
    test_pass!("test_hashmap_collisions");
}

fn test_tree_creation() {
    c_printf!("\n=== Testing Tree Creation ===\n");

    let tree = Tree::create();
    assert!(tree.size() == 0);
    assert!(tree.has_root == 0);

    tree.delete();
    test_pass!("test_tree_creation");
}

fn test_tree_add_root() {
    c_printf!("\n=== Testing Tree Add Root ===\n");

    let mut tree = Tree::create();

    // Add root node
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.size() == 1);
    assert!(tree.has_root == 1);
    assert!(tree.root_id == 1);

    let root = tree.get_node(1);
    assert!(root.is_some());
    let root = root.unwrap();
    assert!(root.id == 1);
    assert!(root.data_bytes() == b"root");
    assert!(root.child_count == 0);

    tree.delete();
    test_pass!("test_tree_add_root");
}

fn test_tree_add_children() {
    c_printf!("\n=== Testing Tree Add Children ===\n");

    let mut tree = Tree::create();

    // Build tree structure
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

    tree.delete();
    test_pass!("test_tree_add_children");
}

fn test_tree_deep_hierarchy() {
    c_printf!("\n=== Testing Tree Deep Hierarchy ===\n");

    let mut tree = Tree::create();

    // Build deep tree
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

    tree.delete();
    test_pass!("test_tree_deep_hierarchy");
}

fn test_tree_remove_leaf() {
    c_printf!("\n=== Testing Tree Remove Leaf ===\n");

    let mut tree = Tree::create();

    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child1")) == 0);
    assert!(tree.add_node(3, 1, Some("child2")) == 0);

    assert!(tree.size() == 3);

    // Remove leaf
    assert!(tree.remove_node(3) == 0);
    assert!(tree.size() == 2);
    assert!(tree.contains(3) == 0);

    let root = tree.get_node(1).unwrap();
    assert!(root.child_count == 1);
    assert!(root.child_ids[0] == 2);

    tree.delete();
    test_pass!("test_tree_remove_leaf");
}

fn test_tree_remove_subtree() {
    c_printf!("\n=== Testing Tree Remove Subtree ===\n");

    let mut tree = Tree::create();

    // Build tree with subtree
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child1")) == 0);
    assert!(tree.add_node(3, 2, Some("grandchild1")) == 0);
    assert!(tree.add_node(4, 2, Some("grandchild2")) == 0);
    assert!(tree.add_node(5, 1, Some("child2")) == 0);

    assert!(tree.size() == 5);

    // Remove node 2 and its children
    assert!(tree.remove_node(2) == 0);
    assert!(tree.size() == 2);
    assert!(tree.contains(2) == 0);
    assert!(tree.contains(3) == 0);
    assert!(tree.contains(4) == 0);
    assert!(tree.contains(1) == 1);
    assert!(tree.contains(5) == 1);

    tree.delete();
    test_pass!("test_tree_remove_subtree");
}

fn test_tree_remove_root() {
    c_printf!("\n=== Testing Tree Remove Root ===\n");

    let mut tree = Tree::create();

    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child1")) == 0);
    assert!(tree.add_node(3, 1, Some("child2")) == 0);

    assert!(tree.size() == 3);

    // Remove root
    assert!(tree.remove_node(1) == 0);
    assert!(tree.size() == 0);
    assert!(tree.has_root == 0);

    tree.delete();
    test_pass!("test_tree_remove_root");
}

fn test_tree_count_descendants() {
    c_printf!("\n=== Testing Tree Count Descendants ===\n");

    let mut tree = Tree::create();

    /*
     * Build tree:
     *        1
     *       / \
     *      2   5
     *     / \
     *    3   4
     */
    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child1")) == 0);
    assert!(tree.add_node(3, 2, Some("grandchild1")) == 0);
    assert!(tree.add_node(4, 2, Some("grandchild2")) == 0);
    assert!(tree.add_node(5, 1, Some("child2")) == 0);

    assert!(tree.count_descendants(1) == 4);
    assert!(tree.count_descendants(2) == 2);
    assert!(tree.count_descendants(3) == 0);
    assert!(tree.count_descendants(5) == 0);

    tree.delete();
    test_pass!("test_tree_count_descendants");
}

fn test_tree_find_path() {
    c_printf!("\n=== Testing Tree Find Path ===\n");

    let mut tree = Tree::create();

    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child")) == 0);
    assert!(tree.add_node(3, 2, Some("grandchild")) == 0);

    let mut path = [0u64; 10];
    let mut length: i32;

    length = tree.find_path(3, &mut path, 10);
    assert!(length == 3);
    assert!(path[0] == 1);
    assert!(path[1] == 2);
    assert!(path[2] == 3);

    length = tree.find_path(1, &mut path, 10);
    assert!(length == 1);
    assert!(path[0] == 1);

    tree.delete();
    test_pass!("test_tree_find_path");
}

fn test_tree_duplicate_id() {
    c_printf!("\n=== Testing Tree Duplicate ID ===\n");

    let mut tree = Tree::create();

    assert!(tree.add_node(1, 0, Some("root")) == 0);
    assert!(tree.add_node(2, 1, Some("child")) == 0);

    // Try to add duplicate
    assert!(tree.add_node(2, 1, Some("duplicate")) != 0);
    assert!(tree.size() == 2);

    tree.delete();
    test_pass!("test_tree_duplicate_id");
}

fn test_tree_max_children() {
    c_printf!("\n=== Testing Tree Max Children ===\n");

    let mut tree = Tree::create();

    assert!(tree.add_node(1, 0, Some("root")) == 0);

    // Add MAX_CHILDREN children
    for i in 0..MAX_CHILDREN {
        assert!(tree.add_node(i as u64 + 2, 1, Some("child")) == 0);
    }

    // Try to add one more (should fail)
    assert!(tree.add_node(MAX_CHILDREN as u64 + 2, 1, Some("overflow")) != 0);
    assert!(tree.size() == MAX_CHILDREN + 1);

    tree.delete();
    test_pass!("test_tree_max_children");
}

fn test_tree_complex_structure() {
    c_printf!("\n=== Testing Tree Complex Structure ===\n");

    let mut tree = Tree::create();

    /*
     * Build complex tree:
     *           1
     *        /  |  \
     *       2   3   4
     *      /|   |   |\
     *     5 6   7   8 9
     *           |
     *          10
     */
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

    tree.delete();
    test_pass!("test_tree_complex_structure");
}

fn main() {
    c_printf!("╔════════════════════════════════════════╗\n");
    c_printf!("║  TREE WITH HASHMAP ID MAPPING TESTS   ║\n");
    c_printf!("╚════════════════════════════════════════╝\n");

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

    c_printf!("\n");
    c_printf!("========================================\n");
    c_printf!("  All tests passed successfully!\n");
    c_printf!("========================================\n");

    // C flushes stdout on return from main; do the same before exiting.
    out_flush();
}
