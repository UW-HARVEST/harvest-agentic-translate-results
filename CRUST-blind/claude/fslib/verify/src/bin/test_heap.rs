use fslib::heap::{left, parent, right, Heap};
use std::cmp::Ordering;

fn min_cmp_i32(a: &i32, b: &i32) -> Ordering {
    a.cmp(b)
}

#[test]
fn test_heap_index_ops() {
    // parent(0) wraps to usize::MAX
    assert_eq!(parent(0), usize::MAX);
    assert_eq!(parent(1), 0);
    assert_eq!(parent(2), 0);
    assert_eq!(parent(3), 1);
    assert_eq!(parent(4), 1);
    assert_eq!(parent(5), 2);
    assert_eq!(parent(6), 2);
    assert_eq!(left(0), 1);
    assert_eq!(left(1), 3);
    assert_eq!(left(2), 5);
    assert_eq!(right(0), 2);
    assert_eq!(right(1), 4);
    assert_eq!(right(2), 6);
}

#[test]
fn test_heap_init_size_default() {
    // heap_create(.., init_size=0, limit=0) => n_max=255 (HEAP_INIT_SIZE)
    let h: Heap<i32> = Heap::new(min_cmp_i32, 0, 0, 0);
    assert_eq!(h.n_max, 255);
    assert_eq!(h.n_items, 0);
    assert_eq!(h.limit, 0);
}

#[test]
fn test_heap_init_size_explicit() {
    // heap_create(.., init_size=64, limit=0) => n_max=64
    let h: Heap<i32> = Heap::new(min_cmp_i32, 0, 64, 0);
    assert_eq!(h.n_max, 64);
}

#[test]
fn test_heap_init_size_with_limit() {
    // heap_create(.., init_size=0, limit=10) => n_max=11
    let h: Heap<i32> = Heap::new(min_cmp_i32, 0, 0, 10);
    assert_eq!(h.n_max, 11);
    assert_eq!(h.limit, 10);
}

#[test]
fn test_heap_insert_pop_min_order() {
    let mut h: Heap<i32> = Heap::new(min_cmp_i32, 0, 0, 0);
    let items = vec![5, 3, 9, 1, 7, 2, 8, 4, 6, 0];
    for v in &items {
        h.insert(*v);
    }
    assert_eq!(h.n_items, 10);
    let mut out = Vec::new();
    while let Some(v) = h.pop() {
        out.push(v);
    }
    assert_eq!(out, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn test_heap_pop_empty() {
    let mut h: Heap<i32> = Heap::new(min_cmp_i32, 0, 0, 0);
    assert_eq!(h.pop(), None);
}

#[test]
fn test_heap_single_item() {
    let mut h: Heap<i32> = Heap::new(min_cmp_i32, 0, 0, 0);
    h.insert(42);
    assert_eq!(h.n_items, 1);
    assert_eq!(h.pop(), Some(42));
    assert_eq!(h.n_items, 0);
    assert_eq!(h.pop(), None);
}

#[test]
fn test_heap_index_find() {
    let mut h: Heap<i32> = Heap::new(min_cmp_i32, 0, 0, 0);
    for v in [3, 1, 4, 1, 5, 9].iter() {
        h.insert(*v);
    }
    h.index(|x| *x as u64, |a, b| a == b);
    // Find: returns index in heap, but exact location depends on heap layout
    // We just verify that the index can find existing items
    let r = h.find(&3);
    assert!(r.is_some());
    let r = h.find(&100);
    assert!(r.is_none());
}

fn main() {}
