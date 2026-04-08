use hamta::hamta::*;

/// Helper: leak a boxed value to get a `&'static mut T`.
fn leak<T>(val: T) -> &'static mut T {
    Box::leak(Box::new(val))
}

// ── Hash function tests ──

#[test]
fn test_constants() {
    assert_eq!(FNV_BASE as u32, 2216829733);
    assert_eq!(FNV_PRIME as u32, 435);
    assert_eq!(CHUNK_SIZE, 6);
}

#[test]
fn test_int_hash_values() {
    assert_eq!(hamt_int_hash(&mut 0i32), 2647528437);
    assert_eq!(hamt_int_hash(&mut 1i32), 2565215562);
    assert_eq!(hamt_int_hash(&mut 2i32), 2482902687);
    assert_eq!(hamt_int_hash(&mut 42i32), 163444391);
    assert_eq!(hamt_int_hash(&mut 100i32), 3979232233);
    assert_eq!(hamt_int_hash(&mut -1i32), 2729652521);
    assert_eq!(hamt_int_hash(&mut 1000000i32), 1686955798);
}

#[test]
fn test_str_hash_empty() {
    assert_eq!(hamt_str_hash(&mut [0u8; 1]), 2216829733);
}

#[test]
fn test_str_hash_a() {
    assert_eq!(hamt_str_hash(&mut [b'a', 0u8]), 2248259518);
}

#[test]
fn test_str_hash_bb() {
    assert_eq!(hamt_str_hash(&mut [b'b', b'b', 0u8]), 3035313733);
}

#[test]
fn test_str_hash_hello() {
    assert_eq!(hamt_str_hash(&mut [b'h', b'e', b'l', b'l', b'o', 0u8]), 3183334599);
}

// ── Equals function tests ──

#[test]
fn test_int_equals() {
    assert!(hamt_int_equals(&mut 5i32, &mut 5i32));
    assert!(!hamt_int_equals(&mut 5i32, &mut 6i32));
    assert!(hamt_int_equals(&mut 0i32, &mut 0i32));
    assert!(!hamt_int_equals(&mut 0i32, &mut 1i32));
}

#[test]
fn test_str_equals() {
    assert!(hamt_str_equals(&mut [b'a', 0u8], &mut [b'a', 0u8]));
    assert!(!hamt_str_equals(&mut [b'a', 0u8], &mut [b'b', 0u8]));
}

// ── Hamt lifecycle tests ──

#[test]
fn test_new_hamt_empty() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    assert_eq!(h.hamt_size(), 0);
}

#[test]
fn test_search_empty() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    assert!(!h.hamt_search(&mut 1));
}

#[test]
fn test_remove_empty() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut removed_kv = KeyValue { key: leak(0i32), value: leak(0i32) };
    assert!(!h.hamt_remove(&mut 1, &mut removed_kv));
}

#[test]
fn test_single_insert_and_search() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };

    // First insert: no conflict
    assert!(!h.hamt_set(leak(10i32), leak(100i32), &mut conflict_kv));
    assert_eq!(h.hamt_size(), 1);

    // Search for existing key
    assert!(h.hamt_search(&mut 10i32));

    // Search for non-existing key
    assert!(!h.hamt_search(&mut 20i32));
}

#[test]
fn test_overwrite_returns_conflict() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };

    assert!(!h.hamt_set(leak(10i32), leak(100i32), &mut conflict_kv));
    assert_eq!(h.hamt_size(), 1);

    // Overwrite: should return true (conflict)
    assert!(h.hamt_set(leak(10i32), leak(200i32), &mut conflict_kv));
    assert_eq!(h.hamt_size(), 1);
}

#[test]
fn test_remove_existing() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };

    h.hamt_set(leak(10i32), leak(100i32), &mut conflict_kv);
    assert_eq!(h.hamt_size(), 1);

    let mut removed_kv = KeyValue { key: leak(0i32), value: leak(0i32) };
    assert!(h.hamt_remove(&mut 10i32, &mut removed_kv));
    assert_eq!(h.hamt_size(), 0);
}

#[test]
fn test_remove_nonexistent() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };

    h.hamt_set(leak(10i32), leak(100i32), &mut conflict_kv);

    let mut removed_kv = KeyValue { key: leak(0i32), value: leak(0i32) };
    assert!(!h.hamt_remove(&mut 20i32, &mut removed_kv));
    assert_eq!(h.hamt_size(), 1);
}

// ── Multiple inserts ──

#[test]
fn test_multiple_inserts_size() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };

    for i in 0..10 {
        assert!(!h.hamt_set(leak(i), leak(i * 10), &mut conflict_kv));
    }
    assert_eq!(h.hamt_size(), 10);
}

#[test]
fn test_multiple_inserts_search_all() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };

    for i in 0..10 {
        h.hamt_set(leak(i), leak(i * 10), &mut conflict_kv);
    }

    for i in 0..10 {
        assert!(h.hamt_search(&mut i.clone()), "key {} not found", i);
    }
    assert!(!h.hamt_search(&mut 99i32));
}

