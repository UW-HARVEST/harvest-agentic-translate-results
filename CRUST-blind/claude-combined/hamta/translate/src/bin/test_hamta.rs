#![allow(dead_code)]
use hamta::hamta::*;

// =====================
// Helper functions
// =====================

fn int_hash(k: &mut i32) -> u32 {
    hamt_int_hash(k)
}
fn int_eq(a: &mut i32, b: &mut i32) -> bool {
    hamt_int_equals(a, b)
}
fn str_hash_bytes(k: &mut [u8]) -> u32 {
    let p: &mut u8 = &mut k[0];
    hamt_str_hash(p)
}
fn str_eq_bytes(a: &mut [u8], b: &mut [u8]) -> bool {
    let pa: &mut u8 = &mut a[0];
    let pb: &mut u8 = &mut b[0];
    hamt_str_equals(pa, pb)
}

fn alloc_int(v: i32) -> &'static mut i32 {
    Box::leak(Box::new(v))
}

fn make_kv_int() -> KeyValue<'static, i32, i32> {
    let dk = Box::leak(Box::new(0i32));
    let dv = Box::leak(Box::new(0i32));
    KeyValue { key: dk, value: dv }
}

// ============================================================
// Constants
// ============================================================

#[test]
fn test_constants() {
    assert_eq!(FNV_PRIME, 1099511628211u64);
    assert_eq!(FNV_BASE, 14695981039346656037u64);
    assert_eq!(HAMT_NODE_T_FLAG, 1);
    assert_eq!(KEY_VALUE_T_FLAG, 0);
    assert_eq!(CHUNK_SIZE, 6);
}

// ============================================================
// hamt_int_hash
// ============================================================

#[test]
fn test_int_hash_zero() {
    let mut k: i32 = 0;
    assert_eq!(hamt_int_hash(&mut k), 2647528437);
}

#[test]
fn test_int_hash_one() {
    let mut k: i32 = 1;
    assert_eq!(hamt_int_hash(&mut k), 2565215562);
}

#[test]
fn test_int_hash_two() {
    let mut k: i32 = 2;
    assert_eq!(hamt_int_hash(&mut k), 2482902687);
}

#[test]
fn test_int_hash_three() {
    let mut k: i32 = 3;
    assert_eq!(hamt_int_hash(&mut k), 2400589812);
}

#[test]
fn test_int_hash_seven() {
    let mut k: i32 = 7;
    assert_eq!(hamt_int_hash(&mut k), 2071338312);
}

#[test]
fn test_int_hash_42() {
    let mut k: i32 = 42;
    assert_eq!(hamt_int_hash(&mut k), 163444391);
}

#[test]
fn test_int_hash_100() {
    let mut k: i32 = 100;
    assert_eq!(hamt_int_hash(&mut k), 3979232233);
}

#[test]
fn test_int_hash_1000() {
    let mut k: i32 = 1000;
    assert_eq!(hamt_int_hash(&mut k), 3458511334);
}

#[test]
fn test_int_hash_neg_one() {
    let mut k: i32 = -1;
    assert_eq!(hamt_int_hash(&mut k), 2729652521);
}

// ============================================================
// hamt_str_hash
// ============================================================

#[test]
fn test_str_hash_empty() {
    let mut bytes = b"\0".to_vec();
    assert_eq!(str_hash_bytes(&mut bytes), 2216829733);
}

#[test]
fn test_str_hash_a() {
    let mut bytes = b"a\0".to_vec();
    assert_eq!(str_hash_bytes(&mut bytes), 2248259518);
}

#[test]
fn test_str_hash_ab() {
    let mut bytes = b"ab\0".to_vec();
    assert_eq!(str_hash_bytes(&mut bytes), 3035314104);
}

#[test]
fn test_str_hash_hello() {
    let mut bytes = b"hello\0".to_vec();
    assert_eq!(str_hash_bytes(&mut bytes), 3183334599);
}

#[test]
fn test_str_hash_world() {
    let mut bytes = b"world\0".to_vec();
    assert_eq!(str_hash_bytes(&mut bytes), 3299234831);
}

#[test]
fn test_str_hash_xx() {
    let mut bytes = b"xx\0".to_vec();
    assert_eq!(str_hash_bytes(&mut bytes), 3035304125);
}

#[test]
fn test_str_hash_yy() {
    let mut bytes = b"yy\0".to_vec();
    assert_eq!(str_hash_bytes(&mut bytes), 3035303787);
}

#[test]
fn test_str_hash_aut() {
    let mut bytes = b"aut\0".to_vec();
    assert_eq!(str_hash_bytes(&mut bytes), 1806671401);
}

