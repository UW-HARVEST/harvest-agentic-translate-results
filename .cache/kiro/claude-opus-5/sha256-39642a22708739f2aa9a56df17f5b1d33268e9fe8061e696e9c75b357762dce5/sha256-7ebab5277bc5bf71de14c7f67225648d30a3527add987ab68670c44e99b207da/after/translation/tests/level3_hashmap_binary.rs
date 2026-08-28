//! Level 3: binary-keyed hash maps — `stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmput_default`, `stbds_hmdel_key`,
//! `stbds_hmfree_func` (and, indirectly, `stbds_make_hash_index`,
//! `stbds_hm_find_slot` and `stbds_is_key_equal`).

mod harness;

use harness::map::*;
use harness::*;
use std::ffi::c_void;

/// `int key; int value;`
const INT_ELEM: usize = 8;
const INT_KEY: usize = 4;

fn ikey(k: i32) -> Vec<u8> {
    k.to_ne_bytes().to_vec()
}

#[test]
fn hmput_key_on_null_creates_default_slot() {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    let mut k = ikey(1);
    m.put(&mut k, &[7, 0, 0, 0], "first put");
    let s = m.snap_c();
    assert_eq!(s.header.length, 2, "default element + one entry");
    assert_eq!(s.index.as_ref().unwrap().slot_count, 8);
    assert_eq!(s.index.as_ref().unwrap().used_count, 1);
    m.free();
}

#[test]
fn hmget_key_on_null() {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    let mut k = ikey(5);
    let t = m.get(&mut k, "get on null map");
    assert_eq!(t, -1);
    m.free();
}

#[test]
fn hmget_key_ts_on_null() {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    let mut k = ikey(5);
    let t = m.get_ts(&mut k, "get_ts on null map");
    assert_eq!(t, -1);
    m.free();
}

#[test]
fn hmdel_key_on_null_returns_null() {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    let mut k = ikey(5);
    let r = m.del(&mut k, 0, "del on null map");
    assert_eq!(r, 0);
    assert!(m.ct.is_null() && m.rt.is_null());
}

#[test]
fn hmput_default_then_puts() {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    m.put_default(&[0xaa], "hmdefault on null");
    m.put_default(&[0xbb], "hmdefault again");
    for k in 0..20i32 {
        let mut kb = ikey(k);
        m.put(&mut kb, &(k * 3).to_ne_bytes(), &format!("put {k} after default"));
    }
    for k in -5..25i32 {
        let mut kb = ikey(k);
        m.get(&mut kb, &format!("get {k} after default"));
    }
    m.free();
}

/// The default slot must also be creatable on an existing map whose length is 0
/// (the `length == 0` branch of `stbds_hmput_default`).
#[test]
fn hmput_default_on_zero_length_array() {
    let p = pair();
    unsafe {
        // hand-build an array with length 0 to hit the second branch
        let ca = (p.c.arrgrowf)(std::ptr::null_mut(), INT_ELEM, 0, 1);
        let ra = (p.r.arrgrowf)(std::ptr::null_mut(), INT_ELEM, 0, 1);
        let ct = (p.c.hmput_default)((ca as *mut u8).add(INT_ELEM) as *mut c_void, INT_ELEM);
        let rt = (p.r.hmput_default)((ra as *mut u8).add(INT_ELEM) as *mut c_void, INT_ELEM);
        // zero the default element so the comparison is over defined bytes
        std::ptr::write_bytes((ct as *mut u8).sub(INT_ELEM), 0, INT_ELEM);
        std::ptr::write_bytes((rt as *mut u8).sub(INT_ELEM), 0, INT_ELEM);
        assert_eq!(
            snap::snap_map(ct, INT_ELEM, snap::KeyKind::Binary),
            snap::snap_map(rt, INT_ELEM, snap::KeyKind::Binary),
        );
        (p.c.hmfree_func)((ct as *mut u8).sub(INT_ELEM) as *mut c_void, INT_ELEM);
        (p.r.hmfree_func)((rt as *mut u8).sub(INT_ELEM) as *mut c_void, INT_ELEM);
    }
}

fn insert_range(n: i32) {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    for k in 0..n {
        let mut kb = ikey(k);
        m.put(&mut kb, &(k ^ 0x5a5a).to_ne_bytes(), &format!("put {k}/{n}"));
    }
    for k in 0..n {
        let mut kb = ikey(k);
        let t = m.get(&mut kb, &format!("get {k}/{n}"));
        assert!(t >= 0, "key {k} should be present");
    }
    for k in n..(n + 20) {
        let mut kb = ikey(k);
        let t = m.get(&mut kb, &format!("get-absent {k}/{n}"));
        assert_eq!(t, -1, "key {k} should be absent");
    }
    m.free();
}

#[test]
fn hmput_growth_boundaries() {
    // slot_count 8 => used_count_threshold 6, so 6 triggers the first rehash
    for n in [1, 2, 5, 6, 7, 8, 12, 13, 24, 25, 48, 49] {
        insert_range(n);
    }
}

#[test]
fn hmput_many() {
    insert_range(500);
}

#[test]
fn hmput_duplicate_keys_overwrite() {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    for round in 0..4 {
        for k in 0..30i32 {
            let mut kb = ikey(k);
            m.put(
                &mut kb,
                &(k * 100 + round).to_ne_bytes(),
                &format!("put round {round} key {k}"),
            );
        }
    }
    m.free();
}

