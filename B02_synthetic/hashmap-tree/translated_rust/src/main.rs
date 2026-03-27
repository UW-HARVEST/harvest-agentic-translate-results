use hashmap_tree::{Hashmap, Tree, MAX_CHILDREN};

fn test_hashmap_basic() {
    print!("\n=== Testing Hashmap Basic Operations ===\n");

    let mut map = Hashmap::new();
    assert!(map.size() == 0);

    let mut vals: Vec<i32> = vec![42, 100, 200];
    assert!(map.put(1, 0) == 0);
    assert!(map.put(2, 1) == 0);
    assert!(map.put(3, 2) == 0);
    assert!(map.size() == 3);

    assert!(vals[map.get(1).unwrap()] == 42);
    assert!(vals[map.get(2).unwrap()] == 100);
    assert!(vals[map.get(3).unwrap()] == 200);

    vals.push(500);
    assert!(map.put(1, 3) == 0);
    assert!(map.size() == 3);
    assert!(vals[map.get(1).unwrap()] == 500);

    let removed = map.remove(2);
    assert!(removed == Some(1));
    assert!(map.size() == 2);
    assert!(map.get(2).is_none());

    assert!(map.contains(1));
    assert!(!map.contains(2));
    assert!(map.contains(3));

    print!("\u{2713} PASS: test_hashmap_basic\n");
}

fn test_hashmap_collisions() {
    print!("\n=== Testing Hashmap Collisions ===\n");

    let mut map = Hashmap::new();

    let mut values: Vec<i32> = Vec::new();
    for i in 0..100 {
        values.push(i * 10);
        assert!(map.put(i as u64, i as usize) == 0);
    }
    assert!(map.size() == 100);

    for i in 0..100 {
        let val = map.get(i as u64);
        assert!(val.is_some());
        assert!(values[val.unwrap()] == i * 10);
    }

    print!("\u{2713} PASS: test_hashmap_collisions\n");
}

fn test_tree_creation() {
    print!("\n=== Testing Tree Creation ===\n");

    let tree = Tree::new();
    assert!(tree.size() == 0);
    assert!(!tree.has_root);

    print!("\u{2713} PASS: test_tree_creation\n");
}

fn test_tree_add_root() {
    print!("\n=== Testing Tree Add Root ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.size() == 1);
    assert!(tree.has_root);
    assert!(tree.root_id == 1);

    let root = tree.get_node(1).unwrap();
    assert!(root.id == 1);
    assert!(root.data == "root");
    assert!(root.child_count == 0);

    print!("\u{2713} PASS: test_tree_add_root\n");
}

fn test_tree_add_children() {
    print!("\n=== Testing Tree Add Children ===\n");

    let mut tree = Tree::new();
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

    print!("\u{2713} PASS: test_tree_add_children\n");
}

fn test_tree_deep_hierarchy() {
    print!("\n=== Testing Tree Deep Hierarchy ===\n");

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

    print!("\u{2713} PASS: test_tree_deep_hierarchy\n");
}

fn test_tree_remove_leaf() {
    print!("\n=== Testing Tree Remove Leaf ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 1, "child2") == 0);
    assert!(tree.size() == 3);

    assert!(tree.remove_node(3) == 0);
    assert!(tree.size() == 2);
    assert!(!tree.contains(3));

    let root = tree.get_node(1).unwrap();
    assert!(root.child_count == 1);
    assert!(root.child_ids[0] == 2);

    print!("\u{2713} PASS: test_tree_remove_leaf\n");
}

fn test_tree_remove_subtree() {
    print!("\n=== Testing Tree Remove Subtree ===\n");

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

    print!("\u{2713} PASS: test_tree_remove_subtree\n");
}

fn test_tree_remove_root() {
    print!("\n=== Testing Tree Remove Root ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child1") == 0);
    assert!(tree.add_node(3, 1, "child2") == 0);
    assert!(tree.size() == 3);

    assert!(tree.remove_node(1) == 0);
    assert!(tree.size() == 0);
    assert!(!tree.has_root);

    print!("\u{2713} PASS: test_tree_remove_root\n");
}

fn test_tree_count_descendants() {
    print!("\n=== Testing Tree Count Descendants ===\n");

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

    print!("\u{2713} PASS: test_tree_count_descendants\n");
}

fn test_tree_find_path() {
    print!("\n=== Testing Tree Find Path ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child") == 0);
    assert!(tree.add_node(3, 2, "grandchild") == 0);

    let mut path = [0u64; 10];

    let length = tree.find_path(3, &mut path, 10);
    assert!(length == 3);
    assert!(path[0] == 1);
    assert!(path[1] == 2);
    assert!(path[2] == 3);

    let length = tree.find_path(1, &mut path, 10);
    assert!(length == 1);
    assert!(path[0] == 1);

    print!("\u{2713} PASS: test_tree_find_path\n");
}

fn test_tree_duplicate_id() {
    print!("\n=== Testing Tree Duplicate ID ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);
    assert!(tree.add_node(2, 1, "child") == 0);

    assert!(tree.add_node(2, 1, "duplicate") != 0);
    assert!(tree.size() == 2);

    print!("\u{2713} PASS: test_tree_duplicate_id\n");
}

fn test_tree_max_children() {
    print!("\n=== Testing Tree Max Children ===\n");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, "root") == 0);

    for i in 0..MAX_CHILDREN as u64 {
        assert!(tree.add_node(i + 2, 1, "child") == 0);
    }

    assert!(tree.add_node(MAX_CHILDREN as u64 + 2, 1, "overflow") != 0);
    assert!(tree.size() == MAX_CHILDREN + 1);

    print!("\u{2713} PASS: test_tree_max_children\n");
}

fn test_tree_complex_structure() {
    print!("\n=== Testing Tree Complex Structure ===\n");

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

    print!("\u{2713} PASS: test_tree_complex_structure\n");
}

fn main() {
    print!("\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}\n");
    print!("\u{2551}  TREE WITH HASHMAP ID MAPPING TESTS   \u{2551}\n");
    print!("\u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}\n");

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

    print!("\n");
    print!("========================================\n");
    print!("  All tests passed successfully!\n");
    print!("========================================\n");
}
