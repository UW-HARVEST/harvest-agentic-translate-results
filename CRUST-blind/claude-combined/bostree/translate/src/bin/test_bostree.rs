use std::rc::Rc;
use Bostree::bostree::{
    bostree_next_node, bostree_node_weak_ref, bostree_previous_node, bostree_rank, BOSTree,
};
use Bostree::test_tree_sanity;

fn cmp(a: &str, b: &str) -> i32 {
    // C strcmp-style: return 0 if equal, <0 if a<b, >0 if a>b.
    a.cmp(b) as i32
}

fn build_alphabet(tree: &mut BOSTree, end: u8) {
    for c in b'A'..=end {
        let s = (c as char).to_string();
        tree.bostree_insert(s, Some("Value".to_string()));
    }
}

#[test]
fn test_new_tree_empty() {
    let tree = BOSTree::bostree_new(cmp, None);
    assert_eq!(tree.bostree_node_count(), 0);
    assert!(tree.bostree_select(0).is_none());
    assert!(tree.bostree_select(100).is_none());
    assert!(tree.bostree_lookup("anything").is_none());
    assert!(tree.root_node.is_none());
}

#[test]
fn test_insert_single() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let n = tree.bostree_insert("X".to_string(), Some("data".to_string()));
    assert_eq!(tree.bostree_node_count(), 1);
    {
        let nb = n.borrow();
        assert_eq!(nb.key, "X");
        assert_eq!(nb.data, Some("data".to_string()));
        assert_eq!(nb.depth, 0);
        assert_eq!(nb.left_child_count, 0);
        assert_eq!(nb.right_child_count, 0);
        assert!(nb.left_child_node.is_none());
        assert!(nb.right_child_node.is_none());
        assert!(nb.parent_node.is_none());
        assert_eq!(nb.weak_ref_count, 1);
        assert_eq!(nb.weak_ref_node_valid, 1);
    }
    let sel = tree.bostree_select(0).unwrap();
    assert!(Rc::ptr_eq(&sel, &n));
    let look = tree.bostree_lookup("X").unwrap();
    assert!(Rc::ptr_eq(&look, &n));
    assert_eq!(bostree_rank(&n), 0);
}

#[test]
fn test_insert_asc_3_balances() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert("A".to_string(), None);
    tree.bostree_insert("B".to_string(), None);
    tree.bostree_insert("C".to_string(), None);
    assert_eq!(tree.bostree_node_count(), 3);
    // Root should be B
    let root = tree.root_node.as_ref().unwrap().clone();
    assert_eq!(root.borrow().key, "B");
    assert_eq!(root.borrow().depth, 1);
    assert_eq!(root.borrow().left_child_count, 1);
    assert_eq!(root.borrow().right_child_count, 1);
    // Inorder
    let n0 = tree.bostree_select(0).unwrap();
    let n1 = tree.bostree_select(1).unwrap();
    let n2 = tree.bostree_select(2).unwrap();
    assert_eq!(n0.borrow().key, "A");
    assert_eq!(n1.borrow().key, "B");
    assert_eq!(n2.borrow().key, "C");
    test_tree_sanity(&tree);
}

#[test]
fn test_insert_desc_3_balances() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert("C".to_string(), None);
    tree.bostree_insert("B".to_string(), None);
    tree.bostree_insert("A".to_string(), None);
    assert_eq!(tree.bostree_node_count(), 3);
    let root = tree.root_node.as_ref().unwrap().clone();
    assert_eq!(root.borrow().key, "B");
    assert_eq!(root.borrow().depth, 1);
    test_tree_sanity(&tree);
}

#[test]
fn test_insert_left_right() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert("C".to_string(), None);
    tree.bostree_insert("A".to_string(), None);
    tree.bostree_insert("B".to_string(), None);
    let root = tree.root_node.as_ref().unwrap().clone();
    assert_eq!(root.borrow().key, "B");
    assert_eq!(root.borrow().depth, 1);
    test_tree_sanity(&tree);
}

#[test]
fn test_insert_right_left() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert("A".to_string(), None);
    tree.bostree_insert("C".to_string(), None);
    tree.bostree_insert("B".to_string(), None);
    let root = tree.root_node.as_ref().unwrap().clone();
    assert_eq!(root.borrow().key, "B");
    assert_eq!(root.borrow().depth, 1);
    test_tree_sanity(&tree);
}

