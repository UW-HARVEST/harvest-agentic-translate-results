//! Phase B — CONFIGS.md rows 23..42: the hash map driven through the LOW-LEVEL
//! entry points (`stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`,
//! `stbds_hmdel_key`, `stbds_hmput_default`, `stbds_hmfree_func`) in binary
//! key mode.
//!
//! Every operation is applied to the C map and the Rust map and the FULL
//! observable state (array header, every hash bucket's hash/index arrays, the
//! hash-index bookkeeping fields, and every element's bytes) is compared
//! byte-for-byte afterwards, by `common::Pair`.
mod common;

use common::*;
use core::ffi::{c_int, c_void};
use std::collections::BTreeSet;

/// `n` distinct random keys of `keysize` bytes, kept alive at a stable address
/// so both libraries see the identical pointer AND identical bytes.
///
/// `n_extra` further keys are returned that are distinct from the first `n`;
/// with small `keysize` (e.g. 1 byte => only 256 possible keys) the "absent"
/// set must be generated together with the live set to stay disjoint.
fn distinct_keys_split(
    rng: &mut Rng,
    keys: &mut Keys,
    n: usize,
    n_extra: usize,
    keysize: usize,
) -> (Vec<*mut c_void>, Vec<*mut c_void>) {
    if keysize == 0 {
        // every 0-byte key compares equal, so "distinct" is meaningless
        let k = keys.raw(&[]);
        return (vec![k; n.min(1)], Vec::new());
    }
    let total = n + n_extra;
    let space: u128 = 1u128 << (8 * keysize.min(8));
    assert!(
        (total as u128) < space,
        "cannot draw {total} distinct {keysize}-byte keys"
    );
    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    let mut all = Vec::new();
    let mut guard = 0;
    while all.len() < total {
        guard += 1;
        assert!(guard < 1_000_000, "key generation stalled");
        let b = rng.bytes(keysize);
        if !seen.insert(b.clone()) {
            continue;
        }
        all.push(keys.raw(&b));
    }
    let extra = all.split_off(n);
    (all, extra)
}

fn distinct_keys(rng: &mut Rng, keys: &mut Keys, n: usize, keysize: usize) -> Vec<*mut c_void> {
    distinct_keys_split(rng, keys, n, 0, keysize).0
}

/// Insert `n` distinct keys, verify every lookup, verify misses, delete in
/// `order`, verify again, free.
unsafe fn lifecycle(
    c: &Lib,
    r: &Lib,
    elemsize: usize,
    keysize: usize,
    n: usize,
    seed: usize,
    del_order: DelOrder,
    rng: &mut Rng,
) {
    (c.rand_seed)(seed);
    (r.rand_seed)(seed);
    let mut keys = Keys::new();
    let n_absent = if keysize == 0 { 0 } else { 20.min((1usize << (8 * keysize.min(4))) - n - 1) };
    let (ks, absent) = distinct_keys_split(rng, &mut keys, n, n_absent, keysize);
    let label = format!("bin/es{elemsize}/ks{keysize}/n{n}/seed{seed}/{del_order:?}");
    let mut p = Pair::new(c, r, elemsize, keysize, ElemCmp::Raw, label);

    p.set_default(0xD3FA);
    for (i, &k) in ks.iter().enumerate() {
        let idx = p.put(k, HM_BINARY, i as u64);
        assert_eq!(idx, i as isize, "insertion index");
        assert_eq!(p.len(), i as isize + 1, "hmlen after insert");
    }
    // every key must be found at its insertion index
    for (i, &k) in ks.iter().enumerate() {
        assert_eq!(p.geti(k, HM_BINARY), i as isize);
        assert_eq!(p.geti_ts(k, HM_BINARY), i as isize);
        p.value_of(i as isize);
    }
    // misses
    if keysize > 0 {
        for &k in &absent {
            assert_eq!(p.geti(k, HM_BINARY), -1, "absent key must miss");
            assert_eq!(p.geti_ts(k, HM_BINARY), -1);
        }
    }

    let mut order: Vec<usize> = (0..ks.len()).collect();
    match del_order {
        DelOrder::Forward => {}
        DelOrder::Reverse => order.reverse(),
        DelOrder::Random => {
            for i in (1..order.len()).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
        }
    }
    for &i in &order {
        assert_eq!(p.del(ks[i], 0, HM_BINARY), 1, "delete of a live key");
    }
    assert_eq!(p.len(), 0, "map empty after deleting everything");
    for &k in &ks {
        assert_eq!(p.geti(k, HM_BINARY), -1, "deleted key must miss");
    }
    p.free();
}

