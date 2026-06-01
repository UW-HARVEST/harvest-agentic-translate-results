use rubiksolver::heap::Heap;

fn str_smaller(a: &&'static str, b: &&'static str) -> bool {
    *a < *b
}

fn int_smaller(a: &i32, b: &i32) -> bool {
    *a < *b
}

#[test]
fn test_heap_empty() {
    let h: Heap<&'static str> = Heap::new(10, str_smaller);
    assert_eq!(h.is_empty(), true);
    assert!(h.find_min().is_none());
}

#[test]
fn test_heap_insert_and_min() {
    let mut h: Heap<&'static str> = Heap::new(10, str_smaller);
    h.insert("charlie");
    h.insert("alpha");
    h.insert("bravo");

    assert_eq!(h.is_empty(), false);
    assert_eq!(*h.find_min().unwrap(), "alpha");

    let v = h.delete_min().unwrap();
    assert_eq!(v, "alpha");
    let v = h.delete_min().unwrap();
    assert_eq!(v, "bravo");
    let v = h.delete_min().unwrap();
    assert_eq!(v, "charlie");

    assert_eq!(h.is_empty(), true);
    assert!(h.delete_min().is_none());
}

#[test]
fn test_heap_int_descending_inserts() {
    let mut h: Heap<i32> = Heap::new(10, int_smaller);
    // Insert 30 down to 11
    for i in (11..=30).rev() {
        h.insert(i);
    }
    // First min should be 11 (smallest of inserted)
    assert_eq!(*h.find_min().unwrap(), 11);
    let v = h.delete_min().unwrap();
    assert_eq!(v, 11);
    let v = h.delete_min().unwrap();
    assert_eq!(v, 12);
}

#[test]
fn test_heap_grow_beyond_capacity() {
    // init_size=2 but insert many elements
    let mut h: Heap<i32> = Heap::new(2, int_smaller);
    for i in (1..=20).rev() {
        h.insert(i);
    }
    // Pop them in ascending order
    for i in 1..=20 {
        let v = h.delete_min().unwrap();
        assert_eq!(v, i);
    }
    assert_eq!(h.is_empty(), true);
}

#[test]
fn test_heap_full_extraction_sorted() {
    let mut h: Heap<i32> = Heap::new(10, int_smaller);
    let input = [5, 3, 8, 1, 9, 2, 7, 4, 6, 0];
    for v in input.iter() {
        h.insert(*v);
    }
    let mut out = Vec::new();
    while !h.is_empty() {
        out.push(h.delete_min().unwrap());
    }
    assert_eq!(out, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

fn main() {}
