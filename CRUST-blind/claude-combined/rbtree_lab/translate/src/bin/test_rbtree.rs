#![allow(dead_code, unused_imports)]

use rbtree::rbtree::{Color, Node, NodeRef, RBTree};
use std::cell::RefCell;
use std::rc::Rc;

// ---------- helpers ----------

fn key(n: &NodeRef) -> i32 {
    n.borrow().key
}

fn color(n: &NodeRef) -> Color {
    n.borrow().color.clone()
}

fn left(n: &NodeRef) -> Option<NodeRef> {
    n.borrow().left.clone()
}

fn right(n: &NodeRef) -> Option<NodeRef> {
    n.borrow().right.clone()
}

fn parent(n: &NodeRef) -> Option<NodeRef> {
    n.borrow().parent.clone()
}

/// Validate: all NIL nodes are conceptually black; red nodes have black children;
/// every path from a node to a NIL leaf has the same number of black nodes.
fn check_color_invariants(t: &RBTree) {
    if let Some(root) = &t.root {
        assert_eq!(color(root), Color::Black, "root must be black");
        let mut min_black = -1i32;
        let mut max_black = -1i32;
        check_color_recursive(Some(root.clone()), 0, &mut min_black, &mut max_black, Color::Black);
        assert_eq!(min_black, max_black, "black-height not equal on all paths");
    }
}

fn check_color_recursive(
    n: Option<NodeRef>,
    black_depth: i32,
    min_black: &mut i32,
    max_black: &mut i32,
    parent_color: Color,
) {
    match n {
        None => {
            // NIL leaf — record black-height
            let depth = black_depth; // NIL counts as black but we record before this NIL
            if *min_black == -1 || depth < *min_black {
                *min_black = depth;
            }
            if *max_black == -1 || depth > *max_black {
                *max_black = depth;
            }
        }
        Some(node) => {
            let c = color(&node);
            if parent_color == Color::Red && c == Color::Red {
                panic!("red node has red parent");
            }
            let next_depth = if c == Color::Black { black_depth + 1 } else { black_depth };
            check_color_recursive(left(&node), next_depth, min_black, max_black, c.clone());
            check_color_recursive(right(&node), next_depth, min_black, max_black, c);
        }
    }
}

fn check_bst(t: &RBTree) {
    fn rec(n: Option<NodeRef>, lo: Option<i32>, hi: Option<i32>) {
        if let Some(node) = n {
            let k = key(&node);
            if let Some(lo) = lo {
                assert!(k >= lo, "BST violation: key {} < lo {}", k, lo);
            }
            if let Some(hi) = hi {
                assert!(k <= hi, "BST violation: key {} > hi {}", k, hi);
            }
            // Allow duplicates: left subtree keys <= k, right subtree keys >= k
            rec(left(&node), lo, Some(k));
            rec(right(&node), Some(k), hi);
        }
    }
    rec(t.root.clone(), None, None);
}

fn check_parent_links(t: &RBTree) {
    fn rec(n: Option<NodeRef>, expected_parent: Option<NodeRef>) {
        if let Some(node) = n {
            match (&expected_parent, &parent(&node)) {
                (None, None) => {}
                (Some(a), Some(b)) => assert!(Rc::ptr_eq(a, b), "parent mismatch"),
                _ => panic!("parent mismatch (one None)"),
            }
            rec(left(&node), Some(node.clone()));
            rec(right(&node), Some(node.clone()));
        }
    }
    rec(t.root.clone(), None);
}

fn insert_arr(t: &mut RBTree, arr: &[i32]) {
    for k in arr {
        t.rbtree_insert(*k);
    }
}

fn check_invariants(t: &RBTree) {
    check_bst(t);
    check_color_invariants(t);
    check_parent_links(t);
}

// ---------- Tests ----------

#[test]
fn test_new_empty() {
    let t = RBTree::new();
    assert!(t.root.is_none());
}

#[test]
fn test_default_empty() {
    let t = RBTree::default();
    assert!(t.root.is_none());
}

