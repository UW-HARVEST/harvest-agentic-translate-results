//! Phase B rows 18-27, 37, 39-47, 52, 67: binary-mode (`memcmp`) hash maps
//! driven through the low-level `stbds_hm*` entry points.
mod common;
use common::*;
use std::ffi::c_void;

/// Put `key` (4 bytes) and then write a 4-byte value, mimicking `stbds_hmput`.
fn put_kv(m: &mut MapPair, key: u32, value: u32, mode: i32) {
    let mut k = key;
    m.put(&mut k as *mut u32 as *mut c_void, 4, mode);
    unsafe { m.fill_tail(4, value as u64) };
}

// ---------------------------------------------------------------------------
// rows 18-20: stbds_hmput_default
// ---------------------------------------------------------------------------

// row 18
#[test]
fn row18_put_default_from_null() {
    let _s = session(0x31415926);
    for &elemsize in &[4usize, 8, 12, 16, 24, 32, 64] {
        seed_both(0x31415926);
        let mut m = MapPair::null(elemsize, "put_default/null");
        m.put_default();
        m.assert_same("after first hmput_default");
        assert_eq!(m.snap_c().length, 1);
        assert!(m.snap_c().table.is_none(), "no hash table yet");
        m.free();
    }
}

// row 19
#[test]
fn row19_put_default_twice_is_noop() {
    let _s = session(0x31415926);
    seed_both(0x31415926);
    let mut m = MapPair::null(16, "put_default/twice");
    m.put_default();
    let (c1, r1) = (m.c, m.r);
    m.assert_same("first");
    m.put_default();
    assert_eq!(m.c, c1, "C second hmput_default must be a no-op");
    assert_eq!(m.r, r1, "Rust second hmput_default must be a no-op");
    m.assert_same("second");
    m.free();
}

// row 20
#[test]
fn row20_put_default_on_populated() {
    let _s = session(0x31415926);
    seed_both(7);
    let mut m = MapPair::null(8, "put_default/populated");
    for i in 0..20u32 {
        put_kv(&mut m, i, i * 3, HM_BINARY);
    }
    m.assert_same("populated");
    m.put_default();
    m.assert_same("hmput_default on populated map (must be a no-op)");
    m.free();
}

// ---------------------------------------------------------------------------
// rows 21-27: stbds_hmput_key, binary mode
// ---------------------------------------------------------------------------

// row 21: keysize == elemsize (set-like map)
#[test]
fn row21_put_keysize_eq_elemsize() {
    let _s = session(0x31415926);
    for &sz in &[1usize, 2, 4, 8, 16, 32] {
        let mut rng = Rng::with(21 + sz as u64);
        seed_both(0x31415926);
        let mut m = MapPair::null(sz, "put/keysize==elemsize");
        for n in 0..60 {
            let key = rng.bytes(sz);
            let kp = m.own(&key);
            m.put(kp, sz, HM_BINARY);
            m.assert_same(&format!("after put #{n} sz={sz}"));
        }
        m.free();
    }
}

// row 22: crossing the 8-slot grow threshold precisely
#[test]
fn row22_put_grow_threshold() {
    let _s = session(0x31415926);
    for n in 0..12u32 {
        seed_both(0x31415926);
        let mut m = MapPair::null(8, "put/threshold");
        for i in 0..n {
            put_kv(&mut m, i + 1, i * 7 + 1, HM_BINARY);
            m.assert_same(&format!("n={n} after insert {i}"));
        }
        m.free();
    }
}