#[test]
fn test_insert_alphabet_expected_layout() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    build_alphabet(&mut tree, b'Z');
    assert_eq!(tree.bostree_node_count(), 26);

    // Root should be P
    let root = tree.root_node.as_ref().unwrap().clone();
    assert_eq!(root.borrow().key, "P");

    // Compare ground truth from C: per-index key, depth, lcc, rcc
    // (idx, key, depth, lcc, rcc)
    let expected: Vec<(u32, &str, u32, u32, u32)> = vec![
        (0, "A", 0, 0, 0),
        (1, "B", 1, 1, 1),
        (2, "C", 0, 0, 0),
        (3, "D", 2, 3, 3),
        (4, "E", 0, 0, 0),
        (5, "F", 1, 1, 1),
        (6, "G", 0, 0, 0),
        (7, "H", 3, 7, 7),
        (8, "I", 0, 0, 0),
        (9, "J", 1, 1, 1),
        (10, "K", 0, 0, 0),
        (11, "L", 2, 3, 3),
        (12, "M", 0, 0, 0),
        (13, "N", 1, 1, 1),
        (14, "O", 0, 0, 0),
        (15, "P", 4, 15, 10),
        (16, "Q", 0, 0, 0),
        (17, "R", 1, 1, 1),
        (18, "S", 0, 0, 0),
        (19, "T", 3, 3, 6),
        (20, "U", 0, 0, 0),
        (21, "V", 1, 1, 1),
        (22, "W", 0, 0, 0),
        (23, "X", 2, 3, 2),
        (24, "Y", 1, 0, 1),
        (25, "Z", 0, 0, 0),
    ];
    for (idx, key, depth, lcc, rcc) in expected.iter() {
        let n = tree.bostree_select(*idx).unwrap();
        let nb = n.borrow();
        assert_eq!(nb.key, *key, "key at idx {}", idx);
        assert_eq!(nb.depth, *depth, "depth at idx {}", idx);
        assert_eq!(nb.left_child_count, *lcc, "lcc at idx {}", idx);
        assert_eq!(nb.right_child_count, *rcc, "rcc at idx {}", idx);
        assert_eq!(bostree_rank(&n), *idx);
    }
    test_tree_sanity(&tree);
}

#[test]
fn test_lookup_present_and_missing() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert("A".to_string(), None);
    tree.bostree_insert("B".to_string(), None);
    let a = tree.bostree_lookup("A").unwrap();
    assert_eq!(a.borrow().key, "A");
    let b = tree.bostree_lookup("B").unwrap();
    assert_eq!(b.borrow().key, "B");
    assert!(tree.bostree_lookup("Z").is_none());
    assert!(tree.bostree_lookup("").is_none());
}

#[test]
fn test_select_out_of_range() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert("A".to_string(), None);
    tree.bostree_insert("B".to_string(), None);
    assert!(tree.bostree_select(2).is_none());
    assert!(tree.bostree_select(100).is_none());
}

#[test]
fn test_next_and_previous_node_traversal_alphabet() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    build_alphabet(&mut tree, b'Z');
    let mut current = tree.bostree_select(0);
    let mut keys: Vec<String> = Vec::new();
    while let Some(n) = current {
        keys.push(n.borrow().key.clone());
        current = bostree_next_node(&n);
    }
    let expected: Vec<String> = (b'A'..=b'Z').map(|c| (c as char).to_string()).collect();
    assert_eq!(keys, expected);

    // backward
    let last = tree.bostree_select(25).unwrap();
    let mut current = Some(last);
    let mut bkeys: Vec<String> = Vec::new();
    while let Some(n) = current {
        bkeys.push(n.borrow().key.clone());
        current = bostree_previous_node(&n);
    }
    let mut rev: Vec<String> = expected.clone();
    rev.reverse();
    assert_eq!(bkeys, rev);
}

#[test]
fn test_rank_matches_select_for_alphabet() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    build_alphabet(&mut tree, b'Z');
    for i in 0..26u32 {
        let n = tree.bostree_select(i).unwrap();
        assert_eq!(bostree_rank(&n), i);
    }
}

