//! Phase B — CONFIGS.md rows 14..33 and 52: the binary-key hash map driven
//! through the *low-level* exported entry points
//! (`stbds_hmput_default`, `stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmdel_key`, `stbds_hmfree_func`), exactly the
//! way the `stbds_hmput`/`hmget`/`hmdel` macros do.

mod common;

use common::*;
use std::collections::{HashMap, HashSet};
use std::ffi::{c_int, c_void};

/// Deterministic distinct key bytes for a logical key id.
fn key_of(v: u64, keysize: usize) -> Vec<u8> {
    let mut out = vec![0u8; keysize];
    let b = v.to_le_bytes();
    let n = keysize.min(8);
    out[..n].copy_from_slice(&b[..n]);
    for i in 8..keysize {
        out[i] = (v >> ((i % 8) * 8)) as u8 ^ (i as u8).wrapping_mul(37);
    }
    out
}

fn value_of(v: u64, n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n);
    let mut s = v ^ 0xA5A5_5A5A_1234_9876;
    while out.len() < n {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        out.extend_from_slice(&s.to_le_bytes());
    }
    out.truncate(n);
    out
}

/// The (elemsize, keysize) shapes the C treats differently.
const SHAPES: [(usize, usize); 5] = [
    (8, 4),   // int -> int
    (16, 8),  // int64 -> int64 (keysize == sizeof(size_t))
    (24, 16), // 16-byte key: two full siphash blocks
    (16, 12), // 12-byte key: one block + a 4-byte tail
    (8, 8),   // key fills the whole element (no value bytes)
];

fn value_len(elemsize: usize, keysize: usize) -> usize {
    elemsize.saturating_sub(keysize)
}

// ---------------------------------------------------------------------------
// Row 14/15/16 — stbds_hmput_default
// ---------------------------------------------------------------------------

