//! Phase B — valid-path differential tests for the BINARY-key hash map.
//!
//! Covers `CONFIGS.md` rows 23-36. Everything is driven through the low-level
//! exports (`stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`,
//! `stbds_hmput_default`, `stbds_hmdel_key`, `stbds_hmfree_func`,
//! `stbds_shmode_func`) exactly the way the `stbds_hmput` / `stbds_hmgeti` /
//! `stbds_hmdel` macros drive them.

mod common;

use common::*;
use std::ffi::c_void;

const SEED: u64 = 0xC0FF_EE00_1234_5678;

/// Build a `keysize`-byte key that is guaranteed distinct for distinct `i`
/// (little-endian index in the low bytes) but otherwise randomised.
fn mkkey(rng: &mut Rng, i: usize, keysize: usize) -> Vec<u8> {
    let idx = (i as u64).to_le_bytes();
    let mut k = Vec::with_capacity(keysize);
    for b in 0..keysize {
        if b < 8 {
            k.push(idx[b]);
        } else {
            k.push(rng.byte());
        }
    }
    k
}

// ===========================================================================
// row 23 - hmput_key(NULL, ...) bootstrap, binary, keysize 8, elemsize 16, N=1
// ===========================================================================
#[test]
fn cfg23_binary_bootstrap_single_insert() {
    diff("cfg23", |lib, log| unsafe {
        for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
            (lib.rand_seed)(seed);
            let es = 16usize;
            let mut t: *mut c_void = std::ptr::null_mut();
            t = hmput(lib, t, es, &[1u8, 2, 3, 4, 5, 6, 7, 8], HM_BINARY, 0xAABB_CCDD_EEFF_0011);
            log.usz("seed", seed);
            snap_map(log, t, es, KeyKind::Binary);
            let (t2, idx) = hmgeti(lib, t, es, &[1u8, 2, 3, 4, 5, 6, 7, 8], HM_BINARY);
            log.isz("found", idx);
            snap_map(log, t2, es, KeyKind::Binary);
            hmfree(lib, t2, es);
        }
    });
}

// ===========================================================================
// row 24 - binary map, keysize sweep, N random distinct keys
// ===========================================================================
#[test]
fn cfg24_binary_keysize_sweep() {
    diff("cfg24", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 24);
        for &keysize in &[1usize, 2, 3, 4, 5, 6, 7, 8, 9, 16, 32] {
            let es = keysize + 8;
            for rep in 0..6usize {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let n = rng.range(1, 40);
                let mut t: *mut c_void = std::ptr::null_mut();
                for i in 0..n {
                    let k = mkkey(&mut rng, i, keysize);
                    t = hmput(lib, t, es, &k, HM_BINARY, rng.next_u64());
                    log.usz("ks", keysize);
                    log.usz("rep", rep);
                    log.usz("i", i);
                    snap_map(log, t, es, KeyKind::Binary);
                }
                // look every key up again
                let mut rng2 = Rng::new(hs as u64 ^ 0x5555);
                for i in 0..n {
                    let k = mkkey(&mut rng2, i, keysize);
                    let (nt, idx) = hmgeti(lib, t, es, &k, HM_BINARY);
                    t = nt;
                    log.isz("lookup", idx);
                }
                hmfree(lib, t, es);
            }
        }
    });
}

// ===========================================================================
// row 25 - binary map, N at every table-growth threshold
// ===========================================================================
#[test]
fn cfg25_binary_growth_thresholds() {
    diff("cfg25", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 25);
        for &n in &[
            1usize, 2, 5, 6, 7, 8, 11, 12, 13, 23, 24, 25, 47, 48, 49, 100, 200,
        ] {
            for rep in 0..3usize {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let es = 16usize;
                let mut t: *mut c_void = std::ptr::null_mut();
                for i in 0..n {
                    let k = mkkey(&mut rng, i, 8);
                    t = hmput(lib, t, es, &k, HM_BINARY, rng.next_u64());
                }
                log.usz("n", n);
                log.usz("rep", rep);
                snap_map(log, t, es, KeyKind::Binary);
                hmfree(lib, t, es);
            }
        }
    });
}