#[test]
fn hmdel_all_forward() {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    for k in 0..80i32 {
        let mut kb = ikey(k);
        m.put(&mut kb, &k.to_ne_bytes(), &format!("put {k}"));
    }
    for k in 0..80i32 {
        let mut kb = ikey(k);
        let r = m.del(&mut kb, 0, &format!("del {k}"));
        assert_eq!(r, 1, "key {k} should have been deleted");
    }
    m.free();
}

#[test]
fn hmdel_all_backward() {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    for k in 0..80i32 {
        let mut kb = ikey(k);
        m.put(&mut kb, &k.to_ne_bytes(), &format!("put {k}"));
    }
    for k in (0..80i32).rev() {
        let mut kb = ikey(k);
        let r = m.del(&mut kb, 0, &format!("del {k}"));
        assert_eq!(r, 1, "key {k} should have been deleted");
    }
    m.free();
}

#[test]
fn hmdel_absent_keys() {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    for k in 0..10i32 {
        let mut kb = ikey(k);
        m.put(&mut kb, &k.to_ne_bytes(), &format!("put {k}"));
    }
    for k in 100..110i32 {
        let mut kb = ikey(k);
        let r = m.del(&mut kb, 0, &format!("del-absent {k}"));
        assert_eq!(r, 0, "key {k} was never inserted");
    }
    m.free();
}

/// Interleaved insert / delete / lookup — exercises tombstone reuse, the
/// tombstone rebuild threshold and the shrink threshold.
#[test]
fn hmap_random_churn() {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    let mut live: Vec<i32> = Vec::new();
    let mut s: u64 = 0x1234_5678_9abc_def0;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    for step in 0..3000u32 {
        let r = next();
        let op = r % 100;
        if op < 55 || live.is_empty() {
            let k = ((r >> 8) % 400) as i32;
            let mut kb = ikey(k);
            m.put(&mut kb, &(k ^ 0x33).to_ne_bytes(), &format!("churn {step}: put {k}"));
            if !live.contains(&k) {
                live.push(k);
            }
        } else if op < 85 {
            let idx = ((r >> 8) as usize) % live.len();
            let k = live.swap_remove(idx);
            let mut kb = ikey(k);
            let got = m.del(&mut kb, 0, &format!("churn {step}: del {k}"));
            assert_eq!(got, 1, "churn {step}: key {k} should have been live");
        } else {
            let k = ((r >> 8) % 400) as i32;
            let mut kb = ikey(k);
            let t = m.get(&mut kb, &format!("churn {step}: get {k}"));
            assert_eq!(
                t >= 0,
                live.contains(&k),
                "churn {step}: presence of {k} disagrees with model"
            );
        }
    }
    m.free();
}

/// Different `keysize` / `elemsize` combinations put `stbds_hash_bytes` and
/// `memcmp` through all of their partial-word paths.
#[test]
fn hmap_various_key_sizes() {
    for (elemsize, keysize) in [
        (2usize, 1usize),
        (4, 2),
        (4, 3),
        (8, 4),
        (8, 5),
        (8, 6),
        (8, 7),
        (8, 8),
        (12, 8),
        (16, 9),
        (20, 8),
        (24, 12),
        (24, 16),
        (32, 20),
        (40, 33),
    ] {
        let p = pair();
        let mut m = MapPair::binary(&p, elemsize, keysize);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..60u64 {
            keys.push(key_bytes(keysize, i));
        }
        for (i, k) in keys.iter().enumerate() {
            let mut kb = k.clone();
            m.put(
                &mut kb,
                &[(i as u8).wrapping_mul(7), 0x11, 0x22],
                &format!("es={elemsize} ks={keysize} put {i}"),
            );
        }
        for (i, k) in keys.iter().enumerate() {
            let mut kb = k.clone();
            let t = m.get(&mut kb, &format!("es={elemsize} ks={keysize} get {i}"));
            assert!(t >= 0);
        }
        for (i, k) in keys.iter().enumerate().rev() {
            let mut kb = k.clone();
            m.del(&mut kb, 0, &format!("es={elemsize} ks={keysize} del {i}"));
        }
        m.free();
    }
}

/// Keys crafted so that `hash < 2` (and thus the `hash += 2` fixup) and
/// colliding low bits are hit; also mixes get / get_ts.
#[test]
fn hmap_get_ts_interleaved() {
    let p = pair();
    let mut m = MapPair::binary(&p, INT_ELEM, INT_KEY);
    for k in 0..64i32 {
        let mut kb = ikey(k << 3);
        m.put(&mut kb, &k.to_ne_bytes(), &format!("put {k}"));
    }
    for k in 0..70i32 {
        let mut kb = ikey(k << 3);
        let a = m.get_ts(&mut kb, &format!("get_ts {k}"));
        let b = m.get(&mut kb, &format!("get {k}"));
        assert_eq!(a, b, "get_ts and get disagree for {k}");
    }
    m.free();
}

#[test]
fn hmfree_func_on_null_is_noop() {
    let p = pair();
    unsafe {
        (p.c.hmfree_func)(std::ptr::null_mut(), INT_ELEM);
        (p.r.hmfree_func)(std::ptr::null_mut(), INT_ELEM);
    }
}