#[derive(Clone, Copy, Debug)]
enum DelOrder {
    Forward,
    Reverse,
    Random,
}

fn shapes() -> Vec<(usize, usize, &'static str)> {
    vec![
        (8, 4, "row23"),   // row 23
        (16, 8, "row24"),  // row 24
        (4, 1, "row25"),   // row 25
        (24, 16, "row26"), // row 26
        (8, 0, "row27"),   // row 27
        (8, 3, "row28"),   // row 28
        (17, 5, "extra"),
        (64, 32, "extra"),
    ]
}

/// Rows 23..28 — every (elemsize, keysize) shape × element counts 1..40
/// (crossing table growth at 6, 12 and 24) × 3 delete orders × random seeds.
#[test]
fn cfg23_28_binary_shapes() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x2328_2328);
    unsafe {
        for (elemsize, keysize, tag) in shapes() {
            let ns: Vec<usize> = if keysize == 0 {
                vec![1]
            } else {
                (1..=40).collect()
            };
            for n in ns {
                let order = match n % 3 {
                    0 => DelOrder::Forward,
                    1 => DelOrder::Reverse,
                    _ => DelOrder::Random,
                };
                let seed = rng.next_u64() as usize;
                lifecycle(&c, &r, elemsize, keysize, n, seed, order, &mut rng);
            }
            // repeat the boundary counts with the boundary seeds
            for n in [1usize, 5, 6, 7, 11, 12, 13, 23, 24, 25] {
                if keysize == 0 {
                    continue;
                }
                for &seed in &[0usize, 1, 0x31415926, usize::MAX] {
                    lifecycle(
                        &c,
                        &r,
                        elemsize,
                        keysize,
                        n,
                        seed,
                        DelOrder::Forward,
                        &mut rng,
                    );
                }
            }
            let _ = tag;
        }
    }
}

/// Row 27 — `keysize == 0`: every key compares equal, so every put after the
/// first takes the found-key path.
#[test]
fn cfg27_keysize_zero() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x2727_2727);
    unsafe {
        for &seed in &[0usize, 1, 0x31415926, usize::MAX] {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut keys = Keys::new();
            let k0 = keys.raw(&rng.bytes(8));
            let k1 = keys.raw(&rng.bytes(8));
            let mut p = Pair::new(&c, &r, 8, 0, ElemCmp::Raw, format!("ks0/seed{seed}"));
            p.set_default(1);
            assert_eq!(p.put(k0, HM_BINARY, 10), 0);
            assert_eq!(p.len(), 1);
            // different bytes, but keysize 0 => memcmp(...,0) == 0 => same entry
            assert_eq!(p.put(k1, HM_BINARY, 11), 0);
            assert_eq!(p.len(), 1, "keysize 0 collapses every key to one entry");
            assert_eq!(p.geti(k1, HM_BINARY), 0);
            assert_eq!(p.del(k0, 0, HM_BINARY), 1);
            assert_eq!(p.len(), 0);
            assert_eq!(p.geti(k0, HM_BINARY), -1);
            p.free();
        }
    }
}

/// Row 29 — duplicate-key puts take the found-key path and must not grow
/// `used_count` or `length`.
#[test]
fn cfg29_duplicate_puts() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x2929_2929);
    unsafe {
        for keysize in [1usize, 3, 4, 8, 16] {
            for reps in 2..=8usize {
                let seed = rng.next_u64() as usize;
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let elemsize = keysize + 8;
                let mut keys = Keys::new();
                let ks = distinct_keys(&mut rng, &mut keys, 5, keysize);
                let mut p = Pair::new(
                    &c,
                    &r,
                    elemsize,
                    keysize,
                    ElemCmp::Raw,
                    format!("dup/ks{keysize}/r{reps}"),
                );
                p.set_default(7);
                for (i, &k) in ks.iter().enumerate() {
                    assert_eq!(p.put(k, HM_BINARY, i as u64), i as isize);
                }
                let before_len = p.len();
                let before = p.c.snap(ElemCmp::Raw).used_count;
                for round in 0..reps {
                    for (i, &k) in ks.iter().enumerate() {
                        assert_eq!(
                            p.put(k, HM_BINARY, (100 + round) as u64),
                            i as isize,
                            "duplicate put must reuse the slot"
                        );
                    }
                }
                assert_eq!(p.len(), before_len, "length must not grow on duplicate put");
                assert_eq!(p.c.snap(ElemCmp::Raw).used_count, before);
                p.free();
            }
        }
    }
}

