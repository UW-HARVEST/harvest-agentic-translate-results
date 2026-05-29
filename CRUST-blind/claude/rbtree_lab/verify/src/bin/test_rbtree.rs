#![allow(dead_code)]

use rbtree::rbtree::{Color, Node, NodeRef, RBTree};
use std::cell::RefCell;
use std::rc::Rc;

// Helper: insert all keys
fn insert_all(tree: &mut RBTree, keys: &[i32]) -> Vec<NodeRef> {
    let mut nodes = Vec::new();
    for &k in keys {
        let n = tree.rbtree_insert(k).expect("insert returned None");
        nodes.push(n);
    }
    nodes
}

// Verify search-tree (BST) property
fn check_bst(node: Option<NodeRef>, min: &mut i32, max: &mut i32) -> bool {
    let n = match node {
        Some(n) => n,
        None => return true,
    };
    let k = n.borrow().key;
    *min = k;
    *max = k;
    let mut l_min = k;
    let mut l_max = k;
    let mut r_min = k;
    let mut r_max = k;
    let left = n.borrow().left.clone();
    let right = n.borrow().right.clone();
    if !check_bst(left, &mut l_min, &mut l_max) || l_max > k {
        return false;
    }
    if !check_bst(right, &mut r_min, &mut r_max) || r_min < k {
        return false;
    }
    *min = l_min;
    *max = r_max;
    true
}

// Verify red-black color constraint
fn check_color(
    node: Option<NodeRef>,
    parent_color: Color,
    black_depth: i32,
    touched: &mut bool,
    max_depth: &mut i32,
) -> bool {
    let n = match node {
        Some(n) => n,
        None => {
            if !*touched {
                *touched = true;
                *max_depth = black_depth;
            } else if black_depth != *max_depth {
                return false;
            }
            return true;
        }
    };
    let c = n.borrow().color.clone();
    if parent_color == Color::Red && c == Color::Red {
        return false;
    }
    let next = if c == Color::Black { 1 } else { 0 } + black_depth;
    let l = n.borrow().left.clone();
    let r = n.borrow().right.clone();
    check_color(l, c.clone(), next, touched, max_depth)
        && check_color(r, c, next, touched, max_depth)
}

fn assert_rb_constraints(tree: &RBTree) {
    // Root must be None or black
    if let Some(root) = &tree.root {
        assert_eq!(root.borrow().color, Color::Black, "root must be black");
    }
    // Search constraint
    let mut min = 0;
    let mut max = 0;
    assert!(
        check_bst(tree.root.clone(), &mut min, &mut max),
        "BST constraint violated"
    );
    // Color constraint
    let mut touched = false;
    let mut max_depth = 0;
    assert!(
        check_color(
            tree.root.clone(),
            Color::Black,
            0,
            &mut touched,
            &mut max_depth
        ),
        "RB color constraint violated"
    );
}

// ============================================================
// Tests for RBTree::new()
// ============================================================
#[test]
fn test_new_empty_tree() {
    let t = RBTree::new();
    assert!(t.root.is_none());
}

#[test]
fn test_default() {
    let t: RBTree = Default::default();
    assert!(t.root.is_none());
}

// ============================================================
// Tests for rbtree_insert
// ============================================================
#[test]
fn test_insert_single() {
    let mut t = RBTree::new();
    let p = t.rbtree_insert(1024).expect("insert returned None");
    // root should be p
    assert!(t.root.is_some());
    let root = t.root.clone().unwrap();
    assert!(Rc::ptr_eq(&root, &p));
    assert_eq!(p.borrow().key, 1024);
    // After fixup, root must be black
    assert_eq!(p.borrow().color, Color::Black);
    assert!(p.borrow().left.is_none());
    assert!(p.borrow().right.is_none());
    assert!(p.borrow().parent.is_none());
}

#[test]
fn test_insert_returns_node_with_correct_key() {
    let mut t = RBTree::new();
    let p = t.rbtree_insert(42).expect("insert returned None");
    assert_eq!(p.borrow().key, 42);
}

