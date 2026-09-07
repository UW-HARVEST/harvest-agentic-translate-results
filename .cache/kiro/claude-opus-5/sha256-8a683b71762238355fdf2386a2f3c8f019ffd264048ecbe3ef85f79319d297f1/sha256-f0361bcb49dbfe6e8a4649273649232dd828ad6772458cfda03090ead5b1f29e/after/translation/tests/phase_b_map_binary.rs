//! Phase B — CONFIGS.md rows 23-30, 39-44, 48, 49: BINARY-mode hash maps
//! driven through the low-level `stbds_hm*_key` exports.

mod common;
use common::*;
use std::ffi::c_void;

fn i32k(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}

#[test]
fn row23_binary_struct_map_counts() {
    let (p, _g) = libs();
    for &n in &[1usize, 2, 5, 6, 7, 8, 9, 17, 100] {
        for trial in 0..6u64 {
            let mut rng = Rng::new(0xC0FFEE ^ 23 ^ (n as u64) << 8 ^ trial);
            reset_seed(p, DEFAULT_SEED);
            let mut m = Maps::empty(p, cfg_struct());
            let mut keys: Vec<i32> = Vec::new();
            for _ in 0..n {
                let k = rng.next_i32();
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k);
                let v: Vec<u8> = rng.bytes(12);
                let idx = m.put_binary(&i32k(k), &v);
                assert!(idx >= 0);
            }
            // every present key must be found at the same index in both
            for &k in &keys {
                let i = m.get(&i32k(k));
                assert!(i >= 0, "key {k} missing after insert");
            }
            // absent keys
            for _ in 0..n.max(8) {
                let mut k = rng.next_i32();
                while keys.contains(&k) {
                    k = k.wrapping_add(1);
                }
                assert_eq!(m.get(&i32k(k)), -1, "absent key {k} reported present");
            }
            m.free();
        }
    }
}

#[test]
fn row24_binary_struct2_map() {
    let (p, _g) = libs();
    for &n in &[1usize, 5, 6, 8, 20, 100] {
        let mut rng = Rng::new(0xC0FFEE ^ 24 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg_struct2());
        let mut keys: Vec<[u8; 8]> = Vec::new();
        for _ in 0..n {
            let k: [u8; 8] = rng.bytes(8).try_into().unwrap();
            if keys.contains(&k) {
                continue;
            }
            keys.push(k);
            let v = rng.bytes(12);
            m.put_binary(&k, &v);
        }
        for k in &keys {
            assert!(m.get(k) >= 0);
        }
        for _ in 0..16 {
            let k: [u8; 8] = rng.bytes(8).try_into().unwrap();
            if !keys.contains(&k) {
                assert_eq!(m.get(&k), -1);
            }
        }
        m.free();
    }
}

fn keysize_cfg(keysize: usize, elemsize: usize) -> MapCfg {
    // value covers every byte the library does not write itself, so that the
    // whole element is defined and can be compared byte-for-byte.
    MapCfg {
        elemsize,
        keysize,
        keyoffset: 0,
        put_mode: HM_BINARY,
        del_mode: HM_BINARY,
        fmt: ElemFmt::Raw,
        value_off: keysize,
        value_len: elemsize - keysize,
    }
}

#[test]
fn row25_26_27_28_binary_keysizes() {
    // keysize 1 (heavy collisions), 2, 16 (two siphash body blocks) and 0
    // (every key aliases, ERRORS.md row 29).
    let (p, _g) = libs();
    for &keysize in &[1usize, 2, 16, 0] {
        let elemsize = 32usize;
        let cfg = keysize_cfg(keysize, elemsize);
        for &n in &[1usize, 5, 8, 20, 60] {
            let mut rng = Rng::new(0xC0FFEE ^ 25 ^ (keysize as u64) << 16 ^ n as u64);
            reset_seed(p, DEFAULT_SEED);
            let mut m = Maps::empty(p, cfg);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for _ in 0..n {
                let k = rng.bytes(keysize);
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k.clone());
                let v = rng.bytes(cfg.value_len);
                m.put_binary(&k, &v);
            }
            for k in &keys {
                let i = m.get(k);
                assert!(i >= 0, "keysize={keysize} key={k:02x?} missing");
            }
            for _ in 0..10 {
                let k = rng.bytes(keysize);
                let expect_present = keysize == 0 || keys.contains(&k);
                let got = m.get(&k);
                if expect_present {
                    assert!(got >= 0 || keys.is_empty());
                } else {
                    assert_eq!(got, -1, "keysize={keysize} absent key {k:02x?}");
                }
            }
            m.free();
        }
    }
}

#[test]
fn row29_binary_duplicate_put_overwrite() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 29);
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    let keys: Vec<i32> = (0..40).map(|_| rng.next_i32()).collect();
    let mut first_idx = Vec::new();
    for &k in &keys {
        first_idx.push(m.put_binary(&i32k(k), &rng.bytes(12)));
    }
    // re-put every key many times: must reuse the same index and not grow
    for round in 0..5 {
        for (j, &k) in keys.iter().enumerate() {
            let i = m.put_binary(&i32k(k), &rng.bytes(12));
            assert_eq!(
                i, first_idx[j],
                "round {round}: re-put of {k} moved from {} to {i}",
                first_idx[j]
            );
        }
    }
    let cs = m.snap_c();
    assert_eq!(cs.length, keys.len() + 1);
    m.free();
}