/// Row 14 — `stbds_hmput_default(NULL, elemsize)`.
#[test]
fn cfg_14_hmput_default_null() {
    let p = Pair::new();
    for &elemsize in &[1usize, 4, 8, 16, 24, 40] {
        p.seed(0x1234_5678);
        let (hc, hr) = unsafe {
            (
                (p.c.hmput_default)(std::ptr::null_mut(), elemsize),
                (p.r.hmput_default)(std::ptr::null_mut(), elemsize),
            )
        };
        let (sc, sr) = unsafe {
            (
                snap_map(hc, elemsize, KeyKind::Binary, false),
                snap_map(hr, elemsize, KeyKind::Binary, false),
            )
        };
        eq_snap(&format!("hmput_default(NULL,{elemsize})"), &sc, &sr);
        assert_eq!(sc.length, 1);
        assert_eq!(sc.capacity, 4);
        assert!(!sc.has_table);
        unsafe {
            (p.c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// Row 15 — a second `stbds_hmput_default` is a no-op (`length == 1`).
#[test]
fn cfg_15_hmput_default_twice() {
    let p = Pair::new();
    for &elemsize in &[4usize, 8, 16, 24] {
        let (mut hc, mut hr) = unsafe {
            (
                (p.c.hmput_default)(std::ptr::null_mut(), elemsize),
                (p.r.hmput_default)(std::ptr::null_mut(), elemsize),
            )
        };
        // write a default value like `hmdefault(t, v)` does: t[-1].value = v
        let val = value_of(7, elemsize);
        unsafe {
            std::ptr::copy_nonoverlapping(
                val.as_ptr(),
                (hc as *mut u8).sub(elemsize),
                elemsize,
            );
            std::ptr::copy_nonoverlapping(
                val.as_ptr(),
                (hr as *mut u8).sub(elemsize),
                elemsize,
            );
        }
        for _ in 0..3 {
            unsafe {
                hc = (p.c.hmput_default)(hc, elemsize);
                hr = (p.r.hmput_default)(hr, elemsize);
            }
            let (sc, sr) = unsafe {
                (
                    snap_map(hc, elemsize, KeyKind::Binary, false),
                    snap_map(hr, elemsize, KeyKind::Binary, false),
                )
            };
            eq_snap("hmput_default repeated", &sc, &sr);
            assert_eq!(sc.length, 1, "must stay at length 1");
            assert_eq!(sc.elems[0], val, "default element must be preserved");
        }
        unsafe {
            (p.c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// Row 16 — a default-only map has `hash_table == NULL`; a lookup must report
/// `temp == -1` without allocating a table.
#[test]
fn cfg_16_default_only_map_lookup() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_16);
    for &(elemsize, keysize) in SHAPES.iter() {
        let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
        unsafe {
            m.hc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            m.hr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
        }
        for _ in 0..32 {
            let mut k = key_of(rng.next_u64(), keysize);
            assert_eq!(m.get(&p, &mut k), -1, "lookup in a table-less map");
            assert_eq!(m.get_ts(&p, &mut k), -1, "_ts lookup in a table-less map");
            assert_eq!(m.del(&p, &mut k, 0), 0, "delete in a table-less map");
            m.check("table-less map");
        }
        m.free(&p);
    }
}

// ---------------------------------------------------------------------------
// Rows 17..25 — put / get across sizes and counts
// ---------------------------------------------------------------------------

fn fill_and_check(
    p: &Pair,
    elemsize: usize,
    keysize: usize,
    mode: c_int,
    counts: &[usize],
    rng: &mut Rng,
    global_seed: usize,
) {
    let vlen = value_len(elemsize, keysize);
    for &n in counts {
        p.seed(global_seed);
        let mut m = MapPair::null(elemsize, keysize, mode, KeyKind::Binary);
        let mut ids: Vec<u64> = Vec::new();
        let mut seen: HashSet<Vec<u8>> = HashSet::new();
        while ids.len() < n {
            let v = rng.next_u64();
            let k = key_of(v, keysize);
            if seen.insert(k) {
                ids.push(v);
            }
        }
        for (i, &v) in ids.iter().enumerate() {
            let mut k = key_of(v, keysize);
            let val = value_of(v, vlen);
            let t = m.put(p, &mut k, &val);
            assert_eq!(t, i as isize, "temp should be the new element index");
            m.check(&format!("put #{i} (n={n}, elemsize={elemsize}, keysize={keysize})"));
        }
        // every key must be found, at the right index, with the right value
        for (i, &v) in ids.iter().enumerate() {
            let mut k = key_of(v, keysize);
            let t = m.get(p, &mut k);
            assert_eq!(t, i as isize, "get index (n={n})");
            m.check("get hit");
            let t2 = m.get_ts(p, &mut k);
            assert_eq!(t2, i as isize, "get_ts index (n={n})");
            m.check("get_ts hit");
        }
        // and a batch of misses
        for _ in 0..16 {
            let mut k = key_of(rng.next_u64() | 0x8000_0000_0000_0000, keysize);
            if seen.contains(&k) {
                continue;
            }
            assert_eq!(m.get(p, &mut k), -1, "miss must be -1");
            m.check("get miss");
            assert_eq!(m.get_ts(p, &mut k), -1, "miss must be -1 (_ts)");
            m.check("get_ts miss");
        }
        m.free(p);
    }
}

/// Row 17 — `int -> int`, counts around every growth threshold.
#[test]
fn cfg_17_put_int_int() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_17);
    fill_and_check(
        &p,
        8,
        4,
        STBDS_HM_BINARY,
        &[0, 1, 2, 5, 6, 7, 11, 12, 13, 23, 24, 25, 47, 48, 49, 95, 96, 97, 200],
        &mut rng,
        0x3141_5926,
    );
}

/// Row 18 — `int64 -> int64` (keysize == `sizeof(size_t)`), up to 300 keys.
#[test]
fn cfg_18_put_i64_i64() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_18);
    fill_and_check(
        &p,
        16,
        8,
        STBDS_HM_BINARY,
        &[0, 1, 6, 7, 12, 13, 24, 25, 100, 300],
        &mut rng,
        0x9E37_79B9,
    );
}

/// Row 19 — 16-byte key (two siphash blocks).
#[test]
fn cfg_19_put_key16() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_19);
    fill_and_check(
        &p,
        24,
        16,
        STBDS_HM_BINARY,
        &[0, 1, 6, 7, 12, 13, 24, 25, 200],
        &mut rng,
        1,
    );
}

/// Row 20 — 12-byte key (one block + a 4-byte tail).
#[test]
fn cfg_20_put_key12() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_20);
    fill_and_check(
        &p,
        16,
        12,
        STBDS_HM_BINARY,
        &[0, 1, 6, 7, 13, 25, 150],
        &mut rng,
        usize::MAX,
    );
    // key fills the element exactly (no value bytes)
    fill_and_check(&p, 8, 8, STBDS_HM_BINARY, &[1, 7, 13, 60], &mut rng, 0);
}

