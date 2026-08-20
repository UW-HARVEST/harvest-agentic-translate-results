//! Phase B — CONFIGS.md rows C24..C34 and C45: deletion, tombstones,
//! rebuild/shrink, randomized op sequences and teardown.
mod common;

use common::*;
use std::collections::HashSet;
use std::ffi::c_int;

fn distinct_bin_keys(rng: &mut Rng, keysize: usize, n: usize) -> Vec<Vec<u8>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let k = rng.bytes(keysize);
        if seen.insert(k.clone()) {
            out.push(k);
        }
    }
    out
}

fn distinct_str_keys(rng: &mut Rng, n: usize, minlen: usize, maxlen: usize) -> Vec<Vec<u8>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let len = minlen + rng.below(maxlen - minlen + 1);
        let k = rng.cstring(len);
        if seen.insert(k.clone()) {
            out.push(k);
        }
    }
    out
}

/// C24 — delete the last element (`old_index == final_index`): no `memmove`.
#[test]
fn cfg_del_last() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x24_0000);
    let shape = Shape::bin(16, 8);
    for n in [1usize, 2, 3, 6, 7, 8, 20] {
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("del-last n={n}");
        let keys = distinct_bin_keys(&mut rng, 8, n);
        unsafe {
            for (i, k) in keys.iter().enumerate() {
                let v = rng.bytes(8);
                m.put(k, HM_BINARY, &v, &format!("key#{i}"));
            }
            // repeatedly delete the *last* entry
            for i in (0..n).rev() {
                assert_eq!(
                    m.del(&keys[i], HM_BINARY, &format!("del last #{i}")),
                    1,
                    "delete must report 1"
                );
                assert_eq!(m.c.hmlen(), i as isize);
            }
            assert_eq!(m.c.hmlen(), 0);
            m.free();
        }
    }
}

/// C25 — delete the first / a middle element: `memmove` of the tail element plus
/// the slot re-point (`b->index[i] = old_index`).
#[test]
fn cfg_del_first_and_middle() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x25_0000);
    let shape = Shape::bin(24, 16);
    for n in [2usize, 3, 6, 7, 9, 20, 40] {
        for which in ["first", "middle"] {
            seed_both(0x3141_5926);
            let mut m = MapPair::empty(shape);
            m.ctx = format!("del-{which} n={n}");
            let keys = distinct_bin_keys(&mut rng, 16, n);
            unsafe {
                for (i, k) in keys.iter().enumerate() {
                    let v = rng.bytes(8);
                    m.put(k, HM_BINARY, &v, &format!("key#{i}"));
                }
                let mut live: Vec<usize> = (0..n).collect();
                while !live.is_empty() {
                    let pos = if which == "first" { 0 } else { live.len() / 2 };
                    let ki = live.remove(pos);
                    assert_eq!(
                        m.del(&keys[ki], HM_BINARY, &format!("del key#{ki}")),
                        1
                    );
                    // everything still present must still be findable
                    for &lk in &live {
                        assert!(
                            m.get(&keys[lk], HM_BINARY, &format!("survivor#{lk}")) >= 0,
                            "survivor key#{lk} vanished"
                        );
                    }
                }
                m.free();
            }
        }
    }
}

/// C26 — delete every element, forwards and backwards, incl. the shrink chain.
#[test]
fn cfg_del_all_orders() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x26_0000);
    for shape in [Shape::bin(16, 8), Shape::bin(8, 4)] {
        for n in [1usize, 5, 6, 7, 12, 13, 30, 60, 120] {
            for reverse in [false, true] {
                seed_both(0x3141_5926);
                let mut m = MapPair::empty(shape);
                m.ctx = format!("del-all {shape:?} n={n} reverse={reverse}");
                let keys = distinct_bin_keys(&mut rng, shape.keysize, n);
                unsafe {
                    for (i, k) in keys.iter().enumerate() {
                        let v = rng.bytes(8);
                        m.put(k, HM_BINARY, &v, &format!("key#{i}"));
                    }
                    let order: Vec<usize> = if reverse {
                        (0..n).rev().collect()
                    } else {
                        (0..n).collect()
                    };
                    for &i in &order {
                        assert_eq!(m.del(&keys[i], HM_BINARY, &format!("del#{i}")), 1);
                    }
                    assert_eq!(m.c.hmlen(), 0);
                    // deleting again must report "not found"
                    for &i in &order {
                        assert_eq!(m.del(&keys[i], HM_BINARY, &format!("redel#{i}")), 0);
                    }
                    m.free();
                }
            }
        }
    }
}