#[test]
fn test_str_hash_bus() {
    let mut bytes = b"bus\0".to_vec();
    assert_eq!(str_hash_bytes(&mut bytes), 1806519589);
}

#[test]
fn test_str_hash_vlak() {
    let mut bytes = b"vlak\0".to_vec();
    assert_eq!(str_hash_bytes(&mut bytes), 2502912359);
}

// ============================================================
// hamt_int_equals
// ============================================================

#[test]
fn test_int_equals_same() {
    let mut a = 5;
    let mut b = 5;
    assert_eq!(hamt_int_equals(&mut a, &mut b), true);
}

#[test]
fn test_int_equals_different() {
    let mut a = 5;
    let mut b = 6;
    assert_eq!(hamt_int_equals(&mut a, &mut b), false);
}

#[test]
fn test_int_equals_neg_pos() {
    let mut a: i32 = -1;
    let mut b: i32 = 1;
    assert_eq!(hamt_int_equals(&mut a, &mut b), false);
}

// ============================================================
// hamt_str_equals
// ============================================================

#[test]
fn test_str_equals_same() {
    let mut a = b"hello\0".to_vec();
    let mut b = b"hello\0".to_vec();
    assert_eq!(str_eq_bytes(&mut a, &mut b), true);
}

#[test]
fn test_str_equals_different() {
    let mut a = b"hello\0".to_vec();
    let mut b = b"world\0".to_vec();
    assert_eq!(str_eq_bytes(&mut a, &mut b), false);
}

#[test]
fn test_str_equals_prefix() {
    let mut a = b"abc\0".to_vec();
    let mut b = b"abcd\0".to_vec();
    assert_eq!(str_eq_bytes(&mut a, &mut b), false);
}

// ============================================================
// hamt_fnv1_hash and hamt_get_symbol (signatures return ())
// ============================================================

#[test]
fn test_fnv1_hash_returns_unit() {
    let mut k: i32 = 42;
    // Just verify that the function can be called without panicking.
    let _: () = hamt_fnv1_hash(&mut k, std::mem::size_of::<i32>());
}

#[test]
fn test_get_symbol_returns_unit() {
    let _: () = hamt_get_symbol(0xdeadbeef, 0);
    let _: () = hamt_get_symbol(0xdeadbeef, 1);
    let _: () = hamt_get_symbol(0xdeadbeef, 2);
}

// ============================================================
// new_hamt + hamt_size
// ============================================================

#[test]
fn test_new_hamt_starts_empty() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    assert_eq!(h.hamt_size(), 0);
}

// ============================================================
// hamt_set: single-insert
// ============================================================

#[test]
fn test_set_single() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let k = alloc_int(5);
    let v = alloc_int(500);
    let mut conflict_kv = make_kv_int();
    let conflict = h.hamt_set(k, v, &mut conflict_kv);
    assert_eq!(conflict, false);
    assert_eq!(h.hamt_size(), 1);
}

// ============================================================
// hamt_set: multi-insert
// ============================================================

#[test]
fn test_set_multi() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let n = 10;
    for i in 0..n {
        let k = alloc_int(i);
        let v = alloc_int(i * 100);
        let mut conflict_kv = make_kv_int();
        let conflict = h.hamt_set(k, v, &mut conflict_kv);
        assert_eq!(conflict, false);
        assert_eq!(h.hamt_size(), i + 1);
    }
    assert_eq!(h.hamt_size(), n);
}

// ============================================================
// hamt_set: re-insert (collision)
// ============================================================

#[test]
fn test_set_reinsert_collision() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let n = 10;
    for i in 0..n {
        let k = alloc_int(i);
        let v = alloc_int(i * 100);
        let mut conflict_kv = make_kv_int();
        h.hamt_set(k, v, &mut conflict_kv);
    }
    // Re-insert key=5 with new value
    let new_k = alloc_int(5);
    let new_v = alloc_int(9999);
    let mut conflict_kv = make_kv_int();
    let conflict = h.hamt_set(new_k, new_v, &mut conflict_kv);
    // C output: re-insert key=5 val=9999, conflict=1, conflict_key=5, conflict_val=500, size=10
    assert_eq!(conflict, true);
    assert_eq!(*conflict_kv.key, 5);
    assert_eq!(*conflict_kv.value, 500);
    assert_eq!(h.hamt_size(), 10);

    // Verify search returns the new value
    let mut q = 5i32;
    let f = h.hamt_search(&mut q);
    assert!(f.is_some());
    let kv = f.unwrap();
    assert_eq!(*kv.key, 5);
    assert_eq!(*kv.value, 9999);
}

