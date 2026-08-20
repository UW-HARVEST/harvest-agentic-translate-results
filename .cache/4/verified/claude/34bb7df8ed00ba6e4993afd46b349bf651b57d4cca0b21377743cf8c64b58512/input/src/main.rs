//! Faithful translation of `c_src/src/main.c`.

#[macro_use]
mod cio;
mod hashmap;
mod tree;

use hashmap::Hashmap;
use tree::{Tree, MAX_CHILDREN};

/// Mirrors C's `assert()` from <assert.h>: on failure report to stderr and
/// abort the process.
macro_rules! cassert {
    ($cond:expr) => {
        if !$cond {
            $crate::cio::err_str(&format!(
                "driver: {}:{}: Assertion `{}' failed.\n",
                file!(),
                line!(),
                stringify!($cond)
            ));
            std::process::abort();
        }
    };
}

/// `#define TEST_PASS printf("\u{2713} PASS: %s\n", __func__)`
macro_rules! test_pass {
    ($func:expr) => {
        cprintf!("\u{2713} PASS: {}\n", $func)
    };
}

fn test_hashmap_basic() {
    cprintf!("\n=== Testing Hashmap Basic Operations ===\n");

    let mut map: Hashmap<usize> = Hashmap::create();
    cassert!(map.size() == 0);

    // Test put and get.  The C code stores `&val1`, `&val2`, `&val3`; here the
    // stored value is the index of the variable inside `vals`, which preserves
    // both the pointed-to value and pointer identity.
    let vals: [i32; 4] = [42, 100, 200, 500];
    let (val1, val2, val3, val4) = (0usize, 1usize, 2usize, 3usize);
    cassert!(map.put(1, val1) == 0);
    cassert!(map.put(2, val2) == 0);
    cassert!(map.put(3, val3) == 0);
    cassert!(map.size() == 3);

    cassert!(vals[map.get(1).unwrap()] == 42);
    cassert!(vals[map.get(2).unwrap()] == 100);
    cassert!(vals[map.get(3).unwrap()] == 200);

    // Test update
    cassert!(map.put(1, val4) == 0);
    cassert!(map.size() == 3);
    cassert!(vals[map.get(1).unwrap()] == 500);

    // Test remove
    let removed = map.remove(2);
    cassert!(removed == Some(val2));
    cassert!(map.size() == 2);
    cassert!(map.get(2).is_none());

    // Test contains
    cassert!(map.contains(1) == 1);
    cassert!(map.contains(2) == 0);
    cassert!(map.contains(3) == 1);

    drop(map);
    test_pass!("test_hashmap_basic");
}

fn test_hashmap_collisions() {
    cprintf!("\n=== Testing Hashmap Collisions ===\n");

    let mut map: Hashmap<usize> = Hashmap::create();

    // Add many items to force collisions
    let mut values = [0i32; 100];
    for i in 0..100usize {
        values[i] = (i as i32) * 10;
        cassert!(map.put(i as u64, i) == 0);
    }

    cassert!(map.size() == 100);

    // Verify all values
    for i in 0..100usize {
        let val = map.get(i as u64);
        cassert!(val.is_some());
        cassert!(values[val.unwrap()] == (i as i32) * 10);
    }

    drop(map);
    test_pass!("test_hashmap_collisions");
}

fn test_tree_creation() {
    cprintf!("\n=== Testing Tree Creation ===\n");

    let tree = Tree::create();
    cassert!(tree.size() == 0);
    cassert!(tree.has_root == 0);

    tree.delete();
    test_pass!("test_tree_creation");
}

fn test_tree_add_root() {
    cprintf!("\n=== Testing Tree Add Root ===\n");

    let mut tree = Tree::create();

    // Add root node
    cassert!(tree.add_node(1, 0, Some("root")) == 0);
    cassert!(tree.size() == 1);
    cassert!(tree.has_root == 1);
    cassert!(tree.root_id == 1);

    let root = tree.get_node(1);
    cassert!(root.is_some());
    let root = root.unwrap();
    cassert!(root.id == 1);
    cassert!(root.data_eq("root"));
    cassert!(root.child_count == 0);

    tree.delete();
    test_pass!("test_tree_add_root");
}