#[test]
fn test_insert_single() {
    let mut t = RBTree::new();
    let p = t.rbtree_insert(1024).expect("insert returned None");
    // Root match
    assert!(Rc::ptr_eq(t.root.as_ref().unwrap(), &p));
    assert_eq!(key(&p), 1024);
    // Root must be black after fixup
    assert_eq!(color(&p), Color::Black);
    // No children, no parent
    assert!(left(&p).is_none());
    assert!(right(&p).is_none());
    assert!(parent(&p).is_none());
}

#[test]
fn test_insert_multiple_keeps_invariants() {
    let mut t = RBTree::new();
    let entries = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12, 24, 36, 990, 25];
    insert_arr(&mut t, &entries);
    assert!(t.root.is_some());
    check_invariants(&t);
}

#[test]
fn test_insert_returns_node_with_correct_key() {
    let mut t = RBTree::new();
    let n1 = t.rbtree_insert(50).unwrap();
    let n2 = t.rbtree_insert(25).unwrap();
    let n3 = t.rbtree_insert(75).unwrap();
    assert_eq!(key(&n1), 50);
    assert_eq!(key(&n2), 25);
    assert_eq!(key(&n3), 75);
}

// Validates the well-known textbook fixture (CLRS) that ascending insertion of
// 1..=8 yields a specific tree. Verified against C reference output.
#[test]
fn test_insert_ascending_1_to_8_structure() {
    let mut t = RBTree::new();
    for i in 1..=8 {
        t.rbtree_insert(i);
    }
    // Expected (from C output):
    // root: 4(black)
    //   2(red): left=1(black), right=3(black)
    //   6(red): left=5(black), right=7(black){right=8(red)}
    let root = t.root.clone().unwrap();
    assert_eq!(key(&root), 4);
    assert_eq!(color(&root), Color::Black);

    let l = left(&root).unwrap();
    assert_eq!(key(&l), 2);
    assert_eq!(color(&l), Color::Red);
    let ll = left(&l).unwrap();
    let lr = right(&l).unwrap();
    assert_eq!(key(&ll), 1);
    assert_eq!(color(&ll), Color::Black);
    assert_eq!(key(&lr), 3);
    assert_eq!(color(&lr), Color::Black);

    let r = right(&root).unwrap();
    assert_eq!(key(&r), 6);
    assert_eq!(color(&r), Color::Red);
    let rl = left(&r).unwrap();
    let rr = right(&r).unwrap();
    assert_eq!(key(&rl), 5);
    assert_eq!(color(&rl), Color::Black);
    assert_eq!(key(&rr), 7);
    assert_eq!(color(&rr), Color::Black);
    let rrr = right(&rr).unwrap();
    assert_eq!(key(&rrr), 8);
    assert_eq!(color(&rrr), Color::Red);
    assert!(left(&rr).is_none());

    check_invariants(&t);
}

// Verified against C reference output for descending 8..=1 insertions.
#[test]
fn test_insert_descending_8_to_1_structure() {
    let mut t = RBTree::new();
    for i in (1..=8).rev() {
        t.rbtree_insert(i);
    }
    // Expected:
    // root: 5(black)
    //   3(red): left=2(black){left=1(red)}, right=4(black)
    //   7(red): left=6(black), right=8(black)
    let root = t.root.clone().unwrap();
    assert_eq!(key(&root), 5);
    assert_eq!(color(&root), Color::Black);

    let l = left(&root).unwrap();
    assert_eq!(key(&l), 3);
    assert_eq!(color(&l), Color::Red);
    let ll = left(&l).unwrap();
    assert_eq!(key(&ll), 2);
    assert_eq!(color(&ll), Color::Black);
    let lll = left(&ll).unwrap();
    assert_eq!(key(&lll), 1);
    assert_eq!(color(&lll), Color::Red);
    let lr = right(&l).unwrap();
    assert_eq!(key(&lr), 4);
    assert_eq!(color(&lr), Color::Black);

    let r = right(&root).unwrap();
    assert_eq!(key(&r), 7);
    assert_eq!(color(&r), Color::Red);
    let rl = left(&r).unwrap();
    let rr = right(&r).unwrap();
    assert_eq!(key(&rl), 6);
    assert_eq!(color(&rl), Color::Black);
    assert_eq!(key(&rr), 8);
    assert_eq!(color(&rr), Color::Black);

    check_invariants(&t);
}