#[test]
fn test_multiple_inserts_remove_all() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };

    for i in 0..10i32 {
        h.hamt_set(leak(i), leak(i * 10), &mut conflict_kv);
    }

    let mut removed_kv = KeyValue { key: leak(0i32), value: leak(0i32) };
    for i in 0..10i32 {
        assert!(h.hamt_remove(&mut i.clone(), &mut removed_kv), "failed to remove {}", i);
        assert_eq!(h.hamt_size(), 9 - i);
    }
    assert_eq!(h.hamt_size(), 0);
}

// ── Stress test (mirrors C test_big) ──

#[test]
fn test_big_insert_search() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };
    let n = 100i32;

    for i in 0..n {
        let key = (i % (n / 7)) + 1;
        let value = i * i + 10;
        h.hamt_set(leak(key), leak(value), &mut conflict_kv);
        assert!(h.hamt_search(&mut key.clone()));
    }
}

// ── String-based HAMT tests ──

#[test]
fn test_str_hamt_basic() {
    let mut h: Hamt<[u8; 3], [u8; 3]> = Hamt::new_hamt(hamt_str_hash, hamt_str_equals);
    let mut conflict_kv = KeyValue { key: leak([0u8; 3]), value: leak([0u8; 3]) };

    let xx = [b'x', b'x', 0u8];
    let yy = [b'y', b'y', 0u8];

    // Insert xx -> xx
    assert!(!h.hamt_set(leak(xx), leak(xx), &mut conflict_kv));
    assert_eq!(h.hamt_size(), 1);

    // Insert yy -> yy
    assert!(!h.hamt_set(leak(yy), leak(yy), &mut conflict_kv));
    assert_eq!(h.hamt_size(), 2);

    // Overwrite xx -> yy (conflict)
    assert!(h.hamt_set(leak(xx), leak(yy), &mut conflict_kv));
    assert_eq!(h.hamt_size(), 2);

    // Overwrite yy -> xx (conflict)
    assert!(h.hamt_set(leak(yy), leak(xx), &mut conflict_kv));
    assert_eq!(h.hamt_size(), 2);

    // Remove xx
    let mut removed_kv = KeyValue { key: leak([0u8; 3]), value: leak([0u8; 3]) };
    assert!(h.hamt_remove(&mut [b'x', b'x', 0u8], &mut removed_kv));
    assert_eq!(h.hamt_size(), 1);
}

// ── HamtNode is_leaf test ──

#[test]
fn test_node_is_leaf() {
    let mut leaf: HamtNode<i32, i32> = HamtNode::Leaf(None);
    assert!(leaf.is_leaf());

    let mut sub: HamtNode<i32, i32> = HamtNode::Sub(SubNode {
        bitmap: 0,
        children: Vec::new(),
    });
    assert!(!sub.is_leaf());
}

// ── Print and destroy (smoke tests) ──

#[test]
fn test_print_empty() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    fn int_str(x: &mut i32) -> String { x.to_string() }
    h.hamt_print(int_str, int_str);
}

#[test]
fn test_print_nonempty() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };
    h.hamt_set(leak(1i32), leak(2i32), &mut conflict_kv);
    fn int_str(x: &mut i32) -> String { x.to_string() }
    h.hamt_print(int_str, int_str);
}

#[test]
fn test_destroy_empty() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    fn noop(_: &mut i32) {}
    h.hamt_destroy(noop, noop);
}

#[test]
fn test_destroy_nonempty() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };
    h.hamt_set(leak(1i32), leak(2i32), &mut conflict_kv);
    fn noop(_: &mut i32) {}
    h.hamt_destroy(noop, noop);
}

// ── Boundary: insert then remove then re-insert ──

#[test]
fn test_reinsert_after_remove() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };

    h.hamt_set(leak(5i32), leak(50i32), &mut conflict_kv);
    assert_eq!(h.hamt_size(), 1);

    let mut removed_kv = KeyValue { key: leak(0i32), value: leak(0i32) };
    h.hamt_remove(&mut 5i32, &mut removed_kv);
    assert_eq!(h.hamt_size(), 0);

    assert!(!h.hamt_set(leak(5i32), leak(500i32), &mut conflict_kv));
    assert_eq!(h.hamt_size(), 1);
    assert!(h.hamt_search(&mut 5i32));
}

// ── Remove from size=1 with wrong key ──

#[test]
fn test_remove_wrong_key_size_one() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut conflict_kv = KeyValue { key: leak(0i32), value: leak(0i32) };

    h.hamt_set(leak(10i32), leak(100i32), &mut conflict_kv);
    assert_eq!(h.hamt_size(), 1);

    let mut removed_kv = KeyValue { key: leak(0i32), value: leak(0i32) };
    assert!(!h.hamt_remove(&mut 20i32, &mut removed_kv));
    assert_eq!(h.hamt_size(), 1);
}

fn main() {}