/// C27 — tombstone rebuild: on an 8-slot index `tombstone_count_threshold == 1`,
/// so the *second* delete rebuilds the index at the same size.
#[test]
fn cfg_del_tombstone_rebuild() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x27_0000);
    let shape = Shape::bin(16, 8);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "tombstone rebuild".into();
    let keys = distinct_bin_keys(&mut rng, 8, 5);
    unsafe {
        for (i, k) in keys.iter().enumerate() {
            let v = rng.bytes(8);
            m.put(k, HM_BINARY, &v, &format!("key#{i}"));
        }
        assert_eq!((*m.c.table()).slot_count, 8);
        assert_eq!((*m.c.table()).tombstone_count_threshold, 1);
        m.del(&keys[0], HM_BINARY, "del 1st");
        assert_eq!((*m.c.table()).tombstone_count, 1, "1 tombstone, no rebuild yet");
        m.del(&keys[1], HM_BINARY, "del 2nd -> rebuild");
        assert_eq!(
            (*m.c.table()).tombstone_count,
            0,
            "the 2nd delete must rebuild the index and drop the tombstones"
        );
        assert_eq!((*m.c.table()).slot_count, 8);
        for k in keys.iter().skip(2) {
            assert!(m.get(k, HM_BINARY, "survivor") >= 0);
        }
        m.free();
    }
}

/// C28 — shrink chain: grow to >= 32 slots, then delete down so that
/// `used_count < used_count_shrink_threshold` repeatedly halves the index.
#[test]
fn cfg_del_shrink_chain() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x28_0000);
    let shape = Shape::bin(16, 8);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "shrink chain".into();
    let keys = distinct_bin_keys(&mut rng, 8, 60);
    unsafe {
        for (i, k) in keys.iter().enumerate() {
            let v = rng.bytes(8);
            m.put(k, HM_BINARY, &v, &format!("key#{i}"));
        }
        let big = (*m.c.table()).slot_count;
        assert!(big >= 64, "expected a big index, got {big}");
        let mut seen_shrink = Vec::new();
        for (i, k) in keys.iter().enumerate() {
            m.del(k, HM_BINARY, &format!("del#{i}"));
            let sc = (*m.c.table()).slot_count;
            if seen_shrink.last() != Some(&sc) {
                seen_shrink.push(sc);
            }
        }
        assert!(
            seen_shrink.len() > 2,
            "expected the index to shrink several times, saw {seen_shrink:?}"
        );
        assert_eq!(*seen_shrink.last().unwrap(), 8, "shrinks stop at 8 slots");
        m.free();
    }
}

/// C29 — SH_DEFAULT string mode: delete / re-insert cycles (tombstone reuse).
#[test]
fn cfg_string_del_reinsert() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x29_0000);
    let shape = Shape::str_ptr(16);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "SH_DEFAULT del/reinsert".into();
    let keys = distinct_str_keys(&mut rng, 40, 0, 30);
    unsafe {
        for (i, k) in keys.iter().enumerate() {
            let v = rng.bytes(8);
            m.put_check_temp_key(k, HM_STRING, &v, &format!("key#{i}"));
        }
        for round in 0..4 {
            for i in (round..keys.len()).step_by(4) {
                assert_eq!(m.del(&keys[i], HM_STRING, &format!("del#{i} r{round}")), 1);
            }
            for i in (round..keys.len()).step_by(4) {
                let v = rng.bytes(8);
                m.put_check_temp_key(&keys[i], HM_STRING, &v, &format!("reput#{i} r{round}"));
            }
        }
        m.free();
    }
}