// ===========================================================================
// row 26 - binary map, ~50 % duplicate keys (duplicate-hit path)
// ===========================================================================
#[test]
fn cfg26_binary_duplicate_keys() {
    diff("cfg26", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 26);
        for rep in 0..8usize {
            let hs = rng.next_u64() as usize;
            (lib.rand_seed)(hs);
            let es = 24usize;
            let keysize = 8usize;
            let mut t: *mut c_void = std::ptr::null_mut();
            let universe = 30usize;
            for step in 0..200usize {
                let i = rng.below(universe);
                let k = mkkey(&mut Rng::new(0x1234 + i as u64), i, keysize);
                let before_len = if t.is_null() {
                    0
                } else {
                    (*header((t as *mut u8).sub(es) as *mut c_void)).length
                };
                t = hmput(lib, t, es, &k, HM_BINARY, rng.next_u64());
                let after_len = (*header((t as *mut u8).sub(es) as *mut c_void)).length;
                log.usz("rep", rep);
                log.usz("step", step);
                log.usz("i", i);
                log.flag("grew", after_len != before_len);
                snap_map(log, t, es, KeyKind::Binary);
            }
            hmfree(lib, t, es);
        }
    });
}

// ===========================================================================
// row 27 - binary map with keysize == 0 (degenerate: memcmp(...,0) == 0)
// ===========================================================================
#[test]
fn cfg27_binary_zero_keysize() {
    diff("cfg27", |lib, log| unsafe {
        for &seed in &[0usize, 0x3141_5926, usize::MAX] {
            (lib.rand_seed)(seed);
            let es = 16usize;
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..10usize {
                t = hmput(lib, t, es, &[], HM_BINARY, 0x1000 + i as u64);
                log.usz("seed", seed);
                log.usz("i", i);
                snap_map(log, t, es, KeyKind::Binary);
            }
            let (t2, idx) = hmgeti(lib, t, es, &[], HM_BINARY);
            log.isz("get", idx);
            let (t3, del) = hmdel(lib, t2, es, &[], 0, 0, HM_BINARY);
            log.isz("del", del);
            snap_map(log, t3, es, KeyKind::Binary);
            hmfree(lib, t3, es);
        }
    });
}

// ===========================================================================
// row 28 - binary map, elemsize sweep
// ===========================================================================
#[test]
fn cfg28_binary_elemsize_sweep() {
    diff("cfg28", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 28);
        for &es in &[8usize, 9, 12, 16, 17, 24, 32, 64] {
            for &n in &[1usize, 10, 60] {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let keysize = 8usize.min(es);
                let mut t: *mut c_void = std::ptr::null_mut();
                for i in 0..n {
                    let k = mkkey(&mut rng, i, keysize);
                    t = hmput(lib, t, es, &k, HM_BINARY, rng.next_u64());
                }
                log.usz("es", es);
                log.usz("n", n);
                snap_map(log, t, es, KeyKind::Binary);
                hmfree(lib, t, es);
            }
        }
    });
}

// ===========================================================================
// row 29 - hmget_key / hmget_key_ts hits and misses
// ===========================================================================
#[test]
fn cfg29_binary_get_hit_and_miss() {
    diff("cfg29", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 29);
        for &n in &[0usize, 1, 2, 10, 60] {
            let hs = rng.next_u64() as usize;
            (lib.rand_seed)(hs);
            let es = 16usize;
            let keysize = 8usize;
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..n {
                let k = mkkey(&mut Rng::new(0xBEEF + i as u64), i, keysize);
                t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
            }
            log.usz("n", n);
            snap_map(log, t, es, KeyKind::Binary);
            // hits
            for i in 0..n {
                let mut k = mkkey(&mut Rng::new(0xBEEF + i as u64), i, keysize);
                let (nt, idx) = hmgeti(lib, t, es, &k, HM_BINARY);
                t = nt;
                log.isz("hit", idx);
                let mut temp: isize = 0x7777;
                let nt = (lib.hmget_key_ts)(
                    t,
                    es,
                    k.as_mut_ptr() as *mut c_void,
                    keysize,
                    &mut temp,
                    HM_BINARY,
                );
                t = nt;
                log.isz("hit_ts", temp);
            }
            // misses (indices far outside the inserted range)
            for j in 0..60usize {
                let i = 100_000 + j;
                let mut k = mkkey(&mut Rng::new(0xBEEF + i as u64), i, keysize);
                let (nt, idx) = hmgeti(lib, t, es, &k, HM_BINARY);
                t = nt;
                log.isz("miss", idx);
                let mut temp: isize = 0x7777;
                let nt = (lib.hmget_key_ts)(
                    t,
                    es,
                    k.as_mut_ptr() as *mut c_void,
                    keysize,
                    &mut temp,
                    HM_BINARY,
                );
                t = nt;
                log.isz("miss_ts", temp);
            }
            snap_map(log, t, es, KeyKind::Binary);
            hmfree(lib, t, es);
        }
    });
}

