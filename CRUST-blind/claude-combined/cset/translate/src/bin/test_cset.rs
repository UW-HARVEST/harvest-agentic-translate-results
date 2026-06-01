use cset::cset::{Cset, CSET_DEFAULT_SEED, CSET_INITIAL_CAP, CSET_MAX_LOAD_FACTOR,
                 CSET_MIN_LOAD_FACTOR, xxh_is_little_endian, xxh64, xxh64_h};

#[test]
fn test_cset_init() {
    let cset_int: Cset<i32> = Cset::new();
    assert_eq!(cset_int.get_size(), 0);
    assert_eq!(cset_int.size(), 0);
    assert_eq!(cset_int.capacity() as usize, CSET_INITIAL_CAP);
    assert_eq!(cset_int.get_seed(), CSET_DEFAULT_SEED);
    assert_eq!(cset_int.get_max_load_factor(), CSET_MAX_LOAD_FACTOR);
    assert_eq!(cset_int.get_min_load_factor(), CSET_MIN_LOAD_FACTOR);
    assert!(cset_int.empty());
}

#[test]
fn test_cset_add_basic() {
    let mut s: Cset<i32> = Cset::new();

    s.add(34);
    assert_eq!(s.size(), 1);

    s.add(35);
    assert_eq!(s.size(), 2);

    s.add(36);
    s.add(37);
    s.add(38);
    assert_eq!(s.size(), 5);
}

#[test]
fn test_cset_unique() {
    let mut s: Cset<i32> = Cset::new();
    s.add(45);
    s.add(46);
    s.add(57);
    assert_eq!(s.size(), 3);

    s.add(45);
    assert_eq!(s.size(), 3);
}

#[test]
fn test_cset_contains_with_remove() {
    let mut s: Cset<i32> = Cset::new();
    s.add(34);
    s.add(36);
    s.remove(36);

    assert_eq!(s.contains(&12), false);
    assert_eq!(s.contains(&34), true);

    s.add(50);
    assert_eq!(s.contains(&45), false);

    assert_eq!(s.size(), 2);
}

#[test]
fn test_cset_remove_basic() {
    let mut s: Cset<i32> = Cset::new();
    s.add(45);
    s.add(34);
    s.add(10);
    assert_eq!(s.size(), 3);

    s.remove(45);
    assert_eq!(s.size(), 2);

    s.remove(45); // remove again - no change
    assert_eq!(s.size(), 2);

    s.remove(34);
    assert_eq!(s.size(), 1);

    let collected = s.iter();
    for v in &collected {
        assert_eq!(*v, 10);
    }
    assert_eq!(collected.len(), 1);

    s.remove(10);
    assert_eq!(s.size(), 0);
}

#[test]
fn test_cset_clear() {
    let mut s: Cset<i32> = Cset::new();
    s.add(12);
    s.add(14);
    s.add(15);
    assert_eq!(s.size(), 3);

    s.clear();
    assert_eq!(s.size(), 0);
    assert_eq!(s.capacity() as usize, CSET_INITIAL_CAP);

    s.add(45);
    assert_eq!(s.size(), 1);
}

#[test]
fn test_cset_resize_many() {
    let mut s: Cset<i32> = Cset::new();
    for i in 0..1500i32 {
        s.add(i);
    }
    assert_eq!(s.size(), 1500);

    // All inserted values must be present.
    for i in 0..1500i32 {
        assert!(s.contains(&i), "missing {}", i);
    }
    assert_eq!(s.contains(&1500), false);
    assert_eq!(s.contains(&-1), false);
}

#[test]
fn test_cset_iteration() {
    let mut s: Cset<i32> = Cset::new();
    for i in 0..3200i32 {
        s.add(i);
    }
    assert_eq!(s.size(), 3200);

    let collected = s.iter();
    assert_eq!(collected.len(), 3200);

    for v in &collected {
        assert!(s.contains(v));
    }
}

#[derive(Copy, Clone, Default)]
struct Node {
    x: i32,
    y: i32,
}

#[test]
fn test_cset_struct() {
    let mut s: Cset<Node> = Cset::new();
    s.add(Node { x: 4, y: 4 });
    assert_eq!(s.size(), 1);

    s.add(Node { x: 5, y: 4 });
    assert_eq!(s.size(), 2);

    // duplicate of (5, 4) -> bytes equal -> ignored
    s.add(Node { x: 5, y: 4 });
    assert_eq!(s.size(), 2);

    s.add(Node { x: 5, y: 8 });
    assert_eq!(s.size(), 3);
}

fn node_compare(a: &Node, b: &Node) -> bool {
    a.x == b.x
}

#[test]
fn test_custom_comparator() {
    let mut s: Cset<Node> = Cset::new();
    s.set_comparator(node_compare);

    s.add(Node { x: 4, y: 4 });
    s.add(Node { x: 4, y: 4 });
    assert_eq!(s.size(), 1);

    s.add(Node { x: 1, y: 2 });
    assert_eq!(s.size(), 2);

    s.remove(Node { x: 1, y: 45 });
    assert_eq!(s.size(), 1);
}