/// Rows 30, 31 — delete the tail entry (`old_index == final_index`, no memmove)
/// vs. delete an interior entry (memmove + slot re-find + index fixup).
#[test]
fn cfg30_31_tail_vs_interior_delete() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x3031_3031);
    unsafe {
        for n in 1..=24usize {
            for victim in 0..n {
                let seed = rng.next_u64() as usize;
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let mut keys = Keys::new();
                let ks = distinct_keys(&mut rng, &mut keys, n, 8);
                let mut p = Pair::new(
                    &c,
                    &r,
                    16,
                    8,
                    ElemCmp::Raw,
                    format!("del/n{n}/v{victim}"),
                );
                p.set_default(3);
                for (i, &k) in ks.iter().enumerate() {
                    p.put(k, HM_BINARY, i as u64);
                }
                assert_eq!(p.del(ks[victim], 0, HM_BINARY), 1);
                assert_eq!(p.len(), n as isize - 1);
                assert_eq!(p.geti(ks[victim], HM_BINARY), -1);
                // all the survivors must still be findable, at valid indices
                for (i, &k) in ks.iter().enumerate() {
                    if i == victim {
                        continue;
                    }
                    let idx = p.geti(k, HM_BINARY);
                    assert!(
                        idx >= 0 && idx < n as isize - 1,
                        "survivor {i} lost (idx={idx})"
                    );
                    p.value_of(idx);
                }
                p.free();
            }
        }
    }
}

/// Row 32 — delete every key, in forward / reverse / random order, then verify
/// the map really is empty. (Also covered inside `lifecycle`.)
#[test]
fn cfg32_delete_all_orders() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x3232_3232);
    unsafe {
        for n in 1..=40usize {
            for order in [DelOrder::Forward, DelOrder::Reverse, DelOrder::Random] {
                let seed = rng.next_u64() as usize;
                lifecycle(&c, &r, 16, 8, n, seed, order, &mut rng);
            }
        }
    }
}

/// Row 33 — a tombstone left by a delete must be reused by a later insert
/// (`tombstone >= 0` at `found_empty_slot`, which also decrements
/// `tombstone_count`).
#[test]
fn cfg33_tombstone_reuse() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x3333_3333);
    unsafe {
        let mut saw_reuse = false;
        for trial in 0..64u64 {
            let seed = rng.next_u64() as usize;
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut keys = Keys::new();
            let ks = distinct_keys(&mut rng, &mut keys, 20, 8);
            let mut p = Pair::new(&c, &r, 16, 8, ElemCmp::Raw, format!("tomb/{trial}"));
            p.set_default(1);
            for (i, &k) in ks.iter().enumerate() {
                p.put(k, HM_BINARY, i as u64);
            }
            // delete a handful, then insert fresh keys into the tombstones
            for i in [1usize, 4, 7, 11, 15] {
                p.del(ks[i], 0, HM_BINARY);
            }
            let tombs = p.c.snap(ElemCmp::Raw).tomb_count;
            let fresh = distinct_keys(&mut rng, &mut keys, 8, 8);
            for (i, &k) in fresh.iter().enumerate() {
                p.put(k, HM_BINARY, 900 + i as u64);
            }
            let after = p.c.snap(ElemCmp::Raw).tomb_count;
            if tombs > 0 && after < tombs {
                saw_reuse = true;
            }
            for &k in &fresh {
                assert!(p.geti(k, HM_BINARY) >= 0);
            }
            p.free();
        }
        assert!(saw_reuse, "no insert ever reused a tombstone — row 33 untested");
    }
}