// row 23: many random distinct 32-bit keys (several doublings)
#[test]
fn row23_put_many_u32_keys() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(23);
    for &n in &[1usize, 2, 5, 7, 8, 9, 16, 17, 50, 128, 300] {
        seed_both(0x31415926);
        let mut m = MapPair::null(8, "put/many-u32");
        let mut seen = std::collections::HashSet::new();
        let mut inserted = 0;
        while inserted < n {
            let k = rng.next_u64() as u32;
            if !seen.insert(k) {
                continue;
            }
            put_kv(&mut m, k, inserted as u32, HM_BINARY);
            inserted += 1;
        }
        m.assert_same(&format!("after {n} inserts"));
        // every key must be found at the same index in both
        for &k in seen.iter() {
            let mut kk = k;
            let (a, b) = m.get(&mut kk as *mut u32 as *mut c_void, 4, HM_BINARY);
            assert_eq!(a, b, "get index differs for key {k}");
            assert!(a >= 0, "key {k} must be present");
        }
        m.assert_same("after gets");
        m.free();
    }
}

// row 24: 64-bit keys
#[test]
fn row24_put_u64_keys() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(24);
    for &n in &[1usize, 8, 9, 64, 200] {
        seed_both(0x31415926);
        let mut m = MapPair::null(16, "put/u64");
        for i in 0..n {
            let k = rng.next_u64();
            let kp = m.own(&k.to_ne_bytes());
            m.put(kp, 8, HM_BINARY);
            unsafe { m.fill_tail(8, i as u64) };
        }
        m.assert_same(&format!("u64 keys n={n}"));
        m.free();
    }
}

// row 25: 16-byte struct keys (`int key[2]` style)
#[test]
fn row25_put_struct_keys() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(25);
    for &n in &[1usize, 8, 9, 100] {
        seed_both(0x31415926);
        let mut m = MapPair::null(32, "put/struct16");
        for i in 0..n {
            let k = rng.bytes(16);
            let kp = m.own(&k);
            m.put(kp, 16, HM_BINARY);
            unsafe { m.fill_tail(16, i as u64) };
        }
        m.assert_same(&format!("struct keys n={n}"));
        m.free();
    }
}

// row 26: duplicate keys interleaved => update path
#[test]
fn row26_put_duplicate_keys() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(26);
    seed_both(0x31415926);
    let mut m = MapPair::null(8, "put/dupes");
    let keys: Vec<u32> = (0..12).map(|_| rng.next_u64() as u32).collect();
    for round in 0..25u32 {
        for (i, &k) in keys.iter().enumerate() {
            put_kv(&mut m, k, round * 100 + i as u32, HM_BINARY);
            m.assert_same(&format!("round {round} key {i}"));
        }
    }
    m.free();
}

// row 27: sub-word keys => massive hash collisions
#[test]
fn row27_put_tiny_keys() {
    let _s = session(0x31415926);
    for &keysize in &[1usize, 2] {
        seed_both(0x31415926);
        let n = 1usize << (8 * keysize);
        let mut m = MapPair::null(8, "put/tiny");
        for i in 0..n.min(600) {
            let k = (i as u64).to_ne_bytes();
            let kp = m.own(&k[..keysize]);
            m.put(kp, keysize, HM_BINARY);
            unsafe { m.fill_tail(keysize, i as u64) };
        }
        m.assert_same(&format!("tiny keys keysize={keysize}"));
        m.free();
    }
}

// ---------------------------------------------------------------------------
// rows 37, 39-42: hmget_key / hmget_key_ts
// ---------------------------------------------------------------------------

// row 37
#[test]
fn row37_get_present_and_absent() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(37);
    seed_both(0x31415926);
    let mut m = MapPair::null(8, "get/binary");
    let present: Vec<u32> = (0..80).map(|i| i * 3 + 1).collect();
    for (i, &k) in present.iter().enumerate() {
        put_kv(&mut m, k, i as u32, HM_BINARY);
    }
    m.assert_same("populated");
    for &k in &present {
        let (a, b) = m.get(&mut { k } as *mut u32 as *mut c_void, 4, HM_BINARY);
        assert_eq!(a, b, "present key {k}");
        assert!(a >= 0);
    }
    for _ in 0..500 {
        let k = rng.next_u64() as u32 | 0x8000_0000;
        let (a, b) = m.get(&mut { k } as *mut u32 as *mut c_void, 4, HM_BINARY);
        assert_eq!(a, b, "absent key {k}");
    }
    m.assert_same("after gets");
    m.free();
}