// Verified against C reference output (CLRS Figure 13.4 example).
#[test]
fn test_insert_clrs_fixture_structure() {
    let mut t = RBTree::new();
    let arr = [11, 2, 14, 1, 7, 15, 5, 8, 4];
    insert_arr(&mut t, &arr);
    // Expected:
    // root: 7(black)
    //   2(red): left=1(black), right=5(black){left=4(red)}
    //   11(red): left=8(black), right=14(black){right=15(red)}
    let root = t.root.clone().unwrap();
    assert_eq!(key(&root), 7);
    assert_eq!(color(&root), Color::Black);

    let l = left(&root).unwrap();
    assert_eq!(key(&l), 2);
    assert_eq!(color(&l), Color::Red);
    let ll = left(&l).unwrap();
    let lr = right(&l).unwrap();
    assert_eq!(key(&ll), 1);
    assert_eq!(color(&ll), Color::Black);
    assert_eq!(key(&lr), 5);
    assert_eq!(color(&lr), Color::Black);
    let lrl = left(&lr).unwrap();
    assert_eq!(key(&lrl), 4);
    assert_eq!(color(&lrl), Color::Red);

    let r = right(&root).unwrap();
    assert_eq!(key(&r), 11);
    assert_eq!(color(&r), Color::Red);
    let rl = left(&r).unwrap();
    let rr = right(&r).unwrap();
    assert_eq!(key(&rl), 8);
    assert_eq!(color(&rl), Color::Black);
    assert_eq!(key(&rr), 14);
    assert_eq!(color(&rr), Color::Black);
    let rrr = right(&rr).unwrap();
    assert_eq!(key(&rrr), 15);
    assert_eq!(color(&rrr), Color::Red);

    check_invariants(&t);
}

#[test]
fn test_insert_duplicates() {
    let mut t = RBTree::new();
    let arr = [10, 5, 5, 34, 6, 23, 12, 12, 6, 12];
    insert_arr(&mut t, &arr);
    check_invariants(&t);
    // to_array returns sorted, allowing duplicates
    let v = t.to_array(arr.len());
    let mut expected = arr.to_vec();
    expected.sort();
    assert_eq!(v, expected);
}

#[test]
fn test_find_existing_and_missing() {
    let mut t = RBTree::new();
    let inserted = t.rbtree_insert(512).unwrap();
    let found = t.rbtree_find(512).expect("should find inserted key");
    assert_eq!(key(&found), 512);
    assert!(Rc::ptr_eq(&found, &inserted));

    let missing = t.rbtree_find(1024);
    assert!(missing.is_none());
}

#[test]
fn test_find_in_complex_tree() {
    let mut t = RBTree::new();
    let arr = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12];
    insert_arr(&mut t, &arr);
    for k in &arr {
        let found = t.rbtree_find(*k).expect("found");
        assert_eq!(key(&found), *k);
    }
    assert!(t.rbtree_find(9999).is_none());
    assert!(t.rbtree_find(-1).is_none());
}

#[test]
fn test_min_max_empty() {
    let t = RBTree::new();
    assert!(t.rbtree_min().is_none());
    assert!(t.rbtree_max().is_none());
}

#[test]
fn test_min_max_single() {
    let mut t = RBTree::new();
    t.rbtree_insert(42);
    assert_eq!(key(&t.rbtree_min().unwrap()), 42);
    assert_eq!(key(&t.rbtree_max().unwrap()), 42);
}

#[test]
fn test_min_max_with_arr() {
    let mut t = RBTree::new();
    let arr = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12];
    insert_arr(&mut t, &arr);
    let mut sorted = arr.to_vec();
    sorted.sort();
    assert_eq!(key(&t.rbtree_min().unwrap()), sorted[0]); // 2
    assert_eq!(key(&t.rbtree_max().unwrap()), sorted[sorted.len() - 1]); // 156
}

