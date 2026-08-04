use Bostree::bostree::{
    bostree_next_node, bostree_previous_node, bostree_rank, BOSTree,
};
use std::rc::Rc;

fn cmp(a: &str, b: &str) -> i32 {
    use std::cmp::Ordering;
    match a.cmp(b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[test]
fn test_next_previous_at_boundaries() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    for k in &["c", "a", "b", "d", "e"] {
        tree.bostree_insert(k.to_string(), None);
    }
    let first = tree.bostree_select(0).unwrap();
    let last = tree.bostree_select(4).unwrap();
    assert_eq!(first.borrow().key, "a");
    assert_eq!(last.borrow().key, "e");
    assert!(bostree_previous_node(&first).is_none());
    assert!(bostree_next_node(&last).is_none());
}

#[test]
fn test_next_walks_full_inorder() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    for k in &["c", "a", "b", "d", "e"] {
        tree.bostree_insert(k.to_string(), None);
    }
    let mut keys = Vec::new();
    let mut current = tree.bostree_select(0);
    while let Some(n) = current {
        keys.push(n.borrow().key.clone());
        current = bostree_next_node(&n);
    }
    assert_eq!(keys, vec!["a", "b", "c", "d", "e"]);
}

#[test]
fn test_previous_walks_full_inorder_reverse() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    for k in &["c", "a", "b", "d", "e"] {
        tree.bostree_insert(k.to_string(), None);
    }
    let last = tree.bostree_select(tree.bostree_node_count() - 1).unwrap();
    let mut keys = Vec::new();
    let mut current = Some(last);
    while let Some(n) = current {
        keys.push(n.borrow().key.clone());
        current = bostree_previous_node(&n);
    }
    assert_eq!(keys, vec!["e", "d", "c", "b", "a"]);
}

#[test]
fn test_select_rank_round_trip() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let keys = ["mango", "apple", "banana", "kiwi", "orange", "fig", "grape"];
    for k in keys.iter() {
        tree.bostree_insert(k.to_string(), None);
    }
    let n = tree.bostree_node_count();
    for i in 0..n {
        let node = tree.bostree_select(i).unwrap();
        let rank = bostree_rank(&node);
        assert_eq!(rank, i);
        let n2 = tree.bostree_select(rank).unwrap();
        assert!(Rc::ptr_eq(&node, &n2));
    }
}

#[test]
fn test_lookup_returns_same_pointer_as_insert() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let inserted = tree.bostree_insert(String::from("xyz"), Some(String::from("d")));
    let found = tree.bostree_lookup("xyz").unwrap();
    assert!(Rc::ptr_eq(&inserted, &found));
}

#[test]
fn test_select_returns_same_pointer_as_lookup() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    for k in &["b", "a", "c"] {
        tree.bostree_insert(k.to_string(), None);
    }
    let by_lookup = tree.bostree_lookup("b").unwrap();
    let by_select = tree.bostree_select(1).unwrap();
    assert!(Rc::ptr_eq(&by_lookup, &by_select));
}

#[test]
fn test_node_count_returns_zero_for_new_tree() {
    let tree = BOSTree::bostree_new(cmp, None);
    assert_eq!(tree.bostree_node_count(), 0);
}

#[test]
fn test_rank_for_root_only_tree() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let n = tree.bostree_insert(String::from("solo"), None);
    assert_eq!(bostree_rank(&n), 0);
}

fn main() {}
