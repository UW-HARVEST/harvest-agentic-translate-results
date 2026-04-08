use cset::cset::*;

// --- Init tests ---

#[test]
fn test_init() {
    let s: Cset<i32> = Cset::new();
    assert_eq!(s.get_size(), 0);
    assert_eq!(s.capacity(), CSET_INITIAL_CAP as i32);
    assert_eq!(s.get_seed(), CSET_DEFAULT_SEED);
    assert!((s.get_max_load_factor() - CSET_MAX_LOAD_FACTOR).abs() < f64::EPSILON);
    assert!((s.get_min_load_factor() - CSET_MIN_LOAD_FACTOR).abs() < f64::EPSILON);
}

// --- Add tests ---

#[test]
fn test_add() {
    let mut s: Cset<i32> = Cset::new();
    s.add(34);
    assert_eq!(s.get_size(), 1);
    s.add(35);
    assert_eq!(s.get_size(), 2);
    s.add(36);
    s.add(37);
    s.add(38);
    assert_eq!(s.get_size(), 5);
}

// --- Contains tests ---

#[test]
fn test_contains() {
    let mut s: Cset<i32> = Cset::new();
    s.add(34);
    s.add(36);
    s.remove(36);

    assert!(!s.contains(&12));
    assert!(s.contains(&34));

    s.add(50);
    assert!(!s.contains(&45));
    assert_eq!(s.get_size(), 2);
}

// --- Unique tests ---

#[test]
fn test_unique() {
    let mut s: Cset<i32> = Cset::new();
    s.add(45);
    s.add(46);
    s.add(57);
    assert_eq!(s.get_size(), 3);
    s.add(45);
    assert_eq!(s.get_size(), 3);
}

// --- Struct tests ---

#[derive(Copy, Clone, Default, PartialEq)]
struct Node {
    x: i32,
    y: i32,
}

#[test]
fn test_struct() {
    let mut s: Cset<Node> = Cset::new();
    s.add(Node { x: 4, y: 4 });
    assert_eq!(s.get_size(), 1);
    s.add(Node { x: 5, y: 4 });
    assert_eq!(s.get_size(), 2);
    s.add(Node { x: 5, y: 4 });
    assert_eq!(s.get_size(), 2);
    s.add(Node { x: 5, y: 8 });
    assert_eq!(s.get_size(), 3);
}

// --- Remove tests ---

#[test]
fn test_remove() {
    let mut s: Cset<i32> = Cset::new();
    s.add(45);
    s.add(34);
    s.add(10);
    assert_eq!(s.get_size(), 3);

    s.remove(45);
    assert_eq!(s.get_size(), 2);

    // removing again should be no-op
    s.remove(45);
    assert_eq!(s.get_size(), 2);

    s.remove(34);
    assert_eq!(s.get_size(), 1);

    // iterate and check only 10 remains
    let items = s.iter();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0], 10);

    s.remove(10);
    assert_eq!(s.get_size(), 0);
}

// --- Resize tests ---

#[test]
fn test_resize() {
    let mut s: Cset<i32> = Cset::new();
    for i in 0..1500 {
        s.add(i);
    }
    assert_eq!(s.get_size(), 1500);
    for i in 0..1500 {
        assert!(s.contains(&i));
    }
}

// --- Default bytes comparator tests ---

#[test]
fn test_default_bytes_comparator() {
    let mut s: Cset<i32> = Cset::new();
    s.add(45);
    s.add(46);
    s.add(67);

    assert!(s.contains(&45));
    assert!(!s.contains(&68));
    assert!(s.contains(&46));

    s.remove(46);
    assert!(!s.contains(&46));

    s.remove(46);
    assert!(!s.contains(&46));

    assert_eq!(s.get_size(), 2);

    s.remove(45);
    assert_eq!(s.get_size(), 1);

    s.remove(67);
    assert_eq!(s.get_size(), 0);

    s.remove(67);
    assert_eq!(s.get_size(), 0);

    for i in 0..2000 {
        s.add(i);
    }
    assert_eq!(s.get_size(), 2000);
}

// --- Clear tests ---

#[test]
fn test_clear() {
    let mut s: Cset<i32> = Cset::new();
    s.add(12);
    s.add(14);
    s.add(15);
    assert_eq!(s.get_size(), 3);

    s.clear();
    assert_eq!(s.get_size(), 0);

    s.add(45);
    assert_eq!(s.get_size(), 1);
}

// --- Intersection tests ---

#[test]
fn test_intersection() {
    let mut a: Cset<i32> = Cset::new();
    let mut b: Cset<i32> = Cset::new();

    a.add(12);
    a.add(13);
    a.add(14);

    b.add(12);
    b.add(13);
    b.add(16);

    let mut result: Cset<i32> = Cset::new();
    result.intersect(&a, &b);
    assert_eq!(result.get_size(), 2);

    b.add(14);
    result.intersect(&a, &b);
    assert_eq!(result.get_size(), 3);
}

// --- Union tests ---

