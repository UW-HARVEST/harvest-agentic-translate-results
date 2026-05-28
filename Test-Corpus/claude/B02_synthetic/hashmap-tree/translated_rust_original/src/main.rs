// Translated from C source
mod hashmap;
mod tree;

use tree::{Tree, MAX_CHILDREN};

fn test_pass(name: &str) {
    println!("✓ PASS: {}", name);
}

fn test_hashmap_basic() {
    println!();
    println!("=== Testing Hashmap Basic Operations ===");

    let mut map = hashmap::Hashmap::<i32>::new();
    assert!(map.size() == 0);

    // Test put and get
    let val1: i32 = 42;
    let val2: i32 = 100;
    let val3: i32 = 200;
    assert!(map.put(1, val1) == 0);
    assert!(map.put(2, val2) == 0);
    assert!(map.put(3, val3) == 0);
    assert!(map.size() == 3);

    assert!(*map.get(1).unwrap() == 42);
    assert!(*map.get(2).unwrap() == 100);
    assert!(*map.get(3).unwrap() == 200);

    // Test update
    let val4: i32 = 500;
    assert!(map.put(1, val4) == 0);
    assert!(map.size() == 3);
    assert!(*map.get(1).unwrap() == 500);

    // Test remove
    let removed = map.remove(2);
    assert!(removed == Some(val2));
    assert!(map.size() == 2);
    assert!(map.get(2).is_none());

    // Test contains
    assert!(map.contains(1));
    assert!(!map.contains(2));
    assert!(map.contains(3));

    test_pass("test_hashmap_basic");
}

fn test_hashmap_collisions() {
    println!();
    println!("=== Testing Hashmap Collisions ===");

    let mut map = hashmap::Hashmap::<i32>::new();

    // Add many items to force collisions
    for i in 0..100i32 {
        let value = i * 10;
        assert!(map.put(i as u64, value) == 0);
    }

    assert!(map.size() == 100);

    // Verify all values
    for i in 0..100i32 {
        let val = map.get(i as u64);
        assert!(val.is_some());
        assert!(*val.unwrap() == i * 10);
    }

    test_pass("test_hashmap_collisions");
}

fn test_tree_creation() {
    println!();
    println!("=== Testing Tree Creation ===");

    let tree = Tree::new();
    assert!(tree.size() == 0);
    assert!(!tree.has_root);

    test_pass("test_tree_creation");
}

fn test_tree_add_root() {
    println!();
    println!("=== Testing Tree Add Root ===");

    let mut tree = Tree::new();

    // Add root node
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.size() == 1);
    assert!(tree.has_root);
    assert!(tree.root_id == 1);

    let root = tree.get_node(1);
    assert!(root.is_some());
    let root = root.unwrap();
    assert!(root.id == 1);
    assert!(root.data_str() == "root");
    assert!(root.child_count == 0);

    test_pass("test_tree_add_root");
}

fn test_tree_add_children() {
    println!();
    println!("=== Testing Tree Add Children ===");

    let mut tree = Tree::new();

    // Build tree structure
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

    test_pass("test_tree_add_children");
}

fn test_tree_deep_hierarchy() {
    println!();
    println!("=== Testing Tree Deep Hierarchy ===");

    let mut tree = Tree::new();

    // Build deep tree
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

    test_pass("test_tree_deep_hierarchy");
}

fn test_tree_remove_leaf() {
    println!();
    println!("=== Testing Tree Remove Leaf ===");

    let mut tree = Tree::new();

    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 1, "child2") == 0);

    assert!(tree.size() == 3);

    // Remove leaf
    assert!(tree.remove_node(3) == 0);
    assert!(tree.size() == 2);
    assert!(!tree.contains(3));

    let root = tree.get_node(1).unwrap();
    assert!(root.child_count == 1);
    assert!(root.child_ids[0] == 2);

    test_pass("test_tree_remove_leaf");
}

fn test_tree_remove_subtree() {
    println!();
    println!("=== Testing Tree Remove Subtree ===");

    let mut tree = Tree::new();

    // Build tree with subtree
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 2, "grandchild1") == 0);
    assert!(tree.add_node(4, 2, "grandchild2") == 0);
    assert!(tree.add_node(5, 1, "child2") == 0);

    assert!(tree.size() == 5);

    // Remove node 2 and its children
    assert!(tree.remove_node(2) == 0);
    assert!(tree.size() == 2);
    assert!(!tree.contains(2));
    assert!(!tree.contains(3));
    assert!(!tree.contains(4));
    assert!(tree.contains(1));
    assert!(tree.contains(5));

    test_pass("test_tree_remove_subtree");
}

fn test_tree_remove_root() {
    println!();
    println!("=== Testing Tree Remove Root ===");

    let mut tree = Tree::new();

    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 1, "child2") == 0);

    assert!(tree.size() == 3);

    // Remove root
    assert!(tree.remove_node(1) == 0);
    assert!(tree.size() == 0);
    assert!(!tree.has_root);

    test_pass("test_tree_remove_root");
}

fn test_tree_count_descendants() {
    println!();
    println!("=== Testing Tree Count Descendants ===");

    let mut tree = Tree::new();

    /*
     * Build tree:
     *        1
     *       / \
     *      2   5
     *     / \
     *    3   4
     */
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 2, "grandchild1") == 0);
    assert!(tree.add_node(4, 2, "grandchild2") == 0);
    assert!(tree.add_node(5, 1, "child2") == 0);

    assert!(tree.count_descendants(1) == 4);
    assert!(tree.count_descendants(2) == 2);
    assert!(tree.count_descendants(3) == 0);
    assert!(tree.count_descendants(5) == 0);

    test_pass("test_tree_count_descendants");
}

fn test_tree_find_path() {
    println!();
    println!("=== Testing Tree Find Path ===");

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

    test_pass("test_tree_find_path");
}

fn test_tree_duplicate_id() {
    println!();
    println!("=== Testing Tree Duplicate ID ===");

    let mut tree = Tree::new();

    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child") == 0);

    // Try to add duplicate
    assert!(tree.add_node(2, 1, "duplicate") != 0);
    assert!(tree.size() == 2);

    test_pass("test_tree_duplicate_id");
}

fn test_tree_max_children() {
    println!();
    println!("=== Testing Tree Max Children ===");

    let mut tree = Tree::new();

    assert!(tree.add_node(1, 0, "root") == 0);

    // Add MAX_CHILDREN children
    for i in 0..MAX_CHILDREN {
        assert!(tree.add_node((i + 2) as u64, 1, "child") == 0);
    }

    // Try to add one more (should fail)
    assert!(tree.add_node((MAX_CHILDREN + 2) as u64, 1, "overflow") != 0);
    assert!(tree.size() == MAX_CHILDREN + 1);

    test_pass("test_tree_max_children");
}

fn test_tree_complex_structure() {
    println!();
    println!("=== Testing Tree Complex Structure ===");

    let mut tree = Tree::new();

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

    test_pass("test_tree_complex_structure");
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║  TREE WITH HASHMAP ID MAPPING TESTS   ║");
    println!("╚════════════════════════════════════════╝");

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

    println!();
    println!("========================================");
    println!("  All tests passed successfully!");
    println!("========================================");
}
