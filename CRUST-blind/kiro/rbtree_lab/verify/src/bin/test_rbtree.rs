use rbtree::rbtree::{Color, Key, NodeRef, RBTree};
use std::rc::Rc;

// Helper: check BST property recursively, returns (min, max) of subtree
fn check_bst(node: &Option<NodeRef>) -> Option<(Key, Key)> {
    let n = match node.as_ref() {
        Some(n) => n,
        None => return None,
    };
    let b = n.borrow();
    let key = b.key;
    let mut min = key;
    let mut max = key;

    if let Some((l_min, l_max)) = check_bst(&b.left) {
        assert!(l_max <= key, "BST violation: left max {} > node {}", l_max, key);
        min = l_min;
    }
    if let Some((r_min, r_max)) = check_bst(&b.right) {
        assert!(r_min >= key, "BST violation: right min {} < node {}", r_min, key);
        max = r_max;
    }
    Some((min, max))
}

// Helper: check RB color constraints, returns black-height
fn check_color(node: &Option<NodeRef>, parent_color: Color) -> usize {
    let n = match node.as_ref() {
        Some(n) => n,
        None => return 1, // NIL counts as black
    };
    let b = n.borrow();
    assert!(
        !(parent_color == Color::Red && b.color == Color::Red),
        "Red-red violation at key {}",
        b.key
    );
    let left_bh = check_color(&b.left, b.color.clone());
    let right_bh = check_color(&b.right, b.color.clone());
    assert_eq!(left_bh, right_bh, "Black-height mismatch at key {}", b.key);
    left_bh + if b.color == Color::Black { 1 } else { 0 }
}

fn check_rb(t: &RBTree) {
    // Root must be black (or None)
    if let Some(r) = t.root.as_ref() {
        assert_eq!(r.borrow().color, Color::Black, "Root must be black");
    }
    check_bst(&t.root);
    check_color(&t.root, Color::Black);
}

fn insert_arr(t: &mut RBTree, arr: &[Key]) {
    for &k in arr {
        t.rbtree_insert(k);
    }
}

// --- Tests ---

#[test]
fn test_init() {
    let t = RBTree::new();
    assert!(t.root.is_none());
    t.delete_rbtree();
}

#[test]
fn test_insert_single() {
    let mut t = RBTree::new();
    let p = t.rbtree_insert(1024).unwrap();
    assert!(t.root.is_some());
    assert!(Rc::ptr_eq(t.root.as_ref().unwrap(), &p));
    assert_eq!(p.borrow().key, 1024);
    assert_eq!(p.borrow().color, Color::Black);
    assert!(p.borrow().left.is_none());
    assert!(p.borrow().right.is_none());
    assert!(p.borrow().parent.is_none());
    t.delete_rbtree();
}

#[test]
fn test_find_single() {
    let mut t = RBTree::new();
    let p = t.rbtree_insert(512).unwrap();

    let q = t.rbtree_find(512);
    assert!(q.is_some());
    let q = q.unwrap();
    assert_eq!(q.borrow().key, 512);
    assert!(Rc::ptr_eq(&q, &p));

    let missing = t.rbtree_find(1024);
    assert!(missing.is_none());

    t.delete_rbtree();
}

#[test]
fn test_erase_root() {
    let mut t = RBTree::new();
    let p = t.rbtree_insert(128).unwrap();
    assert!(Rc::ptr_eq(t.root.as_ref().unwrap(), &p));
    t.erase(p);
    assert!(t.root.is_none());
    t.delete_rbtree();
}

#[test]
fn test_minmax() {
    let mut entries = vec![10, 5, 8, 34, 67, 23, 156, 24, 2, 12];
    let mut t = RBTree::new();
    insert_arr(&mut t, &entries);

    entries.sort();

    let p = t.rbtree_min().unwrap();
    assert_eq!(p.borrow().key, entries[0]);

    let q = t.rbtree_max().unwrap();
    assert_eq!(q.borrow().key, *entries.last().unwrap());

    // Erase min, check new min
    t.erase(p);
    let p2 = t.rbtree_min().unwrap();
    assert_eq!(p2.borrow().key, entries[1]);

    // Erase max, check new max
    t.erase(q);
    let q2 = t.rbtree_max().unwrap();
    assert_eq!(q2.borrow().key, entries[entries.len() - 2]);

    t.delete_rbtree();
}

#[test]
fn test_to_array() {
    let mut t = RBTree::new();
    let entries = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12, 24, 36, 990, 25];
    insert_arr(&mut t, &entries);

    let mut sorted = entries.to_vec();
    sorted.sort();

    let res = t.to_array(entries.len());
    assert_eq!(res, sorted);

    t.delete_rbtree();
}