#[test]
fn test_insert_two_nodes_smaller() {
    let mut t = RBTree::new();
    let _a = t.rbtree_insert(10);
    let b = t.rbtree_insert(5).expect("insert");
    // root should be 10, left child = 5
    let root = t.root.clone().unwrap();
    assert_eq!(root.borrow().key, 10);
    assert_eq!(root.borrow().color, Color::Black);
    let l = root.borrow().left.clone().unwrap();
    assert_eq!(l.borrow().key, 5);
    assert_eq!(l.borrow().color, Color::Red);
    assert!(Rc::ptr_eq(&l, &b));
}

#[test]
fn test_insert_two_nodes_larger() {
    let mut t = RBTree::new();
    let _a = t.rbtree_insert(10);
    let b = t.rbtree_insert(20).expect("insert");
    let root = t.root.clone().unwrap();
    assert_eq!(root.borrow().key, 10);
    let r = root.borrow().right.clone().unwrap();
    assert_eq!(r.borrow().key, 20);
    assert_eq!(r.borrow().color, Color::Red);
    assert!(Rc::ptr_eq(&r, &b));
}

#[test]
fn test_insert_distinct_to_array() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let arr = t.to_array(10);
    assert_eq!(arr, vec![2, 5, 8, 10, 12, 23, 24, 34, 67, 156]);
    assert_rb_constraints(&t);
}

#[test]
fn test_insert_duplicates_to_array() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 5, 34, 6, 23, 12, 12, 6, 12]);
    let arr = t.to_array(10);
    assert_eq!(arr, vec![5, 5, 6, 6, 10, 12, 12, 12, 23, 34]);
    assert_rb_constraints(&t);
}

#[test]
fn test_insert_ascending() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[1, 2, 3, 4, 5]);
    let arr = t.to_array(5);
    assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    // The root after this sequence should be 2 (per C output)
    assert_eq!(t.root.clone().unwrap().borrow().key, 2);
    assert_rb_constraints(&t);
}

#[test]
fn test_insert_descending() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[5, 4, 3, 2, 1]);
    let arr = t.to_array(5);
    assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    // Root after this sequence should be 4 (per C output)
    assert_eq!(t.root.clone().unwrap().borrow().key, 4);
    assert_rb_constraints(&t);
}

#[test]
fn test_insert_root_evolves() {
    let mut t = RBTree::new();
    let arr = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12];
    let expected_roots = [10, 10, 8, 8, 8, 8, 8, 8, 8, 23];
    for (i, &k) in arr.iter().enumerate() {
        t.rbtree_insert(k);
        let r = t.root.clone().unwrap();
        assert_eq!(
            r.borrow().key,
            expected_roots[i],
            "after inserting {}",
            k
        );
        assert_eq!(r.borrow().color, Color::Black, "root must be black");
    }
}

#[test]
fn test_insert_multi_long() {
    let mut t = RBTree::new();
    insert_all(
        &mut t,
        &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12, 24, 36, 990, 25],
    );
    let arr = t.to_array(14);
    assert_eq!(
        arr,
        vec![2, 5, 8, 10, 12, 23, 24, 24, 25, 34, 36, 67, 156, 990]
    );
    assert_rb_constraints(&t);
}

// ============================================================
// Tests for rbtree_find
// ============================================================
#[test]
fn test_find_existing() {
    let mut t = RBTree::new();
    let p = t.rbtree_insert(512).unwrap();
    let q = t.rbtree_find(512).unwrap();
    assert_eq!(q.borrow().key, 512);
    assert!(Rc::ptr_eq(&q, &p));
}

#[test]
fn test_find_missing() {
    let mut t = RBTree::new();
    t.rbtree_insert(512);
    let q = t.rbtree_find(1024);
    assert!(q.is_none());
}

#[test]
fn test_find_in_larger_tree() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let q = t.rbtree_find(8).unwrap();
    assert_eq!(q.borrow().key, 8);
    assert!(t.rbtree_find(99).is_none());
    assert!(t.rbtree_find(11).is_none());
    let q156 = t.rbtree_find(156).unwrap();
    assert_eq!(q156.borrow().key, 156);
}

