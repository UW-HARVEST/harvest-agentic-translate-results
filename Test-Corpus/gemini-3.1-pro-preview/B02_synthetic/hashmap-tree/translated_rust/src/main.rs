mod hashmap;
mod tree;

use hashmap::HashMap;
use tree::{Tree, MAX_CHILDREN};

fn test_hashmap_basic() {
    println!("\n=== Testing Hashmap Basic Operations ===");
    
    let mut map = HashMap::<i32>::new();
    assert_eq!(map.size(), 0);
    
    let val1 = 42;
    let val2 = 100;
    let val3 = 200;
    assert_eq!(map.put(1, val1), Ok(()));
    assert_eq!(map.put(2, val2), Ok(()));
    assert_eq!(map.put(3, val3), Ok(()));
    assert_eq!(map.size(), 3);
    
    assert_eq!(map.get(1), Some(&42));
    assert_eq!(map.get(2), Some(&100));
    assert_eq!(map.get(3), Some(&200));
    
    let val4 = 500;
    assert_eq!(map.put(1, val4), Ok(()));
    assert_eq!(map.size(), 3);
    assert_eq!(map.get(1), Some(&500));
    
    let removed = map.remove(2);
    assert_eq!(removed, Some(100));
    assert_eq!(map.size(), 2);
    assert_eq!(map.get(2), None);
    
    assert_eq!(map.contains(1), true);
    assert_eq!(map.contains(2), false);
    assert_eq!(map.contains(3), true);
    
    println!("✓ PASS: test_hashmap_basic");
}

fn test_hashmap_collisions() {
    println!("\n=== Testing Hashmap Collisions ===");
    
    let mut map = HashMap::<i32>::new();
    
    for i in 0..100 {
        assert_eq!(map.put(i, (i * 10) as i32), Ok(()));
    }
    
    assert_eq!(map.size(), 100);
    
    for i in 0..100 {
        let val = map.get(i);
        assert!(val.is_some());
        assert_eq!(val.unwrap(), &((i * 10) as i32));
    }
    
    println!("✓ PASS: test_hashmap_collisions");
}

fn test_tree_creation() {
    println!("\n=== Testing Tree Creation ===");
    
    let tree = Tree::new();
    assert_eq!(tree.size(), 0);
    assert_eq!(tree.has_root(), false);
    
    println!("✓ PASS: test_tree_creation");
}

fn test_tree_add_root() {
    println!("\n=== Testing Tree Add Root ===");
    
    let mut tree = Tree::new();
    
    assert_eq!(tree.add_node(1, 0, "root"), Ok(()));
    assert_eq!(tree.size(), 1);
    assert_eq!(tree.has_root(), true);
    assert_eq!(tree.root_id(), 1);
    
    let root = tree.get_node(1).unwrap();
    assert_eq!(root.id, 1);
    assert_eq!(root.data, "root");
    assert_eq!(root.child_ids.len(), 0);
    
    println!("✓ PASS: test_tree_add_root");
}

fn test_tree_add_children() {
    println!("\n=== Testing Tree Add Children ===");
    
    let mut tree = Tree::new();
    
    assert_eq!(tree.add_node(1, 0, "root"), Ok(()));
    assert_eq!(tree.add_node(2, 1, "child1"), Ok(()));
    assert_eq!(tree.add_node(3, 1, "child2"), Ok(()));
    assert_eq!(tree.add_node(4, 1, "child3"), Ok(()));
    
    assert_eq!(tree.size(), 4);
    
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
    
    assert_eq!(tree.add_node(1, 0, "level0"), Ok(()));
    assert_eq!(tree.add_node(2, 1, "level1"), Ok(()));
    assert_eq!(tree.add_node(3, 2, "level2"), Ok(()));
    assert_eq!(tree.add_node(4, 3, "level3"), Ok(()));
    assert_eq!(tree.add_node(5, 4, "level4"), Ok(()));
    
    assert_eq!(tree.size(), 5);
    
    assert_eq!(tree.get_depth(1), Some(0));
    assert_eq!(tree.get_depth(2), Some(1));
    assert_eq!(tree.get_depth(3), Some(2));
    assert_eq!(tree.get_depth(4), Some(3));
    assert_eq!(tree.get_depth(5), Some(4));
    
    assert_eq!(tree.get_height(1), Some(4));
    assert_eq!(tree.get_height(2), Some(3));
    assert_eq!(tree.get_height(5), Some(0));
    
    println!("✓ PASS: test_tree_deep_hierarchy");
}

fn test_tree_remove_leaf() {
    println!("\n=== Testing Tree Remove Leaf ===");
    
    let mut tree = Tree::new();
    
    assert_eq!(tree.add_node(1, 0, "root"), Ok(()));
    assert_eq!(tree.add_node(2, 1, "child1"), Ok(()));
    assert_eq!(tree.add_node(3, 1, "child2"), Ok(()));
    
    assert_eq!(tree.size(), 3);
    
    assert_eq!(tree.remove_node(3), Ok(()));
    assert_eq!(tree.size(), 2);
    assert_eq!(tree.contains(3), false);
    
    let root = tree.get_node(1).unwrap();
    assert_eq!(root.child_ids.len(), 1);
    assert_eq!(root.child_ids[0], 2);
    
    println!("✓ PASS: test_tree_remove_leaf");
}

