// main.rs
//
// Faithful translation of c_src/src/main.c.
//
// The C tests store `&int` in the hashmap; here the hashmap payload is an
// index into a small `Vec<i32>` arena, so pointer identity comparisons become
// index comparisons.

#[macro_use]
mod cstdio;
mod hashmap;
mod tree;

use hashmap::Hashmap;
use tree::{Tree, MAX_CHILDREN};

/// `#define TEST_PASS printf("✓ PASS: %s\n", __func__)`
macro_rules! test_pass {
    ($name:expr) => {
        c_println!("\u{2713} PASS: {}", $name)
    };
}

fn test_hashmap_basic() {
    c_println!("\n=== Testing Hashmap Basic Operations ===");

    let mut map = Hashmap::create();
    assert!(map.size() == 0);

    // Test put and get. `ints` stands in for the addressable locals
    // val1, val2, val3, val4 (indices 0..=3).
    let mut ints: Vec<i32> = vec![42, 100, 200, 500];
    let (val1, val2, val3, val4) = (0usize, 1usize, 2usize, 3usize);

    assert!(map.put(1, Some(val1)) == 0);
    assert!(map.put(2, Some(val2)) == 0);
    assert!(map.put(3, Some(val3)) == 0);
    assert!(map.size() == 3);

    assert!(ints[map.get(1).unwrap()] == 42);
    assert!(ints[map.get(2).unwrap()] == 100);
    assert!(ints[map.get(3).unwrap()] == 200);

    // Test update
    ints[val4] = 500;
    assert!(map.put(1, Some(val4)) == 0);
    assert!(map.size() == 3);
    assert!(ints[map.get(1).unwrap()] == 500);

    // Test remove
    let removed = map.remove(2);
    assert!(removed == Some(val2));
    assert!(map.size() == 2);
    assert!(map.get(2).is_none());

    // Test contains
    assert!(map.contains(1) == 1);
    assert!(map.contains(2) == 0);
    assert!(map.contains(3) == 1);

    test_pass!("test_hashmap_basic");
}

fn test_hashmap_collisions() {
    c_println!("\n=== Testing Hashmap Collisions ===");

    let mut map = Hashmap::create();

    // Add many items to force collisions
    let mut values = [0i32; 100];
    for i in 0..100usize {
        values[i] = (i as i32) * 10;
        assert!(map.put(i as u64, Some(i)) == 0);
    }

    assert!(map.size() == 100);

    // Verify all values
    for i in 0..100usize {
        let slot = map.get(i as u64);
        assert!(slot.is_some());
        assert!(values[slot.unwrap()] == (i as i32) * 10);
    }

    test_pass!("test_hashmap_collisions");
}

fn test_tree_creation() {
    c_println!("\n=== Testing Tree Creation ===");

    let tree = Tree::create();
    assert!(tree.size() == 0);
    assert!(tree.has_root == 0);

    tree.delete();
    test_pass!("test_tree_creation");
}

fn test_tree_add_root() {
    c_println!("\n=== Testing Tree Add Root ===");

    let mut tree = Tree::create();

    // Add root node
    assert!(tree.add_node(1, 0, Some(b"root")) == 0);
    assert!(tree.size() == 1);
    assert!(tree.has_root == 1);
    assert!(tree.root_id == 1);

    let root = tree.node(1);
    assert!(root.is_some());
    let root = root.unwrap();
    assert!(root.id == 1);
    assert!(root.data_cstr() == b"root");
    assert!(root.child_count == 0);

    tree.delete();
    test_pass!("test_tree_add_root");
}

fn test_tree_add_children() {
    c_println!("\n=== Testing Tree Add Children ===");

    let mut tree = Tree::create();

    // Build tree structure
    assert!(tree.add_node(1, 0, Some(b"root")) == 0);
    assert!(tree.add_node(2, 1, Some(b"child1")) == 0);
    assert!(tree.add_node(3, 1, Some(b"child2")) == 0);
    assert!(tree.add_node(4, 1, Some(b"child3")) == 0);

    assert!(tree.size() == 4);

    let root = tree.node(1).unwrap();
    assert!(root.child_count == 3);
    assert!(root.child_ids[0] == 2);
    assert!(root.child_ids[1] == 3);
    assert!(root.child_ids[2] == 4);

    tree.delete();
    test_pass!("test_tree_add_children");
}

fn test_tree_deep_hierarchy() {
    c_println!("\n=== Testing Tree Deep Hierarchy ===");

    let mut tree = Tree::create();

    // Build deep tree
    assert!(tree.add_node(1, 0, Some(b"level0")) == 0);
    assert!(tree.add_node(2, 1, Some(b"level1")) == 0);
    assert!(tree.add_node(3, 2, Some(b"level2")) == 0);
    assert!(tree.add_node(4, 3, Some(b"level3")) == 0);
    assert!(tree.add_node(5, 4, Some(b"level4")) == 0);

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
    c_println!("\n=== Testing Tree Remove Leaf ===");

    let mut tree = Tree::create();

    assert!(tree.add_node(1, 0, Some(b"root")) == 0);
    assert!(tree.add_node(2, 1, Some(b"child1")) == 0);
    assert!(tree.add_node(3, 1, Some(b"child2")) == 0);

    assert!(tree.size() == 3);

    // Remove leaf
    assert!(tree.remove_node(3) == 0);
    assert!(tree.size() == 2);
    assert!(tree.contains(3) == 0);

    let root = tree.node(1).unwrap();
    assert!(root.child_count == 1);
    assert!(root.child_ids[0] == 2);

    tree.delete();
    test_pass!("test_tree_remove_leaf");
}