#[test]
fn test_find_empty_tree() {
    let t = RBTree::new();
    assert!(t.rbtree_find(0).is_none());
    assert!(t.rbtree_find(42).is_none());
}

// ============================================================
// Tests for rbtree_min / rbtree_max
// ============================================================
#[test]
fn test_minmax_empty() {
    let t = RBTree::new();
    assert!(t.rbtree_min().is_none());
    assert!(t.rbtree_max().is_none());
}

#[test]
fn test_minmax_single() {
    let mut t = RBTree::new();
    t.rbtree_insert(42);
    let mn = t.rbtree_min().unwrap();
    let mx = t.rbtree_max().unwrap();
    assert_eq!(mn.borrow().key, 42);
    assert_eq!(mx.borrow().key, 42);
}

#[test]
fn test_minmax_distinct() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let mn = t.rbtree_min().unwrap();
    let mx = t.rbtree_max().unwrap();
    assert_eq!(mn.borrow().key, 2);
    assert_eq!(mx.borrow().key, 156);
}

#[test]
fn test_minmax_duplicates() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 5, 34, 6, 23, 12, 12, 6, 12]);
    let mn = t.rbtree_min().unwrap();
    let mx = t.rbtree_max().unwrap();
    assert_eq!(mn.borrow().key, 5);
    assert_eq!(mx.borrow().key, 34);
}

#[test]
fn test_minmax_after_erase() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let mn = t.rbtree_min().unwrap();
    assert_eq!(mn.borrow().key, 2);
    t.erase(mn);
    let mn2 = t.rbtree_min().unwrap();
    assert_eq!(mn2.borrow().key, 5);

    let mx = t.rbtree_max().unwrap();
    assert_eq!(mx.borrow().key, 156);
    t.erase(mx);
    let mx2 = t.rbtree_max().unwrap();
    assert_eq!(mx2.borrow().key, 67);
    assert_rb_constraints(&t);
}

// ============================================================
// Tests for erase
// ============================================================
#[test]
fn test_erase_root_single() {
    let mut t = RBTree::new();
    let p = t.rbtree_insert(128).unwrap();
    t.erase(p);
    assert!(t.root.is_none());
}

#[test]
fn test_erase_leaf() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let p = t.rbtree_find(5).unwrap();
    t.erase(p);
    let arr = t.to_array(9);
    assert_eq!(arr, vec![2, 8, 10, 12, 23, 24, 34, 67, 156]);
    assert_rb_constraints(&t);
}

#[test]
fn test_erase_internal_two_children() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let p = t.rbtree_find(10).unwrap();
    t.erase(p);
    let arr = t.to_array(9);
    assert_eq!(arr, vec![2, 5, 8, 12, 23, 24, 34, 67, 156]);
    assert_rb_constraints(&t);
}

#[test]
fn test_erase_max() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let p = t.rbtree_find(156).unwrap();
    t.erase(p);
    let arr = t.to_array(9);
    assert_eq!(arr, vec![2, 5, 8, 10, 12, 23, 24, 34, 67]);
    assert_rb_constraints(&t);
}

#[test]
fn test_erase_all_in_sorted_order() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let sorted = [2, 5, 8, 10, 12, 23, 24, 34, 67, 156];
    let expected_after = [
        vec![5, 8, 10, 12, 23, 24, 34, 67, 156],
        vec![8, 10, 12, 23, 24, 34, 67, 156],
        vec![10, 12, 23, 24, 34, 67, 156],
        vec![12, 23, 24, 34, 67, 156],
        vec![23, 24, 34, 67, 156],
        vec![24, 34, 67, 156],
        vec![34, 67, 156],
        vec![67, 156],
        vec![156],
        vec![],
    ];
    for (i, &k) in sorted.iter().enumerate() {
        let p = t.rbtree_find(k).unwrap();
        t.erase(p);
        let n = 10 - 1 - i;
        let arr = t.to_array(n);
        assert_eq!(arr, expected_after[i], "after erasing {}", k);
        assert_rb_constraints(&t);
    }
    assert!(t.root.is_none());
}