fn test_tree_remove_subtree() {
    println!("\n=== Testing Tree Remove Subtree ===");
    
    let mut tree = Tree::new();
    
    assert_eq!(tree.add_node(1, 0, "root"), Ok(()));
    assert_eq!(tree.add_node(2, 1, "child1"), Ok(()));
    assert_eq!(tree.add_node(3, 2, "grandchild1"), Ok(()));
    assert_eq!(tree.add_node(4, 2, "grandchild2"), Ok(()));
    assert_eq!(tree.add_node(5, 1, "child2"), Ok(()));
    
    assert_eq!(tree.size(), 5);
    
    assert_eq!(tree.remove_node(2), Ok(()));
    assert_eq!(tree.size(), 2);
    assert_eq!(tree.contains(2), false);
    assert_eq!(tree.contains(3), false);
    assert_eq!(tree.contains(4), false);
    assert_eq!(tree.contains(1), true);
    assert_eq!(tree.contains(5), true);
    
    println!("✓ PASS: test_tree_remove_subtree");
}

fn test_tree_remove_root() {
    println!("\n=== Testing Tree Remove Root ===");
    
    let mut tree = Tree::new();
    
    assert_eq!(tree.add_node(1, 0, "root"), Ok(()));
    assert_eq!(tree.add_node(2, 1, "child1"), Ok(()));
    assert_eq!(tree.add_node(3, 1, "child2"), Ok(()));
    
    assert_eq!(tree.size(), 3);
    
    assert_eq!(tree.remove_node(1), Ok(()));
    assert_eq!(tree.size(), 0);
    assert_eq!(tree.has_root(), false);
    
    println!("✓ PASS: test_tree_remove_root");
}

fn test_tree_count_descendants() {
    println!("\n=== Testing Tree Count Descendants ===");
    
    let mut tree = Tree::new();
    
    assert_eq!(tree.add_node(1, 0, "root"), Ok(()));
    assert_eq!(tree.add_node(2, 1, "child1"), Ok(()));
    assert_eq!(tree.add_node(3, 2, "grandchild1"), Ok(()));
    assert_eq!(tree.add_node(4, 2, "grandchild2"), Ok(()));
    assert_eq!(tree.add_node(5, 1, "child2"), Ok(()));
    
    assert_eq!(tree.count_descendants(1), Some(4));
    assert_eq!(tree.count_descendants(2), Some(2));
    assert_eq!(tree.count_descendants(3), Some(0));
    assert_eq!(tree.count_descendants(5), Some(0));
    
    println!("✓ PASS: test_tree_count_descendants");
}

fn test_tree_find_path() {
    println!("\n=== Testing Tree Find Path ===");
    
    let mut tree = Tree::new();
    
    assert_eq!(tree.add_node(1, 0, "root"), Ok(()));
    assert_eq!(tree.add_node(2, 1, "child"), Ok(()));
    assert_eq!(tree.add_node(3, 2, "grandchild"), Ok(()));
    
    let mut path = [0; 10];
    
    let length = tree.find_path(3, &mut path).unwrap();
    assert_eq!(length, 3);
    assert_eq!(path[0], 1);
    assert_eq!(path[1], 2);
    assert_eq!(path[2], 3);
    
    let length = tree.find_path(1, &mut path).unwrap();
    assert_eq!(length, 1);
    assert_eq!(path[0], 1);
    
    println!("✓ PASS: test_tree_find_path");
}

fn test_tree_duplicate_id() {
    println!("\n=== Testing Tree Duplicate ID ===");
    
    let mut tree = Tree::new();
    
    assert_eq!(tree.add_node(1, 0, "root"), Ok(()));
    assert_eq!(tree.add_node(2, 1, "child"), Ok(()));
    
    assert!(tree.add_node(2, 1, "duplicate").is_err());
    assert_eq!(tree.size(), 2);
    
    println!("✓ PASS: test_tree_duplicate_id");
}

fn test_tree_max_children() {
    println!("\n=== Testing Tree Max Children ===");
    
    let mut tree = Tree::new();
    
    assert_eq!(tree.add_node(1, 0, "root"), Ok(()));
    
    for i in 0..MAX_CHILDREN {
        assert_eq!(tree.add_node((i + 2) as u64, 1, "child"), Ok(()));
    }
    
    assert!(tree.add_node((MAX_CHILDREN + 2) as u64, 1, "overflow").is_err());
    assert_eq!(tree.size(), MAX_CHILDREN + 1);
    
    println!("✓ PASS: test_tree_max_children");
}

fn test_tree_complex_structure() {
    println!("\n=== Testing Tree Complex Structure ===");
    
    let mut tree = Tree::new();
    
    assert_eq!(tree.add_node(1, 0, "root"), Ok(()));
    assert_eq!(tree.add_node(2, 1, "child1"), Ok(()));
    assert_eq!(tree.add_node(3, 1, "child2"), Ok(()));
    assert_eq!(tree.add_node(4, 1, "child3"), Ok(()));
    assert_eq!(tree.add_node(5, 2, "gc1"), Ok(()));
    assert_eq!(tree.add_node(6, 2, "gc2"), Ok(()));
    assert_eq!(tree.add_node(7, 3, "gc3"), Ok(()));
    assert_eq!(tree.add_node(8, 4, "gc4"), Ok(()));
    assert_eq!(tree.add_node(9, 4, "gc5"), Ok(()));
    assert_eq!(tree.add_node(10, 7, "ggc1"), Ok(()));
    
    assert_eq!(tree.size(), 10);
    assert_eq!(tree.get_height(1), Some(3));
    assert_eq!(tree.count_descendants(1), Some(9));
    assert_eq!(tree.count_descendants(2), Some(2));
    assert_eq!(tree.count_descendants(7), Some(1));
    
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
    
    println!("\n========================================");
    println!("  All tests passed successfully!");
    println!("========================================");
}
