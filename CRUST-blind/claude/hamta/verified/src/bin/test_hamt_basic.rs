use hamta::hamta::*;

// Helper: a placeholder mutable reference for KeyValue conflict slots, since
// the C version takes pointers to a stack key_value_t struct.
// In Rust, KeyValue requires &mut T and &mut U references — we initialize
// conflict_kv with dummy values and reassign via the API.

#[test]
fn test_new_hamt_empty() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    assert_eq!(h.hamt_size(), 0);
    assert_eq!(h.size, 0);
}

#[test]
fn test_set_single_int() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut k1 = 1i32;
    let mut v1 = 10i32;

    let mut conflict_k = 0i32;
    let mut conflict_v = 0i32;
    let mut conflict_kv = KeyValue {
        key: &mut conflict_k,
        value: &mut conflict_v,
    };

    let conflict = h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);
    assert!(!conflict);
    assert_eq!(h.hamt_size(), 1);
}

#[test]
fn test_set_two_ints() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut k1 = 1i32;
    let mut v1 = 10i32;
    let mut k2 = 2i32;
    let mut v2 = 20i32;

    let mut conflict_k = 0i32;
    let mut conflict_v = 0i32;
    let mut conflict_kv = KeyValue {
        key: &mut conflict_k,
        value: &mut conflict_v,
    };

    let c1 = h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);
    assert!(!c1);
    assert_eq!(h.hamt_size(), 1);

    let c2 = h.hamt_set(&mut k2, &mut v2, &mut conflict_kv);
    assert!(!c2);
    assert_eq!(h.hamt_size(), 2);
}

#[test]
fn test_search_present_int() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut k1 = 1i32;
    let mut v1 = 10i32;

    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };

    h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);

    let mut search_key = 1i32;
    let result = h.hamt_search(&mut search_key);
    assert!(result.is_some());
    let kv = result.unwrap();
    assert_eq!(*kv.key, 1);
    assert_eq!(*kv.value, 10);
}

#[test]
fn test_search_missing_int() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut k1 = 1i32;
    let mut v1 = 10i32;

    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };

    h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);

    let mut sk = 99i32;
    let result = h.hamt_search(&mut sk);
    assert!(result.is_none());
}

#[test]
fn test_search_empty_returns_none() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut sk = 1i32;
    let result = h.hamt_search(&mut sk);
    assert!(result.is_none());
    assert_eq!(h.hamt_size(), 0);
}

#[test]
fn test_set_replace_existing() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    let mut k1 = 1i32;
    let mut v1 = 10i32;
    let mut k2 = 2i32;
    let mut v2 = 20i32;
    let mut k1b = 1i32;
    let mut v1b = 100i32;

    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };

    h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);
    h.hamt_set(&mut k2, &mut v2, &mut conflict_kv);

    let conflict = h.hamt_set(&mut k1b, &mut v1b, &mut conflict_kv);
    // C returns !inserted for the replace path — i.e., true means there was a conflict.
    assert!(conflict);
    assert_eq!(h.hamt_size(), 2);
    assert_eq!(*conflict_kv.key, 1);
    assert_eq!(*conflict_kv.value, 10);

    let mut sk = 1i32;
    let result = h.hamt_search(&mut sk);
    assert!(result.is_some());
    assert_eq!(*result.unwrap().value, 100);
}

#[test]
fn test_remove_existing_int() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    let mut k1 = 1i32;
    let mut v1 = 10i32;
    let mut k2 = 2i32;
    let mut v2 = 20i32;
    let mut k3 = 3i32;
    let mut v3 = 30i32;

    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };

    h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);
    h.hamt_set(&mut k2, &mut v2, &mut conflict_kv);
    h.hamt_set(&mut k3, &mut v3, &mut conflict_kv);
    assert_eq!(h.hamt_size(), 3);

    let mut rk = 2i32;
    let mut rkk = 0i32;
    let mut rkv_v = 0i32;
    let mut removed_kv = KeyValue { key: &mut rkk, value: &mut rkv_v };
    let removed = h.hamt_remove(&mut rk, &mut removed_kv);
    assert!(removed);
    assert_eq!(h.hamt_size(), 2);
    assert_eq!(*removed_kv.key, 2);
    assert_eq!(*removed_kv.value, 20);

    let mut sk = 2i32;
    assert!(h.hamt_search(&mut sk).is_none());

    let mut sk1 = 1i32;
    let r1 = h.hamt_search(&mut sk1);
    assert!(r1.is_some());
    assert_eq!(*r1.unwrap().value, 10);

    let mut sk3 = 3i32;
    let r3 = h.hamt_search(&mut sk3);
    assert!(r3.is_some());
    assert_eq!(*r3.unwrap().value, 30);
}

#[test]
fn test_remove_missing() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    let mut k1 = 1i32;
    let mut v1 = 10i32;
    let mut k2 = 2i32;
    let mut v2 = 20i32;
    let mut k3 = 3i32;
    let mut v3 = 30i32;

    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };

    h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);
    h.hamt_set(&mut k2, &mut v2, &mut conflict_kv);
    h.hamt_set(&mut k3, &mut v3, &mut conflict_kv);

    let mut rk = 999i32;
    let mut rkk = 0i32;
    let mut rkv_v = 0i32;
    let mut removed_kv = KeyValue { key: &mut rkk, value: &mut rkv_v };
    let removed = h.hamt_remove(&mut rk, &mut removed_kv);
    assert!(!removed);
    assert_eq!(h.hamt_size(), 3);
}

