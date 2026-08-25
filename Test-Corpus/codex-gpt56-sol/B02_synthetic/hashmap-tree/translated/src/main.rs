mod hashmap;
mod tree;

use std::io::{self, Write};

use hashmap::HashMap;
use tree::{Tree, MAX_CHILDREN};

fn heading(output: &mut Vec<u8>, text: &str) {
    writeln!(output, "\n=== {text} ===").unwrap();
}

fn pass(output: &mut Vec<u8>, function_name: &str) {
    writeln!(output, "\u{2713} PASS: {function_name}").unwrap();
}

fn test_hashmap_basic(output: &mut Vec<u8>) {
    heading(output, "Testing Hashmap Basic Operations");

    let val1 = 42;
    let val2 = 100;
    let val3 = 200;
    let val4 = 500;
    let mut map = HashMap::new();
    assert_eq!(map.len(), 0);

    map.put(1, &val1);
    map.put(2, &val2);
    map.put(3, &val3);
    assert_eq!(map.len(), 3);
    assert_eq!(**map.get(1).unwrap(), 42);
    assert_eq!(**map.get(2).unwrap(), 100);
    assert_eq!(**map.get(3).unwrap(), 200);

    map.put(1, &val4);
    assert_eq!(map.len(), 3);
    assert_eq!(**map.get(1).unwrap(), 500);

    let removed = map.remove(2).unwrap();
    assert!(std::ptr::eq(removed, &val2));
    assert_eq!(map.len(), 2);
    assert!(map.get(2).is_none());
    assert!(map.contains(1));
    assert!(!map.contains(2));
    assert!(map.contains(3));

    pass(output, "test_hashmap_basic");
}

fn test_hashmap_collisions(output: &mut Vec<u8>) {
    heading(output, "Testing Hashmap Collisions");

    let values: Vec<i32> = (0..100).map(|value| value * 10).collect();
    let mut map = HashMap::new();
    for (index, value) in values.iter().enumerate() {
        map.put(index as u64, value);
    }
    assert_eq!(map.len(), 100);

    for index in 0..100 {
        let value = map.get(index as u64).unwrap();
        assert_eq!(**value, index as i32 * 10);
    }

    pass(output, "test_hashmap_collisions");
}

fn test_tree_creation(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Creation");

    let tree = Tree::new();
    assert_eq!(tree.len(), 0);
    assert!(!tree.has_root);

    pass(output, "test_tree_creation");
}

fn test_tree_add_root(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Add Root");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, Some(b"root")).is_ok());
    assert_eq!(tree.len(), 1);
    assert!(tree.has_root);
    assert_eq!(tree.root_id, 1);

    let root = tree.get_node(1).unwrap();
    assert_eq!(root.id, 1);
    assert_eq!(root.data, b"root");
    assert_eq!(root.child_ids.len(), 0);

    pass(output, "test_tree_add_root");
}

fn test_tree_add_children(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Add Children");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, Some(b"root")).is_ok());
    assert!(tree.add_node(2, 1, Some(b"child1")).is_ok());
    assert!(tree.add_node(3, 1, Some(b"child2")).is_ok());
    assert!(tree.add_node(4, 1, Some(b"child3")).is_ok());
    assert_eq!(tree.len(), 4);

    let root = tree.get_node(1).unwrap();
    assert_eq!(root.child_ids.len(), 3);
    assert_eq!(root.child_ids[0], 2);
    assert_eq!(root.child_ids[1], 3);
    assert_eq!(root.child_ids[2], 4);

    pass(output, "test_tree_add_children");
}

fn test_tree_deep_hierarchy(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Deep Hierarchy");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, Some(b"level0")).is_ok());
    assert!(tree.add_node(2, 1, Some(b"level1")).is_ok());
    assert!(tree.add_node(3, 2, Some(b"level2")).is_ok());
    assert!(tree.add_node(4, 3, Some(b"level3")).is_ok());
    assert!(tree.add_node(5, 4, Some(b"level4")).is_ok());
    assert_eq!(tree.len(), 5);

    assert_eq!(tree.depth(1), 0);
    assert_eq!(tree.depth(2), 1);
    assert_eq!(tree.depth(3), 2);
    assert_eq!(tree.depth(4), 3);
    assert_eq!(tree.depth(5), 4);
    assert_eq!(tree.height(1), 4);
    assert_eq!(tree.height(2), 3);
    assert_eq!(tree.height(5), 0);

    pass(output, "test_tree_deep_hierarchy");
}

fn test_tree_remove_leaf(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Remove Leaf");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, Some(b"root")).is_ok());
    assert!(tree.add_node(2, 1, Some(b"child1")).is_ok());
    assert!(tree.add_node(3, 1, Some(b"child2")).is_ok());
    assert_eq!(tree.len(), 3);

    assert!(tree.remove_node(3).is_ok());
    assert_eq!(tree.len(), 2);
    assert!(!tree.contains(3));
    let root = tree.get_node(1).unwrap();
    assert_eq!(root.child_ids.len(), 1);
    assert_eq!(root.child_ids[0], 2);

    pass(output, "test_tree_remove_leaf");
}

fn test_tree_remove_subtree(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Remove Subtree");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, Some(b"root")).is_ok());
    assert!(tree.add_node(2, 1, Some(b"child1")).is_ok());
    assert!(tree.add_node(3, 2, Some(b"grandchild1")).is_ok());
    assert!(tree.add_node(4, 2, Some(b"grandchild2")).is_ok());
    assert!(tree.add_node(5, 1, Some(b"child2")).is_ok());
    assert_eq!(tree.len(), 5);

    assert!(tree.remove_node(2).is_ok());
    assert_eq!(tree.len(), 2);
    assert!(!tree.contains(2));
    assert!(!tree.contains(3));
    assert!(!tree.contains(4));
    assert!(tree.contains(1));
    assert!(tree.contains(5));

    pass(output, "test_tree_remove_subtree");
}

