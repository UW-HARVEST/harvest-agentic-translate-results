use Bostree::bostree::{bostree_node_weak_ref, BOSTree, BOSNode};
use std::cell::RefCell;
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
fn test_weak_ref_increments_count() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let n = tree.bostree_insert(String::from("a"), None);
    assert_eq!(n.borrow().weak_ref_count, 1);
    let r1 = bostree_node_weak_ref(&n);
    assert_eq!(n.borrow().weak_ref_count, 2);
    assert!(Rc::ptr_eq(&r1, &n));
    let r2 = bostree_node_weak_ref(&n);
    assert_eq!(n.borrow().weak_ref_count, 3);
    assert!(Rc::ptr_eq(&r2, &n));
}

#[test]
fn test_weak_unref_decrements_count_returns_node_when_valid() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let n = tree.bostree_insert(String::from("a"), None);
    bostree_node_weak_ref(&n);
    assert_eq!(n.borrow().weak_ref_count, 2);
    // weak_ref_node_valid is 1, so unref returns Some(node)
    let result = tree.bostree_node_weak_unref(&n);
    assert!(result.is_some());
    assert!(Rc::ptr_eq(result.as_ref().unwrap(), &n));
    assert_eq!(n.borrow().weak_ref_count, 1);
}

#[test]
fn test_weak_unref_after_removal_returns_none() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let n = tree.bostree_insert(String::from("a"), None);
    bostree_node_weak_ref(&n);
    // n.weak_ref_count == 2 now
    tree.bostree_remove(&n);
    // bostree_remove invalidates the node and unrefs once.
    // weak_ref_count was 2, becomes 1; weak_ref_node_valid becomes 0.
    assert_eq!(n.borrow().weak_ref_count, 1);
    assert_eq!(n.borrow().weak_ref_node_valid, 0);
    // Calling unref now: count goes from 1 to 0; should return None and (in C) free the node.
    let result = tree.bostree_node_weak_unref(&n);
    assert!(result.is_none());
}

#[test]
fn test_weak_unref_invalid_count_nonzero_returns_none() {
    // After removal, with extra weak refs, unref returns None until count reaches 0.
    let mut tree = BOSTree::bostree_new(cmp, None);
    let n = tree.bostree_insert(String::from("a"), None);
    bostree_node_weak_ref(&n);
    bostree_node_weak_ref(&n); // count == 3
    tree.bostree_remove(&n);   // invalidates, count==2
    assert_eq!(n.borrow().weak_ref_count, 2);
    assert_eq!(n.borrow().weak_ref_node_valid, 0);
    let r = tree.bostree_node_weak_unref(&n);
    assert!(r.is_none());
    assert_eq!(n.borrow().weak_ref_count, 1);
    let r2 = tree.bostree_node_weak_unref(&n);
    assert!(r2.is_none());
    assert_eq!(n.borrow().weak_ref_count, 0);
}

#[test]
fn test_free_function_called_on_zero_count() {
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    static FREE_CALLS: AtomicUsize = AtomicUsize::new(0);
    fn free_fn(_node: &Rc<RefCell<BOSNode>>) {
        FREE_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
    }
    FREE_CALLS.store(0, AtomicOrdering::SeqCst);
    let mut tree = BOSTree::bostree_new(cmp, Some(free_fn));
    let n = tree.bostree_insert(String::from("a"), None);
    // Initial weak_ref_count == 1
    tree.bostree_remove(&n);
    // unref drops count from 1 to 0 -> free function called
    assert_eq!(FREE_CALLS.load(AtomicOrdering::SeqCst), 1);
}

fn main() {}
