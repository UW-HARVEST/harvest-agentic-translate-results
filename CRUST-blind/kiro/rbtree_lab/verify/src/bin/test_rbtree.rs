use rbtree::rbtree::{Color, RBTree};

fn insert_keys(t: &mut RBTree, keys: &[i32]) {
    for &k in keys {
        t.rbtree_insert(k);
    }
}

// === RB-tree structural validators ===

fn check_bst(t: &RBTree) -> bool {
    fn recurse(node: &Option<rbtree::rbtree::NodeRef>, min: i32, max: i32) -> bool {
        match node {
            None => true,
            Some(n) => {
                let b = n.borrow();
                // duplicates go right, so left <= key, right >= key
                recurse(&b.left, min, b.key) && recurse(&b.right, b.key, max)
            }
        }
    }
    recurse(&t.root, i32::MIN, i32::MAX)
}

fn check_rb_properties(t: &RBTree) -> bool {
    // Root must be black
    if let Some(ref r) = t.root {
        if r.borrow().color != Color::Black {
            return false;
        }
    }
    // No red-red parent-child, and equal black-height on all paths
    fn black_height(node: &Option<rbtree::rbtree::NodeRef>) -> Option<usize> {
        match node {
            None => Some(1), // NIL counts as black
            Some(n) => {
                let b = n.borrow();
                // red-red check
                if b.color == Color::Red {
                    if b.left.as_ref().map_or(false, |l| l.borrow().color == Color::Red) {
                        return None;
                    }
                    if b.right.as_ref().map_or(false, |r| r.borrow().color == Color::Red) {
                        return None;
                    }
                }
                let lh = black_height(&b.left)?;
                let rh = black_height(&b.right)?;
                if lh != rh {
                    return None;
                }
                Some(lh + if b.color == Color::Black { 1 } else { 0 })
            }
        }
    }
    black_height(&t.root).is_some()
}

// === Tests ===

#[test]
fn test_new_empty() {
    let t = RBTree::new();
    assert!(t.root.is_none());
}

#[test]
fn test_insert_single() {
    let mut t = RBTree::new();
    let p = t.rbtree_insert(42).unwrap();
    assert_eq!(p.borrow().key, 42);
    assert_eq!(p.borrow().color, Color::Black);
    assert!(t.root.is_some());
    let root = t.root.as_ref().unwrap();
    assert_eq!(root.borrow().key, 42);
    assert_eq!(root.borrow().color, Color::Black);
    assert!(root.borrow().left.is_none());
    assert!(root.borrow().right.is_none());
    assert!(root.borrow().parent.is_none());
}

#[test]
fn test_insert_sequence_to_array() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let arr = t.to_array(10);
    assert_eq!(arr, vec![2, 5, 8, 10, 12, 23, 24, 34, 67, 156]);
}

#[test]
fn test_insert_sequence_root_min_max() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let root = t.root.as_ref().unwrap();
    assert_eq!(root.borrow().key, 23);
    assert_eq!(root.borrow().color, Color::Black);
    assert_eq!(t.rbtree_min().unwrap().borrow().key, 2);
    assert_eq!(t.rbtree_max().unwrap().borrow().key, 156);
}

#[test]
fn test_ascending_insert() {
    let mut t = RBTree::new();
    for i in 1..=8 {
        t.rbtree_insert(i);
    }
    let arr = t.to_array(8);
    assert_eq!(arr, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(t.root.as_ref().unwrap().borrow().key, 4);
    assert_eq!(t.root.as_ref().unwrap().borrow().color, Color::Black);
}

#[test]
fn test_descending_insert() {
    let mut t = RBTree::new();
    for i in (1..=8).rev() {
        t.rbtree_insert(i);
    }
    let arr = t.to_array(8);
    assert_eq!(arr, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(t.root.as_ref().unwrap().borrow().key, 5);
    assert_eq!(t.root.as_ref().unwrap().borrow().color, Color::Black);
}

#[test]
fn test_find_existing() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[10, 20, 30]);
    let f = t.rbtree_find(20);
    assert!(f.is_some());
    assert_eq!(f.unwrap().borrow().key, 20);
}