// row 39: hmget_key on a NULL map creates it
#[test]
fn row39_get_on_null_map() {
    let _s = session(0x31415926);
    for &elemsize in &[4usize, 8, 16, 32] {
        seed_both(0x31415926);
        let mut m = MapPair::null(elemsize, "get/null");
        let mut k = 12345u32;
        let (a, b) = m.get(&mut k as *mut u32 as *mut c_void, 4, HM_BINARY);
        assert_eq!(a, b);
        assert_eq!(a, -1, "must report STBDS_INDEX_EMPTY");
        m.assert_same("after get on NULL map");
        assert_eq!(m.snap_c().length, 1);
        assert!(m.snap_c().table.is_none());
        m.free();
    }
}

// row 40: hmget_key on a map that has no hash table yet
#[test]
fn row40_get_on_tableless_map() {
    let _s = session(0x31415926);
    seed_both(0x31415926);
    let mut m = MapPair::null(8, "get/tableless");
    m.put_default();
    m.assert_same("after put_default");
    for k in 0..20u32 {
        let (a, b) = m.get(&mut { k } as *mut u32 as *mut c_void, 4, HM_BINARY);
        assert_eq!(a, b);
        assert_eq!(a, -1);
    }
    m.assert_same("after gets on tableless map");
    m.free();
}

// row 41: hmget_key_ts uses the out-param, leaving the header `temp` alone
#[test]
fn row41_get_ts() {
    let _s = session(0x31415926);
    seed_both(0x31415926);
    let mut m = MapPair::null(8, "get_ts");
    for i in 0..40u32 {
        put_kv(&mut m, i * 5, i, HM_BINARY);
    }
    m.assert_same("populated");
    let before_c = unsafe { m.temp_c() };
    let before_r = unsafe { m.temp_r() };
    assert_eq!(before_c, before_r);
    for i in 0..40u32 {
        let (a, b) = m.get_ts(&mut { i * 5 } as *mut u32 as *mut c_void, 4, HM_BINARY);
        assert_eq!(a, b, "get_ts index for key {}", i * 5);
        assert!(a >= 0);
    }
    for i in 0..40u32 {
        let (a, b) = m.get_ts(&mut { i * 5 + 1 } as *mut u32 as *mut c_void, 4, HM_BINARY);
        assert_eq!(a, b);
        assert_eq!(a, -1, "absent key must yield -1");
    }
    unsafe {
        assert_eq!(m.temp_c(), before_c, "C header temp must be untouched");
        assert_eq!(m.temp_r(), before_r, "Rust header temp must be untouched");
    }
    m.assert_same("after get_ts sweep");
    m.free();
}

// row 41 (NULL / tableless variants)
#[test]
fn row41b_get_ts_null_and_tableless() {
    let _s = session(0x31415926);
    seed_both(0x31415926);
    let mut m = MapPair::null(8, "get_ts/null");
    let (a, b) = m.get_ts(&mut 9u32 as *mut u32 as *mut c_void, 4, HM_BINARY);
    assert_eq!((a, b), (-1, -1));
    m.assert_same("get_ts created the map");
    let (a, b) = m.get_ts(&mut 9u32 as *mut u32 as *mut c_void, 4, HM_BINARY);
    assert_eq!((a, b), (-1, -1), "tableless map path");
    m.assert_same("get_ts on tableless map");
    m.free();
}