fn test_tree_remove_subtree() {
    c_println!("\n=== Testing Tree Remove Subtree ===");

    let mut tree = Tree::create();

    // Build tree with subtree
    assert!(tree.add_node(1, 0, Some(b"root")) == 0);
    assert!(tree.add_node(2, 1, Some(b"child1")) == 0);
    assert!(tree.add_node(3, 2, Some(b"grandchild1")) == 0);
    assert!(tree.add_node(4, 2, Some(b"grandchild2")) == 0);
    assert!(tree.add_node(5, 1, Some(b"child2")) == 0);

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
    c_println!("\n=== Testing Tree Remove Root ===");

    let mut tree = Tree::create();

    assert!(tree.add_node(1, 0, Some(b"root")) == 0);
    assert!(tree.add_node(2, 1, Some(b"child1")) == 0);
    assert!(tree.add_node(3, 1, Some(b"child2")) == 0);

    assert!(tree.size() == 3);

    // Remove root
    assert!(tree.remove_node(1) == 0);
    assert!(tree.size() == 0);
    assert!(tree.has_root == 0);

    tree.delete();
    test_pass!("test_tree_remove_root");
}

fn test_tree_count_descendants() {
    c_println!("\n=== Testing Tree Count Descendants ===");

    let mut tree = Tree::create();

    /*
     * Build tree:
     *        1
     *       / \
     *      2   5
     *     / \
     *    3   4
     */
    assert!(tree.add_node(1, 0, Some(b"root")) == 0);
    assert!(tree.add_node(2, 1, Some(b"child1")) == 0);
    assert!(tree.add_node(3, 2, Some(b"grandchild1")) == 0);
    assert!(tree.add_node(4, 2, Some(b"grandchild2")) == 0);
    assert!(tree.add_node(5, 1, Some(b"child2")) == 0);

    assert!(tree.count_descendants(1) == 4);
    assert!(tree.count_descendants(2) == 2);
    assert!(tree.count_descendants(3) == 0);
    assert!(tree.count_descendants(5) == 0);

    tree.delete();
    test_pass!("test_tree_count_descendants");
}

fn test_tree_find_path() {
    c_println!("\n=== Testing Tree Find Path ===");

    let mut tree = Tree::create();

    assert!(tree.add_node(1, 0, Some(b"root")) == 0);
    assert!(tree.add_node(2, 1, Some(b"child")) == 0);
    assert!(tree.add_node(3, 2, Some(b"grandchild")) == 0);

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
    c_println!("\n=== Testing Tree Duplicate ID ===");

    let mut tree = Tree::create();

    assert!(tree.add_node(1, 0, Some(b"root")) == 0);
    assert!(tree.add_node(2, 1, Some(b"child")) == 0);

    // Try to add duplicate
    assert!(tree.add_node(2, 1, Some(b"duplicate")) != 0);
    assert!(tree.size() == 2);

    tree.delete();
    test_pass!("test_tree_duplicate_id");
}

fn test_tree_max_children() {
    c_println!("\n=== Testing Tree Max Children ===");

    let mut tree = Tree::create();

    assert!(tree.add_node(1, 0, Some(b"root")) == 0);

    // Add MAX_CHILDREN children
    for i in 0..MAX_CHILDREN as u64 {
        assert!(tree.add_node(i + 2, 1, Some(b"child")) == 0);
    }

    // Try to add one more (should fail)
    assert!(tree.add_node(MAX_CHILDREN as u64 + 2, 1, Some(b"overflow")) != 0);
    assert!(tree.size() == MAX_CHILDREN + 1);

    tree.delete();
    test_pass!("test_tree_max_children");
}

fn test_tree_complex_structure() {
    c_println!("\n=== Testing Tree Complex Structure ===");

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
    assert!(tree.add_node(1, 0, Some(b"root")) == 0);
    assert!(tree.add_node(2, 1, Some(b"child1")) == 0);
    assert!(tree.add_node(3, 1, Some(b"child2")) == 0);
    assert!(tree.add_node(4, 1, Some(b"child3")) == 0);
    assert!(tree.add_node(5, 2, Some(b"gc1")) == 0);
    assert!(tree.add_node(6, 2, Some(b"gc2")) == 0);
    assert!(tree.add_node(7, 3, Some(b"gc3")) == 0);
    assert!(tree.add_node(8, 4, Some(b"gc4")) == 0);
    assert!(tree.add_node(9, 4, Some(b"gc5")) == 0);
    assert!(tree.add_node(10, 7, Some(b"ggc1")) == 0);

    assert!(tree.size() == 10);
    assert!(tree.get_height(1) == 3);
    assert!(tree.count_descendants(1) == 9);
    assert!(tree.count_descendants(2) == 2);
    assert!(tree.count_descendants(7) == 1);

    tree.print();

    tree.delete();
    test_pass!("test_tree_complex_structure");
}

/// Rust's runtime installs `SIG_IGN` for `SIGPIPE` before `main` runs; a C
/// program starts with the default disposition. Without this reset, a write to
/// a closed pipe makes the C program die from `SIGPIPE` (wait status 141) while
/// the Rust program would silently ignore the failed write and exit 0.
fn reset_sigpipe() {
    // <signal.h>: SIGPIPE == 13, SIG_DFL == 0 on Linux.
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;

    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }

    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    reset_sigpipe();

    c_println!("\u{2554}════════════════════════════════════════\u{2557}");
    c_println!("\u{2551}  TREE WITH HASHMAP ID MAPPING TESTS   \u{2551}");
    c_println!("\u{255A}════════════════════════════════════════\u{255D}");

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

    c_println!();
    c_println!("========================================");
    c_println!("  All tests passed successfully!");
    c_println!("========================================");

    // `return 0` from main: flush stdout the way exit() does.
    cstdio::flush();
    std::process::exit(0);
}