/// Row 21 — out-of-range negative `mode` must behave exactly like `mode == 0`.
#[test]
fn cfg_21_mode_negative_equals_binary() {
    let p = Pair::new();
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        for &mode in &[0 as c_int, -1, -2, c_int::MIN] {
            let mut rng = Rng::new(0xC0FFEE_21);
            p.seed(0xDEAD_BEEF);
            let mut m = MapPair::null(elemsize, keysize, mode, KeyKind::Binary);
            let mut snaps = Vec::new();
            for i in 0..40u64 {
                let mut k = key_of(i.wrapping_mul(0x9E37_79B9_7F4A_7C15), keysize);
                let val = value_of(i, vlen);
                m.put(&p, &mut k, &val);
                m.check(&format!("mode={mode} put #{i}"));
                snaps.push(m.snaps().0);
                let _ = rng.next_u64();
            }
            // compare the whole trace against the mode == 0 trace
            if mode == 0 {
                BASELINE.with(|b| *b.borrow_mut() = Some(snaps.clone()));
            } else {
                BASELINE.with(|b| {
                    let base = b.borrow();
                    let base = base.as_ref().expect("mode 0 must run first");
                    assert_eq!(
                        base.len(),
                        snaps.len(),
                        "trace length differs for mode={mode}"
                    );
                    for (i, (x, y)) in base.iter().zip(snaps.iter()).enumerate() {
                        assert_eq!(x, y, "mode={mode} step {i} differs from mode=0");
                    }
                });
            }
            m.free(&p);
        }
    }
}

thread_local! {
    static BASELINE: std::cell::RefCell<Option<Vec<Snap>>> = const { std::cell::RefCell::new(None) };
}

/// Row 22 — repeated puts of already-present keys (the found-early-return path).
#[test]
fn cfg_22_put_existing_keys() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_22);
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        p.seed(0x5555_AAAA);
        let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
        let ids: Vec<u64> = (0..30).map(|i| i as u64 * 7 + 1).collect();
        for &v in &ids {
            let mut k = key_of(v, keysize);
            m.put(&p, &mut k, &value_of(v, vlen));
        }
        m.check("initial fill");
        // now re-put every key many times with new values; length must not grow
        let before = m.snaps().0;
        for round in 1..=5u64 {
            for (i, &v) in ids.iter().enumerate() {
                let mut k = key_of(v, keysize);
                let t = m.put(&p, &mut k, &value_of(v ^ round, vlen));
                assert_eq!(t, i as isize, "re-put must reuse the element index");
                m.check(&format!("re-put round {round} key {i}"));
            }
            let now = m.snaps().0;
            assert_eq!(now.length, before.length, "re-put must not grow the map");
            assert_eq!(now.used_count, before.used_count);
        }
        // interleave new keys with re-puts
        for j in 0..40u64 {
            let mut k = key_of(if j % 2 == 0 { ids[(j / 2) as usize % ids.len()] } else { 1000 + j }, keysize);
            m.put(&p, &mut k, &value_of(j, vlen));
            m.check(&format!("interleaved put {j}"));
            let _ = rng.next_u64();
        }
        m.free(&p);
    }
}

/// Find `n` distinct keys whose probe position lands in the same bucket.
fn colliding_keys(
    p: &Pair,
    seed: usize,
    slot_count: usize,
    bucket: usize,
    keysize: usize,
    n: usize,
    rng: &mut Rng,
) -> Vec<u64> {
    let mut out = Vec::new();
    let mut guard = 0;
    while out.len() < n {
        guard += 1;
        assert!(guard < 2_000_000, "could not find enough colliding keys");
        let v = rng.next_u64();
        let mut k = key_of(v, keysize);
        let mut h = unsafe { (p.c.hash_bytes)(k.as_mut_ptr() as *mut c_void, keysize, seed) };
        if h < 2 {
            h += 2;
        }
        let pos = h & (slot_count - 1);
        if pos >> BUCKET_SHIFT == bucket && !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Find `n` distinct keys whose probe position is *exactly* `pos`.
fn keys_with_exact_pos(
    p: &Pair,
    seed: usize,
    slot_count: usize,
    pos: usize,
    keysize: usize,
    n: usize,
    rng: &mut Rng,
) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new();
    let mut guard = 0u64;
    while out.len() < n {
        guard += 1;
        assert!(guard < 5_000_000, "no key found with pos={pos}");
        let v = rng.next_u64();
        let mut k = key_of(v, keysize);
        let mut h = unsafe { (p.c.hash_bytes)(k.as_mut_ptr() as *mut c_void, keysize, seed) };
        if h < 2 {
            h += 2;
        }
        if h & (slot_count - 1) == pos && !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Row 23 — keys forced into the same bucket, so the forward scan, the
/// wrap-around scan and the `pos += step` bucket walk are all exercised.
#[test]
fn cfg_23_bucket_collisions() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_23);
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        for &gseed in &[0usize, 1, 0x3141_5926, usize::MAX] {
            p.seed(gseed);
            // bootstrap so the table (and therefore its seed) exists
            let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
            let mut k0 = key_of(0xFFFF_0000_1111_2222, keysize);
            m.put(&p, &mut k0, &value_of(0, vlen));
            let s = m.snaps().0;
            assert!(s.has_table);
            let tseed = s.seed;

            // 30 keys all probing into bucket 0 of an 8-slot table: the first 8
            // fill the bucket (forward + wrap scans), the rest walk on.
            let ids = colliding_keys(&p, tseed, s.slot_count, 0, keysize, 30, &mut rng);
            for (i, &v) in ids.iter().enumerate() {
                let mut k = key_of(v, keysize);
                m.put(&p, &mut k, &value_of(v, vlen));
                m.check(&format!("collision put #{i} gseed={gseed:#x}"));
            }
            for &v in ids.iter() {
                let mut k = key_of(v, keysize);
                assert!(m.get(&p, &mut k) >= 0, "collided key must be found");
                m.check("collision get");
                assert!(m.get_ts(&p, &mut k) >= 0);
                m.check("collision get_ts");
            }
            // misses that probe into the same crowded bucket
            let miss = colliding_keys(&p, tseed, s.slot_count, 0, keysize, 10, &mut rng);
            for &v in miss.iter() {
                if ids.contains(&v) {
                    continue;
                }
                let mut k = key_of(v, keysize);
                let t = m.get(&p, &mut k);
                m.check("collision miss get");
                let _ = t;
            }
            m.free(&p);
        }
    }
}