/// C30 — deletion across all three pointer-key string modes.
#[test]
fn cfg_del_across_string_modes() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x30_0000);
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let shape = Shape::str_ptr(16);
        seed_both(0x3141_5926);
        let mut m = MapPair::shmode(shape, mode);
        m.ctx = format!("del in shmode({mode})");
        let keys = distinct_str_keys(&mut rng, 30, 1, 40);
        unsafe {
            for (i, k) in keys.iter().enumerate() {
                let v = rng.bytes(8);
                m.put_check_temp_key(k, HM_STRING, &v, &format!("key#{i}"));
            }
            for (i, k) in keys.iter().enumerate() {
                assert_eq!(m.del(k, HM_STRING, &format!("del#{i}")), 1);
                assert_eq!(m.del(k, HM_STRING, &format!("redel#{i}")), 0);
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// C31 / C32 / C33 — randomized op sequences (property tests)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Keys {
    Bin(usize),
    Str,
}

fn random_ops(
    tag: &str,
    rng_seed: u64,
    shape: Shape,
    mode: c_int,
    sh_mode: Option<c_int>,
    keys: Keys,
    pool: usize,
    ops: usize,
) {
    let mut rng = Rng::new(rng_seed);
    let pool_keys: Vec<Vec<u8>> = (0..pool)
        .map(|_| match keys {
            Keys::Bin(ks) => rng.bytes(ks),
            Keys::Str => rng.cstring(rng.0 as usize % 24),
        })
        .collect();
    seed_both(0x3141_5926);
    let mut m = match sh_mode {
        Some(sm) => MapPair::shmode(shape, sm),
        None => MapPair::empty(shape),
    };
    m.ctx = format!("{tag} seed={rng_seed:#x} pool={pool}");
    unsafe {
        for step in 0..ops {
            let k = &pool_keys[rng.below(pool)];
            match rng.below(100) {
                0..=54 => {
                    let v = rng.bytes(8);
                    m.put(k, mode, &v, &format!("step{step} put"));
                }
                55..=69 => {
                    m.get(k, mode, &format!("step{step} get"));
                }
                70..=76 => {
                    m.get_ts(k, mode, &format!("step{step} get_ts"));
                }
                _ => {
                    m.del(k, mode, &format!("step{step} del"));
                }
            }
        }
        m.free();
    }
}

/// C31 — randomized put/get/del, binary mode, narrow and wide key pools.
#[test]
fn cfg_random_ops_binary() {
    let _g = seed_lock();
    for seed in [1u64, 2, 3, 4, 5, 6, 7, 8] {
        for pool in [4usize, 17, 400] {
            random_ops(
                "random binary",
                seed * 0x9E37,
                Shape::bin(16, 8),
                HM_BINARY,
                None,
                Keys::Bin(8),
                pool,
                600,
            );
        }
    }
}

/// C31b — same, with a 4-byte key and a tiny element.
#[test]
fn cfg_random_ops_binary_small() {
    let _g = seed_lock();
    for seed in [11u64, 12, 13, 14] {
        for pool in [3usize, 40] {
            random_ops(
                "random binary small",
                seed * 0x1234,
                Shape::bin(8, 4),
                HM_BINARY,
                None,
                Keys::Bin(4),
                pool,
                600,
            );
        }
    }
}

/// C32 — randomized ops, string mode + SH_STRDUP.
#[test]
fn cfg_random_ops_string_strdup() {
    let _g = seed_lock();
    for seed in [21u64, 22, 23, 24] {
        for pool in [4usize, 20, 200] {
            random_ops(
                "random strdup",
                seed * 0x77,
                Shape::str_ptr(16),
                HM_STRING,
                Some(SH_STRDUP),
                Keys::Str,
                pool,
                600,
            );
        }
    }
}

/// C33 — randomized ops, string mode + SH_ARENA, and + SH_DEFAULT.
#[test]
fn cfg_random_ops_string_arena() {
    let _g = seed_lock();
    for seed in [31u64, 32, 33, 34] {
        for pool in [4usize, 20, 200] {
            random_ops(
                "random arena",
                seed * 0x55,
                Shape::str_ptr(24),
                HM_STRING,
                Some(SH_ARENA),
                Keys::Str,
                pool,
                600,
            );
            random_ops(
                "random sh_default",
                seed * 0x33,
                Shape::str_ptr(16),
                HM_STRING,
                None,
                Keys::Str,
                pool,
                600,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C34 / C45 — large maps and teardown
// ---------------------------------------------------------------------------

/// C34 — 1000-key life cycle for every string mode plus binary mode.
#[test]
fn cfg_large_map_lifecycle() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x34_0000);
    // binary
    {
        let shape = Shape::bin(16, 8);
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = "large binary".into();
        let keys = distinct_bin_keys(&mut rng, 8, 1000);
        unsafe {
            for (i, k) in keys.iter().enumerate() {
                let v = rng.bytes(8);
                let idx = m.c.put(k, HM_BINARY, &v);
                let idx2 = m.r.put(k, HM_BINARY, &v);
                assert_eq!(idx, idx2, "index at {i}");
            }
            m.check("after 1000 binary inserts");
            for (i, k) in keys.iter().enumerate() {
                assert_eq!(m.c.get(k, HM_BINARY), i as isize);
                assert_eq!(m.r.get(k, HM_BINARY), i as isize);
            }
            m.check("after 1000 binary gets");
            for k in keys.iter() {
                assert_eq!(m.c.del(k, HM_BINARY), 1);
                assert_eq!(m.r.del(k, HM_BINARY), 1);
            }
            m.check("after 1000 binary deletes");
            m.free();
        }
    }
    // string modes
    for sh in [None, Some(SH_STRDUP), Some(SH_ARENA)] {
        let shape = Shape::str_ptr(16);
        seed_both(0x3141_5926);
        let mut m = match sh {
            Some(sm) => MapPair::shmode(shape, sm),
            None => MapPair::empty(shape),
        };
        m.ctx = format!("large string {sh:?}");
        let keys = distinct_str_keys(&mut rng, 1000, 0, 60);
        unsafe {
            for k in keys.iter() {
                let v = rng.bytes(8);
                let a = m.c.put(k, HM_STRING, &v);
                let b = m.r.put(k, HM_STRING, &v);
                assert_eq!(a, b);
            }
            m.check("after 1000 string inserts");
            for (i, k) in keys.iter().enumerate() {
                assert_eq!(m.c.get(k, HM_STRING), i as isize);
                assert_eq!(m.r.get(k, HM_STRING), i as isize);
            }
            m.check("after 1000 string gets");
            for k in keys.iter().step_by(2) {
                assert_eq!(m.c.del(k, HM_STRING), 1);
                assert_eq!(m.r.del(k, HM_STRING), 1);
            }
            m.check("after 500 string deletes");
            m.free();
        }
    }
}

/// C45 — `stbds_hmfree_func` on every map shape (incl. `hash_table == NULL`).
#[test]
fn cfg_hmfree_all_shapes() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x45_0000);
    let l = libs();
    let shape = Shape::str_ptr(16);

    // (a) NULL
    unsafe {
        (l.c.hmfree_func)(std::ptr::null_mut(), shape.elemsize);
        (l.r.hmfree_func)(std::ptr::null_mut(), shape.elemsize);
    }

    // (b) map with no hash index (created by a get)
    {
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = "free map without index".into();
        unsafe {
            let k = rng.cstring(5);
            m.get(&k, HM_STRING, "create by get");
            assert!(m.c.table().is_null());
            m.free();
        }
    }

    // (c) every string mode, with and without deletes
    for sh in [None, Some(SH_NONE), Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        for del_some in [false, true] {
            seed_both(0x3141_5926);
            let use_bytes = sh == Some(SH_NONE);
            let shape = if use_bytes {
                Shape::bin(16, 8)
            } else {
                Shape::str_ptr(16)
            };
            let mut m = match sh {
                Some(sm) => MapPair::shmode(shape, sm),
                None => MapPair::empty(shape),
            };
            m.ctx = format!("free shmode={sh:?} del={del_some}");
            unsafe {
                let keys = distinct_str_keys(&mut rng, 20, 9, 30);
                for (i, k) in keys.iter().enumerate() {
                    let v = rng.bytes(8);
                    m.put(k, HM_STRING, &v, &format!("key#{i}"));
                }
                if del_some && !use_bytes {
                    for k in keys.iter().step_by(3) {
                        m.del(k, HM_STRING, "del");
                    }
                }
                m.free();
            }
        }
    }
}
