use hamta::hamta::*;

// Each "string" is a NUL-terminated byte buffer ([u8; N]). We use byte arrays
// owned in test scope so their addresses are stable. The hash and equals
// functions read until the terminating NUL byte.

#[test]
fn test_string_set_search() {
    let mut h: Hamt<[u8; 4], [u8; 4]> = Hamt::new_hamt(hamt_str_hash, hamt_str_equals);

    let mut k_aut: [u8; 4] = [b'a', b'u', b't', 0];
    let mut v_aut: [u8; 4] = [b'a', b'u', b't', 0];
    let mut k_bus: [u8; 4] = [b'b', b'u', b's', 0];
    let mut v_bus: [u8; 4] = [b'b', b'u', b's', 0];

    let mut ck: [u8; 4] = [0; 4];
    let mut cv: [u8; 4] = [0; 4];
    {
        let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };
        let r1 = h.hamt_set(&mut k_aut, &mut v_aut, &mut conflict_kv);
        assert!(!r1);
        assert_eq!(h.hamt_size(), 1);
        let r2 = h.hamt_set(&mut k_bus, &mut v_bus, &mut conflict_kv);
        assert!(!r2);
        assert_eq!(h.hamt_size(), 2);
    }

    let mut sk1: [u8; 4] = [b'a', b'u', b't', 0];
    let r = h.hamt_search(&mut sk1);
    assert!(r.is_some());
    assert_eq!(*r.unwrap().value, [b'a', b'u', b't', 0]);

    let mut sk2: [u8; 4] = [b'b', b'u', b's', 0];
    let r = h.hamt_search(&mut sk2);
    assert!(r.is_some());
    assert_eq!(*r.unwrap().value, [b'b', b'u', b's', 0]);

    // missing
    let mut sk3: [u8; 4] = [b'x', b'y', b'z', 0];
    assert!(h.hamt_search(&mut sk3).is_none());
}

#[test]
fn test_strings_nine_elements() {
    // Mirror the C test_search_destroy: insert 9 strings, search for all,
    // then remove all, asserting size after each removal.
    type B8 = [u8; 8]; // hold up to 7 chars + NUL

    fn mk(s: &str) -> B8 {
        let mut buf: B8 = [0; 8];
        let bytes = s.as_bytes();
        buf[..bytes.len()].copy_from_slice(bytes);
        buf
    }

    // owned bufs, so refs are stable for the trie's lifetime
    let mut bufs = [
        mk("aut"),
        mk("bus"),
        mk("vlak"),
        mk("kokos"),
        mk("banan"),
        mk("losos"),
        mk("bro"),
        mk("b"),
        mk("bubakov"),
    ];

    let mut h: Hamt<B8, B8> = Hamt::new_hamt(hamt_str_hash, hamt_str_equals);

    let mut ck: B8 = [0; 8];
    let mut cv: B8 = [0; 8];
    {
        let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };
        for i in 0..bufs.len() {
            // Use the same buffer for both key and value (need to alias)
            let bp: *mut B8 = &mut bufs[i];
            unsafe {
                h.hamt_set(&mut *bp, &mut *bp, &mut conflict_kv);
            }
        }
    }
    assert_eq!(h.hamt_size(), 9);

    // Search for every key
    let queries = [
        mk("losos"), mk("bus"), mk("aut"), mk("vlak"), mk("banan"),
        mk("kokos"), mk("bro"), mk("b"), mk("bubakov"),
    ];
    for mut q in queries {
        let r = h.hamt_search(&mut q);
        assert!(r.is_some(), "missing string");
        let kv = r.unwrap();
        // Compare value byte-for-byte to query (both are NUL-terminated)
        assert_eq!(*kv.value, q);
    }

    // Remove all in some order, verifying size each time
    let mut rkk: B8 = [0; 8];
    let mut rkv_v: B8 = [0; 8];
    let mut removed_kv = KeyValue { key: &mut rkk, value: &mut rkv_v };
    let mut size = 9i32;
    let mut order = [
        mk("losos"), mk("bus"), mk("aut"), mk("vlak"), mk("banan"),
        mk("kokos"), mk("bro"), mk("b"), mk("bubakov"),
    ];
    for i in 0..order.len() {
        let removed = h.hamt_remove(&mut order[i], &mut removed_kv);
        assert!(removed, "failed to remove index {}", i);
        size -= 1;
        assert_eq!(h.hamt_size(), size);
    }

    // After all removed, searches should miss
    let mut q = mk("aut");
    assert!(h.hamt_search(&mut q).is_none());
}

#[test]
fn test_string_replace() {
    let mut h: Hamt<[u8; 4], [u8; 4]> = Hamt::new_hamt(hamt_str_hash, hamt_str_equals);

    let mut k1: [u8; 4] = [b'x', b'x', 0, 0];
    let mut v1: [u8; 4] = [b'x', b'x', 0, 0];
    let mut ck: [u8; 4] = [0; 4];
    let mut cv: [u8; 4] = [0; 4];
    {
        let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };
        let r = h.hamt_set(&mut k1, &mut v1, &mut conflict_kv);
        assert!(!r);
        assert_eq!(h.hamt_size(), 1);
    }

    let mut k1b: [u8; 4] = [b'x', b'x', 0, 0];
    let mut v1b: [u8; 4] = [b'y', b'y', 0, 0];
    {
        let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };
        let r = h.hamt_set(&mut k1b, &mut v1b, &mut conflict_kv);
        assert!(r);
        assert_eq!(h.hamt_size(), 1);
        // The conflict key/value should be the originals
        assert_eq!(*conflict_kv.key, [b'x', b'x', 0, 0]);
        assert_eq!(*conflict_kv.value, [b'x', b'x', 0, 0]);
    }

    let mut sk: [u8; 4] = [b'x', b'x', 0, 0];
    let r = h.hamt_search(&mut sk);
    assert!(r.is_some());
    assert_eq!(*r.unwrap().value, [b'y', b'y', 0, 0]);
}

fn main() {}