#[test]
fn row30_binary_full_growth_chain() {
    // 1000 keys drives make_hash_index through 8 -> 16 -> ... -> 2048.
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 30);
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    let mut keys = Vec::new();
    for _ in 0..1000 {
        let k = rng.next_i32();
        if keys.contains(&k) {
            continue;
        }
        keys.push(k);
        m.put_binary(&i32k(k), &rng.bytes(12));
    }
    let cs = m.snap_c();
    assert!(
        cs.table.as_ref().unwrap().slot_count >= 1024,
        "expected a grown table, got {}",
        cs.table.as_ref().unwrap().slot_count
    );
    for &k in &keys {
        assert!(m.get(&i32k(k)) >= 0);
    }
    m.free();
}

#[test]
fn row39_del_last_element() {
    let (p, _g) = libs();
    for &n in &[1usize, 2, 5, 8, 20, 60] {
        let mut rng = Rng::new(0xC0FFEE ^ 39 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg_struct());
        let mut keys = Vec::new();
        for _ in 0..n {
            let k = rng.next_i32();
            if keys.contains(&k) {
                continue;
            }
            keys.push(k);
            m.put_binary(&i32k(k), &rng.bytes(12));
        }
        // delete from the back: old_index == final_index every time
        while let Some(k) = keys.pop() {
            assert_eq!(m.del(&i32k(k)), 1, "delete of last element failed");
            assert_eq!(m.get(&i32k(k)), -1);
            for &r in &keys {
                assert!(m.get(&i32k(r)) >= 0, "{r} lost after tail delete");
            }
        }
        m.free();
    }
}

#[test]
fn row40_del_middle_element() {
    let (p, _g) = libs();
    for &n in &[2usize, 3, 8, 20, 60] {
        let mut rng = Rng::new(0xC0FFEE ^ 40 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg_struct());
        let mut keys = Vec::new();
        for _ in 0..n {
            let k = rng.next_i32();
            if keys.contains(&k) {
                continue;
            }
            keys.push(k);
            m.put_binary(&i32k(k), &rng.bytes(12));
        }
        // always delete the front element: forces memmove + re-find + patch
        while !keys.is_empty() {
            let k = keys.remove(0);
            assert_eq!(m.del(&i32k(k)), 1);
            assert_eq!(m.get(&i32k(k)), -1);
            for &r in &keys {
                assert!(m.get(&i32k(r)) >= 0, "{r} lost after middle delete");
            }
        }
        m.free();
    }
}

#[test]
fn row41_random_interleaved_ops() {
    let (p, _g) = libs();
    for trial in 0..12u64 {
        let mut rng = Rng::new(0xC0FFEE ^ 41 ^ trial);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg_struct());
        // small key space => many collisions, tombstones and re-inserts
        let space: Vec<i32> = (0..40).map(|i| i * 7 - 100).collect();
        let mut live: Vec<i32> = Vec::new();
        for _ in 0..500 {
            let k = space[rng.below(space.len())];
            match rng.below(10) {
                0..=4 => {
                    m.put_binary(&i32k(k), &rng.bytes(12));
                    if !live.contains(&k) {
                        live.push(k);
                    }
                }
                5..=6 => {
                    let want = if live.contains(&k) { 1 } else { 0 };
                    let got = m.del(&i32k(k));
                    assert_eq!(got, want, "del({k}) expected {want}");
                    live.retain(|&x| x != k);
                }
                7..=8 => {
                    let i = m.get(&i32k(k));
                    assert_eq!(i >= 0, live.contains(&k), "get({k}) presence mismatch");
                }
                _ => {
                    let i = m.get_ts(&i32k(k));
                    assert_eq!(i >= 0, live.contains(&k), "get_ts({k}) presence mismatch");
                }
            }
        }
        m.free();
    }
}

#[test]
fn row42_del_crosses_shrink_threshold() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 42);
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    let mut keys = Vec::new();
    for _ in 0..100 {
        let k = rng.next_i32();
        if keys.contains(&k) {
            continue;
        }
        keys.push(k);
        m.put_binary(&i32k(k), &rng.bytes(12));
    }
    let start_slots = m.snap_c().table.as_ref().unwrap().slot_count;
    // delete until the table shrinks at least twice
    let mut shrinks = 0;
    let mut prev = start_slots;
    while let Some(k) = keys.pop() {
        m.del(&i32k(k));
        let now = m.snap_c().table.as_ref().unwrap().slot_count;
        if now < prev {
            shrinks += 1;
            prev = now;
        }
    }
    assert!(shrinks >= 2, "expected shrinking, saw {shrinks}");
    m.free();
}