// ============================================================
// Tests for to_array / subtree_to_array
// ============================================================
#[test]
fn test_to_array_empty() {
    let t = RBTree::new();
    let arr = t.to_array(5);
    assert_eq!(arr, Vec::<i32>::new());
}

#[test]
fn test_to_array_partial() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let arr = t.to_array(3);
    assert_eq!(arr, vec![2, 5, 8]);
}

#[test]
fn test_to_array_full() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[4, 8, 10, 5, 3]);
    let arr = t.to_array(5);
    assert_eq!(arr, vec![3, 4, 5, 8, 10]);
}

#[test]
fn test_subtree_to_array_direct() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let mut arr: Vec<i32> = Vec::with_capacity(10);
    let mut count: usize = 0;
    t.subtree_to_array(t.root.clone(), &mut arr, 10, &mut count);
    assert_eq!(arr, vec![2, 5, 8, 10, 12, 23, 24, 34, 67, 156]);
    assert_eq!(count, 10);
}

#[test]
fn test_subtree_to_array_partial() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let mut arr: Vec<i32> = Vec::with_capacity(3);
    let mut count: usize = 0;
    t.subtree_to_array(t.root.clone(), &mut arr, 3, &mut count);
    assert_eq!(arr, vec![2, 5, 8]);
    assert_eq!(count, 3);
}

#[test]
fn test_subtree_to_array_nil() {
    let t = RBTree::new();
    let mut arr: Vec<i32> = Vec::new();
    let mut count: usize = 0;
    t.subtree_to_array(None, &mut arr, 10, &mut count);
    assert_eq!(arr, Vec::<i32>::new());
    assert_eq!(count, 0);
}

// ============================================================
// Tests for left_rotate / right_rotate
// ============================================================
fn make_node(key: i32, color: Color) -> NodeRef {
    Rc::new(RefCell::new(Node {
        key,
        color,
        left: None,
        right: None,
        parent: None,
    }))
}

// Build a tree:
//       10
//      /  \
//     5    20
//         /  \
//        15   25
fn build_test_tree() -> (RBTree, NodeRef, NodeRef, NodeRef, NodeRef, NodeRef) {
    let n10 = make_node(10, Color::Black);
    let n5 = make_node(5, Color::Black);
    let n20 = make_node(20, Color::Black);
    let n15 = make_node(15, Color::Black);
    let n25 = make_node(25, Color::Black);

    n10.borrow_mut().left = Some(n5.clone());
    n10.borrow_mut().right = Some(n20.clone());
    n5.borrow_mut().parent = Some(n10.clone());
    n20.borrow_mut().parent = Some(n10.clone());
    n20.borrow_mut().left = Some(n15.clone());
    n20.borrow_mut().right = Some(n25.clone());
    n15.borrow_mut().parent = Some(n20.clone());
    n25.borrow_mut().parent = Some(n20.clone());

    let t = RBTree {
        root: Some(n10.clone()),
    };
    (t, n10, n5, n20, n15, n25)
}

#[test]
fn test_left_rotate_at_root() {
    let (mut t, n10, _n5, n20, n15, n25) = build_test_tree();
    t.left_rotate(n10.clone());
    // n20 should be new root
    let root = t.root.clone().unwrap();
    assert!(Rc::ptr_eq(&root, &n20));
    assert_eq!(root.borrow().key, 20);
    // n20.left = n10, n20.right = n25
    let n20l = n20.borrow().left.clone().unwrap();
    let n20r = n20.borrow().right.clone().unwrap();
    assert!(Rc::ptr_eq(&n20l, &n10));
    assert!(Rc::ptr_eq(&n20r, &n25));
    // n10.left = n5 (unchanged), n10.right = n15
    let n10r = n10.borrow().right.clone().unwrap();
    assert!(Rc::ptr_eq(&n10r, &n15));
    // n10's parent is now n20
    let n10p = n10.borrow().parent.clone().unwrap();
    assert!(Rc::ptr_eq(&n10p, &n20));
    // n15's parent is now n10
    let n15p = n15.borrow().parent.clone().unwrap();
    assert!(Rc::ptr_eq(&n15p, &n10));
    // n20's parent is None (root)
    assert!(n20.borrow().parent.is_none());
}

