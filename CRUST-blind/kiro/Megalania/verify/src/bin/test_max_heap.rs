use Megalania::max_heap::MaxHeap;

#[test]
fn test_empty_heap() {
    let heap = MaxHeap::new(10, Box::new(|a: u32, b: u32| a as i32 - b as i32));
    assert_eq!(heap.count(), 0);
    assert_eq!(heap.maximum(), None);
}

#[test]
fn test_insert_and_maximum() {
    let mut heap = MaxHeap::new(10, Box::new(|a: u32, b: u32| a as i32 - b as i32));
    assert!(heap.insert(5));
    assert_eq!(heap.count(), 1);
    assert_eq!(heap.maximum(), Some(5));
}

#[test]
fn test_sort_descending() {
    // C ground truth: insert [2,9,7,5,4,8,6,3,1,0], extract max -> 9,8,7,6,5,4,3,2,1,0
    let mut heap = MaxHeap::new(10, Box::new(|a: u32, b: u32| a as i32 - b as i32));
    let vals = [2u32, 9, 7, 5, 4, 8, 6, 3, 1, 0];
    for &v in &vals {
        assert!(heap.insert(v));
    }
    assert_eq!(heap.count(), 10);

    let expected = [9u32, 8, 7, 6, 5, 4, 3, 2, 1, 0];
    for &e in &expected {
        assert_eq!(heap.maximum(), Some(e));
        assert!(heap.remove_maximum());
    }
    assert_eq!(heap.count(), 0);
}

#[test]
fn test_insert_full() {
    let mut heap = MaxHeap::new(3, Box::new(|a: u32, b: u32| a as i32 - b as i32));
    assert!(heap.insert(1));
    assert!(heap.insert(2));
    assert!(heap.insert(3));
    assert!(!heap.insert(4)); // full
    assert_eq!(heap.count(), 3);
}

#[test]
fn test_remove_empty() {
    let mut heap = MaxHeap::new(10, Box::new(|a: u32, b: u32| a as i32 - b as i32));
    assert!(!heap.remove_maximum());
}

#[test]
fn test_update_maximum() {
    let mut heap = MaxHeap::new(10, Box::new(|a: u32, b: u32| a as i32 - b as i32));
    assert!(!heap.update_maximum()); // empty
    assert!(heap.insert(5));
    assert!(heap.update_maximum());
}

#[test]
fn test_clear() {
    let mut heap = MaxHeap::new(10, Box::new(|a: u32, b: u32| a as i32 - b as i32));
    heap.insert(1);
    heap.insert(2);
    heap.clear();
    assert_eq!(heap.count(), 0);
    assert_eq!(heap.maximum(), None);
}

#[test]
fn test_top_k() {
    // C ground truth: top-k test with backing store comparator
    // Insert 100 values, keep smallest 10 via max-heap of indices
    let mut backing: Vec<u32> = Vec::new();
    let data: Vec<u32> = {
        // Reproduce the C seed=666 shuffle for 100 elements
        // From C output: 2 49 23 97 60 17 31 3 67 33 90 65 0 47 56 22 5 39 19 72 76 66 99 94 68 93 88 84 26 21 79 6 24 38 11 18 74 96 28 82 87 69 92 42 30 70 80 75 4 83 25 8 35 43 63 15 9 95 85 41 46 89 14 34 91 58 48 36 16 71 62 78 55 10 86 13 50 98 61 81 64 1 73 77 40 37 20 45 52 27 51 29 59 44 57 7 12 32 53 54
        vec![2,49,23,97,60,17,31,3,67,33,90,65,0,47,56,22,5,39,19,72,76,66,99,94,68,93,88,84,26,21,79,6,24,38,11,18,74,96,28,82,87,69,92,42,30,70,80,75,4,83,25,8,35,43,63,15,9,95,85,41,46,89,14,34,91,58,48,36,16,71,62,78,55,10,86,13,50,98,61,81,64,1,73,77,40,37,20,45,52,27,51,29,59,44,57,7,12,32,53,54]
    };

    let heap_size = 10usize;
    // We need a comparator that compares backing[a] vs backing[b]
    // Since MaxHeap takes Box<dyn Fn>, we use a raw pointer approach similar to the Rust code
    let backing_ptr = &backing as *const Vec<u32> as usize;
    let mut heap = MaxHeap::new(heap_size, Box::new(move |a: u32, b: u32| {
        let b_ref = unsafe { &*(backing_ptr as *const Vec<u32>) };
        b_ref[a as usize] as i32 - b_ref[b as usize] as i32
    }));

    for &val in &data {
        if backing.len() < heap_size {
            let pos = backing.len() as u32;
            backing.push(val);
            heap.insert(pos);
        } else {
            if let Some(max_idx) = heap.maximum() {
                if val < backing[max_idx as usize] {
                    backing[max_idx as usize] = val;
                    heap.update_maximum();
                }
            }
        }
    }

    // Extract in order: should be 9,8,7,6,5,4,3,2,1,0
    let expected = [9u32, 8, 7, 6, 5, 4, 3, 2, 1, 0];
    for &e in &expected {
        let max_idx = heap.maximum().unwrap();
        assert_eq!(backing[max_idx as usize], e);
        heap.remove_maximum();
    }
}

fn main() {}
