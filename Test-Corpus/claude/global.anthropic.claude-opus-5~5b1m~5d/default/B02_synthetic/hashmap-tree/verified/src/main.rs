//! Faithful translation of c_src/src/main.c
//!
//! The C `assert()`s in these tests are internal consistency checks; they never
//! produce output unless they fail (in which case glibc aborts the process).

#[macro_use]
mod cio;
mod hashmap;
mod tree;

use hashmap::{Hashmap, TreeId};
use tree::{Tree, MAX_CHILDREN};

macro_rules! test_pass {
    ($func:expr) => {
        c_printf!("\u{2713} PASS: {}\n", $func)
    };
}

fn test_hashmap_basic() {
    let func = "test_hashmap_basic";
    c_printf!("\n=== Testing Hashmap Basic Operations ===\n");

    // The C code stores `void *` values; here the "pointees" live in a small
    // arena and the map stores their indices.
    let mut storage: Vec<i32> = Vec::new();
    let mut map: Hashmap<usize> = Hashmap::create();
    c_assert!(map.size() == 0, func);

    // Test put and get
    let (val1, val2, val3) = (
        {
            storage.push(42);
            storage.len() - 1
        },
        {
            storage.push(100);
            storage.len() - 1
        },
        {
            storage.push(200);
            storage.len() - 1
        },
    );
    c_assert!(map.put(1, val1) == 0, func);
    c_assert!(map.put(2, val2) == 0, func);
    c_assert!(map.put(3, val3) == 0, func);
    c_assert!(map.size() == 3, func);

    c_assert!(storage[map.get(1).unwrap()] == 42, func);
    c_assert!(storage[map.get(2).unwrap()] == 100, func);
    c_assert!(storage[map.get(3).unwrap()] == 200, func);

    // Test update
    let val4 = {
        storage.push(500);
        storage.len() - 1
    };
    c_assert!(map.put(1, val4) == 0, func);
    c_assert!(map.size() == 3, func);
    c_assert!(storage[map.get(1).unwrap()] == 500, func);

    // Test remove
    let removed = map.remove(2);
    c_assert!(removed == Some(val2), func);
    c_assert!(map.size() == 2, func);
    c_assert!(map.get(2).is_none(), func);

    // Test contains
    c_assert!(map.contains(1) == 1, func);
    c_assert!(map.contains(2) == 0, func);
    c_assert!(map.contains(3) == 1, func);

    drop(map); // hashmap_destroy(map)
    test_pass!(func);
}

fn test_hashmap_collisions() {
    let func = "test_hashmap_collisions";
    c_printf!("\n=== Testing Hashmap Collisions ===\n");

    let mut map: Hashmap<usize> = Hashmap::create();

    // Add many items to force collisions
    let mut values: [i32; 100] = [0; 100];
    for i in 0..100i32 {
        values[i as usize] = i * 10;
        c_assert!(map.put(i as TreeId, i as usize) == 0, func);
    }

    c_assert!(map.size() == 100, func);

    // Verify all values
    for i in 0..100i32 {
        let val = map.get(i as TreeId);
        c_assert!(val.is_some(), func);
        c_assert!(values[val.unwrap()] == i * 10, func);
    }

    drop(map); // hashmap_destroy(map)
    test_pass!(func);
}