/// Rows 34, 35, 36 — table shrink, same-size rebuild, and the
/// no-shrink-at-8-slots floor.
#[test]
fn cfg34_36_shrink_and_rebuild() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x3436_3436);
    unsafe {
        let mut saw_shrink = false;
        let mut saw_rebuild = false;
        for trial in 0..48u64 {
            let seed = rng.next_u64() as usize;
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let n = rng.range(30, 90);
            let mut keys = Keys::new();
            let ks = distinct_keys(&mut rng, &mut keys, n, 8);
            let mut p = Pair::new(&c, &r, 16, 8, ElemCmp::Raw, format!("shrink/{trial}"));
            p.set_default(1);
            for (i, &k) in ks.iter().enumerate() {
                p.put(k, HM_BINARY, i as u64);
            }
            let peak = p.c.snap(ElemCmp::Raw).slot_count;
            assert!(peak >= 64, "expected the table to have grown, got {peak}");
            let mut prev = p.c.snap(ElemCmp::Raw);
            for (i, &k) in ks.iter().enumerate() {
                p.del(k, 0, HM_BINARY);
                let now = p.c.snap(ElemCmp::Raw);
                if now.slot_count < prev.slot_count {
                    saw_shrink = true;
                } else if now.slot_count == prev.slot_count
                    && prev.tomb_count > 0
                    && now.tomb_count == 0
                {
                    saw_rebuild = true;
                }
                prev = now;
                // survivors must still be findable after every rehash
                for &k2 in ks.iter().skip(i + 1) {
                    assert!(p.geti(k2, HM_BINARY) >= 0, "survivor lost after rehash");
                }
            }
            let end = p.c.snap(ElemCmp::Raw);
            assert_eq!(end.slot_count, BUCKET_LENGTH, "must shrink back to 8 slots");
            assert_eq!(end.uc_shrink_threshold, 0, "row 36: no-shrink floor at 8");
            p.free();
        }
        assert!(saw_shrink, "row 34 never triggered a shrink");
        assert!(saw_rebuild, "row 35 never triggered a same-size rebuild");
    }
}

/// Rows 37, 38 — `stbds_hmput_default` bootstrap ordering and idempotence.
#[test]
fn cfg37_38_hmput_default() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x3738_3738);
    unsafe {
        for &elemsize in &[8usize, 16, 17, 64] {
            let keysize = 8.min(elemsize);
            let seed = rng.next_u64() as usize;
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut keys = Keys::new();
            let ks = distinct_keys(&mut rng, &mut keys, 10, keysize);
            let mut p = Pair::new(
                &c,
                &r,
                elemsize,
                keysize,
                ElemCmp::Raw,
                format!("default/es{elemsize}"),
            );
            // row 37: default on NULL, then a get with no table yet
            p.set_default(0x11);
            assert_eq!(p.len(), 0);
            assert_eq!(p.geti(ks[0], HM_BINARY), -1, "no table yet => -1");
            assert_eq!(p.geti_ts(ks[0], HM_BINARY), -1);
            // row 38: repeated defaults are no-ops on the map structure
            p.set_default(0x22);
            p.set_default(0x22);
            assert_eq!(p.len(), 0);
            // and after real inserts too
            for (i, &k) in ks.iter().enumerate() {
                p.put(k, HM_BINARY, i as u64);
            }
            let before = p.len();
            p.set_default(0x33);
            assert_eq!(p.len(), before, "default after inserts must be a no-op");
            for (i, &k) in ks.iter().enumerate() {
                assert_eq!(p.geti(k, HM_BINARY), i as isize);
            }
            p.free();
        }
    }
}

/// Row 39 — `stbds_hmget_key_ts` on a NULL map, a table-less map, hits and
/// misses. Note the NULL case *allocates* and returns a new map.
#[test]
fn cfg39_hmget_key_ts_states() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x3939_3939);
    unsafe {
        for n in 1..=40usize {
            let seed = rng.next_u64() as usize;
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut keys = Keys::new();
            let (ks, absent) = distinct_keys_split(&mut rng, &mut keys, n, 5, 8);
            let mut p = Pair::new(&c, &r, 16, 8, ElemCmp::Raw, format!("ts/n{n}"));
            // NULL map: allocates the default entry, temp = -1
            assert_eq!(p.geti_ts(ks[0], HM_BINARY), -1);
            assert!(!p.c.t.is_null() && !p.r.t.is_null());
            // table-less map: temp = -1 again
            assert_eq!(p.geti_ts(ks[0], HM_BINARY), -1);
            for (i, &k) in ks.iter().enumerate() {
                p.put(k, HM_BINARY, i as u64);
                assert_eq!(p.geti_ts(k, HM_BINARY), i as isize);
            }
            for &k in &absent {
                assert_eq!(p.geti_ts(k, HM_BINARY), -1);
            }
            p.free();
        }
    }
}

