use fslib::heap::{Heap, parent, left, right};
use std::cmp::Ordering;

fn min_cmp(a: &i32, b: &i32) -> Ordering {
    a.cmp(b)
}

#[test]
fn test_indices() {
    assert_eq!(left(0), 1);
    assert_eq!(right(0), 2);
    assert_eq!(parent(1), 0);
    assert_eq!(parent(2), 0);
    assert_eq!(left(1), 3);
    assert_eq!(right(1), 4);
}

#[test]
fn test_min_heap() {
    let mut h: Heap<i32> = Heap::new(min_cmp, 4, 0, 0);
    h.insert(5);
    h.insert(3);
    h.insert(8);
    h.insert(1);
    h.insert(7);
    assert_eq!(h.pop(), Some(1));
    assert_eq!(h.pop(), Some(3));
    assert_eq!(h.pop(), Some(5));
    assert_eq!(h.pop(), Some(7));
    assert_eq!(h.pop(), Some(8));
    assert_eq!(h.pop(), None);
}

fn main() {}