// ============================================================
// hamt_search: find existing
// ============================================================

#[test]
fn test_search_existing() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let n = 10;
    for i in 0..n {
        let k = alloc_int(i);
        let v = alloc_int(i * 100);
        let mut conflict_kv = make_kv_int();
        h.hamt_set(k, v, &mut conflict_kv);
    }
    for i in 0..n {
        let mut q = i;
        let f = h.hamt_search(&mut q);
        assert!(f.is_some(), "should find {}", i);
        let kv = f.unwrap();
        assert_eq!(*kv.key, i);
        assert_eq!(*kv.value, i * 100);
    }
}

// ============================================================
// hamt_search: missing
// ============================================================

#[test]
fn test_search_missing() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let n = 10;
    for i in 0..n {
        let k = alloc_int(i);
        let v = alloc_int(i * 100);
        let mut conflict_kv = make_kv_int();
        h.hamt_set(k, v, &mut conflict_kv);
    }
    let mut q = 99i32;
    let f = h.hamt_search(&mut q);
    assert!(f.is_none());
}

// ============================================================
// hamt_search on empty trie
// ============================================================

#[test]
fn test_search_empty() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let mut q = 42i32;
    let f = h.hamt_search(&mut q);
    assert!(f.is_none());
}

// ============================================================
// hamt_remove: single
// ============================================================

#[test]
fn test_remove_single() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let k = alloc_int(5);
    let v = alloc_int(500);
    let mut conflict_kv = make_kv_int();
    h.hamt_set(k, v, &mut conflict_kv);
    assert_eq!(h.hamt_size(), 1);

    let mut q = 5i32;
    let mut removed_kv = make_kv_int();
    let removed = h.hamt_remove(&mut q, &mut removed_kv);
    assert_eq!(removed, true);
    assert_eq!(*removed_kv.key, 5);
    assert_eq!(*removed_kv.value, 500);
    assert_eq!(h.hamt_size(), 0);

    // Re-search
    let f = h.hamt_search(&mut q);
    assert!(f.is_none());
}

// ============================================================
// hamt_remove: empty trie
// ============================================================

#[test]
fn test_remove_empty() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let mut q = 42i32;
    let mut removed_kv = make_kv_int();
    let removed = h.hamt_remove(&mut q, &mut removed_kv);
    assert_eq!(removed, false);
    assert_eq!(h.hamt_size(), 0);
}

// ============================================================
// hamt_remove: missing key
// ============================================================

#[test]
fn test_remove_missing() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let n = 10;
    for i in 0..n {
        let k = alloc_int(i);
        let v = alloc_int(i * 100);
        let mut conflict_kv = make_kv_int();
        h.hamt_set(k, v, &mut conflict_kv);
    }
    let mut q = 999i32;
    let mut removed_kv = make_kv_int();
    let removed = h.hamt_remove(&mut q, &mut removed_kv);
    assert_eq!(removed, false);
    assert_eq!(h.hamt_size(), 10);
}

// ============================================================
// hamt_remove: all keys
// ============================================================

#[test]
fn test_remove_all() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let n = 10;
    for i in 0..n {
        let k = alloc_int(i);
        let v = alloc_int(i * 100);
        let mut conflict_kv = make_kv_int();
        h.hamt_set(k, v, &mut conflict_kv);
    }
    for i in 0..n {
        let mut q = i;
        let mut removed_kv = make_kv_int();
        let removed = h.hamt_remove(&mut q, &mut removed_kv);
        assert_eq!(removed, true);
        assert_eq!(*removed_kv.key, i);
        assert_eq!(*removed_kv.value, i * 100);
        assert_eq!(h.hamt_size(), n - 1 - i);
    }
    assert_eq!(h.hamt_size(), 0);
}

// ============================================================
// Mid-sized stress test mirroring test_big from C
// ============================================================

#[test]
fn test_big() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let n = 200;
    for i in 0..n {
        let key_val = i % (n / 13 + 1) + 1;
        let value_val = i * i + 10;
        let k = alloc_int(key_val);
        let v = alloc_int(value_val);
        let mut conflict_kv = make_kv_int();
        let _ = h.hamt_set(k, v, &mut conflict_kv);

        let mut q = key_val;
        let f = h.hamt_search(&mut q);
        assert!(f.is_some());
        assert_eq!(*f.unwrap().value, value_val);
    }
}

// ============================================================
// Stress test with insertion + removal
// ============================================================