#[test]
fn test_to_array_basic() {
    let mut t = RBTree::new();
    let arr = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12, 24, 36, 990, 25];
    insert_arr(&mut t, &arr);
    let v = t.to_array(arr.len());
    let mut sorted = arr.to_vec();
    sorted.sort();
    assert_eq!(v, sorted);
}

#[test]
fn test_to_array_partial() {
    let mut t = RBTree::new();
    let arr = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12];
    insert_arr(&mut t, &arr);
    let v = t.to_array(5);
    let mut sorted = arr.to_vec();
    sorted.sort();
    assert_eq!(v, sorted[..5].to_vec());
}

#[test]
fn test_to_array_empty() {
    let t = RBTree::new();
    let v = t.to_array(10);
    assert_eq!(v, Vec::<i32>::new());
}

#[test]
fn test_subtree_to_array_direct() {
    let mut t = RBTree::new();
    let arr = [10, 5, 8, 34, 67];
    insert_arr(&mut t, &arr);
    let mut out: Vec<i32> = Vec::new();
    let mut count = 0usize;
    t.subtree_to_array(t.root.clone(), &mut out, 5, &mut count);
    assert_eq!(count, 5);
    let mut sorted = arr.to_vec();
    sorted.sort();
    assert_eq!(out, sorted);
}

#[test]
fn test_subtree_to_array_nil_returns_empty() {
    let t = RBTree::new();
    let mut out: Vec<i32> = Vec::new();
    let mut count = 0usize;
    t.subtree_to_array(None, &mut out, 5, &mut count);
    assert_eq!(count, 0);
    assert!(out.is_empty());
}

#[test]
fn test_erase_root_only() {
    let mut t = RBTree::new();
    let p = t.rbtree_insert(128).unwrap();
    assert!(Rc::ptr_eq(t.root.as_ref().unwrap(), &p));
    t.erase(p);
    assert!(t.root.is_none());
}

#[test]
fn test_erase_min_then_check_min() {
    let mut t = RBTree::new();
    let arr = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12];
    insert_arr(&mut t, &arr);
    let mut sorted = arr.to_vec();
    sorted.sort();
    let min_node = t.rbtree_min().unwrap();
    assert_eq!(key(&min_node), sorted[0]);
    t.erase(min_node);
    let new_min = t.rbtree_min().unwrap();
    assert_eq!(key(&new_min), sorted[1]);
    check_invariants(&t);
}

#[test]
fn test_erase_max_then_check_max() {
    let mut t = RBTree::new();
    let arr = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12];
    insert_arr(&mut t, &arr);
    let mut sorted = arr.to_vec();
    sorted.sort();
    let max_node = t.rbtree_max().unwrap();
    assert_eq!(key(&max_node), *sorted.last().unwrap());
    t.erase(max_node);
    let new_max = t.rbtree_max().unwrap();
    assert_eq!(key(&new_max), sorted[sorted.len() - 2]);
    check_invariants(&t);
}

#[test]
fn test_erase_all_in_order() {
    let mut t = RBTree::new();
    let arr = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12];
    insert_arr(&mut t, &arr);
    let mut remaining = arr.to_vec();
    remaining.sort();

    while !remaining.is_empty() {
        let target = remaining.remove(0);
        let node = t.rbtree_find(target).expect("must be present");
        t.erase(node);
        check_invariants(&t);
        let v = t.to_array(remaining.len());
        assert_eq!(v, remaining);
    }
    assert!(t.root.is_none());
}

#[test]
fn test_erase_all_random_order() {
    let mut t = RBTree::new();
    let arr = [50, 30, 70, 20, 40, 60, 80, 10, 25, 35, 45, 55, 65, 75, 85];
    insert_arr(&mut t, &arr);

    let order = [40, 50, 70, 25, 10, 80, 30, 60, 20, 75, 65, 55, 45, 35, 85];
    let mut remaining = arr.to_vec();
    remaining.sort();
    for k in &order {
        let node = t.rbtree_find(*k).expect("present");
        t.erase(node);
        let pos = remaining.iter().position(|x| x == k).unwrap();
        remaining.remove(pos);
        check_invariants(&t);
        let v = t.to_array(remaining.len());
        assert_eq!(v, remaining);
    }
    assert!(t.root.is_none());
}