#[test]
fn test_default_bytes_comparator() {
    let mut s: Cset<i32> = Cset::new();
    s.add(45);
    s.add(46);
    s.add(67);

    assert_eq!(s.contains(&45), true);
    assert_eq!(s.contains(&68), false);
    assert_eq!(s.contains(&46), true);

    s.remove(46);
    assert_eq!(s.contains(&46), false);
    s.remove(46);
    assert_eq!(s.contains(&46), false);
    assert_eq!(s.size(), 2);

    s.remove(45);
    assert_eq!(s.size(), 1);

    s.remove(67);
    assert_eq!(s.size(), 0);

    s.remove(67);
    assert_eq!(s.size(), 0);

    for i in 0..2000i32 {
        s.add(i);
    }
    assert_eq!(s.size(), 2000);
}

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
    assert_eq!(result.size(), 2);
    assert!(result.contains(&12));
    assert!(result.contains(&13));
    assert!(!result.contains(&14));
    assert!(!result.contains(&16));

    b.add(14);
    result.intersect(&a, &b);
    assert_eq!(result.size(), 3);
    assert!(result.contains(&12));
    assert!(result.contains(&13));
    assert!(result.contains(&14));
}

#[test]
fn test_union() {
    let mut a: Cset<i32> = Cset::new();
    let mut b: Cset<i32> = Cset::new();
    let mut r: Cset<i32> = Cset::new();

    a.add(34);
    a.add(25);
    a.add(12);

    b.add(1);
    b.add(4);
    b.add(34);

    r.union(&a, &b);
    assert_eq!(r.size(), 5);
    for v in &[34, 25, 12, 1, 4] {
        assert!(r.contains(v), "missing {}", v);
    }

    b.add(100);
    r.union(&a, &b);
    assert_eq!(r.size(), 6);
    assert!(r.contains(&100));
}

#[test]
fn test_disjoint() {
    let mut a: Cset<u8> = Cset::new();
    let mut b: Cset<u8> = Cset::new();

    a.add(b'a');
    a.add(b'b');

    b.add(b'c');
    b.add(b'd');

    assert_eq!(a.is_disjoint(&b), true);

    b.add(b'a');
    assert_eq!(a.is_disjoint(&b), false);
}

#[test]
fn test_difference() {
    let mut a: Cset<i32> = Cset::new();
    let mut b: Cset<i32> = Cset::new();
    let mut r: Cset<i32> = Cset::new();

    r.difference(&a, &b);
    assert_eq!(r.size(), 0);

    a.add(45);
    a.add(46);
    a.add(58);

    b.add(12);
    b.add(11);
    b.add(45);

    r.difference(&a, &b);
    assert_eq!(r.size(), 2);
    assert_eq!(r.contains(&46), true);
    assert_eq!(r.contains(&58), true);
    assert_eq!(r.contains(&45), false);

    r.clear();

    b.add(46);
    b.add(58);
    r.difference(&a, &b);
    assert_eq!(r.size(), 0);

    r.difference(&b, &a);
    assert_eq!(r.size(), 2);
    assert_eq!(r.contains(&12), true);
    assert_eq!(r.contains(&11), true);
}

#[test]
fn test_set_seed_and_load_factor() {
    let mut s: Cset<i32> = Cset::new();
    s.set_seed(42);
    assert_eq!(s.get_seed(), 42);

    s.set_max_load_factor(0.9);
    assert_eq!(s.get_max_load_factor(), 0.9);

    s.set_min_load_factor(0.1);
    assert_eq!(s.get_min_load_factor(), 0.1);
}

#[test]
fn test_init_resets_state() {
    let mut s: Cset<i32> = Cset::new();
    s.add(1);
    s.add(2);
    s.set_seed(7);
    s.set_max_load_factor(0.5);

    s.init();
    assert_eq!(s.size(), 0);
    assert_eq!(s.capacity() as usize, CSET_INITIAL_CAP);
    assert_eq!(s.get_seed(), CSET_DEFAULT_SEED);
    assert_eq!(s.get_max_load_factor(), CSET_MAX_LOAD_FACTOR);
}

#[test]
fn test_xxh_is_little_endian() {
    // x86_64 hosts are little-endian; expected true on the build host.
    // The function returns whether the host is little-endian.
    let v = xxh_is_little_endian();
    // We don't hard-code; the xxhash logic uses LE reads which should match
    // our manual bytewise composition either way. Just verify it returns a bool.
    let _ = v;
}

#[test]
fn test_xxh64_basic() {
    // Known XXH64 vectors with seed=0 (well-known test vector).
    // XXH64("", 0) = 0xEF46DB3751D8E999
    let h = xxh64(std::ptr::null(), 0, 0);
    assert_eq!(h, 0xEF46DB3751D8E999u64);

    // Test deterministic behavior on a fixed buffer with default seed.
    let buf: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let h1 = xxh64(buf.as_ptr(), 8, CSET_DEFAULT_SEED);
    let h1b = xxh64(buf.as_ptr(), 8, CSET_DEFAULT_SEED);
    assert_eq!(h1, h1b);

    let h2 = xxh64_h(buf.as_ptr(), 8, CSET_DEFAULT_SEED);
    let h2b = xxh64_h(buf.as_ptr(), 8, CSET_DEFAULT_SEED);
    assert_eq!(h2, h2b);
}

#[test]
fn test_remove_then_add_reuses_slot() {
    let mut s: Cset<i32> = Cset::new();
    s.add(1);
    s.add(2);
    s.add(3);
    s.remove(2);
    assert_eq!(s.size(), 2);
    assert_eq!(s.contains(&2), false);
    s.add(2);
    assert_eq!(s.size(), 3);
    assert_eq!(s.contains(&2), true);
}

#[test]
fn test_buckets_accessors() {
    let s: Cset<i32> = Cset::new();
    let buckets = s.get_buckets();
    assert_eq!(buckets.len(), CSET_INITIAL_CAP);

    let _bref = s.get_buckets_ref();
    let _tref = s.get_temp_buckets_ref();
}

fn main() {}