/// Rows 24/25 — hits and misses via `stbds_hmget_key` (temp in the header) and
/// `stbds_hmget_key_ts` (temp via the out-parameter); the `_ts` form must NOT
/// touch the header's `temp`.
#[test]
fn cfg_24_25_get_and_get_ts() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_24);
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        p.seed(0x0BAD_F00D);
        let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
        let ids: Vec<u64> = (0..50).map(|_| rng.next_u64()).collect();
        for &v in &ids {
            let mut k = key_of(v, keysize);
            m.put(&p, &mut k, &value_of(v, vlen));
        }
        // hmget_key writes temp into the header
        for &v in &ids {
            let mut k = key_of(v, keysize);
            let t = m.get(&p, &mut k);
            let (sc, sr) = m.snaps();
            eq_snap("get header temp", &sc, &sr);
            assert_eq!(sc.temp, t, "hmget_key must store temp in the header");
        }
        // hmget_key_ts must leave the header temp alone
        for &v in &ids {
            let mut k = key_of(v, keysize);
            let before = m.snaps().0.temp;
            let t = m.get_ts(&p, &mut k);
            let (sc, sr) = m.snaps();
            eq_snap("get_ts header temp", &sc, &sr);
            assert_eq!(sc.temp, before, "hmget_key_ts must not touch header temp");
            assert!(t >= 0);
        }
        m.free(&p);
    }
}

// ---------------------------------------------------------------------------
// Rows 26..32 — deletion
// ---------------------------------------------------------------------------

/// Row 26 — delete the last element (`old_index == final_index`, no memmove).
#[test]
fn cfg_26_del_last() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_26);
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        for &n in &[1usize, 2, 6, 7, 13, 40] {
            p.seed(0x1111_2222);
            let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
            let ids: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
            for &v in &ids {
                let mut k = key_of(v, keysize);
                m.put(&p, &mut k, &value_of(v, vlen));
            }
            for (i, &v) in ids.iter().enumerate().rev() {
                let mut k = key_of(v, keysize);
                let t = m.del(&p, &mut k, 0);
                assert_eq!(t, 1, "delete-last must report 1");
                m.check(&format!("del last #{i} (n={n})"));
            }
            let (sc, _) = m.snaps();
            assert_eq!(sc.length, 1, "only the default element remains");
            m.free(&p);
        }
    }
}

/// Row 27 — delete a middle element (memmove + re-index of the moved element).
#[test]
fn cfg_27_del_middle() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_27);
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        for &n in &[2usize, 3, 7, 13, 40] {
            p.seed(0x3333_4444);
            let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
            let ids: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
            for &v in &ids {
                let mut k = key_of(v, keysize);
                m.put(&p, &mut k, &value_of(v, vlen));
            }
            // always delete element 0 (never the last while n > 1)
            let mut live: Vec<u64> = ids.clone();
            while live.len() > 1 {
                let v = live[0];
                let mut k = key_of(v, keysize);
                assert_eq!(m.del(&p, &mut k, 0), 1);
                m.check(&format!("del middle (n={n}, {} live)", live.len()));
                // the moved element must still be findable
                live.swap_remove(0);
                for &w in &live {
                    let mut kk = key_of(w, keysize);
                    assert!(m.get(&p, &mut kk) >= 0, "survivor must remain findable");
                    m.check("survivor get");
                }
                // and the deleted one must not
                assert_eq!(m.get(&p, &mut k), -1, "deleted key must be gone");
                m.check("deleted get");
            }
            m.free(&p);
        }
    }
}