fn test_tree_remove_root(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Remove Root");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, Some(b"root")).is_ok());
    assert!(tree.add_node(2, 1, Some(b"child1")).is_ok());
    assert!(tree.add_node(3, 1, Some(b"child2")).is_ok());
    assert_eq!(tree.len(), 3);

    assert!(tree.remove_node(1).is_ok());
    assert_eq!(tree.len(), 0);
    assert!(!tree.has_root);

    pass(output, "test_tree_remove_root");
}

fn test_tree_count_descendants(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Count Descendants");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, Some(b"root")).is_ok());
    assert!(tree.add_node(2, 1, Some(b"child1")).is_ok());
    assert!(tree.add_node(3, 2, Some(b"grandchild1")).is_ok());
    assert!(tree.add_node(4, 2, Some(b"grandchild2")).is_ok());
    assert!(tree.add_node(5, 1, Some(b"child2")).is_ok());

    assert_eq!(tree.count_descendants(1), 4);
    assert_eq!(tree.count_descendants(2), 2);
    assert_eq!(tree.count_descendants(3), 0);
    assert_eq!(tree.count_descendants(5), 0);

    pass(output, "test_tree_count_descendants");
}

fn test_tree_find_path(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Find Path");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, Some(b"root")).is_ok());
    assert!(tree.add_node(2, 1, Some(b"child")).is_ok());
    assert!(tree.add_node(3, 2, Some(b"grandchild")).is_ok());

    let mut path = [0_u64; 10];
    let mut length = tree.find_path(3, &mut path, 10);
    assert_eq!(length, 3);
    assert_eq!(path[0], 1);
    assert_eq!(path[1], 2);
    assert_eq!(path[2], 3);

    length = tree.find_path(1, &mut path, 10);
    assert_eq!(length, 1);
    assert_eq!(path[0], 1);

    pass(output, "test_tree_find_path");
}

fn test_tree_duplicate_id(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Duplicate ID");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, Some(b"root")).is_ok());
    assert!(tree.add_node(2, 1, Some(b"child")).is_ok());
    assert!(tree.add_node(2, 1, Some(b"duplicate")).is_err());
    assert_eq!(tree.len(), 2);

    pass(output, "test_tree_duplicate_id");
}

fn test_tree_max_children(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Max Children");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, Some(b"root")).is_ok());
    for index in 0..MAX_CHILDREN {
        assert!(tree
            .add_node(index as u64 + 2, 1, Some(b"child"))
            .is_ok());
    }
    assert!(tree
        .add_node(MAX_CHILDREN as u64 + 2, 1, Some(b"overflow"))
        .is_err());
    assert_eq!(tree.len(), MAX_CHILDREN + 1);

    pass(output, "test_tree_max_children");
}

fn test_tree_complex_structure(output: &mut Vec<u8>) {
    heading(output, "Testing Tree Complex Structure");

    let mut tree = Tree::new();
    assert!(tree.add_node(1, 0, Some(b"root")).is_ok());
    assert!(tree.add_node(2, 1, Some(b"child1")).is_ok());
    assert!(tree.add_node(3, 1, Some(b"child2")).is_ok());
    assert!(tree.add_node(4, 1, Some(b"child3")).is_ok());
    assert!(tree.add_node(5, 2, Some(b"gc1")).is_ok());
    assert!(tree.add_node(6, 2, Some(b"gc2")).is_ok());
    assert!(tree.add_node(7, 3, Some(b"gc3")).is_ok());
    assert!(tree.add_node(8, 4, Some(b"gc4")).is_ok());
    assert!(tree.add_node(9, 4, Some(b"gc5")).is_ok());
    assert!(tree.add_node(10, 7, Some(b"ggc1")).is_ok());

    assert_eq!(tree.len(), 10);
    assert_eq!(tree.height(1), 3);
    assert_eq!(tree.count_descendants(1), 9);
    assert_eq!(tree.count_descendants(2), 2);
    assert_eq!(tree.count_descendants(7), 1);
    tree.print(output).unwrap();

    pass(output, "test_tree_complex_structure");
}

fn main() {
    let mut output = Vec::new();
    writeln!(
        output,
        "\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}"
    )
    .unwrap();
    writeln!(
        output,
        "\u{2551}  TREE WITH HASHMAP ID MAPPING TESTS   \u{2551}"
    )
    .unwrap();
    writeln!(
        output,
        "\u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}"
    )
    .unwrap();

    test_hashmap_basic(&mut output);
    test_hashmap_collisions(&mut output);
    test_tree_creation(&mut output);
    test_tree_add_root(&mut output);
    test_tree_add_children(&mut output);
    test_tree_deep_hierarchy(&mut output);
    test_tree_complex_structure(&mut output);
    test_tree_remove_leaf(&mut output);
    test_tree_remove_subtree(&mut output);
    test_tree_remove_root(&mut output);
    test_tree_count_descendants(&mut output);
    test_tree_find_path(&mut output);
    test_tree_duplicate_id(&mut output);
    test_tree_max_children(&mut output);

    writeln!(output).unwrap();
    writeln!(output, "========================================").unwrap();
    writeln!(output, "  All tests passed successfully!").unwrap();
    writeln!(output, "========================================").unwrap();

    let mut stdout = io::stdout().lock();
    stdout.write_all(&output).unwrap();
}
