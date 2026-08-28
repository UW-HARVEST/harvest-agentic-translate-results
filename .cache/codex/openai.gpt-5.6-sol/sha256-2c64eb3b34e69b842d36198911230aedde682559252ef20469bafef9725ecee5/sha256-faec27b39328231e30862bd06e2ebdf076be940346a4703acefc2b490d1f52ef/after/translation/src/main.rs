mod hashmap;
mod tree;

use hashmap::HashMap;
use tree::{Tree, MAX_CHILDREN};

fn test_hashmap_basic() {
    println!("\n=== Testing Hashmap Basic Operations ===");

    let mut map = HashMap::new();
    assert_eq!(map.len(), 0);

    assert_eq!(map.put(1, 42), 0);
    assert_eq!(map.put(2, 100), 0);
    assert_eq!(map.put(3, 200), 0);
    assert_eq!(map.len(), 3);

    assert_eq!(map.get(1), Some(&42));
    assert_eq!(map.get(2), Some(&100));
    assert_eq!(map.get(3), Some(&200));

    assert_eq!(map.put(1, 500), 0);
    assert_eq!(map.len(), 3);
    assert_eq!(map.get(1), Some(&500));

    assert_eq!(map.remove(2), Some(100));
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(2), None);

    assert!(map.contains(1));
    assert!(!map.contains(2));
    assert!(map.contains(3));

    println!("✓ PASS: test_hashmap_basic");
}

fn test_hashmap_collisions() {
    println!("\n=== Testing Hashmap Collisions ===");

    let mut map = HashMap::new();
    for i in 0..100 {
        assert_eq!(map.put(i, i * 10), 0);
    }
    assert_eq!(map.len(), 100);

    for i in 0..100 {
        assert_eq!(map.get(i), Some(&(i * 10)));
    }

    println!("✓ PASS: test_hashmap_collisions");
}

fn test_tree_creation() {
    println!("\n=== Testing Tree Creation ===");

    let tree = Tree::new();
    assert_eq!(tree.len(), 0);
    assert!(!tree.has_root);

    println!("✓ PASS: test_tree_creation");
}

