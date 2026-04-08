use fslib::heap::Heap;
use std::cmp::Ordering;

fn min_cmp(a: &i32, b: &i32) -> Ordering {
    a.cmp(b)
}

#[test]
fn test_min_heap_pop_order() {
    let mut h = Heap::new(min_cmp, 0, 0, 0);
    for &v in &[5, 3, 8, 1, 4] {
        h.insert(v);
    }
    assert_eq!(h.n_items, 5);
    assert_eq!(h.pop(), Some(1));
    assert_eq!(h.pop(), Some(3));
    assert_eq!(h.pop(), Some(4));
    assert_eq!(h.pop(), Some(5));
    assert_eq!(h.pop(), Some(8));
    assert_eq!(h.pop(), None);
}

#[test]
fn test_heap_with_limit() {
    let mut h = Heap::new(min_cmp, 0, 0, 3);
    for &v in &[5, 3, 8, 1, 4, 2] {
        h.insert(v);
    }
    assert_eq!(h.n_items, 3);
    assert_eq!(h.pop(), Some(1));
    assert_eq!(h.pop(), Some(2));
    assert_eq!(h.pop(), Some(3));
}

#[test]
fn test_heap_find() {
    let mut h = Heap::new(min_cmp, 0, 0, 0);
    h.insert(10);
    h.insert(20);
    h.insert(5);
    assert!(h.find(&5).is_some());
    assert!(h.find(&99).is_none());
}

fn main() {}