// ===========================================================================
// row 30 - hmget_key_ts with a == NULL, and with hash_table == NULL
// ===========================================================================
#[test]
fn cfg30_hmget_key_ts_no_table() {
    diff("cfg30", |lib, log| unsafe {
        for &es in &[8usize, 16, 24] {
            let mut key = [7u8; 8];

            // (a) a == NULL
            let mut temp: isize = 0x5555;
            let t = (lib.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                8,
                &mut temp,
                HM_BINARY,
            );
            log.usz("es", es);
            log.tag("null_a");
            log.isz("temp", temp);
            snap_map(log, t, es, KeyKind::Binary);

            // (b) same array again - now non-NULL but still hash_table == NULL
            let mut temp2: isize = 0x5555;
            let t2 = (lib.hmget_key_ts)(
                t,
                es,
                key.as_mut_ptr() as *mut c_void,
                8,
                &mut temp2,
                HM_BINARY,
            );
            log.tag("no_table");
            log.isz("temp2", temp2);
            log.flag("same", t2 == t);
            snap_map(log, t2, es, KeyKind::Binary);

            // (c) hmget_key (non-_ts) on the same table-less array
            let (t3, idx) = hmgeti(lib, t2, es, &key, HM_BINARY);
            log.isz("hmget_idx", idx);
            snap_map(log, t3, es, KeyKind::Binary);

            hmfree(lib, t3, es);
        }
    });
}

// ===========================================================================
// row 31 - hmput_default: NULL / length==0 / no-op, then hmput_key
// ===========================================================================
#[test]
fn cfg31_hmput_default() {
    diff("cfg31", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 31);
        for &es in &[8usize, 16, 24] {
            (lib.rand_seed)(0x3141_5926);

            // (a) a == NULL
            let mut t = (lib.hmput_default)(std::ptr::null_mut(), es);
            log.usz("es", es);
            log.tag("from_null");
            snap_map(log, t, es, KeyKind::Binary);

            // stbds_hmdefault(t, v):  t[-1].value = v
            let raw = (t as *mut u8).sub(es);
            let mut v = 0xDEAD_BEEF_1234_5678u64;
            for k in 0..es {
                *raw.add(k) = (v & 0xff) as u8;
                v = v.rotate_left(8);
            }
            snap_map(log, t, es, KeyKind::Binary);

            // (b) no-op path (length != 0)
            let t2 = (lib.hmput_default)(t, es);
            log.tag("noop");
            log.flag("same", t2 == t);
            snap_map(log, t2, es, KeyKind::Binary);
            t = t2;

            // (c) length == 0 path
            (*header((t as *mut u8).sub(es) as *mut c_void)).length = 0;
            let t3 = (lib.hmput_default)(t, es);
            log.tag("zero_len");
            snap_map(log, t3, es, KeyKind::Binary);
            t = t3;

            // (d) now put real keys on top of it
            for i in 0..12usize {
                let k = mkkey(&mut rng, i, 8usize.min(es));
                t = hmput(lib, t, es, &k, HM_BINARY, rng.next_u64());
                log.usz("i", i);
                snap_map(log, t, es, KeyKind::Binary);
            }
            hmfree(lib, t, es);
        }
    });
}

