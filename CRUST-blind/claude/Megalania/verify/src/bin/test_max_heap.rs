use Megalania::max_heap::MaxHeap;

fn make_heap(cap: usize) -> MaxHeap {
    MaxHeap::new(
        cap,
        Box::new(|a, b| (a as i64 - b as i64) as i32),
    )
}

#[test]
fn test_empty_heap() {
    let h = make_heap(5);
    assert_eq!(h.count(), 0);
    assert!(h.maximum().is_none());
}

#[test]
fn test_remove_on_empty() {
    let mut h = make_heap(5);
    assert!(!h.remove_maximum());
}

#[test]
fn test_update_on_empty() {
    let mut h = make_heap(5);
    assert!(!h.update_maximum());
}

#[test]
fn test_insert_and_max() {
    let mut h = make_heap(5);
    assert!(h.insert(3));
    assert!(h.insert(1));
    assert!(h.insert(5));
    assert!(h.insert(2));
    assert!(h.insert(4));
    assert_eq!(h.count(), 5);
    // Heap is full, additional insertion should fail
    assert!(!h.insert(6));
    assert_eq!(h.count(), 5);

    // Pop in descending order
    assert_eq!(h.maximum(), Some(5));
    assert!(h.remove_maximum());
    assert_eq!(h.maximum(), Some(4));
    assert!(h.remove_maximum());
    assert_eq!(h.maximum(), Some(3));
    assert!(h.remove_maximum());
    assert_eq!(h.maximum(), Some(2));
    assert!(h.remove_maximum());
    assert_eq!(h.maximum(), Some(1));
    assert!(h.remove_maximum());
    assert_eq!(h.count(), 0);
    assert!(h.maximum().is_none());
}

#[test]
fn test_clear() {
    let mut h = make_heap(5);
    h.insert(1);
    h.insert(2);
    assert_eq!(h.count(), 2);
    h.clear();
    assert_eq!(h.count(), 0);
    assert!(h.maximum().is_none());
}

#[test]
fn test_full_sort() {
    // A larger sort test via the heap
    let mut h = make_heap(10);
    let data = [2u32, 9, 7, 5, 4, 8, 6, 3, 1, 0];
    for &v in data.iter() {
        assert!(h.insert(v));
    }
    assert_eq!(h.count(), 10);
    // Pop and verify descending order: 9 8 7 6 5 4 3 2 1 0
    let expected = [9u32, 8, 7, 6, 5, 4, 3, 2, 1, 0];
    for &exp in expected.iter() {
        assert_eq!(h.maximum(), Some(exp));
        assert!(h.remove_maximum());
    }
    assert_eq!(h.count(), 0);
}

fn main() {}