/// Row 40 — `stbds_hmdel_key` with `keyoffset != 0`.
///
/// `stbds_hmput_key` hard-codes `keyoffset = 0`, so the element is crafted so
/// the key bytes ALSO appear at offset 8; then `hmdel_key(keyoffset = 8)` finds
/// and deletes it. The non-matching variant (offset 8 holds something else) is
/// the ERRORS.md row G5 case and must miss identically.
#[test]
fn cfg40_keyoffset_nonzero() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4040_4040);
    unsafe {
        for n in 1..=16usize {
            for victim in 0..n {
                let seed = rng.next_u64() as usize;
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let elemsize = 16usize;
                let keysize = 8usize;
                let mut keys = Keys::new();
                let ks = distinct_keys(&mut rng, &mut keys, n, keysize);
                let mut p = Pair::new(
                    &c,
                    &r,
                    elemsize,
                    keysize,
                    ElemCmp::Raw,
                    format!("koff/n{n}/v{victim}"),
                );
                p.set_default(1);
                for (i, &k) in ks.iter().enumerate() {
                    let idx = p.put(k, HM_BINARY, i as u64);
                    // mirror the key into bytes [8..16) of the element on BOTH
                    let src = core::slice::from_raw_parts(k as *const u8, keysize);
                    for (b, &v) in src.iter().enumerate() {
                        *elem(p.c.t, elemsize, idx).add(8 + b) = v;
                        *elem(p.r.t, elemsize, idx).add(8 + b) = v;
                    }
                }
                // keyoffset = 8 now matches
                assert_eq!(
                    p.del(ks[victim], 8, HM_BINARY),
                    1,
                    "delete with keyoffset=8 must find the mirrored key"
                );
                assert_eq!(p.len(), n as isize - 1);
                p.free();
            }
        }
    }
}

/// Row 41 — `stbds_hmfree_func` on a binary map with a live table.
#[test]
fn cfg41_hmfree_binary() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4141_4141);
    unsafe {
        for n in [0usize, 1, 6, 12, 40] {
            for &elemsize in &[8usize, 16, 64] {
                let seed = rng.next_u64() as usize;
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let mut keys = Keys::new();
                let ks = distinct_keys(&mut rng, &mut keys, n, 8);
                let mut p = Pair::new(&c, &r, elemsize, 8, ElemCmp::Raw, "hmfree");
                p.set_default(1);
                for (i, &k) in ks.iter().enumerate() {
                    p.put(k, HM_BINARY, i as u64);
                }
                p.free(); // both must survive and both must be NULL afterwards
                assert!(p.c.t.is_null() && p.r.t.is_null());
            }
        }
    }
}

/// Row 42 — the fuzz row: 200 randomized op scripts over random shapes.
#[test]
fn cfg42_binary_fuzz() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4242_4242);
    unsafe {
        for script in 0..200u64 {
            let keysize = [1usize, 2, 3, 4, 5, 7, 8, 9, 16, 24][rng.below(10)];
            let elemsize = keysize + [0usize, 1, 4, 8, 16][rng.below(5)];
            let seed = rng.next_u64() as usize;
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut keys = Keys::new();
            let pool = distinct_keys(&mut rng, &mut keys, 32, keysize);
            let mut p = Pair::new(
                &c,
                &r,
                elemsize,
                keysize,
                ElemCmp::Raw,
                format!("fuzz/{script}/es{elemsize}/ks{keysize}"),
            );
            if rng.below(2) == 0 {
                p.set_default(script);
            }
            let ops = rng.range(20, 120);
            for op in 0..ops {
                let k = pool[rng.below(pool.len())];
                match rng.below(10) {
                    0..=4 => {
                        p.put(k, HM_BINARY, op as u64);
                    }
                    5..=6 => {
                        p.geti(k, HM_BINARY);
                    }
                    7 => {
                        p.geti_ts(k, HM_BINARY);
                    }
                    8 => {
                        p.del(k, 0, HM_BINARY);
                    }
                    _ => {
                        p.len();
                    }
                }
            }
            // final sweep: every pooled key, then free
            for &k in &pool {
                p.geti(k, HM_BINARY);
            }
            p.free();
        }
    }
}