/// Row 28 — delete every element in insertion / reverse / random order.
#[test]
fn cfg_28_del_all_orders() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_28);
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        for &n in &[1usize, 7, 13, 30, 80] {
            for order in 0..3 {
                p.seed(0x5555_6666);
                let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
                let ids: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
                for &v in &ids {
                    let mut k = key_of(v, keysize);
                    m.put(&p, &mut k, &value_of(v, vlen));
                }
                let mut order_ids = ids.clone();
                match order {
                    0 => {}
                    1 => order_ids.reverse(),
                    _ => {
                        for i in (1..order_ids.len()).rev() {
                            let j = rng.below(i + 1);
                            order_ids.swap(i, j);
                        }
                    }
                }
                for (step, &v) in order_ids.iter().enumerate() {
                    let mut k = key_of(v, keysize);
                    assert_eq!(m.del(&p, &mut k, 0), 1, "delete must succeed");
                    m.check(&format!("del order={order} step={step} n={n}"));
                    // deleting again must be a no-op
                    assert_eq!(m.del(&p, &mut k, 0), 0, "double delete must be a no-op");
                    m.check("double delete");
                }
                let (sc, _) = m.snaps();
                assert_eq!(sc.length, 1);
                assert_eq!(sc.used_count, 0);
                m.free(&p);
            }
        }
    }
}

/// Row 29 — `slot_count == 8` ⇒ `used_count_shrink_threshold == 0` ⇒ never
/// shrinks, no matter how much is deleted.
#[test]
fn cfg_29_no_shrink_at_8_slots() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_29);
    for &(elemsize, keysize) in SHAPES.iter() {
        p.seed(0x7777_8888);
        let vlen = value_len(elemsize, keysize);
        let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
        let ids: Vec<u64> = (0..5).map(|_| rng.next_u64()).collect();
        for &v in &ids {
            let mut k = key_of(v, keysize);
            m.put(&p, &mut k, &value_of(v, vlen));
        }
        let s = m.snaps().0;
        assert_eq!(s.slot_count, 8);
        assert_eq!(s.used_count_shrink_threshold, 0);
        for &v in &ids {
            let mut k = key_of(v, keysize);
            m.del(&p, &mut k, 0);
            m.check("del at 8 slots");
            let s = m.snaps().0;
            assert_eq!(s.slot_count, 8, "must never shrink below 8 slots");
        }
        m.free(&p);
    }
}

/// Row 30 — grow past 8 slots then delete until the shrink threshold trips.
#[test]
fn cfg_30_shrink() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_30);
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        for &n in &[10usize, 20, 40, 90] {
            p.seed(0x9999_AAAA);
            let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
            let ids: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
            for &v in &ids {
                let mut k = key_of(v, keysize);
                m.put(&p, &mut k, &value_of(v, vlen));
            }
            let big = m.snaps().0.slot_count;
            assert!(big > 8, "expected a grown table, got {big}");
            let mut saw_shrink = false;
            let mut prev = big;
            for &v in &ids {
                let mut k = key_of(v, keysize);
                m.del(&p, &mut k, 0);
                m.check(&format!("shrink-path del (n={n})"));
                let now = m.snaps().0.slot_count;
                if now < prev {
                    saw_shrink = true;
                }
                prev = now;
            }
            assert!(saw_shrink, "expected at least one shrink for n={n}");
            m.free(&p);
        }
    }
}