// ===========================================================================
// row 32 - hmdel_key, keyoffset 0, delete everything in random order
// ===========================================================================
#[test]
fn cfg32_binary_delete_all_random_order() {
    diff("cfg32", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 32);
        for &n in &[1usize, 2, 10, 60, 200] {
            for rep in 0..3usize {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let es = 16usize;
                let keysize = 8usize;
                let mut t: *mut c_void = std::ptr::null_mut();
                let keys: Vec<Vec<u8>> = (0..n)
                    .map(|i| mkkey(&mut Rng::new(0xC0DE + i as u64), i, keysize))
                    .collect();
                for (i, k) in keys.iter().enumerate() {
                    t = hmput(lib, t, es, k, HM_BINARY, i as u64 * 0x0101_0101);
                }
                log.usz("n", n);
                log.usz("rep", rep);
                snap_map(log, t, es, KeyKind::Binary);
                // random deletion order (deterministic permutation)
                let mut order: Vec<usize> = (0..n).collect();
                for i in (1..n).rev() {
                    let j = rng.below(i + 1);
                    order.swap(i, j);
                }
                for &i in &order {
                    let (nt, d) = hmdel(lib, t, es, &keys[i], keysize, 0, HM_BINARY);
                    t = nt;
                    log.usz("del_i", i);
                    log.isz("deleted", d);
                    snap_map(log, t, es, KeyKind::Binary);
                }
                // deleting again must report "not found"
                for &i in &order {
                    let (nt, d) = hmdel(lib, t, es, &keys[i], keysize, 0, HM_BINARY);
                    t = nt;
                    log.isz("redel", d);
                }
                snap_map(log, t, es, KeyKind::Binary);
                hmfree(lib, t, es);
            }
        }
    });
}

// ===========================================================================
// row 33 - hmdel_key with a non-zero keyoffset
// ===========================================================================
#[test]
fn cfg33_binary_nonzero_keyoffset() {
    diff("cfg33", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 33);
        for &keyoffset in &[4usize, 8, 16] {
            for &keysize in &[4usize, 8] {
                let es = keyoffset + keysize + 8;
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                // Build the map with keys stored at `keyoffset` inside the
                // element: put with keysize covering [0,keyoffset+keysize) is
                // not what the macros do, so instead build the elements by
                // hand the way `stbds_hmput` would if the struct's key field
                // sat at `keyoffset`: put the *whole prefix* as the key.
                //
                // hmput_key always uses keyoffset 0, so the key must live at
                // element offset 0 for insertion; hmdel_key however accepts an
                // arbitrary keyoffset. Using keyoffset != 0 for deletion on a
                // map built with keyoffset 0 is exactly what the C does when
                // `STBDS_OFFSETOF(t,key) != 0`, and both implementations must
                // agree on the (garbage-but-deterministic) outcome.
                let n = 20usize;
                let mut t: *mut c_void = std::ptr::null_mut();
                let keys: Vec<Vec<u8>> = (0..n)
                    .map(|i| {
                        // element layout: [0,keyoffset) filler, then the key
                        let mut v = vec![0u8; keyoffset];
                        v.extend_from_slice(&mkkey(&mut Rng::new(0x77 + i as u64), i, keysize));
                        v
                    })
                    .collect();
                for (i, k) in keys.iter().enumerate() {
                    // insert with keysize = keyoffset+keysize so the full
                    // prefix (filler + key) is memcpy'd into the element
                    t = hmput(lib, t, es, k, HM_BINARY, i as u64);
                }
                log.usz("keyoffset", keyoffset);
                log.usz("keysize", keysize);
                snap_map(log, t, es, KeyKind::Binary);
                for (i, k) in keys.iter().enumerate() {
                    // now delete using keyoffset != 0 and keysize
                    let (nt, d) = hmdel(lib, t, es, &k[keyoffset..], keysize, keyoffset, HM_BINARY);
                    t = nt;
                    log.usz("i", i);
                    log.isz("deleted", d);
                    snap_map(log, t, es, KeyKind::Binary);
                }
                hmfree(lib, t, es);
            }
        }
    });
}