// row 42: interleave hmget_key and hmget_key_ts
#[test]
fn row42_get_and_get_ts_interleaved() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(42);
    seed_both(0x31415926);
    let mut m = MapPair::null(8, "get+get_ts");
    for i in 0..60u32 {
        put_kv(&mut m, i, i, HM_BINARY);
    }
    for _ in 0..600 {
        let k = (rng.below(120)) as u32;
        if rng.next_u64() & 1 == 0 {
            let (a, b) = m.get(&mut { k } as *mut u32 as *mut c_void, 4, HM_BINARY);
            assert_eq!(a, b);
        } else {
            let (a, b) = m.get_ts(&mut { k } as *mut u32 as *mut c_void, 4, HM_BINARY);
            assert_eq!(a, b);
        }
        m.assert_same("interleaved");
    }
    m.free();
}

// ---------------------------------------------------------------------------
// rows 43-47, 52: stbds_hmdel_key, binary mode
// ---------------------------------------------------------------------------

// row 43: delete the tail element (old_index == final_index, no relocate)
#[test]
fn row43_del_tail() {
    let _s = session(0x31415926);
    for &n in &[1usize, 2, 5, 8, 20, 100] {
        seed_both(0x31415926);
        let mut m = MapPair::null(8, "del/tail");
        for i in 0..n as u32 {
            put_kv(&mut m, i + 1, i, HM_BINARY);
        }
        m.assert_same("populated");
        for i in (0..n as u32).rev() {
            m.del(&mut { i + 1 } as *mut u32 as *mut c_void, 4, 0, HM_BINARY);
            m.assert_same(&format!("n={n} after deleting tail {i}"));
            unsafe { assert_eq!(m.temp_c(), 1, "temp must be 1 for a successful delete") };
        }
        m.free();
    }
}

// row 44: delete a middle element => relocate + re-find + index fixup
#[test]
fn row44_del_middle() {
    let _s = session(0x31415926);
    for &n in &[3usize, 8, 9, 33, 120] {
        seed_both(0x31415926);
        let mut m = MapPair::null(8, "del/middle");
        for i in 0..n as u32 {
            put_kv(&mut m, i + 1, i * 11, HM_BINARY);
        }
        m.assert_same("populated");
        let mut alive: Vec<u32> = (1..=n as u32).collect();
        while alive.len() > 1 {
            let idx = alive.len() / 2;
            let k = alive.remove(idx);
            m.del(&mut { k } as *mut u32 as *mut c_void, 4, 0, HM_BINARY);
            m.assert_same(&format!("n={n} after deleting middle key {k}"));
        }
        m.free();
    }
}

// row 45: delete every key in a random order (drives the shrink chain)
#[test]
fn row45_del_all_random_order() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(45);
    for &n in &[1usize, 8, 40, 150, 400] {
        seed_both(0x31415926);
        let mut m = MapPair::null(8, "del/all");
        let mut keys: Vec<u32> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while keys.len() < n {
            let k = rng.next_u64() as u32;
            if seen.insert(k) {
                keys.push(k);
            }
        }
        for (i, &k) in keys.iter().enumerate() {
            put_kv(&mut m, k, i as u32, HM_BINARY);
        }
        m.assert_same("populated");
        // shuffle
        for i in (1..keys.len()).rev() {
            let j = rng.below(i as u64 + 1) as usize;
            keys.swap(i, j);
        }
        for (step, &k) in keys.iter().enumerate() {
            m.del(&mut { k } as *mut u32 as *mut c_void, 4, 0, HM_BINARY);
            m.assert_same(&format!("n={n} step {step} deleted {k}"));
        }
        assert_eq!(m.snap_c().length, 1, "only the default element left");
        m.free();
    }
}

// row 46: churn to exceed tombstone_count_threshold => same-size rebuild
#[test]
fn row46_del_tombstone_rebuild() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(46);
    seed_both(0x31415926);
    let mut m = MapPair::null(8, "del/tombstones");
    // fill enough to reach slot_count 128+, then delete/insert in a pattern
    for i in 0..200u32 {
        put_kv(&mut m, i, i, HM_BINARY);
    }
    m.assert_same("filled");
    for step in 0..600u32 {
        // delete an existing key then insert a brand-new one: used_count stays
        // roughly constant while tombstone_count climbs.
        let victim = rng.below(200) as u32;
        m.del(&mut { victim } as *mut u32 as *mut c_void, 4, 0, HM_BINARY);
        put_kv(&mut m, 1_000_000 + step, step, HM_BINARY);
        m.assert_same(&format!("churn step {step}"));
    }
    m.free();
}