/// Row 31 — delete/re-put churn until `tombstone_count > tombstone_count_threshold`
/// forces a rebuild at the same `slot_count`.
#[test]
fn cfg_31_rebuild_on_tombstones() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_31);

    // ---- (a) deterministic, guaranteed rebuild -------------------------------
    // At slot_count == 8: tombstone_count_threshold == (8>>3)+(8>>4) == 1 and
    // used_count_shrink_threshold == 0 (so a shrink can never pre-empt it).
    // Two deletes therefore push tombstone_count to 2 > 1 and force a rebuild
    // at the SAME slot_count, which resets tombstone_count to 0.
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        p.seed(0xBBBB_CCCC);
        let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
        let ids: Vec<u64> = (1..=3u64).map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15)).collect();
        for &v in &ids {
            let mut k = key_of(v, keysize);
            m.put(&p, &mut k, &value_of(v, vlen));
        }
        let s0 = m.snaps().0;
        assert_eq!(s0.slot_count, 8);
        assert_eq!(s0.tombstone_count_threshold, 1);
        assert_eq!(s0.used_count_shrink_threshold, 0);

        let mut k0 = key_of(ids[0], keysize);
        assert_eq!(m.del(&p, &mut k0, 0), 1);
        m.check("deterministic rebuild: delete 1");
        let s1 = m.snaps().0;
        assert_eq!(s1.tombstone_count, 1, "first delete leaves one tombstone");
        assert_eq!(s1.slot_count, 8);

        let mut k1 = key_of(ids[1], keysize);
        assert_eq!(m.del(&p, &mut k1, 0), 1);
        m.check("deterministic rebuild: delete 2");
        let s2 = m.snaps().0;
        assert_eq!(
            s2.slot_count, 8,
            "a rebuild must keep slot_count (no shrink at 8 slots)"
        );
        assert_eq!(
            s2.tombstone_count, 0,
            "tombstone_count > threshold must trigger a same-slot_count rebuild"
        );
        assert_eq!(s2.seed, s0.seed, "the rebuilt table must inherit the seed");
        // the survivor is still reachable through the rebuilt index
        let mut k2 = key_of(ids[2], keysize);
        assert!(m.get(&p, &mut k2) >= 0, "survivor after rebuild");
        m.check("deterministic rebuild: survivor get");
        m.free(&p);
    }

    // ---- (b) randomized churn (divergence hunting) ---------------------------
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        p.seed(0xBBBB_CCCC);
        let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
        let mut live: Vec<u64> = Vec::new();
        let mut saw_rebuild = false;
        for step in 0..400u64 {
            let s_before = m.snaps().0;
            if live.len() < 5 || rng.next_u64() % 3 != 0 {
                let v = rng.next_u64();
                let mut k = key_of(v, keysize);
                m.put(&p, &mut k, &value_of(v, vlen));
                live.push(v);
            } else {
                let i = rng.below(live.len());
                let v = live.swap_remove(i);
                let mut k = key_of(v, keysize);
                m.del(&p, &mut k, 0);
            }
            m.check(&format!("churn step {step}"));
            let s_after = m.snaps().0;
            if s_after.slot_count == s_before.slot_count
                && s_before.tombstone_count > 0
                && s_after.tombstone_count == 0
            {
                saw_rebuild = true;
            }
        }
        // (a) above already guarantees the rebuild path is covered; this is only
        // extra information about the random stream.
        let _ = saw_rebuild;
        m.free(&p);
    }
}