#[test]
fn test_union() {
    let mut a: Cset<i32> = Cset::new();
    let mut b: Cset<i32> = Cset::new();

    a.add(34);
    a.add(25);
    a.add(12);

    b.add(1);
    b.add(4);
    b.add(34);

    let mut result: Cset<i32> = Cset::new();
    result.union(&a, &b);
    assert_eq!(result.get_size(), 5);

    b.add(100);
    result.union(&a, &b);
    assert_eq!(result.get_size(), 6);
}

// --- Disjoint tests ---

#[test]
fn test_disjoint() {
    let mut a: Cset<i8> = Cset::new();
    let mut b: Cset<i8> = Cset::new();

    a.add(b'a' as i8);
    a.add(b'b' as i8);

    b.add(b'c' as i8);
    b.add(b'd' as i8);

    assert!(a.is_disjoint(&b));

    b.add(b'a' as i8);
    assert!(!a.is_disjoint(&b));
}

// --- Difference tests ---

#[test]
fn test_difference() {
    let mut a: Cset<i32> = Cset::new();
    let mut b: Cset<i32> = Cset::new();
    let mut result: Cset<i32> = Cset::new();

    // empty difference
    result.difference(&a, &b);
    assert_eq!(result.get_size(), 0);

    a.add(45);
    a.add(46);
    a.add(58);

    b.add(12);
    b.add(11);
    b.add(45);

    result.difference(&a, &b);
    assert_eq!(result.get_size(), 2);

    assert!(result.contains(&46));
    assert!(result.contains(&58));
    assert!(!result.contains(&45));

    result.clear();

    b.add(46);
    b.add(58);

    result.difference(&a, &b);
    assert_eq!(result.get_size(), 0);

    result.difference(&b, &a);
    assert_eq!(result.get_size(), 2);
}

// --- Iteration tests ---

#[test]
fn test_iteration() {
    let mut s: Cset<i32> = Cset::new();
    for i in 0..3200 {
        s.add(i);
    }
    let items = s.iter();
    assert_eq!(items.len(), 3200);
    for item in &items {
        assert!(s.contains(item));
    }
}

// --- Setters tests ---

#[test]
fn test_setters() {
    let mut s: Cset<i32> = Cset::new();
    s.set_seed(12345);
    assert_eq!(s.get_seed(), 12345);
    s.set_max_load_factor(0.9);
    assert!((s.get_max_load_factor() - 0.9).abs() < f64::EPSILON);
    s.set_min_load_factor(0.1);
    assert!((s.get_min_load_factor() - 0.1).abs() < f64::EPSILON);
}

// --- size() and capacity() ---

#[test]
fn test_size_and_capacity() {
    let mut s: Cset<i32> = Cset::new();
    assert_eq!(s.size(), 0);
    assert_eq!(s.capacity(), 2);
    s.add(1);
    assert_eq!(s.size(), 1);
}

// --- empty() ---

#[test]
fn test_empty() {
    let mut s: Cset<i32> = Cset::new();
    assert!(s.empty());
    s.add(1);
    assert!(!s.empty());
    s.remove(1);
    assert!(s.empty());
}

// --- xxh hash function tests ---

#[test]
fn test_xxh64_round_fn() {
    let r = xxh64_round(0, 0);
    assert_eq!(r, xxh64_round(0, 0)); // deterministic

    let r2 = xxh64_round(123, 456);
    assert_ne!(r2, 0);
}

#[test]
fn test_xxh64_merge_round_fn() {
    let r = xxh64_merge_round(0, 0);
    assert_ne!(r, 0);
}

#[test]
fn test_xxh64_avalanche_fn() {
    let r = xxh64_avalanche(0);
    assert_eq!(r, xxh64_avalanche(0));
    assert_ne!(xxh64_avalanche(1), xxh64_avalanche(2));
}

#[test]
fn test_xxh64_basic() {
    let data = [1u8, 2, 3, 4];
    let h1 = xxh64(data.as_ptr(), data.len(), 0);
    let h2 = xxh64(data.as_ptr(), data.len(), 0);
    assert_eq!(h1, h2);

    let h3 = xxh64(data.as_ptr(), data.len(), 1);
    assert_ne!(h1, h3);
}

#[test]
fn test_xxh64_h_basic() {
    let data = [1u8, 2, 3, 4];
    let h1 = xxh64_h(data.as_ptr(), data.len(), 0);
    let h2 = xxh64(data.as_ptr(), data.len(), 0);
    // h variant uses different init constants, so results differ
    assert_ne!(h1, h2);
}

#[test]
fn test_xxh64_empty() {
    let h = xxh64(std::ptr::null(), 0, 0);
    assert_ne!(h, 0);
}

#[test]
fn test_xxh64_long_input() {
    // >= 32 bytes to exercise the main loop
    let data: Vec<u8> = (0..64).collect();
    let h = xxh64(data.as_ptr(), data.len(), CSET_DEFAULT_SEED);
    assert_ne!(h, 0);
}

fn main() {}