#[test]
fn test_big2() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    let n = 200;
    for i in 0..n {
        let key_val = i % (n / 7 + 1);
        let value_val = i * i * i;
        let k = alloc_int(key_val);
        let v = alloc_int(value_val);
        let mut conflict_kv = make_kv_int();
        let _ = h.hamt_set(k, v, &mut conflict_kv);

        let mut q = key_val;
        let f = h.hamt_search(&mut q);
        assert!(f.is_some());
        assert_eq!(*f.unwrap().value, value_val);
    }

    for i in 0..n {
        let mut q = i;
        let mut removed_kv = make_kv_int();
        let _ = h.hamt_remove(&mut q, &mut removed_kv);
    }
    assert_eq!(h.hamt_size(), 0);
}

// ============================================================
// HamtNode methods: is_leaf
// ============================================================

#[test]
fn test_hamt_node_is_leaf_on_leaf() {
    let mut node: HamtNode<'_, i32, i32> = HamtNode::Leaf(None);
    assert_eq!(node.is_leaf(), true);
}

#[test]
fn test_hamt_node_is_leaf_on_sub() {
    let mut node: HamtNode<'_, i32, i32> = HamtNode::Sub(SubNode { bitmap: 0, children: None });
    assert_eq!(node.is_leaf(), false);
}

// ============================================================
// String HAMT tests (using a fixed-size CStr-like buffer)
// ============================================================

#[repr(C)]
struct CStr {
    bytes: [u8; 32],
}
impl CStr {
    fn new(s: &str) -> Self {
        let mut bytes = [0u8; 32];
        let src = s.as_bytes();
        let n = src.len().min(31);
        bytes[..n].copy_from_slice(&src[..n]);
        CStr { bytes }
    }
}

fn cstr_hash(k: &mut CStr) -> u32 {
    hamt_str_hash(&mut k.bytes[0])
}
fn cstr_eq(a: &mut CStr, b: &mut CStr) -> bool {
    hamt_str_equals(&mut a.bytes[0], &mut b.bytes[0])
}

fn alloc_cstr(s: &str) -> &'static mut CStr {
    Box::leak(Box::new(CStr::new(s)))
}

fn make_kv_cstr() -> KeyValue<'static, CStr, CStr> {
    let dk = alloc_cstr("");
    let dv = alloc_cstr("");
    KeyValue { key: dk, value: dv }
}

#[test]
fn test_str_hamt_basic() {
    let mut h: Hamt<CStr, CStr> = Hamt::new_hamt(cstr_hash, cstr_eq);

    let key = alloc_cstr("hello");
    let val = alloc_cstr("hello");
    let mut conflict_kv = make_kv_cstr();
    let conflict = h.hamt_set(key, val, &mut conflict_kv);
    assert_eq!(conflict, false);
    assert_eq!(h.hamt_size(), 1);

    // Search
    let mut q = CStr::new("hello");
    let f = h.hamt_search(&mut q);
    assert!(f.is_some());

    // Remove
    let mut removed_kv = make_kv_cstr();
    let removed = h.hamt_remove(&mut q, &mut removed_kv);
    assert_eq!(removed, true);
    assert_eq!(h.hamt_size(), 0);
}

#[test]
fn test_str_hamt_many() {
    let mut h: Hamt<CStr, CStr> = Hamt::new_hamt(cstr_hash, cstr_eq);

    let words = ["a", "bb", "auto", "bus", "vlak", "kokos", "banan", "losos", "bubakov"];
    for w in &words {
        let k = alloc_cstr(w);
        let v = alloc_cstr(w);
        let mut conflict_kv = make_kv_cstr();
        h.hamt_set(k, v, &mut conflict_kv);
    }
    assert_eq!(h.hamt_size(), words.len() as i32);

    for w in &words {
        let mut q = CStr::new(w);
        let f = h.hamt_search(&mut q);
        assert!(f.is_some(), "should find {}", w);
    }
}

// ============================================================
// hamt_destroy
// ============================================================

fn dealloc_int(_v: &mut i32) {
    // No-op since we used Box::leak
}

#[test]
fn test_hamt_destroy_empty() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    h.hamt_destroy(dealloc_int, dealloc_int);
    // Should not panic.
    assert_eq!(h.hamt_size(), 0);
}

#[test]
fn test_hamt_destroy_non_empty() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(int_hash, int_eq);
    for i in 0..5 {
        let k = alloc_int(i);
        let v = alloc_int(i * 10);
        let mut conflict_kv = make_kv_int();
        h.hamt_set(k, v, &mut conflict_kv);
    }
    h.hamt_destroy(dealloc_int, dealloc_int);
    // Should not panic.
}

fn main() {}