/// Row 32 — a put that reuses a tombstone slot (`--tombstone_count`).
#[test]
fn cfg_32_tombstone_reuse() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_32);

    // ---- (a) deterministic, guaranteed tombstone reuse -----------------------
    // Three keys that all probe to slot 0 of an 8-slot table land in slots 0, 1
    // and 2 (forward scan). Deleting the slot-0 key leaves a DELETED marker
    // there (tombstone_count == 1, which is NOT > the threshold 1, so no
    // rebuild wipes it). Inserting a fourth slot-0 key then walks 0 (tombstone,
    // recorded), 1 and 2 (occupied) and finds the empty slot 3 — at which point
    // `if (tombstone >= 0) { pos = tombstone; --tombstone_count; }` reuses
    // slot 0 instead.
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        p.seed(0x2468_1357);
        let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
        // bootstrap so the table (hence its seed) exists, using a key we then
        // delete again so it leaves no trace beyond a cleared tombstone
        let mut boot = key_of(0xFEED_FACE_CAFE_BEEF, keysize);
        m.put(&p, &mut boot, &value_of(0, vlen));
        let seed = m.snaps().0.seed;
        let ids = keys_with_exact_pos(&p, seed, 8, 0, keysize, 4, &mut rng);
        // clear the bootstrap key, then rebuild from a clean slate
        m.free(&p);

        p.seed(0x2468_1357);
        let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
        for &v in ids.iter().take(3) {
            let mut k = key_of(v, keysize);
            m.put(&p, &mut k, &value_of(v, vlen));
        }
        let s0 = m.snaps().0;
        assert_eq!(s0.seed, seed, "same global seed => same table seed");
        assert_eq!(s0.used_count, 3);
        assert!(s0.buckets[0].1[0] >= 0 && s0.buckets[0].1[1] >= 0 && s0.buckets[0].1[2] >= 0,
                "slots 0,1,2 must be occupied: {:?}", s0.buckets);

        let mut k0 = key_of(ids[0], keysize);
        assert_eq!(m.del(&p, &mut k0, 0), 1);
        m.check("deterministic tombstone: delete slot 0");
        let s1 = m.snaps().0;
        assert_eq!(s1.tombstone_count, 1, "one tombstone, below the threshold");
        assert_eq!(s1.buckets[0].0[0], 1, "slot 0 hash must be STBDS_HASH_DELETED");
        assert_eq!(s1.buckets[0].1[0], -2, "slot 0 index must be STBDS_INDEX_DELETED");

        let mut k3 = key_of(ids[3], keysize);
        m.put(&p, &mut k3, &value_of(ids[3], vlen));
        m.check("deterministic tombstone: reuse");
        let s2 = m.snaps().0;
        assert_eq!(
            s2.tombstone_count, 0,
            "the put must consume the tombstone (--tombstone_count)"
        );
        assert!(
            s2.buckets[0].1[0] >= 0,
            "the new key must land in the reused slot 0: {:?}",
            s2.buckets
        );
        assert!(m.get(&p, &mut k3) >= 0);
        m.check("deterministic tombstone: get reused key");
        m.free(&p);
    }

    // ---- (b) randomized churn (divergence hunting) ---------------------------
    let mut saw_reuse = 0usize;
    for &(elemsize, keysize) in SHAPES.iter() {
        let vlen = value_len(elemsize, keysize);
        for &gseed in &[0usize, 7, 0x3141_5926] {
            p.seed(gseed);
            let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
            let mut live: Vec<u64> = Vec::new();
            for step in 0..600u64 {
                let before = m.snaps().0;
                let is_put = live.len() < 3 || rng.next_u64() % 2 == 0;
                if is_put {
                    let v = rng.next_u64();
                    let mut k = key_of(v, keysize);
                    m.put(&p, &mut k, &value_of(v, vlen));
                    live.push(v);
                } else {
                    let i = rng.below(live.len());
                    let v = live.swap_remove(i);
                    let mut k = key_of(v, keysize);
                    m.del(&p, &mut k, 0);
                }
                m.check(&format!("tombstone churn step {step}"));
                let after = m.snaps().0;
                if is_put
                    && after.slot_count == before.slot_count
                    && after.tombstone_count + 1 == before.tombstone_count
                {
                    saw_reuse += 1;
                }
            }
            m.free(&p);
        }
    }
    // (a) above already guarantees the tombstone-reuse path is covered.
    let _ = saw_reuse;
}

/// Row 33 — long randomized op-stream mixing put / get / get_ts / del over a
/// small key pool, so hits, misses, growth, shrink, rebuild and tombstone reuse
/// all interleave.
#[test]
fn cfg_33_random_op_stream() {
    let p = Pair::new();
    for &(elemsize, keysize) in &[(8usize, 4usize), (16, 8), (24, 16)] {
        let vlen = value_len(elemsize, keysize);
        for &gseed in &[0usize, 1, 0x3141_5926, usize::MAX] {
            let mut rng = Rng::new(0xC0FFEE_33 ^ gseed as u64 ^ elemsize as u64);
            p.seed(gseed);
            let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
            // small pool -> lots of hits, re-puts and repeat deletes
            let pool: Vec<u64> = (0..60).map(|_| rng.next_u64()).collect();
            let mut model: HashMap<u64, Vec<u8>> = HashMap::new();
            for step in 0..2000u64 {
                let v = pool[rng.below(pool.len())];
                let mut k = key_of(v, keysize);
                match rng.below(10) {
                    0..=4 => {
                        let val = value_of(v ^ step, vlen);
                        m.put(&p, &mut k, &val);
                        model.insert(v, val);
                    }
                    5..=6 => {
                        let t = m.get(&p, &mut k);
                        assert_eq!(
                            t >= 0,
                            model.contains_key(&v),
                            "hit/miss disagrees with the model at step {step}"
                        );
                    }
                    7 => {
                        let t = m.get_ts(&p, &mut k);
                        assert_eq!(t >= 0, model.contains_key(&v));
                    }
                    _ => {
                        let t = m.del(&p, &mut k, 0);
                        assert_eq!(
                            t, model.remove(&v).is_some() as isize,
                            "delete result disagrees with the model at step {step}"
                        );
                    }
                }
                m.check(&format!(
                    "op-stream step {step} (elemsize={elemsize} gseed={gseed:#x})"
                ));
            }
            // final consistency sweep
            for &v in &pool {
                let mut k = key_of(v, keysize);
                let t = m.get(&p, &mut k);
                assert_eq!(t >= 0, model.contains_key(&v));
                m.check("final sweep");
            }
            m.free(&p);
        }
    }
}