fn test_tree_creation() {
    let func = "test_tree_creation";
    c_printf!("\n=== Testing Tree Creation ===\n");

    let tree = Tree::create();
    c_assert!(tree.size() == 0, func);
    c_assert!(tree.has_root == 0, func);

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn test_tree_add_root() {
    let func = "test_tree_add_root";
    c_printf!("\n=== Testing Tree Add Root ===\n");

    let mut tree = Tree::create();

    // Add root node
    c_assert!(tree.add_node(1, 0, Some("root")) == 0, func);
    c_assert!(tree.size() == 1, func);
    c_assert!(tree.has_root == 1, func);
    c_assert!(tree.root_id == 1, func);

    let root = tree.get_node(1);
    c_assert!(root.is_some(), func);
    let root = tree.node(root.unwrap());
    c_assert!(root.id == 1, func);
    c_assert!(root.data_cstr() == b"root", func);
    c_assert!(root.child_count == 0, func);

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn test_tree_add_children() {
    let func = "test_tree_add_children";
    c_printf!("\n=== Testing Tree Add Children ===\n");

    let mut tree = Tree::create();

    // Build tree structure
    c_assert!(tree.add_node(1, 0, Some("root")) == 0, func);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0, func);
    c_assert!(tree.add_node(3, 1, Some("child2")) == 0, func);
    c_assert!(tree.add_node(4, 1, Some("child3")) == 0, func);

    c_assert!(tree.size() == 4, func);

    let root = tree.node(tree.get_node(1).unwrap());
    c_assert!(root.child_count == 3, func);
    c_assert!(root.child_ids[0] == 2, func);
    c_assert!(root.child_ids[1] == 3, func);
    c_assert!(root.child_ids[2] == 4, func);

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn test_tree_deep_hierarchy() {
    let func = "test_tree_deep_hierarchy";
    c_printf!("\n=== Testing Tree Deep Hierarchy ===\n");

    let mut tree = Tree::create();

    // Build deep tree
    c_assert!(tree.add_node(1, 0, Some("level0")) == 0, func);
    c_assert!(tree.add_node(2, 1, Some("level1")) == 0, func);
    c_assert!(tree.add_node(3, 2, Some("level2")) == 0, func);
    c_assert!(tree.add_node(4, 3, Some("level3")) == 0, func);
    c_assert!(tree.add_node(5, 4, Some("level4")) == 0, func);

    c_assert!(tree.size() == 5, func);

    c_assert!(tree.get_depth(1) == 0, func);
    c_assert!(tree.get_depth(2) == 1, func);
    c_assert!(tree.get_depth(3) == 2, func);
    c_assert!(tree.get_depth(4) == 3, func);
    c_assert!(tree.get_depth(5) == 4, func);

    c_assert!(tree.get_height(1) == 4, func);
    c_assert!(tree.get_height(2) == 3, func);
    c_assert!(tree.get_height(5) == 0, func);

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn test_tree_remove_leaf() {
    let func = "test_tree_remove_leaf";
    c_printf!("\n=== Testing Tree Remove Leaf ===\n");

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0, func);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0, func);
    c_assert!(tree.add_node(3, 1, Some("child2")) == 0, func);

    c_assert!(tree.size() == 3, func);

    // Remove leaf
    c_assert!(tree.remove_node(3) == 0, func);
    c_assert!(tree.size() == 2, func);
    c_assert!(tree.contains(3) == 0, func);

    let root = tree.node(tree.get_node(1).unwrap());
    c_assert!(root.child_count == 1, func);
    c_assert!(root.child_ids[0] == 2, func);

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn test_tree_remove_subtree() {
    let func = "test_tree_remove_subtree";
    c_printf!("\n=== Testing Tree Remove Subtree ===\n");

    let mut tree = Tree::create();

    // Build tree with subtree
    c_assert!(tree.add_node(1, 0, Some("root")) == 0, func);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0, func);
    c_assert!(tree.add_node(3, 2, Some("grandchild1")) == 0, func);
    c_assert!(tree.add_node(4, 2, Some("grandchild2")) == 0, func);
    c_assert!(tree.add_node(5, 1, Some("child2")) == 0, func);

    c_assert!(tree.size() == 5, func);

    // Remove node 2 and its children
    c_assert!(tree.remove_node(2) == 0, func);
    c_assert!(tree.size() == 2, func);
    c_assert!(tree.contains(2) == 0, func);
    c_assert!(tree.contains(3) == 0, func);
    c_assert!(tree.contains(4) == 0, func);
    c_assert!(tree.contains(1) == 1, func);
    c_assert!(tree.contains(5) == 1, func);

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn test_tree_remove_root() {
    let func = "test_tree_remove_root";
    c_printf!("\n=== Testing Tree Remove Root ===\n");

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0, func);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0, func);
    c_assert!(tree.add_node(3, 1, Some("child2")) == 0, func);

    c_assert!(tree.size() == 3, func);

    // Remove root
    c_assert!(tree.remove_node(1) == 0, func);
    c_assert!(tree.size() == 0, func);
    c_assert!(tree.has_root == 0, func);

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn test_tree_count_descendants() {
    let func = "test_tree_count_descendants";
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
    c_assert!(tree.add_node(1, 0, Some("root")) == 0, func);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0, func);
    c_assert!(tree.add_node(3, 2, Some("grandchild1")) == 0, func);
    c_assert!(tree.add_node(4, 2, Some("grandchild2")) == 0, func);
    c_assert!(tree.add_node(5, 1, Some("child2")) == 0, func);

    c_assert!(tree.count_descendants(1) == 4, func);
    c_assert!(tree.count_descendants(2) == 2, func);
    c_assert!(tree.count_descendants(3) == 0, func);
    c_assert!(tree.count_descendants(5) == 0, func);

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn test_tree_find_path() {
    let func = "test_tree_find_path";
    c_printf!("\n=== Testing Tree Find Path ===\n");

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0, func);
    c_assert!(tree.add_node(2, 1, Some("child")) == 0, func);
    c_assert!(tree.add_node(3, 2, Some("grandchild")) == 0, func);

    let mut path: [TreeId; 10] = [0; 10];
    let mut length: i32;

    length = tree.find_path(3, &mut path, 10);
    c_assert!(length == 3, func);
    c_assert!(path[0] == 1, func);
    c_assert!(path[1] == 2, func);
    c_assert!(path[2] == 3, func);

    length = tree.find_path(1, &mut path, 10);
    c_assert!(length == 1, func);
    c_assert!(path[0] == 1, func);

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn test_tree_duplicate_id() {
    let func = "test_tree_duplicate_id";
    c_printf!("\n=== Testing Tree Duplicate ID ===\n");

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0, func);
    c_assert!(tree.add_node(2, 1, Some("child")) == 0, func);

    // Try to add duplicate
    c_assert!(tree.add_node(2, 1, Some("duplicate")) != 0, func);
    c_assert!(tree.size() == 2, func);

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn test_tree_max_children() {
    let func = "test_tree_max_children";
    c_printf!("\n=== Testing Tree Max Children ===\n");

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0, func);

    // Add MAX_CHILDREN children
    for i in 0..MAX_CHILDREN as i32 {
        c_assert!(tree.add_node((i + 2) as TreeId, 1, Some("child")) == 0, func);
    }

    // Try to add one more (should fail)
    c_assert!(
        tree.add_node((MAX_CHILDREN + 2) as TreeId, 1, Some("overflow")) != 0,
        func
    );
    c_assert!(tree.size() == MAX_CHILDREN + 1, func);

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn test_tree_complex_structure() {
    let func = "test_tree_complex_structure";
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
    c_assert!(tree.add_node(1, 0, Some("root")) == 0, func);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0, func);
    c_assert!(tree.add_node(3, 1, Some("child2")) == 0, func);
    c_assert!(tree.add_node(4, 1, Some("child3")) == 0, func);
    c_assert!(tree.add_node(5, 2, Some("gc1")) == 0, func);
    c_assert!(tree.add_node(6, 2, Some("gc2")) == 0, func);
    c_assert!(tree.add_node(7, 3, Some("gc3")) == 0, func);
    c_assert!(tree.add_node(8, 4, Some("gc4")) == 0, func);
    c_assert!(tree.add_node(9, 4, Some("gc5")) == 0, func);
    c_assert!(tree.add_node(10, 7, Some("ggc1")) == 0, func);

    c_assert!(tree.size() == 10, func);
    c_assert!(tree.get_height(1) == 3, func);
    c_assert!(tree.count_descendants(1) == 9, func);
    c_assert!(tree.count_descendants(2) == 2, func);
    c_assert!(tree.count_descendants(7) == 1, func);

    tree.print();

    drop(tree); // tree_delete(tree)
    test_pass!(func);
}

fn main() {
    // Match the C program's signal dispositions before producing any output.
    cio::restore_default_sigpipe();

    // 40 U+2550 (box drawings double horizontal) between the corner glyphs.
    c_printf!("\u{2554}{}\u{2557}\n", "\u{2550}".repeat(40));
    c_printf!("\u{2551}  TREE WITH HASHMAP ID MAPPING TESTS   \u{2551}\n");
    c_printf!("\u{255a}{}\u{255d}\n", "\u{2550}".repeat(40));

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

    // The C runtime flushes stdout at exit.
    cio::flush();
    std::process::exit(0);
}