// ===========================================================================
// row 34 - delete down through the shrink thresholds (128 -> 64 -> ... -> 8)
// ===========================================================================
#[test]
fn cfg34_binary_shrink_cascade() {
    diff("cfg34", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 34);
        for rep in 0..4usize {
            let hs = rng.next_u64() as usize;
            (lib.rand_seed)(hs);
            let es = 16usize;
            let keysize = 8usize;
            let n = 200usize;
            let mut t: *mut c_void = std::ptr::null_mut();
            let keys: Vec<Vec<u8>> = (0..n)
                .map(|i| mkkey(&mut Rng::new(0xAB01 + i as u64), i, keysize))
                .collect();
            for (i, k) in keys.iter().enumerate() {
                t = hmput(lib, t, es, k, HM_BINARY, i as u64);
            }
            log.usz("rep", rep);
            snap_map(log, t, es, KeyKind::Binary);
            for i in 0..175usize {
                let (nt, d) = hmdel(lib, t, es, &keys[i], keysize, 0, HM_BINARY);
                t = nt;
                log.usz("i", i);
                log.isz("d", d);
                snap_map(log, t, es, KeyKind::Binary);
            }
            hmfree(lib, t, es);
        }
    });
}

// ===========================================================================
// row 35 - alternating insert/delete -> the tombstone rebuild path
// ===========================================================================
#[test]
fn cfg35_binary_tombstone_rebuild() {
    diff("cfg35", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 35);
        for rep in 0..4usize {
            let hs = rng.next_u64() as usize;
            (lib.rand_seed)(hs);
            let es = 16usize;
            let keysize = 8usize;
            let mut t: *mut c_void = std::ptr::null_mut();
            let mut live: Vec<usize> = Vec::new();
            for step in 0..400usize {
                let insert = live.is_empty() || rng.below(2) == 0;
                if insert {
                    let i = step;
                    let k = mkkey(&mut Rng::new(0x900 + i as u64), i, keysize);
                    t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
                    live.push(i);
                    log.usz("put", i);
                } else {
                    let j = rng.below(live.len());
                    let i = live.swap_remove(j);
                    let k = mkkey(&mut Rng::new(0x900 + i as u64), i, keysize);
                    let (nt, d) = hmdel(lib, t, es, &k, keysize, 0, HM_BINARY);
                    t = nt;
                    log.usz("del", i);
                    log.isz("d", d);
                }
                log.usz("step", step);
                snap_map(log, t, es, KeyKind::Binary);
            }
            hmfree(lib, t, es);
        }
    });
}

// ===========================================================================
// row 36 - delete the last element (no memmove) vs a middle element (memmove
//          + re-find_slot)
// ===========================================================================
#[test]
fn cfg36_binary_delete_last_vs_middle() {
    diff("cfg36", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 36);
        for &n in &[1usize, 2, 3, 8, 30] {
            for target in 0..n {
                let hs = rng.next_u64() as usize;
                (lib.rand_seed)(hs);
                let es = 16usize;
                let keysize = 8usize;
                let mut t: *mut c_void = std::ptr::null_mut();
                let keys: Vec<Vec<u8>> = (0..n)
                    .map(|i| mkkey(&mut Rng::new(0xDD00 + i as u64), i, keysize))
                    .collect();
                for (i, k) in keys.iter().enumerate() {
                    t = hmput(lib, t, es, k, HM_BINARY, i as u64);
                }
                log.usz("n", n);
                log.usz("target", target);
                snap_map(log, t, es, KeyKind::Binary);
                let (nt, d) = hmdel(lib, t, es, &keys[target], keysize, 0, HM_BINARY);
                t = nt;
                log.isz("d", d);
                snap_map(log, t, es, KeyKind::Binary);
                // all remaining keys must still be findable
                for (i, k) in keys.iter().enumerate() {
                    if i == target {
                        continue;
                    }
                    let (nt, idx) = hmgeti(lib, t, es, k, HM_BINARY);
                    t = nt;
                    log.usz("i", i);
                    log.isz("idx", idx);
                }
                hmfree(lib, t, es);
            }
        }
    });
}