fn test_tree_add_children() {
    cprintf!("\n=== Testing Tree Add Children ===\n");

    let mut tree = Tree::create();

    // Build tree structure
    cassert!(tree.add_node(1, 0, Some("root")) == 0);
    cassert!(tree.add_node(2, 1, Some("child1")) == 0);
    cassert!(tree.add_node(3, 1, Some("child2")) == 0);
    cassert!(tree.add_node(4, 1, Some("child3")) == 0);

    cassert!(tree.size() == 4);

    let root_idx = tree.get_node_idx(1).unwrap();
    let root = tree.node(root_idx);
    cassert!(root.child_count == 3);
    cassert!(root.child_ids[0] == 2);
    cassert!(root.child_ids[1] == 3);
    cassert!(root.child_ids[2] == 4);

    tree.delete();
    test_pass!("test_tree_add_children");
}

fn test_tree_deep_hierarchy() {
    cprintf!("\n=== Testing Tree Deep Hierarchy ===\n");

    let mut tree = Tree::create();

    // Build deep tree
    cassert!(tree.add_node(1, 0, Some("level0")) == 0);
    cassert!(tree.add_node(2, 1, Some("level1")) == 0);
    cassert!(tree.add_node(3, 2, Some("level2")) == 0);
    cassert!(tree.add_node(4, 3, Some("level3")) == 0);
    cassert!(tree.add_node(5, 4, Some("level4")) == 0);

    cassert!(tree.size() == 5);

    cassert!(tree.get_depth(1) == 0);
    cassert!(tree.get_depth(2) == 1);
    cassert!(tree.get_depth(3) == 2);
    cassert!(tree.get_depth(4) == 3);
    cassert!(tree.get_depth(5) == 4);

    cassert!(tree.get_height(1) == 4);
    cassert!(tree.get_height(2) == 3);
    cassert!(tree.get_height(5) == 0);

    tree.delete();
    test_pass!("test_tree_deep_hierarchy");
}

fn test_tree_remove_leaf() {
    cprintf!("\n=== Testing Tree Remove Leaf ===\n");

    let mut tree = Tree::create();

    cassert!(tree.add_node(1, 0, Some("root")) == 0);
    cassert!(tree.add_node(2, 1, Some("child1")) == 0);
    cassert!(tree.add_node(3, 1, Some("child2")) == 0);

    cassert!(tree.size() == 3);

    // Remove leaf
    cassert!(tree.remove_node(3) == 0);
    cassert!(tree.size() == 2);
    cassert!(tree.contains(3) == 0);

    let root_idx = tree.get_node_idx(1).unwrap();
    let root = tree.node(root_idx);
    cassert!(root.child_count == 1);
    cassert!(root.child_ids[0] == 2);

    tree.delete();
    test_pass!("test_tree_remove_leaf");
}

fn test_tree_remove_subtree() {
    cprintf!("\n=== Testing Tree Remove Subtree ===\n");

    let mut tree = Tree::create();

    // Build tree with subtree
    cassert!(tree.add_node(1, 0, Some("root")) == 0);
    cassert!(tree.add_node(2, 1, Some("child1")) == 0);
    cassert!(tree.add_node(3, 2, Some("grandchild1")) == 0);
    cassert!(tree.add_node(4, 2, Some("grandchild2")) == 0);
    cassert!(tree.add_node(5, 1, Some("child2")) == 0);

    cassert!(tree.size() == 5);

    // Remove node 2 and its children
    cassert!(tree.remove_node(2) == 0);
    cassert!(tree.size() == 2);
    cassert!(tree.contains(2) == 0);
    cassert!(tree.contains(3) == 0);
    cassert!(tree.contains(4) == 0);
    cassert!(tree.contains(1) == 1);
    cassert!(tree.contains(5) == 1);

    tree.delete();
    test_pass!("test_tree_remove_subtree");
}

fn test_tree_remove_root() {
    cprintf!("\n=== Testing Tree Remove Root ===\n");

    let mut tree = Tree::create();

    cassert!(tree.add_node(1, 0, Some("root")) == 0);
    cassert!(tree.add_node(2, 1, Some("child1")) == 0);
    cassert!(tree.add_node(3, 1, Some("child2")) == 0);

    cassert!(tree.size() == 3);

    // Remove root
    cassert!(tree.remove_node(1) == 0);
    cassert!(tree.size() == 0);
    cassert!(tree.has_root == 0);

    tree.delete();
    test_pass!("test_tree_remove_root");
}