/// Extra: `mode` values other than 0 that still take the BINARY path
/// (`mode < STBDS_HM_STRING`), driven through a full life-cycle.
#[test]
fn cfg_binary_negative_modes() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x9999_1111);
    unsafe {
        for mode in [0 as c_int, -1, -2, c_int::MIN] {
            let seed = rng.next_u64() as usize;
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut keys = Keys::new();
            let ks = distinct_keys(&mut rng, &mut keys, 20, 8);
            let mut p = Pair::new(&c, &r, 16, 8, ElemCmp::Raw, format!("mode{mode}"));
            p.set_default(1);
            for (i, &k) in ks.iter().enumerate() {
                assert_eq!(p.put(k, mode, i as u64), i as isize);
            }
            for (i, &k) in ks.iter().enumerate() {
                assert_eq!(p.geti(k, mode), i as isize);
            }
            for &k in &ks {
                assert_eq!(p.del(k, 0, mode), 1);
            }
            p.free();
        }
    }
}

/// Extra (deep-growth soak) — 1200 distinct keys inserted then deleted, so the
/// table doubles 8→16→32→…→2048 and shrinks all the way back. The full state is
/// compared after every operation for the first/last 200 ops of each phase and
/// every 37th op in between (comparing 2048 buckets after all 1200 ops would be
/// quadratic).
#[test]
fn cfg_binary_deep_growth_soak() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0xDEE9_50A4_1234_5678);
    unsafe {
        for &seed in &[1usize, 0x31415926] {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let elemsize = 16usize;
            let keysize = 8usize;
            let n = 1200usize;
            let mut keys = Keys::new();
            let ks = distinct_keys(&mut rng, &mut keys, n, keysize);
            let mut cm = Map::new(&c, elemsize, keysize);
            let mut rm = Map::new(&r, elemsize, keysize);
            cm.set_default(1);
            rm.set_default(1);

            let mut peak = 0usize;
            let cmp_now = |cm: &Map, rm: &Map, what: &str, i: usize| {
                let a = cm.snap(ElemCmp::Raw);
                let b = rm.snap(ElemCmp::Raw);
                if a != b {
                    panic!("[soak] {what} at op {i} diverged\n{}", diff_snaps(&a, &b));
                }
                a
            };
            for (i, &k) in ks.iter().enumerate() {
                let ta = cm.put(k, HM_BINARY, i as u64);
                let tb = rm.put(k, HM_BINARY, i as u64);
                assert_eq!(ta, tb, "put temp at op {i}");
                assert_eq!(ta, i as isize);
                if i < 200 || i >= n - 200 || i % 37 == 0 {
                    let s = cmp_now(&cm, &rm, "insert", i);
                    peak = peak.max(s.slot_count);
                }
            }
            assert!(peak >= 2048, "expected deep growth, peak slot_count = {peak}");
            // every key must still be findable
            for (i, &k) in ks.iter().enumerate() {
                let ta = cm.geti(k, HM_BINARY);
                let tb = rm.geti(k, HM_BINARY);
                assert_eq!(ta, tb, "geti at {i}");
                assert_eq!(ta, i as isize);
            }
            // delete in a shuffled order, all the way down
            let mut order: Vec<usize> = (0..n).collect();
            for i in (1..order.len()).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            for (step, &i) in order.iter().enumerate() {
                let ta = cm.del(ks[i], 0, HM_BINARY);
                let tb = rm.del(ks[i], 0, HM_BINARY);
                assert_eq!(ta, tb, "del temp at step {step}");
                assert_eq!(ta, 1);
                if step < 200 || step >= n - 200 || step % 37 == 0 {
                    cmp_now(&cm, &rm, "delete", step);
                }
            }
            let end = cmp_now(&cm, &rm, "final", n);
            assert_eq!(end.slot_count, BUCKET_LENGTH, "must shrink back to 8 slots");
            assert_eq!(end.length, 1, "only the default entry remains");
            cm.free();
            rm.free();
        }
    }
}