#[test]
fn row43_del_reinsert_crosses_tombstone_threshold() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 43);

    // (a) enough deletes to cross tombstone_count_threshold while used_count
    //     stays above used_count_shrink_threshold => `make_hash_index(same)`
    //     rebuild rather than a shrink.
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    let mut keys: Vec<i32> = Vec::new();
    while keys.len() < 700 {
        let k = rng.next_i32();
        if keys.contains(&k) {
            continue;
        }
        keys.push(k);
        m.put_binary(&i32k(k), &rng.bytes(12));
    }
    let t = m.snap_c().table.clone().unwrap();
    assert_eq!(t.slot_count, 1024, "expected a 1024-slot table");
    let mut rebuilds = 0usize;
    let mut prev_tomb = 0usize;
    let mut prev_slots = t.slot_count;
    for _ in 0..250 {
        let k = keys.pop().unwrap();
        assert_eq!(m.del(&i32k(k)), 1);
        let t = m.snap_c().table.clone().unwrap();
        if t.tombstone_count < prev_tomb && t.slot_count == prev_slots {
            rebuilds += 1;
        }
        prev_tomb = t.tombstone_count;
        prev_slots = t.slot_count;
    }
    assert!(rebuilds > 0, "expected at least one tombstone rebuild");
    for &k in &keys {
        assert!(m.get(&i32k(k)) >= 0, "{k} lost across rebuild");
    }
    m.free();

    // (b) delete/re-insert churn: exercises the tombstone-reuse branch of
    //     hmput_key (CONFIGS.md row 24 / ERRORS.md row 24) over and over.
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    let keys: Vec<i32> = (0..64).map(|i| i * 3 + 1).collect();
    for &k in &keys {
        m.put_binary(&i32k(k), &rng.bytes(12));
    }
    for _ in 0..20 {
        for &k in &keys {
            assert_eq!(m.del(&i32k(k)), 1);
            m.put_binary(&i32k(k), &rng.bytes(12));
        }
    }
    for &k in &keys {
        assert!(m.get(&i32k(k)) >= 0);
    }
    m.free();
}

#[test]
fn row44_delete_all_then_reinsert() {
    let (p, _g) = libs();
    for &n in &[1usize, 8, 40] {
        let mut rng = Rng::new(0xC0FFEE ^ 44 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg_struct());
        let keys: Vec<i32> = (0..n as i32).map(|i| i * 13 - 50).collect();
        for _ in 0..4 {
            for &k in &keys {
                m.put_binary(&i32k(k), &rng.bytes(12));
            }
            for &k in &keys {
                assert_eq!(m.del(&i32k(k)), 1);
            }
            assert_eq!(m.snap_c().length, 1, "map should be logically empty");
            for &k in &keys {
                assert_eq!(m.get(&i32k(k)), -1);
            }
        }
        m.free();
    }
}

#[test]
fn row48_nonzero_keyoffset_delete() {
    // `keyoffset` is only reachable through hmdel_key's explicit parameter.
    // Element layout: 8 bytes of value, then the 4-byte key at offset 8.
    let (p, _g) = libs();
    let cfg = MapCfg {
        elemsize: 24,
        keysize: 4,
        keyoffset: 8,
        put_mode: HM_BINARY,
        del_mode: HM_BINARY,
        fmt: ElemFmt::Raw,
        value_off: 4,
        value_len: 20,
    };
    for &n in &[1usize, 5, 8, 30] {
        let mut rng = Rng::new(0xC0FFEE ^ 48 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg);
        let mut keys = Vec::new();
        for _ in 0..n {
            let k = rng.next_i32();
            if keys.contains(&k) {
                continue;
            }
            keys.push(k);
            // hmput_key memcpys the key at offset 0; put_binary then also
            // writes it at keyoffset, exactly like a real `key` struct field.
            m.put_binary(&i32k(k), &rng.bytes(20));
        }
        while !keys.is_empty() {
            let k = keys.remove(rng.below(keys.len()));
            m.del(&i32k(k));
        }
        m.free();
    }
}

#[test]
fn row49_hmfree_after_binary_map() {
    let (p, _g) = libs();
    for &n in &[0usize, 1, 8, 100] {
        let mut rng = Rng::new(0xC0FFEE ^ 49 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg_struct());
        for _ in 0..n {
            m.put_binary(&i32k(rng.next_i32()), &rng.bytes(12));
        }
        m.compare("before free");
        m.free();
    }
    // hmfree_func(NULL) must be a no-op in both (ERRORS.md row 8)
    unsafe {
        (p.c.hmfree_func)(std::ptr::null_mut(), 16);
        (p.rs.hmfree_func)(std::ptr::null_mut(), 16);
    }
    // hmfree_func on a table-less array (ERRORS.md row 9)
    unsafe {
        let ca = (p.c.arrgrowf)(std::ptr::null_mut(), 16, 3, 0);
        let ra = (p.rs.arrgrowf)(std::ptr::null_mut(), 16, 3, 0);
        (p.c.hmfree_func)(ca, 16);
        (p.rs.hmfree_func)(ra, 16);
        let _ = (ca as *mut c_void, ra as *mut c_void);
    }
}
