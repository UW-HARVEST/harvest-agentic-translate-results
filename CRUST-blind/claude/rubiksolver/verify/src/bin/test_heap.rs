use rubiksolver::heap::Heap;

#[test]
fn test_heap_empty_initial() {
    let heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    assert_eq!(heap.is_empty(), true);
    assert_eq!(heap.find_min(), None);
}

#[test]
fn test_heap_string_min_first() {
    let mut heap: Heap<&str> = Heap::new(10, |a, b| a < b);
    assert_eq!(heap.is_empty(), true);
    heap.insert("charlie");
    heap.insert("alpha");
    heap.insert("bravo");
    assert_eq!(heap.is_empty(), false);

    assert_eq!(heap.delete_min(), Some("alpha"));
    assert_eq!(heap.delete_min(), Some("bravo"));
    assert_eq!(heap.delete_min(), Some("charlie"));
    assert_eq!(heap.is_empty(), true);
}

#[test]
fn test_heap_int_descending_inserts() {
    // Insert 30, 29, 28, ..., 11 -> heap_delete_min should give 11
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    for i in (11..=30).rev() {
        heap.insert(i);
    }
    assert_eq!(heap.delete_min(), Some(11));
}

#[test]
fn test_heap_grow_beyond_initial_capacity() {
    // Initial capacity 10, but should grow as needed
    let mut heap: Heap<i32> = Heap::new(2, |a, b| a < b);
    for i in (1..=20).rev() {
        heap.insert(i);
    }
    let mut out = Vec::new();
    while !heap.is_empty() {
        out.push(heap.delete_min().unwrap());
    }
    let expected: Vec<i32> = (1..=20).collect();
    assert_eq!(out, expected);
}

#[test]
fn test_heap_find_min_after_inserts() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    heap.insert(5);
    assert_eq!(heap.find_min(), Some(&5));
    heap.insert(3);
    assert_eq!(heap.find_min(), Some(&3));
    heap.insert(10);
    assert_eq!(heap.find_min(), Some(&3));
    heap.insert(1);
    assert_eq!(heap.find_min(), Some(&1));
}

#[test]
fn test_heap_delete_min_returns_in_order() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    let values = vec![7, 3, 9, 1, 5, 8, 2, 6, 4];
    for v in &values {
        heap.insert(*v);
    }
    let mut sorted = values.clone();
    sorted.sort();
    let mut out = Vec::new();
    while !heap.is_empty() {
        out.push(heap.delete_min().unwrap());
    }
    assert_eq!(out, sorted);
}

#[test]
fn test_heap_delete_min_empty_returns_none() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    assert_eq!(heap.delete_min(), None);
}

#[test]
fn test_heap_max_heap_using_inverted_comparator() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a > b);
    heap.insert(5);
    heap.insert(15);
    heap.insert(7);
    assert_eq!(heap.delete_min(), Some(15));
    assert_eq!(heap.delete_min(), Some(7));
    assert_eq!(heap.delete_min(), Some(5));
}

#[test]
fn test_heap_single_element() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    heap.insert(42);
    assert_eq!(heap.is_empty(), false);
    assert_eq!(heap.find_min(), Some(&42));
    assert_eq!(heap.delete_min(), Some(42));
    assert_eq!(heap.is_empty(), true);
    assert_eq!(heap.find_min(), None);
}

#[test]
fn test_heap_duplicates() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    heap.insert(3);
    heap.insert(3);
    heap.insert(3);
    heap.insert(1);
    heap.insert(1);
    assert_eq!(heap.delete_min(), Some(1));
    assert_eq!(heap.delete_min(), Some(1));
    assert_eq!(heap.delete_min(), Some(3));
    assert_eq!(heap.delete_min(), Some(3));
    assert_eq!(heap.delete_min(), Some(3));
}

fn main() {}