#[test]
fn test_erase_with_two_children() {
    let mut t = RBTree::new();
    // Create tree where root has 2 children deep
    let arr = [50, 30, 70, 20, 40, 60, 80];
    insert_arr(&mut t, &arr);
    let root = t.root.clone().unwrap();
    // root has 2 children
    assert!(left(&root).is_some());
    assert!(right(&root).is_some());
    t.erase(root);
    // Tree should still satisfy RB invariants
    check_invariants(&t);
    // remaining keys
    let v = t.to_array(arr.len());
    let mut expected: Vec<i32> = arr.iter().filter(|&&k| k != 50).copied().collect();
    expected.sort();
    assert_eq!(v, expected);
}

#[test]
fn test_left_rotate_simple() {
    // Build manual tree:    10
    //                       /\
    //                      5  20
    //                         / \
    //                        15 25
    let mut t = RBTree::new();
    insert_arr(&mut t, &[10, 5, 20, 15, 25]);
    // After inserting these keys, structure is determined by RB. Verify left_rotate
    // can be called on the root and the tree still has same set of keys.
    let root = t.root.clone().unwrap();
    let r_right = right(&root);
    if r_right.is_some() {
        // Left rotate around root
        t.left_rotate(root.clone());
        // root should now be the previous root.right
        let new_root = t.root.clone().unwrap();
        assert!(Rc::ptr_eq(&new_root, r_right.as_ref().unwrap()));
        // BST may now be violated since we rotated arbitrarily, but parent links must still be coherent.
        check_parent_links(&t);
        // Sorted in-order traversal of the keys (regardless of color) should still
        // contain the same multiset.
        let v = t.to_array(5);
        let mut expected = vec![5, 10, 15, 20, 25];
        let mut got = v.clone();
        got.sort();
        expected.sort();
        assert_eq!(got, expected);
    }
}

#[test]
fn test_right_rotate_simple() {
    let mut t = RBTree::new();
    insert_arr(&mut t, &[10, 5, 20, 3, 7]);
    let root = t.root.clone().unwrap();
    let r_left = left(&root);
    if r_left.is_some() {
        t.right_rotate(root.clone());
        let new_root = t.root.clone().unwrap();
        assert!(Rc::ptr_eq(&new_root, r_left.as_ref().unwrap()));
        check_parent_links(&t);
        let v = t.to_array(5);
        let mut got = v.clone();
        got.sort();
        let mut expected = vec![10, 5, 20, 3, 7];
        expected.sort();
        assert_eq!(got, expected);
    }
}

#[test]
fn test_rotate_inverse() {
    // Rotating left then right should restore original structure (around same node).
    let mut t = RBTree::new();
    insert_arr(&mut t, &[10, 5, 20, 3, 7, 15, 25]);
    let before = t.to_array(7);

    let root = t.root.clone().unwrap();
    if right(&root).is_some() {
        t.left_rotate(root.clone());
        // new root is what was root.right; root is now its left child.
        let new_root = t.root.clone().unwrap();
        // root is now new_root.left
        let nl = left(&new_root).unwrap();
        assert!(Rc::ptr_eq(&nl, &root));
        // Right-rotate around the new root to undo
        t.right_rotate(new_root);
        let after = t.to_array(7);
        assert_eq!(before, after);
        check_parent_links(&t);
    }
}

#[test]
fn test_transplant_root() {
    let mut t = RBTree::new();
    let n1 = t.rbtree_insert(10).unwrap();
    let n2 = Rc::new(RefCell::new(Node {
        key: 99,
        color: Color::Black,
        left: None,
        right: None,
        parent: None,
    }));
    t.transplant(n1, Some(n2.clone()));
    assert!(Rc::ptr_eq(t.root.as_ref().unwrap(), &n2));
    assert!(parent(&n2).is_none());
}