#[test]
fn test_remove_from_empty() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut rk = 1i32;
    let mut rkk = 0i32;
    let mut rkv_v = 0i32;
    let mut removed_kv = KeyValue { key: &mut rkk, value: &mut rkv_v };
    let removed = h.hamt_remove(&mut rk, &mut removed_kv);
    assert!(!removed);
    assert_eq!(h.hamt_size(), 0);
}

#[test]
fn test_remove_last_element() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    let mut k1 = 1i32;
    let mut v1 = 10i32;

    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };

    h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);
    assert_eq!(h.hamt_size(), 1);

    let mut rk = 1i32;
    let mut rkk = 0i32;
    let mut rkv_v = 0i32;
    let mut removed_kv = KeyValue { key: &mut rkk, value: &mut rkv_v };
    let removed = h.hamt_remove(&mut rk, &mut removed_kv);
    assert!(removed);
    assert_eq!(h.hamt_size(), 0);
    assert_eq!(*removed_kv.key, 1);
    assert_eq!(*removed_kv.value, 10);

    let mut sk = 1i32;
    assert!(h.hamt_search(&mut sk).is_none());
}

#[test]
fn test_remove_then_insert_again() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    let mut k1 = 1i32;
    let mut v1 = 10i32;
    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };

    h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);

    let mut rk = 1i32;
    let mut rkk = 0i32;
    let mut rkv_v = 0i32;
    let mut removed_kv = KeyValue { key: &mut rkk, value: &mut rkv_v };
    h.hamt_remove(&mut rk, &mut removed_kv);
    assert_eq!(h.hamt_size(), 0);

    let mut k2 = 5i32;
    let mut v2 = 50i32;
    h.hamt_set(&mut k2, &mut v2, &mut conflict_kv);
    assert_eq!(h.hamt_size(), 1);

    let mut sk = 5i32;
    let r = h.hamt_search(&mut sk);
    assert!(r.is_some());
    assert_eq!(*r.unwrap().value, 50);
}

#[test]
fn test_many_int_inserts() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    let n: i32 = 100;
    let mut keys: Vec<i32> = (0..n).collect();
    let mut vals: Vec<i32> = (0..n).map(|i| i * i + 7).collect();

    let mut ck = 0i32;
    let mut cv = 0i32;
    {
        let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };

        for i in 0..n as usize {
            let kp: *mut i32 = &mut keys[i];
            let vp: *mut i32 = &mut vals[i];
            unsafe {
                h.hamt_set(&mut *kp, &mut *vp, &mut conflict_kv);
            }
        }
    }
    assert_eq!(h.hamt_size(), n);

    for i in 0..n {
        let mut sk = i;
        let r = h.hamt_search(&mut sk);
        assert!(r.is_some(), "missing key {}", i);
        assert_eq!(*r.unwrap().value, i * i + 7);
    }
}

#[test]
fn test_many_inserts_then_remove_half() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    let n: i32 = 100;
    let mut keys: Vec<i32> = (0..n).collect();
    let mut vals: Vec<i32> = (0..n).map(|i| i * i + 7).collect();

    let mut ck = 0i32;
    let mut cv = 0i32;
    {
        let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };
        for i in 0..n as usize {
            let kp: *mut i32 = &mut keys[i];
            let vp: *mut i32 = &mut vals[i];
            unsafe {
                h.hamt_set(&mut *kp, &mut *vp, &mut conflict_kv);
            }
        }
    }
    assert_eq!(h.hamt_size(), n);

    let mut rkk = 0i32;
    let mut rkv_v = 0i32;
    let mut removed_count = 0;
    {
        let mut removed_kv = KeyValue { key: &mut rkk, value: &mut rkv_v };
        for i in (0..n).step_by(2) {
            let mut rk = i;
            let removed = h.hamt_remove(&mut rk, &mut removed_kv);
            if removed {
                removed_count += 1;
            }
        }
    }
    assert_eq!(removed_count, 50);
    assert_eq!(h.hamt_size(), 50);

    for i in 0..n {
        let mut sk = i;
        let r = h.hamt_search(&mut sk);
        let should_exist = (i % 2) == 1;
        assert_eq!(r.is_some(), should_exist, "key {} unexpected", i);
        if should_exist {
            assert_eq!(*r.unwrap().value, i * i + 7);
        }
    }
}

fn no_dealloc_int(_: &mut i32) {}

#[test]
fn test_destroy_empty() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    h.hamt_destroy(no_dealloc_int, no_dealloc_int);
}

#[test]
fn test_destroy_with_entries() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut k1 = 1i32;
    let mut v1 = 10i32;
    let mut k2 = 2i32;
    let mut v2 = 20i32;

    let mut ck = 0i32;
    let mut cv = 0i32;
    {
        let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };
        h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);
        h.hamt_set(&mut k2, &mut v2, &mut conflict_kv);
    }
    h.hamt_destroy(no_dealloc_int, no_dealloc_int);
}

fn int_to_str(v: &mut i32) -> String {
    format!("{}", *v)
}

#[test]
fn test_print_does_not_panic() {
    // Just exercise the print path; nothing to assert beyond no panic.
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    h.hamt_print(int_to_str, int_to_str);

    let mut k1 = 1i32;
    let mut v1 = 10i32;
    let mut k2 = 2i32;
    let mut v2 = 20i32;
    let mut ck = 0i32;
    let mut cv = 0i32;
    {
        let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };
        h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);
        h.hamt_set(&mut k2, &mut v2, &mut conflict_kv);
    }
    h.hamt_print(int_to_str, int_to_str);
}

fn main() {}