#[test]
fn test_remove_g_and_h_layout() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    // Insert A..Y (25 letters; matches the C test).
    build_alphabet(&mut tree, b'Y');
    let g = tree.bostree_lookup("G").unwrap();
    tree.bostree_remove(&g);
    let h = tree.bostree_lookup("H").unwrap();
    tree.bostree_remove(&h);
    assert_eq!(tree.bostree_node_count(), 23);

    // ground truth
    let expected: Vec<(u32, &str, u32, u32, u32)> = vec![
        (0, "A", 0, 0, 0),
        (1, "B", 1, 1, 1),
        (2, "C", 0, 0, 0),
        (3, "D", 2, 3, 1),
        (4, "E", 0, 0, 0),
        (5, "F", 3, 5, 7),
        (6, "I", 0, 0, 0),
        (7, "J", 1, 1, 1),
        (8, "K", 0, 0, 0),
        (9, "L", 2, 3, 3),
        (10, "M", 0, 0, 0),
        (11, "N", 1, 1, 1),
        (12, "O", 0, 0, 0),
        (13, "P", 4, 13, 9),
        (14, "Q", 0, 0, 0),
        (15, "R", 1, 1, 1),
        (16, "S", 0, 0, 0),
        (17, "T", 3, 3, 5),
        (18, "U", 0, 0, 0),
        (19, "V", 2, 1, 3),
        (20, "W", 0, 0, 0),
        (21, "X", 1, 1, 1),
        (22, "Y", 0, 0, 0),
    ];
    for (idx, key, depth, lcc, rcc) in expected.iter() {
        let n = tree.bostree_select(*idx).unwrap();
        let nb = n.borrow();
        assert_eq!(nb.key, *key);
        assert_eq!(nb.depth, *depth);
        assert_eq!(nb.left_child_count, *lcc);
        assert_eq!(nb.right_child_count, *rcc);
    }
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "P");
    test_tree_sanity(&tree);

    // E must still be present
    assert!(tree.bostree_lookup("E").is_some());
}

#[test]
fn test_remove_each_letter_a_through_y_keeps_count() {
    // Mirrors the remove_bug.c test: build A..Y, remove each letter individually.
    for letter in b'A'..b'Z' {
        let mut tree = BOSTree::bostree_new(cmp, None);
        build_alphabet(&mut tree, b'Y');
        let key = (letter as char).to_string();
        let node = tree.bostree_lookup(&key).unwrap();
        tree.bostree_remove(&node);
        assert_eq!(tree.bostree_node_count(), 24, "after removing {}", key);
        test_tree_sanity(&tree);
        assert!(tree.bostree_lookup(&key).is_none());
    }
}

#[test]
fn test_remove_all_alphabet() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    build_alphabet(&mut tree, b'Z');
    for c in b'A'..=b'Z' {
        let key = (c as char).to_string();
        let node = tree.bostree_lookup(&key).unwrap();
        tree.bostree_remove(&node);
        test_tree_sanity(&tree);
    }
    assert_eq!(tree.bostree_node_count(), 0);
    assert!(tree.root_node.is_none());
}

#[test]
fn test_remove_a_from_alphabet_layout() {
    // Build A..Y and remove A. Verify against ground truth.
    let mut tree = BOSTree::bostree_new(cmp, None);
    build_alphabet(&mut tree, b'Y');
    let a = tree.bostree_lookup("A").unwrap();
    tree.bostree_remove(&a);
    assert_eq!(tree.bostree_node_count(), 24);

    let expected: Vec<(u32, &str, u32, u32, u32)> = vec![
        (0, "B", 1, 0, 1),
        (1, "C", 0, 0, 0),
        (2, "D", 2, 2, 3),
        (3, "E", 0, 0, 0),
        (4, "F", 1, 1, 1),
        (5, "G", 0, 0, 0),
        (6, "H", 3, 6, 7),
        (7, "I", 0, 0, 0),
        (8, "J", 1, 1, 1),
        (9, "K", 0, 0, 0),
        (10, "L", 2, 3, 3),
        (11, "M", 0, 0, 0),
        (12, "N", 1, 1, 1),
        (13, "O", 0, 0, 0),
        (14, "P", 4, 14, 9),
        (15, "Q", 0, 0, 0),
        (16, "R", 1, 1, 1),
        (17, "S", 0, 0, 0),
        (18, "T", 3, 3, 5),
        (19, "U", 0, 0, 0),
        (20, "V", 2, 1, 3),
        (21, "W", 0, 0, 0),
        (22, "X", 1, 1, 1),
        (23, "Y", 0, 0, 0),
    ];
    for (idx, key, depth, lcc, rcc) in expected.iter() {
        let n = tree.bostree_select(*idx).unwrap();
        let nb = n.borrow();
        assert_eq!(nb.key, *key);
        assert_eq!(nb.depth, *depth);
        assert_eq!(nb.left_child_count, *lcc);
        assert_eq!(nb.right_child_count, *rcc);
    }
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "P");
}

