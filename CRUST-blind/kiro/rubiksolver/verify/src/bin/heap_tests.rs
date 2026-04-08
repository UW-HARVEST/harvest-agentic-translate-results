use rubiksolver::heap::Heap;

#[test]
fn test_empty_heap() {
    let heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    assert!(heap.is_empty());
}

#[test]
fn test_insert_makes_nonempty() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    heap.insert(1);
    assert!(!heap.is_empty());
}

#[test]
fn test_string_heap_ordering() {
    let mut heap: Heap<String> = Heap::new(10, |a: &String, b: &String| a < b);
    heap.insert("charlie".to_string());
    heap.insert("alpha".to_string());
    heap.insert("bravo".to_string());
    assert!(!heap.is_empty());
    assert_eq!(heap.delete_min().unwrap(), "alpha");
    assert_eq!(heap.delete_min().unwrap(), "bravo");
    assert_eq!(heap.delete_min().unwrap(), "charlie");
    assert!(heap.is_empty());
}

#[test]
fn test_int_heap_ordering() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    heap.insert(30);
    heap.insert(20);
    heap.insert(10);
    assert_eq!(heap.delete_min().unwrap(), 10);
}

#[test]
fn test_find_min() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    heap.insert(5);
    heap.insert(3);
    heap.insert(7);
    assert_eq!(*heap.find_min().unwrap(), 3);
}

#[test]
fn test_delete_min_empty_returns_none() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    assert!(heap.delete_min().is_none());
}

#[test]
fn test_many_inserts_descending() {
    let mut heap: Heap<i32> = Heap::new(10, |a, b| a < b);
    for i in (11..=30).rev() {
        heap.insert(i);
    }
    assert_eq!(heap.delete_min().unwrap(), 11);
}

fn main() {}