#[test]
fn test_right_rotate_undoes_left_rotate() {
    let (mut t, n10, _n5, n20, _n15, _n25) = build_test_tree();
    t.left_rotate(n10.clone());
    t.right_rotate(n20.clone());
    let root = t.root.clone().unwrap();
    assert!(Rc::ptr_eq(&root, &n10));
    assert_eq!(root.borrow().key, 10);
    let l = root.borrow().left.clone().unwrap();
    let r = root.borrow().right.clone().unwrap();
    assert_eq!(l.borrow().key, 5);
    assert_eq!(r.borrow().key, 20);
}

#[test]
fn test_right_rotate_at_root() {
    // build:
    //       20
    //      /  \
    //     10   30
    //    / \
    //   5  15
    let n20 = make_node(20, Color::Black);
    let n10 = make_node(10, Color::Black);
    let n30 = make_node(30, Color::Black);
    let n5 = make_node(5, Color::Black);
    let n15 = make_node(15, Color::Black);

    n20.borrow_mut().left = Some(n10.clone());
    n20.borrow_mut().right = Some(n30.clone());
    n10.borrow_mut().parent = Some(n20.clone());
    n30.borrow_mut().parent = Some(n20.clone());
    n10.borrow_mut().left = Some(n5.clone());
    n10.borrow_mut().right = Some(n15.clone());
    n5.borrow_mut().parent = Some(n10.clone());
    n15.borrow_mut().parent = Some(n10.clone());

    let mut t = RBTree {
        root: Some(n20.clone()),
    };

    t.right_rotate(n20.clone());

    let root = t.root.clone().unwrap();
    assert!(Rc::ptr_eq(&root, &n10));
    let n10l = n10.borrow().left.clone().unwrap();
    let n10r = n10.borrow().right.clone().unwrap();
    assert!(Rc::ptr_eq(&n10l, &n5));
    assert!(Rc::ptr_eq(&n10r, &n20));
    // n20.left = n15
    let n20l = n20.borrow().left.clone().unwrap();
    assert!(Rc::ptr_eq(&n20l, &n15));
    // n15's parent is now n20
    let n15p = n15.borrow().parent.clone().unwrap();
    assert!(Rc::ptr_eq(&n15p, &n20));
    // n20's parent is now n10
    let n20p = n20.borrow().parent.clone().unwrap();
    assert!(Rc::ptr_eq(&n20p, &n10));
}

// ============================================================
// Tests for rbtree_insert_fixup (through insert) and direct call
// ============================================================
#[test]
fn test_insert_fixup_root_black_invariant() {
    let mut t = RBTree::new();
    for k in [10, 5, 8, 34, 67, 23, 156, 24, 2, 12].iter() {
        t.rbtree_insert(*k);
        // Root must remain black after each insert
        let r = t.root.clone().unwrap();
        assert_eq!(r.borrow().color, Color::Black);
    }
}

// ============================================================
// Tests for transplant
// ============================================================
#[test]
fn test_transplant_root() {
    let mut t = RBTree::new();
    let _ = t.rbtree_insert(10);
    // root is 10 with no children
    let root = t.root.clone().unwrap();
    let new_node = make_node(99, Color::Red);
    t.transplant(root.clone(), Some(new_node.clone()));
    let r = t.root.clone().unwrap();
    assert!(Rc::ptr_eq(&r, &new_node));
    assert_eq!(r.borrow().key, 99);
    assert!(new_node.borrow().parent.is_none());
}