#[test]
fn test_transplant_child() {
    let mut t = RBTree::new();
    insert_arr(&mut t, &[10, 5, 20]);
    let root = t.root.clone().unwrap();
    let l = left(&root).unwrap();
    let new_node = Rc::new(RefCell::new(Node {
        key: 999,
        color: Color::Red,
        left: None,
        right: None,
        parent: None,
    }));
    t.transplant(l.clone(), Some(new_node.clone()));
    // root.left should now be new_node, new_node.parent = root
    let nl = left(&root).unwrap();
    assert!(Rc::ptr_eq(&nl, &new_node));
    let np = parent(&new_node).unwrap();
    assert!(Rc::ptr_eq(&np, &root));
}

#[test]
fn test_free_node_no_panic_on_none() {
    // Should not panic
    RBTree::free_node(None);
}

#[test]
fn test_free_node_drops_subtree() {
    let mut t = RBTree::new();
    insert_arr(&mut t, &[10, 5, 20]);
    let root = t.root.take();
    RBTree::free_node(root);
    assert!(t.root.is_none());
}

#[test]
fn test_delete_rbtree_consumes() {
    let mut t = RBTree::new();
    insert_arr(&mut t, &[10, 5, 20]);
    t.delete_rbtree();
    // No assertion — just make sure it doesn't panic and doesn't produce a use-after-free.
}

#[test]
fn test_delete_fixup_on_none_is_noop() {
    // calling on None must not panic
    let mut t = RBTree::new();
    t.delete_fixup(None);
    assert!(t.root.is_none());
}

#[test]
fn test_insert_fixup_idempotent_on_root() {
    // Insert one node, call rbtree_insert_fixup explicitly: root must remain black.
    let mut t = RBTree::new();
    let n = t.rbtree_insert(7).unwrap();
    t.rbtree_insert_fixup(n.clone());
    assert_eq!(color(&n), Color::Black);
}

#[test]
fn test_multi_instance() {
    let mut t1 = RBTree::new();
    let mut t2 = RBTree::new();

    let arr1 = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12, 24, 36, 990, 25];
    insert_arr(&mut t1, &arr1);
    let arr2 = [4, 8, 10, 5, 3];
    insert_arr(&mut t2, &arr2);

    let mut s1 = arr1.to_vec();
    s1.sort();
    let mut s2 = arr2.to_vec();
    s2.sort();

    assert_eq!(t1.to_array(arr1.len()), s1);
    assert_eq!(t2.to_array(arr2.len()), s2);

    check_invariants(&t1);
    check_invariants(&t2);
}

#[test]
fn test_large_random_inserts_and_invariants() {
    // Deterministic pseudo-random sequence
    let mut t = RBTree::new();
    let mut x: u32 = 1234567;
    let n = 200;
    let mut keys: Vec<i32> = Vec::with_capacity(n);
    for _ in 0..n {
        // xorshift
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        let v = (x as i32).rem_euclid(1000);
        keys.push(v);
        t.rbtree_insert(v);
    }
    check_invariants(&t);
    let v = t.to_array(n);
    let mut expected = keys.clone();
    expected.sort();
    assert_eq!(v, expected);
}

#[test]
fn test_erase_random_sequence_invariants() {
    let mut t = RBTree::new();
    let keys: Vec<i32> = vec![15, 6, 18, 3, 7, 17, 20, 2, 4, 13, 9];
    insert_arr(&mut t, &keys);
    let erase_order = [13, 6, 17, 18, 15, 9, 7, 3, 4, 20, 2];
    let mut remaining: Vec<i32> = keys.clone();
    remaining.sort();
    for k in erase_order.iter() {
        let node = t.rbtree_find(*k).unwrap();
        t.erase(node);
        let pos = remaining.iter().position(|x| x == k).unwrap();
        remaining.remove(pos);
        check_invariants(&t);
        let v = t.to_array(remaining.len());
        assert_eq!(v, remaining);
    }
    assert!(t.root.is_none());
}

#[test]
fn test_min_max_after_inserts_returns_correct_keys() {
    let mut t = RBTree::new();
    insert_arr(&mut t, &[5, 3, 8, 1, 4, 7, 9, 0, 2, 6, 10]);
    assert_eq!(key(&t.rbtree_min().unwrap()), 0);
    assert_eq!(key(&t.rbtree_max().unwrap()), 10);
}

fn main() {}
