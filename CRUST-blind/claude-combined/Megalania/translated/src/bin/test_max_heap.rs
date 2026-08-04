use Megalania::max_heap::MaxHeap;

#[test]
fn test_count_initially_zero() {
    let h = MaxHeap::new(10, Box::new(|a, b| (a as i32) - (b as i32)));
    assert_eq!(h.count(), 0);
}

#[test]
fn test_insert_and_max() {
    let mut h = MaxHeap::new(10, Box::new(|a, b| (a as i32) - (b as i32)));
    assert!(h.insert(5));
    assert_eq!(h.count(), 1);
    assert_eq!(h.maximum(), Some(5));
}

#[test]
fn test_insert_multiple_returns_max() {
    let mut h = MaxHeap::new(10, Box::new(|a, b| (a as i32) - (b as i32)));
    let values = [3, 1, 4, 1, 5, 9, 2, 6];
    for v in values.iter() {
        h.insert(*v);
    }
    assert_eq!(h.count(), values.len());
    assert_eq!(h.maximum(), Some(9));
}

#[test]
fn test_capacity_full_returns_false() {
    let mut h = MaxHeap::new(3, Box::new(|a, b| (a as i32) - (b as i32)));
    assert!(h.insert(1));
    assert!(h.insert(2));
    assert!(h.insert(3));
    assert_eq!(h.count(), 3);
    assert!(!h.insert(4)); // full
    assert_eq!(h.count(), 3);
}

#[test]
fn test_remove_maximum_orders_descending() {
    let mut h = MaxHeap::new(10, Box::new(|a, b| (a as i32) - (b as i32)));
    let values = [3, 1, 4, 1, 5, 9, 2, 6];
    for v in values.iter() {
        h.insert(*v);
    }
    let mut sorted = Vec::new();
    while let Some(m) = h.maximum() {
        sorted.push(m);
        h.remove_maximum();
    }
    assert_eq!(sorted, vec![9, 6, 5, 4, 3, 2, 1, 1]);
    assert_eq!(h.count(), 0);
}

#[test]
fn test_remove_maximum_empty_returns_false() {
    let mut h = MaxHeap::new(10, Box::new(|a, b| (a as i32) - (b as i32)));
    assert!(!h.remove_maximum());
}

#[test]
fn test_max_empty_returns_none() {
    let h = MaxHeap::new(10, Box::new(|a, b| (a as i32) - (b as i32)));
    assert_eq!(h.maximum(), None);
}

#[test]
fn test_clear() {
    let mut h = MaxHeap::new(10, Box::new(|a, b| (a as i32) - (b as i32)));
    h.insert(1);
    h.insert(2);
    h.insert(3);
    h.clear();
    assert_eq!(h.count(), 0);
    assert_eq!(h.maximum(), None);
}

#[test]
fn test_update_maximum_empty_returns_false() {
    let mut h = MaxHeap::new(10, Box::new(|a, b| (a as i32) - (b as i32)));
    assert!(!h.update_maximum());
}

#[test]
fn test_sort_10_descending() {
    // Like C max_heap_sort_test: insert values 0..9 in some order, pop them
    // and confirm we get them descending.
    let mut h = MaxHeap::new(10, Box::new(|a, b| (a as i32) - (b as i32)));
    for v in [2u32, 9, 7, 5, 4, 8, 6, 3, 1, 0].iter() {
        h.insert(*v);
    }
    assert_eq!(h.count(), 10);
    let mut popped = Vec::new();
    while let Some(m) = h.maximum() {
        popped.push(m);
        h.remove_maximum();
    }
    assert_eq!(popped, vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
    assert_eq!(h.count(), 0);
}

fn main() {}