#[test]
fn test_find_nonexistent() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[10, 20, 30]);
    assert!(t.rbtree_find(99).is_none());
}

#[test]
fn test_find_empty_tree() {
    let t = RBTree::new();
    assert!(t.rbtree_find(1).is_none());
}

#[test]
fn test_min_max_empty() {
    let t = RBTree::new();
    assert!(t.rbtree_min().is_none());
    assert!(t.rbtree_max().is_none());
}

#[test]
fn test_erase_root() {
    let mut t = RBTree::new();
    let p = t.rbtree_insert(128).unwrap();
    t.erase(p);
    assert!(t.root.is_none());
}

#[test]
fn test_erase_min_max() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let mn = t.rbtree_min().unwrap();
    assert_eq!(mn.borrow().key, 2);
    t.erase(mn);
    let mx = t.rbtree_max().unwrap();
    assert_eq!(mx.borrow().key, 156);
    t.erase(mx);
    let arr = t.to_array(8);
    assert_eq!(arr, vec![5, 8, 10, 12, 23, 24, 34, 67]);
    assert_eq!(t.rbtree_min().unwrap().borrow().key, 5);
    assert_eq!(t.rbtree_max().unwrap().borrow().key, 67);
}

#[test]
fn test_duplicates() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[10, 5, 5, 34, 6, 23, 12, 12, 6, 12]);
    let arr = t.to_array(10);
    assert_eq!(arr, vec![5, 5, 6, 6, 10, 12, 12, 12, 23, 34]);
}

#[test]
fn test_insert_tree_structure() {
    // Insert [11,2,14,1,7,15,5,8,4] -> specific tree from C ground truth
    let mut t = RBTree::new();
    insert_keys(&mut t, &[11, 2, 14, 1, 7, 15, 5, 8, 4]);
    let arr = t.to_array(9);
    assert_eq!(arr, vec![1, 2, 4, 5, 7, 8, 11, 14, 15]);

    let root = t.root.as_ref().unwrap();
    assert_eq!(root.borrow().key, 7);
    assert_eq!(root.borrow().color, Color::Black);
    assert!(root.borrow().parent.is_none());

    // root.left = 2 Red
    let rl = root.borrow().left.clone().unwrap();
    assert_eq!(rl.borrow().key, 2);
    assert_eq!(rl.borrow().color, Color::Red);

    // root.left.left = 1 Black (leaf)
    let rll = rl.borrow().left.clone().unwrap();
    assert_eq!(rll.borrow().key, 1);
    assert_eq!(rll.borrow().color, Color::Black);
    assert!(rll.borrow().left.is_none());
    assert!(rll.borrow().right.is_none());

    // root.left.right = 5 Black
    let rlr = rl.borrow().right.clone().unwrap();
    assert_eq!(rlr.borrow().key, 5);
    assert_eq!(rlr.borrow().color, Color::Black);

    // root.left.right.left = 4 Red (leaf)
    let rlrl = rlr.borrow().left.clone().unwrap();
    assert_eq!(rlrl.borrow().key, 4);
    assert_eq!(rlrl.borrow().color, Color::Red);
    assert!(rlrl.borrow().left.is_none());
    assert!(rlrl.borrow().right.is_none());

    // root.right = 11 Red
    let rr = root.borrow().right.clone().unwrap();
    assert_eq!(rr.borrow().key, 11);
    assert_eq!(rr.borrow().color, Color::Red);

    // root.right.left = 8 Black (leaf)
    let rrl = rr.borrow().left.clone().unwrap();
    assert_eq!(rrl.borrow().key, 8);
    assert_eq!(rrl.borrow().color, Color::Black);
    assert!(rrl.borrow().left.is_none());
    assert!(rrl.borrow().right.is_none());

    // root.right.right = 14 Black
    let rrr = rr.borrow().right.clone().unwrap();
    assert_eq!(rrr.borrow().key, 14);
    assert_eq!(rrr.borrow().color, Color::Black);
    assert!(rrr.borrow().left.is_none());

    // root.right.right.right = 15 Red (leaf)
    let rrrr = rrr.borrow().right.clone().unwrap();
    assert_eq!(rrrr.borrow().key, 15);
    assert_eq!(rrrr.borrow().color, Color::Red);
    assert!(rrrr.borrow().left.is_none());
    assert!(rrrr.borrow().right.is_none());
}