// row 47: delete an absent key
#[test]
fn row47_del_absent() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(47);
    seed_both(0x31415926);
    let mut m = MapPair::null(8, "del/absent");
    for i in 0..50u32 {
        put_kv(&mut m, i, i, HM_BINARY);
    }
    m.assert_same("populated");
    for _ in 0..400 {
        let k = 1000 + rng.next_u64() as u32;
        m.del(&mut { k } as *mut u32 as *mut c_void, 4, 0, HM_BINARY);
        unsafe { assert_eq!(m.temp_c(), 0, "temp must be 0 when nothing was deleted") };
        unsafe { assert_eq!(m.temp_r(), 0) };
        m.assert_same("after deleting an absent key");
    }
    assert_eq!(m.snap_c().length, 51);
    m.free();
}

// row 52: full random churn
#[test]
fn row52_random_churn_binary() {
    let _s = session(0x31415926);
    for trial in 0..6u64 {
        let mut rng = Rng::with(520 + trial);
        seed_both(0x31415926);
        let elemsize = [8usize, 16, 32, 8, 12, 24][trial as usize];
        let keysize = [4usize, 8, 16, 4, 4, 8][trial as usize];
        let mut m = MapPair::null(elemsize, "churn/binary");
        let mut live: Vec<Vec<u8>> = Vec::new();
        for step in 0..500 {
            let op = rng.below(10);
            if op < 5 || live.is_empty() {
                let mut k = rng.bytes(keysize);
                // bias toward small key space so collisions & updates happen
                k[0] = (rng.below(40)) as u8;
                let kp = m.own(&k);
                m.put(kp, keysize, HM_BINARY);
                unsafe { m.fill_tail(keysize, step as u64) };
                if !live.contains(&k) {
                    live.push(k);
                }
            } else if op < 8 {
                let i = rng.below(live.len() as u64) as usize;
                let k = live[i].clone();
                let kp = m.own(&k);
                let (a, b) = m.get(kp, keysize, HM_BINARY);
                assert_eq!(a, b, "step {step}");
                assert!(a >= 0);
            } else {
                let i = rng.below(live.len() as u64) as usize;
                let k = live.remove(i);
                let kp = m.own(&k);
                m.del(kp, keysize, 0, HM_BINARY);
            }
            m.assert_same(&format!("trial {trial} step {step}"));
        }
        m.free();
    }
}

// row 67: composed pipeline exercised through the low-level entry points
#[test]
fn row67_pipeline_binary() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(67);
    for &elemsize in &[8usize, 16, 32] {
        seed_both(rng.next_u64() as usize);
        let mut m = MapPair::null(elemsize, "pipeline/binary");
        m.put_default();
        m.assert_same("put_default");
        let mut keys: Vec<u32> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while keys.len() < 200 {
            let k = rng.next_u64() as u32;
            if seen.insert(k) {
                keys.push(k);
            }
        }
        for (i, &k) in keys.iter().enumerate() {
            put_kv(&mut m, k, i as u32, HM_BINARY);
            m.assert_same("pipeline put");
        }
        for &k in &keys {
            let (a, b) = m.get(&mut { k } as *mut u32 as *mut c_void, 4, HM_BINARY);
            assert_eq!(a, b);
        }
        m.assert_same("pipeline gets");
        for &k in &keys {
            m.del(&mut { k } as *mut u32 as *mut c_void, 4, 0, HM_BINARY);
            m.assert_same("pipeline del");
        }
        m.free();
    }
}