fn test_tree_count_descendants() {
    cprintf!("\n=== Testing Tree Count Descendants ===\n");

    let mut tree = Tree::create();

    /*
     * Build tree:
     *        1
     *       / \
     *      2   5
     *     / \
     *    3   4
     */
    cassert!(tree.add_node(1, 0, Some("root")) == 0);
    cassert!(tree.add_node(2, 1, Some("child1")) == 0);
    cassert!(tree.add_node(3, 2, Some("grandchild1")) == 0);
    cassert!(tree.add_node(4, 2, Some("grandchild2")) == 0);
    cassert!(tree.add_node(5, 1, Some("child2")) == 0);

    cassert!(tree.count_descendants(1) == 4);
    cassert!(tree.count_descendants(2) == 2);
    cassert!(tree.count_descendants(3) == 0);
    cassert!(tree.count_descendants(5) == 0);

    tree.delete();
    test_pass!("test_tree_count_descendants");
}

fn test_tree_find_path() {
    cprintf!("\n=== Testing Tree Find Path ===\n");

    let mut tree = Tree::create();

    cassert!(tree.add_node(1, 0, Some("root")) == 0);
    cassert!(tree.add_node(2, 1, Some("child")) == 0);
    cassert!(tree.add_node(3, 2, Some("grandchild")) == 0);

    let mut path = [0u64; 10];
    let mut length: i32;

    length = tree.find_path(3, &mut path, 10);
    cassert!(length == 3);
    cassert!(path[0] == 1);
    cassert!(path[1] == 2);
    cassert!(path[2] == 3);

    length = tree.find_path(1, &mut path, 10);
    cassert!(length == 1);
    cassert!(path[0] == 1);

    tree.delete();
    test_pass!("test_tree_find_path");
}

fn test_tree_duplicate_id() {
    cprintf!("\n=== Testing Tree Duplicate ID ===\n");

    let mut tree = Tree::create();

    cassert!(tree.add_node(1, 0, Some("root")) == 0);
    cassert!(tree.add_node(2, 1, Some("child")) == 0);

    // Try to add duplicate
    cassert!(tree.add_node(2, 1, Some("duplicate")) != 0);
    cassert!(tree.size() == 2);

    tree.delete();
    test_pass!("test_tree_duplicate_id");
}

fn test_tree_max_children() {
    cprintf!("\n=== Testing Tree Max Children ===\n");

    let mut tree = Tree::create();

    cassert!(tree.add_node(1, 0, Some("root")) == 0);

    // Add MAX_CHILDREN children
    for i in 0..MAX_CHILDREN as u64 {
        cassert!(tree.add_node(i + 2, 1, Some("child")) == 0);
    }

    // Try to add one more (should fail)
    cassert!(tree.add_node(MAX_CHILDREN as u64 + 2, 1, Some("overflow")) != 0);
    cassert!(tree.size() == MAX_CHILDREN + 1);

    tree.delete();
    test_pass!("test_tree_max_children");
}

fn test_tree_complex_structure() {
    cprintf!("\n=== Testing Tree Complex Structure ===\n");

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
    cassert!(tree.add_node(1, 0, Some("root")) == 0);
    cassert!(tree.add_node(2, 1, Some("child1")) == 0);
    cassert!(tree.add_node(3, 1, Some("child2")) == 0);
    cassert!(tree.add_node(4, 1, Some("child3")) == 0);
    cassert!(tree.add_node(5, 2, Some("gc1")) == 0);
    cassert!(tree.add_node(6, 2, Some("gc2")) == 0);
    cassert!(tree.add_node(7, 3, Some("gc3")) == 0);
    cassert!(tree.add_node(8, 4, Some("gc4")) == 0);
    cassert!(tree.add_node(9, 4, Some("gc5")) == 0);
    cassert!(tree.add_node(10, 7, Some("ggc1")) == 0);

    cassert!(tree.size() == 10);
    cassert!(tree.get_height(1) == 3);
    cassert!(tree.count_descendants(1) == 9);
    cassert!(tree.count_descendants(2) == 2);
    cassert!(tree.count_descendants(7) == 1);

    tree.print();

    tree.delete();
    test_pass!("test_tree_complex_structure");
}

fn main() {
    // 40 U+2550 (box drawings double horizontal) between the corner glyphs.
    let bar: String = "\u{2550}".repeat(40);
    cprintf!("\u{2554}{}\u{2557}\n", bar);
    cprintf!("\u{2551}  TREE WITH HASHMAP ID MAPPING TESTS   \u{2551}\n");
    cprintf!("\u{255A}{}\u{255D}\n", bar);

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

    cprintf!("\n");
    cprintf!("========================================\n");
    cprintf!("  All tests passed successfully!\n");
    cprintf!("========================================\n");

    cio::out_flush();
    std::process::exit(0);
}