#[test]
fn test_find_erase_all() {
    let mut t = RBTree::new();
    let keys = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12, 24, 36, 990, 25];
    insert_keys(&mut t, &keys);
    for &k in &keys {
        let p = t.rbtree_find(k);
        assert!(p.is_some(), "should find key {}", k);
        t.erase(p.unwrap());
    }
    assert!(t.root.is_none());
}

#[test]
fn test_to_array_partial() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[50, 30, 70, 20, 40, 60, 80]);
    let arr = t.to_array(3);
    assert_eq!(arr, vec![20, 30, 40]);
}

#[test]
fn test_to_array_empty() {
    let t = RBTree::new();
    let arr = t.to_array(0);
    assert!(arr.is_empty());
}

#[test]
fn test_multi_instance() {
    let mut t1 = RBTree::new();
    let mut t2 = RBTree::new();
    insert_keys(&mut t1, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12, 24, 36, 990, 25]);
    insert_keys(&mut t2, &[4, 8, 10, 5, 3]);
    let arr1 = t1.to_array(14);
    let arr2 = t2.to_array(5);
    assert_eq!(arr1, vec![2, 5, 8, 10, 12, 23, 24, 24, 25, 34, 36, 67, 156, 990]);
    assert_eq!(arr2, vec![3, 4, 5, 8, 10]);
}

#[test]
fn test_rb_constraints_distinct() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    assert!(check_bst(&t));
    assert!(check_rb_properties(&t));
}

#[test]
fn test_rb_constraints_duplicates() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[10, 5, 5, 34, 6, 23, 12, 12, 6, 12]);
    assert!(check_bst(&t));
    assert!(check_rb_properties(&t));
}

#[test]
fn test_rb_constraints_after_erase() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    let mn = t.rbtree_min().unwrap();
    t.erase(mn);
    assert!(check_bst(&t));
    assert!(check_rb_properties(&t));
    let mx = t.rbtree_max().unwrap();
    t.erase(mx);
    assert!(check_bst(&t));
    assert!(check_rb_properties(&t));
}

#[test]
fn test_erase_insert_erase_pattern() {
    // Insert each key, find it, erase it, confirm gone
    let mut t = RBTree::new();
    let keys = [10, 5, 8, 34, 67, 23, 156, 24, 2, 12, 24, 36, 990, 25];
    for &k in &keys {
        let p = t.rbtree_insert(k).unwrap();
        let q = t.rbtree_find(k).unwrap();
        assert_eq!(q.borrow().key, k);
        t.erase(p);
        assert!(t.rbtree_find(k).is_none());
    }
}

#[test]
fn test_delete_rbtree() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[10, 20, 30, 40, 50]);
    t.delete_rbtree(); // should not panic
}

#[test]
fn test_minmax_after_erase() {
    let mut t = RBTree::new();
    insert_keys(&mut t, &[10, 5, 8, 34, 67, 23, 156, 24, 2, 12]);
    // sorted: [2,5,8,10,12,23,24,34,67,156]
    let mn = t.rbtree_min().unwrap();
    assert_eq!(mn.borrow().key, 2);
    t.erase(mn);
    assert_eq!(t.rbtree_min().unwrap().borrow().key, 5);
    let mx = t.rbtree_max().unwrap();
    assert_eq!(mx.borrow().key, 156);
    t.erase(mx);
    assert_eq!(t.rbtree_max().unwrap().borrow().key, 67);
}

fn main() {}