#[test]
fn test_to_array_partial() {
    let mut t = RBTree::new();
    insert_arr(&mut t, &[5, 3, 7, 1, 4]);
    // Request fewer elements than in tree
    let res = t.to_array(3);
    assert_eq!(res, vec![1, 3, 4]);
    t.delete_rbtree();
}

#[test]
fn test_to_array_empty() {
    let t = RBTree::new();
    let res = t.to_array(10);
    assert!(res.is_empty());
    t.delete_rbtree();
}

#[test]
fn test_multi_instance() {
    let mut t1 = RBTree::new();
    let mut t2 = RBTree::new();

    let arr1 = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12, 24, 36, 990, 25];
    insert_arr(&mut t1, &arr1);
    let mut sorted1 = arr1.to_vec();
    sorted1.sort();

    let arr2 = [4, 8, 10, 5, 3];
    insert_arr(&mut t2, &arr2);
    let mut sorted2 = arr2.to_vec();
    sorted2.sort();

    assert_eq!(t1.to_array(arr1.len()), sorted1);
    assert_eq!(t2.to_array(arr2.len()), sorted2);

    t2.delete_rbtree();
    t1.delete_rbtree();
}

#[test]
fn test_distinct_values_rb_constraints() {
    let mut t = RBTree::new();
    let entries = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12];
    insert_arr(&mut t, &entries);
    check_rb(&t);
    t.delete_rbtree();
}

#[test]
fn test_duplicate_values_rb_constraints() {
    let mut t = RBTree::new();
    let entries = [10, 5, 5, 34, 6, 23, 12, 12, 6, 12];
    insert_arr(&mut t, &entries);
    check_rb(&t);
    t.delete_rbtree();
}

#[test]
fn test_find_after_multiple_inserts() {
    let mut t = RBTree::new();
    let keys = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12];
    insert_arr(&mut t, &keys);
    for &k in &keys {
        let found = t.rbtree_find(k);
        assert!(found.is_some(), "Should find key {}", k);
        assert_eq!(found.unwrap().borrow().key, k);
    }
    // Key not in tree
    assert!(t.rbtree_find(999).is_none());
    t.delete_rbtree();
}

#[test]
fn test_erase_all_nodes() {
    let mut t = RBTree::new();
    let keys = [10, 5, 8, 34, 67];
    insert_arr(&mut t, &keys);

    // Erase all by finding min repeatedly
    for _ in 0..keys.len() {
        check_rb(&t);
        let m = t.rbtree_min().unwrap();
        t.erase(m);
    }
    assert!(t.root.is_none());
    t.delete_rbtree();
}

#[test]
fn test_rb_constraints_after_erase() {
    let mut t = RBTree::new();
    let entries = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12];
    insert_arr(&mut t, &entries);

    // Erase a few nodes and check constraints each time
    let node = t.rbtree_find(10).unwrap();
    t.erase(node);
    check_rb(&t);

    let node = t.rbtree_find(156).unwrap();
    t.erase(node);
    check_rb(&t);

    let node = t.rbtree_find(2).unwrap();
    t.erase(node);
    check_rb(&t);

    t.delete_rbtree();
}

#[test]
fn test_min_max_empty() {
    let t = RBTree::new();
    assert!(t.rbtree_min().is_none());
    assert!(t.rbtree_max().is_none());
    t.delete_rbtree();
}

#[test]
fn test_find_empty() {
    let t = RBTree::new();
    assert!(t.rbtree_find(42).is_none());
    t.delete_rbtree();
}

#[test]
fn test_insert_duplicate_find_returns_one() {
    let mut t = RBTree::new();
    t.rbtree_insert(5);
    t.rbtree_insert(5);
    t.rbtree_insert(5);
    // find should return one of them
    let found = t.rbtree_find(5);
    assert!(found.is_some());
    assert_eq!(found.unwrap().borrow().key, 5);
    // to_array should have all 3
    let arr = t.to_array(10);
    assert_eq!(arr, vec![5, 5, 5]);
    t.delete_rbtree();
}

#[test]
fn test_large_insert_and_constraints() {
    let mut t = RBTree::new();
    for i in 0..100 {
        t.rbtree_insert(i);
    }
    check_rb(&t);
    let arr = t.to_array(100);
    let expected: Vec<i32> = (0..100).collect();
    assert_eq!(arr, expected);
    t.delete_rbtree();
}

#[test]
fn test_reverse_insert_and_constraints() {
    let mut t = RBTree::new();
    for i in (0..100).rev() {
        t.rbtree_insert(i);
    }
    check_rb(&t);
    let arr = t.to_array(100);
    let expected: Vec<i32> = (0..100).collect();
    assert_eq!(arr, expected);
    t.delete_rbtree();
}

fn main() {}