/// Row 52 — `keysize == 0`: valid but degenerate (all keys hash the same and
/// `memcmp(_, _, 0) == 0`), so the map holds a single entry.
#[test]
fn cfg_52_keysize_zero() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_52);
    for &elemsize in &[4usize, 8, 16] {
        p.seed(0x1357_9BDF);
        let mut m = MapPair::null(elemsize, 0, STBDS_HM_BINARY, KeyKind::Binary);
        for i in 0..40u64 {
            let mut k = key_of(rng.next_u64(), 8);
            m.put(&p, &mut k, &value_of(i, elemsize));
            m.check(&format!("keysize=0 put {i}"));
            let s = m.snaps().0;
            assert_eq!(s.length, 2, "keysize=0 collapses to one element");
        }
        for _ in 0..10 {
            let mut k = key_of(rng.next_u64(), 8);
            assert_eq!(m.get(&p, &mut k), 0, "keysize=0 always hits element 0");
            m.check("keysize=0 get");
        }
        let mut k = key_of(0, 8);
        assert_eq!(m.del(&p, &mut k, 0), 1);
        m.check("keysize=0 del");
        assert_eq!(m.get(&p, &mut k), -1);
        m.check("keysize=0 get after del");
        m.free(&p);
    }
}

/// Row 54 — a **consistent** non-zero `keyoffset`.
///
/// `stbds_hmput_key` hard-codes `keyoffset = 0`, but `stbds_hmdel_key` takes it
/// as a parameter (the header macros pass `STBDS_OFFSETOF(t, key)`). A caller
/// whose element carries a copy of the key at `keyoffset` therefore gets a fully
/// working delete, *including* the re-index probe after the memmove — which
/// reads `elem[old_index] + keyoffset`, i.e. the moved element's copy.
///
/// Element layout here (`elemsize = 16`, `keysize = 4`, `keyoffset = 8`):
/// `[ key | pad | key-copy | value ]`.
#[test]
fn cfg_54_del_consistent_keyoffset() {
    let p = Pair::new();
    let (elemsize, keysize, keyoffset) = (16usize, 4usize, 8usize);
    let mut rng = Rng::new(0xC0FFEE_54);

    for &n in &[1usize, 2, 7, 13, 40, 90] {
        for order in 0..3 {
            p.seed(0x7EA5_1234);
            let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
            let ids: Vec<u32> = {
                let mut v = Vec::new();
                let mut seen = HashSet::new();
                while v.len() < n {
                    let k = rng.next_u32();
                    if seen.insert(k) {
                        v.push(k);
                    }
                }
                v
            };
            for &k in &ids {
                let mut key = k.to_le_bytes();
                // value region = bytes 4..16: pad(4) | key-copy(4) | value(4)
                let mut val = Vec::with_capacity(12);
                val.extend_from_slice(&0xA5A5_A5A5u32.to_le_bytes()); // pad
                val.extend_from_slice(&key); // the key copy at offset 8
                val.extend_from_slice(&(!k).to_le_bytes()); // value
                m.put(&p, &mut key, &val);
                m.check("keyoffset fill");
            }

            let mut del_order = ids.clone();
            match order {
                0 => {}
                1 => del_order.reverse(),
                _ => {
                    for i in (1..del_order.len()).rev() {
                        let j = rng.below(i + 1);
                        del_order.swap(i, j);
                    }
                }
            }

            let mut live: HashSet<u32> = ids.iter().copied().collect();
            for (step, &k) in del_order.iter().enumerate() {
                let mut key = k.to_le_bytes();
                assert_eq!(
                    m.del(&p, &mut key, keyoffset),
                    1,
                    "consistent keyoffset delete must succeed (n={n} order={order} step={step})"
                );
                m.check(&format!("keyoffset del n={n} order={order} step={step}"));
                live.remove(&k);
                // the deleted key is gone and the survivors are all still there
                assert_eq!(m.get(&p, &mut key), -1, "deleted key must be gone");
                m.check("keyoffset post-del get");
                for &w in live.iter() {
                    let mut wk = w.to_le_bytes();
                    assert!(m.get(&p, &mut wk) >= 0, "survivor {w} must remain findable");
                    m.check("keyoffset survivor get");
                }
            }
            let s = m.snaps().0;
            assert_eq!(s.length, 1);
            assert_eq!(s.used_count, 0);
            m.free(&p);
        }
    }
}
