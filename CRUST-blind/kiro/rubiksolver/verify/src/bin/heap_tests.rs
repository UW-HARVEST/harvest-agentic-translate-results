use rubiksolver::heap::Heap;

#[test]
fn test_empty_heap() {
    let heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    assert_eq!(heap.is_empty(), true);
    assert!(heap.find_min().is_none());
}

#[test]
fn test_insert_and_find_min() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    heap.insert(50);
    heap.insert(10);
    heap.insert(30);
    heap.insert(20);
    heap.insert(40);
    assert_eq!(heap.is_empty(), false);
    assert_eq!(*heap.find_min().unwrap(), 10);
}

#[test]
fn test_delete_min_sorted_order() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    heap.insert(50);
    heap.insert(10);
    heap.insert(30);
    heap.insert(20);
    heap.insert(40);
    assert_eq!(heap.delete_min(), Some(10));
    assert_eq!(heap.delete_min(), Some(20));
    assert_eq!(heap.delete_min(), Some(30));
    assert_eq!(heap.delete_min(), Some(40));
    assert_eq!(heap.delete_min(), Some(50));
    assert_eq!(heap.is_empty(), true);
}

#[test]
fn test_delete_min_empty_returns_none() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    assert_eq!(heap.delete_min(), None);
}

#[test]
fn test_string_heap() {
    let mut heap: Heap<&str> = Heap::new(10, |a: &&str, b: &&str| *a < *b);
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
fn test_single_element() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    heap.insert(42);
    assert_eq!(*heap.find_min().unwrap(), 42);
    assert_eq!(heap.delete_min(), Some(42));
    assert_eq!(heap.is_empty(), true);
}

fn main() {}