#[test]
fn test_duplicates_inserted_to_right() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert("A".to_string(), Some("d1".to_string()));
    tree.bostree_insert("A".to_string(), Some("d2".to_string()));
    tree.bostree_insert("A".to_string(), Some("d3".to_string()));
    assert_eq!(tree.bostree_node_count(), 3);
    let root = tree.root_node.as_ref().unwrap().clone();
    assert_eq!(root.borrow().key, "A");
    assert_eq!(root.borrow().depth, 1);
    let n0 = tree.bostree_select(0).unwrap();
    let n1 = tree.bostree_select(1).unwrap();
    let n2 = tree.bostree_select(2).unwrap();
    assert_eq!(n0.borrow().key, "A");
    assert_eq!(n1.borrow().key, "A");
    assert_eq!(n2.borrow().key, "A");
    // Inorder by insertion: first inserted has the smallest in-order index by
    // virtue of going through the right path. Verify data values.
    // From C ground truth: first inserted ("d1") becomes leftmost.
    assert_eq!(n0.borrow().data, Some("d1".to_string()));
    assert_eq!(n1.borrow().data, Some("d2".to_string()));
    assert_eq!(n2.borrow().data, Some("d3".to_string()));
}

#[test]
fn test_weak_ref_and_unref() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let n = tree.bostree_insert("X".to_string(), None);
    // weak_ref_count starts at 1.
    assert_eq!(n.borrow().weak_ref_count, 1);
    let r = bostree_node_weak_ref(&n);
    assert!(Rc::ptr_eq(&r, &n));
    assert_eq!(n.borrow().weak_ref_count, 2);

    // Unref while still in tree → still valid → returns Some
    let res = tree.bostree_node_weak_unref(&n);
    assert!(res.is_some());
    assert!(Rc::ptr_eq(res.as_ref().unwrap(), &n));
    assert_eq!(n.borrow().weak_ref_count, 1);
}

#[test]
fn test_remove_then_weak_unref_returns_none() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let n = tree.bostree_insert("X".to_string(), None);
    // Take an extra weak ref so the Rust Rc keeps the node alive after removal.
    let _r = bostree_node_weak_ref(&n);
    assert_eq!(n.borrow().weak_ref_count, 2);
    tree.bostree_remove(&n);
    // After remove: weak_ref_count was decremented to 1 and weak_ref_node_valid was set to 0.
    assert_eq!(n.borrow().weak_ref_count, 1);
    assert_eq!(n.borrow().weak_ref_node_valid, 0);
    // Now a further unref: count goes to 0, returns None.
    let res = tree.bostree_node_weak_unref(&n);
    assert!(res.is_none());
}

#[test]
fn test_remove_root_only_node() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let n = tree.bostree_insert("X".to_string(), None);
    tree.bostree_remove(&n);
    assert_eq!(tree.bostree_node_count(), 0);
    assert!(tree.root_node.is_none());
}

#[test]
fn test_remove_root_with_left_child_only() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let _root = tree.bostree_insert("B".to_string(), None);
    tree.bostree_insert("A".to_string(), None);
    let root = tree.root_node.as_ref().unwrap().clone();
    tree.bostree_remove(&root);
    assert_eq!(tree.bostree_node_count(), 1);
    let new_root = tree.root_node.as_ref().unwrap().clone();
    assert_eq!(new_root.borrow().key, "A");
    assert!(new_root.borrow().parent_node.is_none());
}

#[test]
fn test_remove_root_with_right_child_only() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert("A".to_string(), None);
    tree.bostree_insert("B".to_string(), None);
    let root = tree.root_node.as_ref().unwrap().clone();
    tree.bostree_remove(&root);
    assert_eq!(tree.bostree_node_count(), 1);
    let new_root = tree.root_node.as_ref().unwrap().clone();
    assert_eq!(new_root.borrow().key, "B");
    assert!(new_root.borrow().parent_node.is_none());
}

#[test]
fn test_next_node_no_successor() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let n = tree.bostree_insert("X".to_string(), None);
    assert!(bostree_next_node(&n).is_none());
    assert!(bostree_previous_node(&n).is_none());
}

#[test]
fn test_count_after_inserts_and_deletes() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    for c in b'A'..=b'F' {
        tree.bostree_insert((c as char).to_string(), None);
    }
    assert_eq!(tree.bostree_node_count(), 6);
    let d = tree.bostree_lookup("D").unwrap();
    tree.bostree_remove(&d);
    assert_eq!(tree.bostree_node_count(), 5);
    test_tree_sanity(&tree);
    let a = tree.bostree_lookup("A").unwrap();
    tree.bostree_remove(&a);
    assert_eq!(tree.bostree_node_count(), 4);
    test_tree_sanity(&tree);
}

fn main() {}