#[test]
fn test_transplant_left_child() {
    // Build tree: root 10, left 5, right 20
    let n10 = make_node(10, Color::Black);
    let n5 = make_node(5, Color::Red);
    let n20 = make_node(20, Color::Red);
    n10.borrow_mut().left = Some(n5.clone());
    n10.borrow_mut().right = Some(n20.clone());
    n5.borrow_mut().parent = Some(n10.clone());
    n20.borrow_mut().parent = Some(n10.clone());

    let mut t = RBTree {
        root: Some(n10.clone()),
    };

    let nx = make_node(7, Color::Red);
    t.transplant(n5.clone(), Some(nx.clone()));
    // n10's left is now nx
    let l = n10.borrow().left.clone().unwrap();
    assert!(Rc::ptr_eq(&l, &nx));
    // nx's parent is n10
    let nxp = nx.borrow().parent.clone().unwrap();
    assert!(Rc::ptr_eq(&nxp, &n10));
}

#[test]
fn test_transplant_with_none() {
    // Build tree: root 10, left 5, right 20
    let n10 = make_node(10, Color::Black);
    let n5 = make_node(5, Color::Red);
    let n20 = make_node(20, Color::Red);
    n10.borrow_mut().left = Some(n5.clone());
    n10.borrow_mut().right = Some(n20.clone());
    n5.borrow_mut().parent = Some(n10.clone());
    n20.borrow_mut().parent = Some(n10.clone());

    let mut t = RBTree {
        root: Some(n10.clone()),
    };

    t.transplant(n5.clone(), None);
    let l = n10.borrow().left.clone();
    assert!(l.is_none());
}

// ============================================================
// Tests for free_node / Drop / delete_rbtree
// ============================================================
#[test]
fn test_free_node_none() {
    // Should not panic
    RBTree::free_node(None);
}

#[test]
fn test_free_node_clears_subtree() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 15]);
    let root = t.root.take();
    RBTree::free_node(root);
    assert!(t.root.is_none());
}

#[test]
fn test_delete_rbtree() {
    let mut t = RBTree::new();
    insert_all(&mut t, &[10, 5, 8, 34, 67]);
    t.delete_rbtree();
    // tree dropped - just ensure no crash
}

#[test]
fn test_drop_works() {
    {
        let mut t = RBTree::new();
        insert_all(&mut t, &[1, 2, 3, 4, 5]);
        // dropped at end of scope
    }
    // No crash
}

// ============================================================
// Multi-instance / independence
// ============================================================
#[test]
fn test_multi_instance() {
    let mut t1 = RBTree::new();
    let mut t2 = RBTree::new();
    insert_all(
        &mut t1,
        &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12, 24, 36, 990, 25],
    );
    insert_all(&mut t2, &[4, 8, 10, 5, 3]);

    let r1 = t1.to_array(14);
    let r2 = t2.to_array(5);
    assert_eq!(
        r1,
        vec![2, 5, 8, 10, 12, 23, 24, 24, 25, 34, 36, 67, 156, 990]
    );
    assert_eq!(r2, vec![3, 4, 5, 8, 10]);
}

// ============================================================
// delete_fixup direct invocation (basic smoke)
// ============================================================
#[test]
fn test_delete_fixup_with_root_node() {
    // Calling delete_fixup with x = root (Some) should be a no-op for the loop
    // (other than ensuring root remains black after final assignment).
    let mut t = RBTree::new();
    t.rbtree_insert(42);
    let root = t.root.clone();
    t.delete_fixup(root.clone());
    let r = t.root.clone().unwrap();
    assert_eq!(r.borrow().color, Color::Black);
    assert_eq!(r.borrow().key, 42);
}

#[test]
fn test_delete_fixup_with_red_node_just_paints_black() {
    // Build a tree with a red leaf, call delete_fixup on that red leaf.
    let mut t = RBTree::new();
    t.rbtree_insert(10);
    t.rbtree_insert(5); // 5 is red
    let n5 = t.rbtree_find(5).unwrap();
    assert_eq!(n5.borrow().color, Color::Red);
    // Calling delete_fixup on a red node: loop exits immediately because
    // x.color != BLACK, then x.color = BLACK at end.
    t.delete_fixup(Some(n5.clone()));
    assert_eq!(n5.borrow().color, Color::Black);
}

fn main() {}
