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
fn test_new_empty_tree() {
    let tree = BOSTree::bostree_new(cmp, None);
    assert_eq!(tree.bostree_node_count(), 0);
    assert!(tree.root_node.is_none());
    assert!(tree.bostree_lookup("X").is_none());
    assert!(tree.bostree_select(0).is_none());
}

#[test]
fn test_insert_single_node() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let node = tree.bostree_insert(String::from("hello"), Some(String::from("world")));
    assert_eq!(tree.bostree_node_count(), 1);
    {
        let nb = node.borrow();
        assert_eq!(nb.left_child_count, 0);
        assert_eq!(nb.right_child_count, 0);
        assert_eq!(nb.depth, 0);
        assert!(nb.left_child_node.is_none());
        assert!(nb.right_child_node.is_none());
        assert!(nb.parent_node.is_none());
        assert_eq!(nb.key, "hello");
        assert_eq!(nb.data.as_deref(), Some("world"));
        assert_eq!(nb.weak_ref_count, 1);
        assert_eq!(nb.weak_ref_node_valid, 1);
    }
    let root = tree.root_node.as_ref().unwrap();
    assert!(Rc::ptr_eq(root, &node));
    // rank
    assert_eq!(bostree_rank(&node), 0);
    // select(0) returns it
    let sel = tree.bostree_select(0).unwrap();
    assert!(Rc::ptr_eq(&sel, &node));
    // lookup
    let look = tree.bostree_lookup("hello").unwrap();
    assert!(Rc::ptr_eq(&look, &node));
    // out of range
    assert!(tree.bostree_select(1).is_none());
    assert!(tree.bostree_lookup("missing").is_none());
    // next/previous
    assert!(bostree_next_node(&node).is_none());
    assert!(bostree_previous_node(&node).is_none());
}

#[test]
fn test_lookup_missing_returns_none() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert(String::from("apple"), None);
    tree.bostree_insert(String::from("banana"), None);
    assert!(tree.bostree_lookup("cherry").is_none());
    assert!(tree.bostree_lookup("").is_none());
}

#[test]
fn test_node_count_after_inserts() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    assert_eq!(tree.bostree_node_count(), 0);
    tree.bostree_insert(String::from("a"), None);
    assert_eq!(tree.bostree_node_count(), 1);
    tree.bostree_insert(String::from("b"), None);
    assert_eq!(tree.bostree_node_count(), 2);
    tree.bostree_insert(String::from("c"), None);
    assert_eq!(tree.bostree_node_count(), 3);
}

#[test]
fn test_select_out_of_range() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert(String::from("a"), None);
    tree.bostree_insert(String::from("b"), None);
    assert!(tree.bostree_select(2).is_none());
    assert!(tree.bostree_select(100).is_none());
}

fn main() {}