fn test_tree_add_root() {
    println!("\n=== Testing Tree Add Root ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, Some(b"root")), 0);
    assert_eq!(tree.len(), 1);
    assert!(tree.has_root);
    assert_eq!(tree.root_id, 1);

    let root = tree.get_node(1).unwrap();
    assert_eq!(root.id, 1);
    assert_eq!(root.data, b"root");
    assert_eq!(root.child_ids.len(), 0);

    println!("✓ PASS: test_tree_add_root");
}

fn test_tree_add_children() {
    println!("\n=== Testing Tree Add Children ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, Some(b"root")), 0);
    assert_eq!(tree.add_node(2, 1, Some(b"child1")), 0);
    assert_eq!(tree.add_node(3, 1, Some(b"child2")), 0);
    assert_eq!(tree.add_node(4, 1, Some(b"child3")), 0);
    assert_eq!(tree.len(), 4);

    let root = tree.get_node(1).unwrap();
    assert_eq!(root.child_ids.len(), 3);
    assert_eq!(root.child_ids[0], 2);
    assert_eq!(root.child_ids[1], 3);
    assert_eq!(root.child_ids[2], 4);

    println!("✓ PASS: test_tree_add_children");
}

fn test_tree_deep_hierarchy() {
    println!("\n=== Testing Tree Deep Hierarchy ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, Some(b"level0")), 0);
    assert_eq!(tree.add_node(2, 1, Some(b"level1")), 0);
    assert_eq!(tree.add_node(3, 2, Some(b"level2")), 0);
    assert_eq!(tree.add_node(4, 3, Some(b"level3")), 0);
    assert_eq!(tree.add_node(5, 4, Some(b"level4")), 0);
    assert_eq!(tree.len(), 5);

    assert_eq!(tree.depth(1), 0);
    assert_eq!(tree.depth(2), 1);
    assert_eq!(tree.depth(3), 2);
    assert_eq!(tree.depth(4), 3);
    assert_eq!(tree.depth(5), 4);
    assert_eq!(tree.height(1), 4);
    assert_eq!(tree.height(2), 3);
    assert_eq!(tree.height(5), 0);

    println!("✓ PASS: test_tree_deep_hierarchy");
}

fn test_tree_remove_leaf() {
    println!("\n=== Testing Tree Remove Leaf ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, Some(b"root")), 0);
    assert_eq!(tree.add_node(2, 1, Some(b"child1")), 0);
    assert_eq!(tree.add_node(3, 1, Some(b"child2")), 0);
    assert_eq!(tree.len(), 3);

    assert_eq!(tree.remove_node(3), 0);
    assert_eq!(tree.len(), 2);
    assert!(!tree.contains(3));

    let root = tree.get_node(1).unwrap();
    assert_eq!(root.child_ids.len(), 1);
    assert_eq!(root.child_ids[0], 2);

    println!("✓ PASS: test_tree_remove_leaf");
}

fn test_tree_remove_subtree() {
    println!("\n=== Testing Tree Remove Subtree ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, Some(b"root")), 0);
    assert_eq!(tree.add_node(2, 1, Some(b"child1")), 0);
    assert_eq!(tree.add_node(3, 2, Some(b"grandchild1")), 0);
    assert_eq!(tree.add_node(4, 2, Some(b"grandchild2")), 0);
    assert_eq!(tree.add_node(5, 1, Some(b"child2")), 0);
    assert_eq!(tree.len(), 5);

    assert_eq!(tree.remove_node(2), 0);
    assert_eq!(tree.len(), 2);
    assert!(!tree.contains(2));
    assert!(!tree.contains(3));
    assert!(!tree.contains(4));
    assert!(tree.contains(1));
    assert!(tree.contains(5));

    println!("✓ PASS: test_tree_remove_subtree");
}

fn test_tree_remove_root() {
    println!("\n=== Testing Tree Remove Root ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, Some(b"root")), 0);
    assert_eq!(tree.add_node(2, 1, Some(b"child1")), 0);
    assert_eq!(tree.add_node(3, 1, Some(b"child2")), 0);
    assert_eq!(tree.len(), 3);

    assert_eq!(tree.remove_node(1), 0);
    assert_eq!(tree.len(), 0);
    assert!(!tree.has_root);

    println!("✓ PASS: test_tree_remove_root");
}

fn test_tree_count_descendants() {
    println!("\n=== Testing Tree Count Descendants ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, Some(b"root")), 0);
    assert_eq!(tree.add_node(2, 1, Some(b"child1")), 0);
    assert_eq!(tree.add_node(3, 2, Some(b"grandchild1")), 0);
    assert_eq!(tree.add_node(4, 2, Some(b"grandchild2")), 0);
    assert_eq!(tree.add_node(5, 1, Some(b"child2")), 0);

    assert_eq!(tree.count_descendants(1), 4);
    assert_eq!(tree.count_descendants(2), 2);
    assert_eq!(tree.count_descendants(3), 0);
    assert_eq!(tree.count_descendants(5), 0);

    println!("✓ PASS: test_tree_count_descendants");
}

fn test_tree_find_path() {
    println!("\n=== Testing Tree Find Path ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, Some(b"root")), 0);
    assert_eq!(tree.add_node(2, 1, Some(b"child")), 0);
    assert_eq!(tree.add_node(3, 2, Some(b"grandchild")), 0);

    let mut path = [0; 10];
    let mut length = tree.find_path(3, &mut path, 10);
    assert_eq!(length, 3);
    assert_eq!(path[0], 1);
    assert_eq!(path[1], 2);
    assert_eq!(path[2], 3);

    length = tree.find_path(1, &mut path, 10);
    assert_eq!(length, 1);
    assert_eq!(path[0], 1);

    println!("✓ PASS: test_tree_find_path");
}

fn test_tree_duplicate_id() {
    println!("\n=== Testing Tree Duplicate ID ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, Some(b"root")), 0);
    assert_eq!(tree.add_node(2, 1, Some(b"child")), 0);
    assert_ne!(tree.add_node(2, 1, Some(b"duplicate")), 0);
    assert_eq!(tree.len(), 2);

    println!("✓ PASS: test_tree_duplicate_id");
}

fn test_tree_max_children() {
    println!("\n=== Testing Tree Max Children ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, Some(b"root")), 0);
    for i in 0..MAX_CHILDREN {
        assert_eq!(tree.add_node((i + 2) as u64, 1, Some(b"child")), 0);
    }
    assert_ne!(
        tree.add_node((MAX_CHILDREN + 2) as u64, 1, Some(b"overflow")),
        0
    );
    assert_eq!(tree.len(), MAX_CHILDREN + 1);

    println!("✓ PASS: test_tree_max_children");
}

fn test_tree_complex_structure() {
    println!("\n=== Testing Tree Complex Structure ===");

    let mut tree = Tree::new();
    assert_eq!(tree.add_node(1, 0, Some(b"root")), 0);
    assert_eq!(tree.add_node(2, 1, Some(b"child1")), 0);
    assert_eq!(tree.add_node(3, 1, Some(b"child2")), 0);
    assert_eq!(tree.add_node(4, 1, Some(b"child3")), 0);
    assert_eq!(tree.add_node(5, 2, Some(b"gc1")), 0);
    assert_eq!(tree.add_node(6, 2, Some(b"gc2")), 0);
    assert_eq!(tree.add_node(7, 3, Some(b"gc3")), 0);
    assert_eq!(tree.add_node(8, 4, Some(b"gc4")), 0);
    assert_eq!(tree.add_node(9, 4, Some(b"gc5")), 0);
    assert_eq!(tree.add_node(10, 7, Some(b"ggc1")), 0);

    assert_eq!(tree.len(), 10);
    assert_eq!(tree.height(1), 3);
    assert_eq!(tree.count_descendants(1), 9);
    assert_eq!(tree.count_descendants(2), 2);
    assert_eq!(tree.count_descendants(7), 1);
    tree.print();

    println!("✓ PASS: test_tree_complex_structure");
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
