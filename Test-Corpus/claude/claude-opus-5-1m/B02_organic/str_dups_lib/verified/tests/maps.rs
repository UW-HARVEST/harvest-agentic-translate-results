//! Phase B (continued) — `CONFIGS.md` rows 25-72: the hash-map entry points.
//!
//! Every row drives BOTH shared libraries in lockstep through their exported
//! symbols and compares the full observable state (array header, hash index,
//! every bucket slot, every element byte) after each operation.

mod harness;

use harness::*;
use std::ffi::{c_int, c_void};

fn bin_cfg(elemsize: usize, keysize: usize) -> MapCfg {
    MapCfg {
        elemsize,
        keysize,
        key_is_ptr: false,
    }
}

fn str_cfg(elemsize: usize) -> MapCfg {
    MapCfg {
        elemsize,
        keysize: 8,
        key_is_ptr: true,
    }
}

/// `char *`-sized key whose 8 bytes are also a valid NUL terminated string, so
/// the very same buffer can be interpreted by the binary *and* the string path.
fn dual_key(rng: &mut Rng) -> Vec<u8> {
    let mut v = rng.ascii(7);
    v.push(0);
    v
}

/// Intern `keysize` key bytes plus a guard NUL so that a mismatched `mode`
/// (string hashing over binary key bytes) cannot read out of bounds.
fn intern_key(p: &mut Pair, bytes: &[u8]) -> *mut c_void {
    let mut v = bytes.to_vec();
    v.push(0);
    p.intern(&v) as *mut c_void
}

// ===========================================================================
// Rows 25-34 — binary keys, insertion
// ===========================================================================

#[test]
fn cfg_bin_single_insert() {
    // Row 25
    let mut rng = Rng::new(0x5001);
    for i in 0..32usize {
        seed_both(DEFAULT_SEED ^ i);
        let mut p = Pair::new(format!("bin_single i={i}"), bin_cfg(16, 8));
        p.check("empty (NULL map)");
        let kb = rng.bytes(8);
        let k = intern_key(&mut p, &kb);
        let t = p.put(k, STBDS_HM_BINARY, rng.next_u64());
        assert_eq!(t, 0, "first insert must land at index 0");
        p.check("after single put");
        assert_eq!(p.get(k, STBDS_HM_BINARY), 0);
        p.check("after get");
        assert_eq!(p.string_mode(0), Some(0), "bootstrap with mode=0 -> SH_NONE");
        p.free();
    }
}

/// Insert `n` random `keysize`-byte keys, then look every one of them up.
fn bin_insert_run(
    label: &str,
    elemsize: usize,
    keysize: usize,
    n: usize,
    rng_seed: u64,
    hash_seed: usize,
    snap_every: usize,
) {
    seed_both(hash_seed);
    let mut rng = Rng::new(rng_seed);
    let mut p = Pair::new(label.to_string(), bin_cfg(elemsize, keysize));
    let mut keys: Vec<*mut c_void> = Vec::new();
    for i in 0..n {
        let kb = rng.bytes(keysize);
        let k = intern_key(&mut p, &kb);
        keys.push(k);
        let t = p.put(k, STBDS_HM_BINARY, rng.next_u64());
        assert!(t >= 0, "{label}: put {i} returned {t}");
        if snap_every != 0 && i % snap_every == 0 {
            p.check(&format!("after put {i}"));
        }
    }
    p.check("after all puts");
    for (i, &k) in keys.iter().enumerate() {
        let t = p.get(k, STBDS_HM_BINARY);
        assert!(t >= 0, "{label}: key {i} unexpectedly absent");
        if snap_every != 0 && i % snap_every == 0 {
            p.check(&format!("after get {i}"));
        }
    }
    p.check("after all gets");
    p.free();
}

#[test]
fn cfg_bin_below_grow_threshold() {
    // Row 26: 5 inserts — used_count stays below 8-8/4 == 6.
    for s in 0..32u64 {
        bin_insert_run("bin_5", 16, 8, 5, 0x5100 + s, DEFAULT_SEED, 1);
    }
}

#[test]
fn cfg_bin_at_grow_threshold() {
    // Row 27: exactly 6 and 7 inserts — the 6th grows 8 -> 16.
    for s in 0..32u64 {
        bin_insert_run("bin_6", 16, 8, 6, 0x5200 + s, DEFAULT_SEED, 1);
        bin_insert_run("bin_7", 16, 8, 7, 0x5280 + s, DEFAULT_SEED, 1);
    }
}

#[test]
fn cfg_bin_multiple_grows() {
    // Row 28: 12 / 24 / 48 inserts — grows 16->32->64->128, rehashing a
    // populated index every time.
    for &n in &[12usize, 24, 48] {
        for s in 0..16u64 {
            bin_insert_run(&format!("bin_{n}"), 16, 8, n, 0x5300 + s + n as u64, DEFAULT_SEED, 1);
        }
    }
}

#[test]
fn cfg_bin_large_random() {
    // Row 29: 1000 random u64 keys, four different PRNG streams.
    for s in 0..4u64 {
        bin_insert_run("bin_1000", 16, 8, 1000, 0x5400 + s, DEFAULT_SEED, 37);
    }
}

#[test]
fn cfg_bin_keysize_sweep_small() {
    // Row 30: keysize 1..=8 with elemsize 8.
    for keysize in 1..=8usize {
        for s in 0..8u64 {
            bin_insert_run(
                &format!("bin_ks{keysize}"),
                8,
                keysize,
                64,
                0x5500 + s + keysize as u64 * 97,
                DEFAULT_SEED,
                7,
            );
        }
    }
}

#[test]
fn cfg_bin_keysize_sweep_large() {
    // Row 31: keysize 9..32 with elemsize = keysize + 8.
    for &keysize in &[9usize, 12, 16, 17, 24, 32] {
        for s in 0..8u64 {
            bin_insert_run(
                &format!("bin_ks{keysize}"),
                keysize + 8,
                keysize,
                64,
                0x5600 + s + keysize as u64 * 131,
                DEFAULT_SEED,
                7,
            );
        }
    }
}

#[test]
fn cfg_bin_unaligned_elemsize() {
    // Row 32: elemsize 7 / 12 / 20 with a 4 byte key.
    for &elemsize in &[7usize, 12, 20] {
        for s in 0..8u64 {
            bin_insert_run(
                &format!("bin_es{elemsize}"),
                elemsize,
                4,
                64,
                0x5700 + s + elemsize as u64 * 17,
                DEFAULT_SEED,
                7,
            );
        }
    }
}

#[test]
fn cfg_bin_duplicates() {
    // Row 33: every key inserted three times -> temp is the existing index and
    // `length` does not change.
    let mut rng = Rng::new(0x5800);
    for s in 0..8u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut p = Pair::new(format!("bin_dup s={s}"), bin_cfg(16, 8));
        let mut keys = Vec::new();
        for _ in 0..40 {
            let kb = rng.bytes(8);
            keys.push(intern_key(&mut p, &kb));
        }
        // first round: all new
        let mut expect = Vec::new();
        for (i, &k) in keys.iter().enumerate() {
            let t = p.put(k, STBDS_HM_BINARY, rng.next_u64());
            assert_eq!(t, i as isize, "insertion order");
            expect.push(t);
            p.check(&format!("dup round0 i={i}"));
        }
        // rounds 2 and 3: all duplicates
        for round in 1..3 {
            for (i, &k) in keys.iter().enumerate() {
                let len_before = p.snapshot(0).length;
                let t = p.put(k, STBDS_HM_BINARY, rng.next_u64());
                assert_eq!(t, expect[i], "duplicate must reuse index");
                assert_eq!(p.snapshot(0).length, len_before, "length must not change");
                p.check(&format!("dup round{round} i={i}"));
            }
        }
        p.free();
    }
}

#[test]
fn cfg_bin_forced_collisions() {
    // Row 34: keys that differ only in the last byte, plus a tiny key domain,
    // to hammer the forward + wrap-around bucket scans and tombstone probing.
    for s in 0..4u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0x5900 + s);
        let mut p = Pair::new(format!("bin_collide s={s}"), bin_cfg(16, 8));
        let prefix = rng.bytes(7);
        let mut keys = Vec::new();
        for b in 0..=255u8 {
            let mut kb = prefix.clone();
            kb.push(b);
            keys.push(intern_key(&mut p, &kb));
        }
        for (i, &k) in keys.iter().enumerate() {
            let t = p.put(k, STBDS_HM_BINARY, rng.next_u64());
            assert_eq!(t, i as isize);
            if i % 8 == 0 {
                p.check(&format!("collide put i={i}"));
            }
        }
        p.check("collide all inserted");
        // tiny key domain: 1-byte keys, all 256 values, inserted repeatedly
        let mut q = Pair::new(format!("bin_tiny s={s}"), bin_cfg(8, 1));
        for round in 0..2 {
            for b in 0..=255u8 {
                let k = intern_key(&mut q, &[b]);
                q.put(k, STBDS_HM_BINARY, rng.next_u64());
                if b % 32 == 0 {
                    q.check(&format!("tiny round={round} b={b}"));
                }
            }
        }
        q.check("tiny all inserted");
        p.free();
        q.free();
    }
}

// ===========================================================================
// Rows 35-36 — lookups
// ===========================================================================

#[test]
fn cfg_bin_get_hits_and_misses() {
    // Row 35: stbds_hmget_key, hits and misses interleaved.
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0x5A00);
    let mut p = Pair::new("bin_get".to_string(), bin_cfg(16, 8));
    let mut present = Vec::new();
    for _ in 0..200 {
        let kb = rng.bytes(8);
        let k = intern_key(&mut p, &kb);
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
        present.push(k);
    }
    p.check("populated");
    let mut absent = Vec::new();
    for _ in 0..200 {
        let mut kb = rng.bytes(8);
        kb[0] ^= 0x80;
        kb[7] ^= 0x40;
        absent.push(intern_key(&mut p, &kb));
    }
    for i in 0..400 {
        if i % 2 == 0 {
            let t = p.get(present[i / 2], STBDS_HM_BINARY);
            assert!(t >= 0, "hit {i} should be found");
        } else {
            p.get(absent[i / 2], STBDS_HM_BINARY);
        }
        if i % 17 == 0 {
            p.check(&format!("get i={i}"));
        }
    }
    p.check("after gets");
    p.free();
}

#[test]
fn cfg_bin_get_ts() {
    // Row 36: stbds_hmget_key_ts — the out-param variant that must NOT write
    // `header->temp`.
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0x5B00);
    let mut p = Pair::new("bin_get_ts".to_string(), bin_cfg(24, 8));
    let mut present = Vec::new();
    for _ in 0..200 {
        let kb = rng.bytes(8);
        let k = intern_key(&mut p, &kb);
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
        present.push(k);
    }
    let temp_before = p.snapshot(0).temp;
    for i in 0..200 {
        let t = p.get_ts(present[i], STBDS_HM_BINARY);
        assert!(t >= 0);
        assert_eq!(
            p.snapshot(0).temp,
            temp_before,
            "hmget_key_ts must not touch header->temp"
        );
        let mut kb = rng.bytes(8);
        kb[0] ^= 0xAA;
        let miss = intern_key(&mut p, &kb);
        assert_eq!(p.get_ts(miss, STBDS_HM_BINARY), -1);
        if i % 17 == 0 {
            p.check(&format!("get_ts i={i}"));
        }
    }
    p.check("after get_ts");
    p.free();
}

// ===========================================================================
// Rows 37-43 — deletions
// ===========================================================================

#[test]
fn cfg_bin_del_last() {
    // Row 37: delete the highest-index element -> old_index == final_index, so
    // the swap-with-last block is skipped entirely.
    for s in 0..32u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0x5C00 + s);
        let mut p = Pair::new(format!("bin_del_last s={s}"), bin_cfg(16, 8));
        let mut keys = Vec::new();
        for _ in 0..5 {
            let kb = rng.bytes(8);
            let k = intern_key(&mut p, &kb);
            p.put(k, STBDS_HM_BINARY, rng.next_u64());
            keys.push(k);
        }
        p.check("before del");
        for i in (0..5).rev() {
            let t = p.del(keys[i], 0, STBDS_HM_BINARY);
            assert_eq!(t, 1, "delete must report 1");
            p.check(&format!("after del of last, i={i}"));
        }
        p.check("all deleted");
        p.free();
    }
}

#[test]
fn cfg_bin_del_swap() {
    // Row 38: delete index 0 / the middle -> memmove + re-find + index fixup.
    for s in 0..32u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0x5D00 + s);
        let mut p = Pair::new(format!("bin_del_swap s={s}"), bin_cfg(16, 8));
        let mut keys = Vec::new();
        for _ in 0..9 {
            let kb = rng.bytes(8);
            let k = intern_key(&mut p, &kb);
            p.put(k, STBDS_HM_BINARY, rng.next_u64());
            keys.push(k);
        }
        p.check("before del");
        // delete the first, then the (new) middle
        assert_eq!(p.del(keys[0], 0, STBDS_HM_BINARY), 1);
        p.check("after del first");
        assert_eq!(p.del(keys[4], 0, STBDS_HM_BINARY), 1);
        p.check("after del middle");
        // deleted keys are gone, the rest is still reachable
        assert_eq!(p.get(keys[0], STBDS_HM_BINARY), -1);
        assert_eq!(p.get(keys[4], STBDS_HM_BINARY), -1);
        for (i, &k) in keys.iter().enumerate() {
            if i != 0 && i != 4 {
                assert!(p.get(k, STBDS_HM_BINARY) >= 0, "key {i} must survive");
            }
        }
        p.check("after post-del gets");
        p.free();
    }
}

#[test]
fn cfg_bin_del_all() {
    // Row 39: delete everything, in insertion order and in reverse.
    for reverse in [false, true] {
        for s in 0..32u64 {
            seed_both(DEFAULT_SEED ^ s as usize);
            let mut rng = Rng::new(0x5E00 + s + if reverse { 1 << 16 } else { 0 });
            let mut p = Pair::new(format!("bin_del_all rev={reverse} s={s}"), bin_cfg(16, 8));
            let mut keys = Vec::new();
            for _ in 0..20 {
                let kb = rng.bytes(8);
                let k = intern_key(&mut p, &kb);
                p.put(k, STBDS_HM_BINARY, rng.next_u64());
                keys.push(k);
            }
            if reverse {
                keys.reverse();
            }
            for (i, &k) in keys.iter().enumerate() {
                assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 1, "del {i}");
                p.check(&format!("del {i}"));
            }
            assert_eq!(p.snapshot(0).length, 1, "only the sentinel must remain");
            // deleting again must report 0
            for &k in &keys {
                assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 0);
            }
            p.check("all deleted twice");
            p.free();
        }
    }
}

#[test]
fn cfg_bin_del_shrink() {
    // Row 40: 100 inserts then 90 deletes crosses used_count_shrink_threshold
    // several times (index halves each time).
    for s in 0..8u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0x5F00 + s);
        let mut p = Pair::new(format!("bin_shrink s={s}"), bin_cfg(16, 8));
        let mut keys = Vec::new();
        for _ in 0..100 {
            let kb = rng.bytes(8);
            let k = intern_key(&mut p, &kb);
            p.put(k, STBDS_HM_BINARY, rng.next_u64());
            keys.push(k);
        }
        p.check("100 inserted");
        for i in 0..90 {
            assert_eq!(p.del(keys[i], 0, STBDS_HM_BINARY), 1);
            p.check(&format!("shrink del {i}"));
        }
        for i in 90..100 {
            assert!(p.get(keys[i], STBDS_HM_BINARY) >= 0, "survivor {i}");
        }
        p.check("after shrink");
        p.free();
    }
}

#[test]
fn cfg_bin_del_tombstone_rebuild() {
    // Row 41: alternate put/del on a wide table so tombstone_count crosses
    // tombstone_count_threshold (same-size rebuild, no shrink).
    for s in 0..8u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0x6000 + s);
        let mut p = Pair::new(format!("bin_tomb s={s}"), bin_cfg(16, 8));
        // build a wide table first
        let mut keep = Vec::new();
        for _ in 0..60 {
            let kb = rng.bytes(8);
            let k = intern_key(&mut p, &kb);
            p.put(k, STBDS_HM_BINARY, rng.next_u64());
            keep.push(k);
        }
        p.check("base built");
        for i in 0..80 {
            let kb = rng.bytes(8);
            let k = intern_key(&mut p, &kb);
            assert!(p.put(k, STBDS_HM_BINARY, rng.next_u64()) >= 0);
            assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 1);
            p.check(&format!("tomb cycle {i}"));
        }
        for (i, &k) in keep.iter().enumerate() {
            assert!(p.get(k, STBDS_HM_BINARY) >= 0, "keeper {i} must survive");
        }
        p.check("after tombstone cycles");
        p.free();
    }
}

#[test]
fn cfg_bin_random_op_stream() {
    // Row 42: property test — long random streams of put / get / get_ts / del /
    // del-missing, snapshot compared after every single operation.
    for s in 0..4u64 {
        seed_both(DEFAULT_SEED ^ (s as usize * 7919));
        let mut rng = Rng::new(0x6100 + s);
        let mut p = Pair::new(format!("bin_stream s={s}"), bin_cfg(16, 8));
        let mut live: Vec<*mut c_void> = Vec::new();
        let mut dead: Vec<*mut c_void> = Vec::new();
        for op in 0..3000 {
            match rng.below(10) {
                0..=4 => {
                    let kb = rng.bytes(8);
                    let k = intern_key(&mut p, &kb);
                    let t = p.put(k, STBDS_HM_BINARY, rng.next_u64());
                    assert!(t >= 0);
                    live.push(k);
                }
                5..=6 => {
                    if !live.is_empty() {
                        let k = live[rng.below(live.len())];
                        assert!(p.get(k, STBDS_HM_BINARY) >= 0);
                    }
                }
                7 => {
                    if !dead.is_empty() {
                        let k = dead[rng.below(dead.len())];
                        assert_eq!(p.get_ts(k, STBDS_HM_BINARY), -1);
                    }
                }
                8 => {
                    if !live.is_empty() {
                        let idx = rng.below(live.len());
                        let k = live.swap_remove(idx);
                        assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 1);
                        dead.push(k);
                    }
                }
                _ => {
                    if !dead.is_empty() {
                        let k = dead[rng.below(dead.len())];
                        assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 0);
                    }
                }
            }
            p.check(&format!("stream op {op}"));
        }
        p.free();
    }
}

#[test]
fn cfg_bin_del_keyoffset() {
    // Row 43: hmdel_key with a non-zero keyoffset.  `hmput_key` always stores
    // the key at offset 0, so the test mirrors the key bytes into `keyoffset`
    // as well (what a real consumer with a struct key does).
    for &(elemsize, keyoffset) in &[(24usize, 8usize), (32, 16), (16, 0), (24, 4)] {
        for s in 0..16u64 {
            seed_both(DEFAULT_SEED ^ s as usize);
            let mut rng = Rng::new(0x6200 + s + keyoffset as u64 * 31);
            let keysize = 8usize;
            let mut p = Pair::new(
                format!("bin_keyoff es={elemsize} ko={keyoffset} s={s}"),
                bin_cfg(elemsize, keysize),
            );
            let mut keys: Vec<(*mut c_void, Vec<u8>)> = Vec::new();
            for _ in 0..12 {
                // For keyoffset == 4 the mirrored copy overlaps the stored key,
                // so use a key whose two halves are equal.
                let kb = if keyoffset == 4 {
                    let h = rng.bytes(4);
                    let mut v = h.clone();
                    v.extend_from_slice(&h);
                    v
                } else {
                    rng.bytes(8)
                };
                let k = intern_key(&mut p, &kb);
                let t = p.put(k, STBDS_HM_BINARY, rng.next_u64());
                assert!(t >= 0);
                // mirror the key bytes to `keyoffset` in both libraries
                if keyoffset + keysize <= elemsize {
                    for i in 0..2 {
                        if !p.t[i].is_null() {
                            unsafe {
                                let e = (p.t[i] as *mut u8)
                                    .add(elemsize * t as usize)
                                    .add(keyoffset);
                                std::ptr::copy_nonoverlapping(kb.as_ptr(), e, keysize);
                            }
                        }
                    }
                }
                keys.push((k, kb));
            }
            p.check("populated");
            for (i, (k, _)) in keys.iter().enumerate() {
                let t = p.del(*k, keyoffset, STBDS_HM_BINARY);
                assert_eq!(t, 1, "keyoffset={keyoffset} del {i}");
                p.check(&format!("keyoff del {i}"));
            }
            p.free();
        }
    }
}

// ===========================================================================
// Rows 44-45 — degenerate sizes
// ===========================================================================

#[test]
fn cfg_bin_keysize_zero() {
    // Row 44: keysize == 0 -> hash_bytes never reads the key and memcmp of 0
    // bytes is always equal, so the table can only ever hold one element.
    for s in 0..32u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0x6300 + s);
        let mut p = Pair::new(format!("bin_ks0 s={s}"), bin_cfg(16, 0));
        let mut prev = None;
        for i in 0..10 {
            let kb = rng.bytes(8);
            let k = intern_key(&mut p, &kb);
            let t = p.put(k, STBDS_HM_BINARY, rng.next_u64());
            assert_eq!(t, 0, "keysize=0: everything maps to index 0");
            if let Some(l) = prev {
                assert_eq!(p.snapshot(0).length, l, "length must stay at 2");
            }
            prev = Some(p.snapshot(0).length);
            p.check(&format!("ks0 put {i}"));
            assert_eq!(p.get(k, STBDS_HM_BINARY), 0);
            p.check(&format!("ks0 get {i}"));
        }
        // deleting once empties it; deleting again reports 0
        assert_eq!(p.del(std::ptr::null_mut(), 0, STBDS_HM_BINARY), 1);
        p.check("ks0 after del");
        assert_eq!(p.del(std::ptr::null_mut(), 0, STBDS_HM_BINARY), 0);
        p.check("ks0 after second del");
        p.free();
    }
}

#[test]
fn cfg_bin_elemsize_zero() {
    // Row 45: elemsize == 0 && keysize == 0 — HASH_TO_ARR / ARR_TO_HASH become
    // the identity and every element write copies 0 bytes.
    for s in 0..32u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut p = Pair::new(format!("bin_es0 s={s}"), bin_cfg(0, 0));
        for i in 0..5 {
            let t = p.put(std::ptr::null_mut(), STBDS_HM_BINARY, 0);
            assert_eq!(t, 0);
            p.check(&format!("es0 put {i}"));
        }
        assert_eq!(p.get(std::ptr::null_mut(), STBDS_HM_BINARY), 0);
        p.check("es0 get");
        assert_eq!(p.del(std::ptr::null_mut(), 0, STBDS_HM_BINARY), 1);
        p.check("es0 del");
        p.free();
    }
}

// ===========================================================================
// Rows 46-50 — string tables, one row per `table->string.mode`
// ===========================================================================

/// Build a string table in the requested ownership mode and insert `keys`.
/// `shmode == None` means "bootstrap through hmput_key(mode=1)", which the C
/// initialises to `SH_DEFAULT`.
///
/// `do_lookups` must be `false` for the `switch` **default** (memcpy) modes:
/// there the element's first `keysize` bytes hold the *key text*, but
/// `stbds_is_key_equal` in string mode reads them as a `char *` — so any string
/// lookup dereferences a wild pointer.  That is genuine C behaviour and is
/// covered by the crash-equivalence rows `err_memcpy_mode_lookup_segv` /
/// `err_memcpy_mode_del_segv` in `tests/errors.rs`.
#[allow(clippy::too_many_arguments)]
fn str_table_run(
    label: &str,
    shmode: Option<c_int>,
    expect_string_mode: u8,
    key_is_ptr: bool,
    keys: &[Vec<u8>],
    hash_seed: usize,
    rng_seed: u64,
    snap_every: usize,
    do_lookups: bool,
) {
    seed_both(hash_seed);
    let mut rng = Rng::new(rng_seed);
    let cfg = if key_is_ptr {
        str_cfg(16)
    } else {
        bin_cfg(16, 8)
    };
    let mut p = Pair::new(label.to_string(), cfg);
    if let Some(m) = shmode {
        p.shmode(m);
        p.check("after shmode_func");
        assert_eq!(p.string_mode(0), Some(expect_string_mode));
        assert_eq!(p.string_mode(1), Some(expect_string_mode));
    }
    let mut ptrs = Vec::new();
    for (i, kb) in keys.iter().enumerate() {
        let k = p.intern_cstr(kb) as *mut c_void;
        ptrs.push(k);
        let t = p.put(k, STBDS_HM_STRING, rng.next_u64());
        assert!(t >= 0, "{label}: put {i}");
        if snap_every != 0 && i % snap_every == 0 {
            p.check(&format!("str put {i}"));
        }
    }
    p.check("after all string puts");
    assert_eq!(
        p.string_mode(0),
        Some(expect_string_mode),
        "{label}: unexpected string.mode"
    );
    if do_lookups {
        for (i, &k) in ptrs.iter().enumerate() {
            let t = p.get(k, STBDS_HM_STRING);
            assert!(t >= 0, "{label}: key {i} absent");
            if snap_every != 0 && i % snap_every == 0 {
                p.check(&format!("str get {i}"));
            }
            let t2 = p.get_ts(k, STBDS_HM_STRING);
            assert_eq!(t2, t, "{label}: get_ts disagrees with get for key {i}");
        }
        p.check("after all string gets");
    }
    p.free();
}

fn distinct_keys(rng: &mut Rng, n: usize, minlen: usize, maxlen: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    while out.len() < n {
        let len = rng.range(minlen, maxlen);
        let k = rng.ascii(len);
        if seen.insert(k.clone()) {
            out.push(k);
        }
    }
    out
}

#[test]
fn cfg_str_default_bootstrap() {
    // Row 46: hmput_key(mode=1) on a NULL map -> string.mode = SH_DEFAULT, the
    // caller's pointer is stored verbatim.
    for s in 0..8u64 {
        let mut rng = Rng::new(0x7000 + s);
        let keys = distinct_keys(&mut rng, 64, 1, 24);
        str_table_run(
            &format!("str_default s={s}"),
            None,
            STBDS_SH_DEFAULT as u8,
            true,
            &keys,
            DEFAULT_SEED ^ s as usize,
            0x7080 + s,
            1,
            true,
        );
    }
}

#[test]
fn cfg_str_strdup() {
    // Row 47: sh_new_strdup -> the library owns a copy of every key.
    for s in 0..8u64 {
        let mut rng = Rng::new(0x7100 + s);
        let keys = distinct_keys(&mut rng, 64, 0, 24);
        str_table_run(
            &format!("str_strdup s={s}"),
            Some(STBDS_SH_STRDUP),
            2,
            true,
            &keys,
            DEFAULT_SEED ^ s as usize,
            0x7180 + s,
            1,
            true,
        );
    }
}

#[test]
fn cfg_str_arena() {
    // Row 48: sh_new_arena -> keys are copied into the index's own arena, whose
    // `block` / `remaining` evolve inside the hash index.
    for s in 0..8u64 {
        let mut rng = Rng::new(0x7200 + s);
        let keys = distinct_keys(&mut rng, 64, 0, 24);
        str_table_run(
            &format!("str_arena s={s}"),
            Some(STBDS_SH_ARENA),
            3,
            true,
            &keys,
            DEFAULT_SEED ^ s as usize,
            0x7280 + s,
            1,
            true,
        );
    }
}

#[test]
fn cfg_str_mode_none_memcpy() {
    // Row 49: string.mode == SH_NONE hits the `switch` default -> the *pointer
    // bytes* are memcpy'd into the element instead of a string copy.
    for s in 0..8u64 {
        let mut rng = Rng::new(0x7300 + s);
        // >= 8 byte keys so the 8 byte memcpy stays inside the key buffer
        let keys = distinct_keys(&mut rng, 64, 8, 24);
        str_table_run(
            &format!("str_none s={s}"),
            Some(STBDS_SH_NONE),
            0,
            false,
            &keys,
            DEFAULT_SEED ^ s as usize,
            0x7380 + s,
            1,
            false,
        );
    }
}

#[test]
fn cfg_str_mode_255_memcpy() {
    // Row 50: an out-of-range string.mode also lands in the default branch.
    for s in 0..8u64 {
        let mut rng = Rng::new(0x7400 + s);
        let keys = distinct_keys(&mut rng, 64, 8, 24);
        str_table_run(
            &format!("str_255 s={s}"),
            Some(255),
            255,
            false,
            &keys,
            DEFAULT_SEED ^ s as usize,
            0x7480 + s,
            1,
            false,
        );
    }
}

// ===========================================================================
// Rows 51-54, 58, 60-61 — string key shapes
// ===========================================================================

#[test]
fn cfg_str_key_length_sweep() {
    // Row 51: key lengths 0..=32 (including "") through the strdup path.
    let mut rng = Rng::new(0x7500);
    let mut keys: Vec<Vec<u8>> = (0..=32usize).map(|l| rng.ascii(l)).collect();
    // make sure they are distinct even for equal lengths
    keys.dedup();
    keys.extend(distinct_keys(&mut rng, 167, 1, 32));
    str_table_run(
        "str_len_sweep",
        Some(STBDS_SH_STRDUP),
        2,
        true,
        &keys,
        DEFAULT_SEED,
        0x7580,
        3,
        true,
    );
}

#[test]
fn cfg_str_arena_block_boundary() {
    // Row 52: arena keys that cross the 512 byte blocksize and the oversized
    // dedicated-block path, inside the hash index's arena.
    let mut rng = Rng::new(0x7600);
    let mut keys = Vec::new();
    for &l in &[1usize, 400, 500, 510, 511, 512, 513, 600, 1024, 2000] {
        for _ in 0..6 {
            keys.push(rng.ascii(l));
        }
    }
    str_table_run(
        "str_arena_boundary",
        Some(STBDS_SH_ARENA),
        3,
        true,
        &keys,
        DEFAULT_SEED,
        0x7680,
        1,
        true,
    );
}

#[test]
fn cfg_str_duplicates_temp_key() {
    // Row 53: duplicate keys.  The forward bucket scan refreshes
    // `stbds_temp_key`, the wrap-around scan does not — both are compared.
    for &shmode in &[STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for s in 0..4u64 {
            seed_both(DEFAULT_SEED ^ s as usize);
            let mut rng = Rng::new(0x7700 + s + shmode as u64 * 101);
            let mut p = Pair::new(format!("str_dup sh={shmode} s={s}"), str_cfg(16));
            p.shmode(shmode);
            let keys = distinct_keys(&mut rng, 32, 1, 12);
            let mut ptrs = Vec::new();
            for kb in &keys {
                let k = p.intern_cstr(kb) as *mut c_void;
                ptrs.push(k);
                p.put(k, STBDS_HM_STRING, rng.next_u64());
            }
            p.check("populated");
            // Re-put every key twice; also re-put with a *different* buffer
            // holding the same bytes (so SH_DEFAULT must keep the original).
            for round in 0..2 {
                for (i, kb) in keys.iter().enumerate() {
                    let t1 = p.put(ptrs[i], STBDS_HM_STRING, rng.next_u64());
                    assert_eq!(t1, i as isize, "dup same pointer");
                    p.check(&format!("dup same round={round} i={i}"));
                    let alias = p.intern_cstr(kb) as *mut c_void;
                    let t2 = p.put(alias, STBDS_HM_STRING, rng.next_u64());
                    assert_eq!(t2, i as isize, "dup aliased pointer");
                    p.check(&format!("dup alias round={round} i={i}"));
                }
            }
            p.free();
        }
    }
}

#[test]
fn cfg_str_large_random() {
    // Row 54: 1000 random keys -> the index grows 8 -> 2048 with string rehashes.
    for s in 0..3u64 {
        let mut rng = Rng::new(0x7800 + s);
        let keys = distinct_keys(&mut rng, 1000, 1, 24);
        str_table_run(
            &format!("str_1000 s={s}"),
            Some(STBDS_SH_STRDUP),
            2,
            true,
            &keys,
            DEFAULT_SEED ^ s as usize,
            0x7880 + s,
            41,
            true,
        );
    }
}

#[test]
fn cfg_str_common_prefix() {
    // Row 60: long shared prefixes -> `hash_string` works hard and `strcmp`
    // does the disambiguation.
    let mut keys = Vec::new();
    for i in 0..256 {
        keys.push(format!("kkkkkkkkkkkkkkkkkkkkkkkkkkkk{i:08}").into_bytes());
    }
    for &sh in &[STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        str_table_run(
            &format!("str_prefix sh={sh}"),
            Some(sh),
            sh as u8,
            true,
            &keys,
            DEFAULT_SEED,
            0x7900 + sh as u64,
            13,
            true,
        );
    }
    // the memcpy (switch default) modes: inserts only, see str_table_run docs
    for &sh in &[STBDS_SH_NONE, 4, 255] {
        str_table_run(
            &format!("str_prefix memcpy sh={sh}"),
            Some(sh),
            sh as u8,
            false,
            &keys,
            DEFAULT_SEED,
            0x7940 + sh as u64,
            13,
            false,
        );
    }
}

#[test]
fn cfg_str_high_byte_keys() {
    // Row 61: keys made of bytes >= 0x80.
    let mut rng = Rng::new(0x7A00);
    let mut keys = Vec::new();
    let mut seen = std::collections::HashSet::new();
    while keys.len() < 128 {
        let len = rng.range(1, 20);
        let k: Vec<u8> = (0..len).map(|_| 0x80 | (rng.next_u64() as u8 & 0x7f)).collect();
        if seen.insert(k.clone()) {
            keys.push(k);
        }
    }
    for &sh in &[STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        str_table_run(
            &format!("str_high sh={sh}"),
            Some(sh),
            sh as u8,
            true,
            &keys,
            DEFAULT_SEED,
            0x7A80 + sh as u64,
            7,
            true,
        );
    }
}

// ===========================================================================
// Rows 55-59 — string deletions
// ===========================================================================

fn str_delete_run(label: &str, shmode: c_int, n: usize, rng_seed: u64) {
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(rng_seed);
    let mut p = Pair::new(label.to_string(), str_cfg(16));
    p.shmode(shmode);
    let keys = distinct_keys(&mut rng, n, 0, 20);
    let mut ptrs = Vec::new();
    for kb in &keys {
        let k = p.intern_cstr(kb) as *mut c_void;
        ptrs.push(k);
        assert!(p.put(k, STBDS_HM_STRING, rng.next_u64()) >= 0);
    }
    p.check("populated");
    // delete every other key, then the rest, checking the survivors each time
    let mut order: Vec<usize> = (0..n).step_by(2).collect();
    order.extend((1..n).step_by(2));
    let mut deleted = std::collections::HashSet::new();
    for (step, &i) in order.iter().enumerate() {
        assert_eq!(p.del(ptrs[i], 0, STBDS_HM_STRING), 1, "{label} del {i}");
        deleted.insert(i);
        p.check(&format!("{label} del step {step}"));
        assert_eq!(p.get(ptrs[i], STBDS_HM_STRING), -1);
        p.check(&format!("{label} get-after-del step {step}"));
    }
    assert_eq!(p.snapshot(0).length, 1);
    p.check("emptied");
    p.free();
}

#[test]
fn cfg_str_del_strdup() {
    // Row 55
    for s in 0..4u64 {
        str_delete_run(&format!("str_del_strdup s={s}"), STBDS_SH_STRDUP, 32, 0x7B00 + s);
    }
}

#[test]
fn cfg_str_del_arena() {
    // Row 56
    for s in 0..4u64 {
        str_delete_run(&format!("str_del_arena s={s}"), STBDS_SH_ARENA, 32, 0x7C00 + s);
    }
}

#[test]
fn cfg_str_del_default() {
    // Row 57
    for s in 0..4u64 {
        str_delete_run(
            &format!("str_del_default s={s}"),
            STBDS_SH_DEFAULT,
            32,
            0x7D00 + s,
        );
    }
}

#[test]
fn cfg_str_del_then_reinsert() {
    // Row 58: delete-all then re-insert -> tombstone reuse on the string path.
    for &shmode in &[STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for s in 0..4u64 {
            seed_both(DEFAULT_SEED ^ s as usize);
            let mut rng = Rng::new(0x7E00 + s + shmode as u64 * 37);
            let mut p = Pair::new(format!("str_reinsert sh={shmode} s={s}"), str_cfg(16));
            p.shmode(shmode);
            let keys = distinct_keys(&mut rng, 16, 1, 16);
            for cycle in 0..3 {
                let mut ptrs = Vec::new();
                for kb in &keys {
                    let k = p.intern_cstr(kb) as *mut c_void;
                    ptrs.push(k);
                    assert!(p.put(k, STBDS_HM_STRING, rng.next_u64()) >= 0);
                    p.check(&format!("reinsert cycle={cycle} put"));
                }
                for (i, &k) in ptrs.iter().enumerate() {
                    assert_eq!(p.del(k, 0, STBDS_HM_STRING), 1);
                    p.check(&format!("reinsert cycle={cycle} del {i}"));
                }
            }
            p.free();
        }
    }
}

#[test]
fn cfg_str_random_op_stream() {
    // Row 59: property test — random put / get / get_ts / del streams on a
    // strdup table, full state compared after every operation.
    for s in 0..2u64 {
        seed_both(DEFAULT_SEED ^ (s as usize * 104729));
        let mut rng = Rng::new(0x7F00 + s);
        let mut p = Pair::new(format!("str_stream s={s}"), str_cfg(16));
        p.shmode(STBDS_SH_STRDUP);
        let mut live: Vec<*mut c_void> = Vec::new();
        let mut dead: Vec<*mut c_void> = Vec::new();
        let mut used = std::collections::HashSet::new();
        for op in 0..2000 {
            match rng.below(10) {
                0..=4 => {
                    let len = rng.range(0, 20);
                    let kb = rng.ascii(len);
                    if !used.insert(kb.clone()) {
                        continue;
                    }
                    let k = p.intern_cstr(&kb) as *mut c_void;
                    assert!(p.put(k, STBDS_HM_STRING, rng.next_u64()) >= 0);
                    live.push(k);
                }
                5..=6 => {
                    if !live.is_empty() {
                        let k = live[rng.below(live.len())];
                        assert!(p.get(k, STBDS_HM_STRING) >= 0);
                    }
                }
                7 => {
                    if !dead.is_empty() {
                        let k = dead[rng.below(dead.len())];
                        assert_eq!(p.get_ts(k, STBDS_HM_STRING), -1);
                    }
                }
                8 => {
                    if !live.is_empty() {
                        let idx = rng.below(live.len());
                        let k = live.swap_remove(idx);
                        assert_eq!(p.del(k, 0, STBDS_HM_STRING), 1);
                        dead.push(k);
                    }
                }
                _ => {
                    if !dead.is_empty() {
                        let k = dead[rng.below(dead.len())];
                        assert_eq!(p.del(k, 0, STBDS_HM_STRING), 0);
                    }
                }
            }
            p.check(&format!("str stream op {op}"));
        }
        p.free();
    }
}

// ===========================================================================
// Rows 62-63 — out-of-range `mode` matrices
// ===========================================================================

const MODES: [c_int; 11] = [-2, -1, 0, 1, 2, 3, 7, 255, 999, i32::MIN, i32::MAX];

#[test]
fn cfg_mode_matrix() {
    // Row 62: every entry point with every out-of-range `mode`.
    //
    // `mode >= 1` selects string hashing/compare, `mode < 1` selects binary.
    // `hmdel_key` however compares `mode == 1` *exactly* (C L836/L842), so for
    // `mode >= 2` the re-find after a swap takes the binary branch and asserts;
    // that abort is covered by ERRORS.md rows 50/51 in a child process, so here
    // only the last element is deleted (old_index == final_index, no re-find).
    for &mode in &MODES {
        let string_shaped = mode >= STBDS_HM_STRING;
        for s in 0..32u64 {
            seed_both(DEFAULT_SEED ^ s as usize);
            let mut rng = Rng::new(
                0x8000u64
                    .wrapping_add(s)
                    .wrapping_add((mode as i64 as u64).wrapping_mul(7)),
            );
            let cfg = if string_shaped {
                str_cfg(16)
            } else {
                bin_cfg(16, 8)
            };
            let mut p = Pair::new(format!("mode={mode} s={s}"), cfg);
            let mut ptrs = Vec::new();
            for i in 0..6 {
                let kb = dual_key(&mut rng);
                let k = if string_shaped {
                    p.intern_cstr(&kb[..7]) as *mut c_void
                } else {
                    intern_key(&mut p, &kb)
                };
                ptrs.push(k);
                let t = p.put(k, mode, rng.next_u64());
                assert!(t >= 0, "mode={mode} put {i} -> {t}");
                p.check(&format!("mode={mode} put {i}"));
            }
            assert_eq!(
                p.string_mode(0),
                Some(if string_shaped { 1 } else { 0 }),
                "bootstrap string.mode for mode={mode}"
            );
            for (i, &k) in ptrs.iter().enumerate() {
                assert!(p.get(k, mode) >= 0, "mode={mode} get {i}");
                p.check(&format!("mode={mode} get {i}"));
                assert!(p.get_ts(k, mode) >= 0);
                p.check(&format!("mode={mode} get_ts {i}"));
            }
            // delete from the top so old_index == final_index every time
            for i in (0..ptrs.len()).rev() {
                let t = p.del(ptrs[i], 0, mode);
                assert_eq!(t, 1, "mode={mode} del {i}");
                p.check(&format!("mode={mode} del {i}"));
            }
            assert_eq!(p.del(ptrs[0], 0, mode), 0, "mode={mode} del missing");
            p.check(&format!("mode={mode} del missing"));
            p.free();
        }
    }
}

#[test]
fn cfg_mode_mismatch_lookup() {
    // Row 62 (continued): look a *binary* table up with a string mode and vice
    // versa.  The two hash functions disagree, so `find_slot` reaches an EMPTY
    // slot and reports -1 without ever dereferencing a bogus key pointer.
    for s in 0..16u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0x8100 + s);
        // binary table, string lookups
        let mut p = Pair::new(format!("mismatch bin s={s}"), bin_cfg(16, 8));
        let mut ptrs = Vec::new();
        for _ in 0..8 {
            let kb = dual_key(&mut rng);
            let k = intern_key(&mut p, &kb);
            ptrs.push(k);
            p.put(k, STBDS_HM_BINARY, rng.next_u64());
        }
        p.check("binary populated");
        for &k in &ptrs {
            // NOTE: a *hash* collision here would make the C dereference the
            // element's first 8 bytes as a char* — astronomically unlikely.
            assert_eq!(p.get(k, STBDS_HM_STRING), -1, "string lookup in binary table");
            p.check("binary table, string lookup");
        }
        p.free();

        // string table, binary lookups
        let mut q = Pair::new(format!("mismatch str s={s}"), str_cfg(16));
        let mut qptrs = Vec::new();
        for _ in 0..8 {
            let kb = dual_key(&mut rng);
            let k = q.intern_cstr(&kb[..7]) as *mut c_void;
            qptrs.push(k);
            q.put(k, STBDS_HM_STRING, rng.next_u64());
        }
        q.check("string populated");
        for &k in &qptrs {
            assert_eq!(q.get(k, STBDS_HM_BINARY), -1, "binary lookup in string table");
            q.check("string table, binary lookup");
        }
        q.free();
    }
}

#[test]
fn cfg_shmode_matrix() {
    // Row 63: stbds_shmode_func truncates `mode` to `unsigned char`.
    let cases: [(c_int, u8); 11] = [
        (0, 0),
        (1, 1),
        (2, 2),
        (3, 3),
        (4, 4),
        (255, 255),
        (256, 0),
        (257, 1),
        (-1, 255),
        (i32::MIN, 0),
        (i32::MAX, 255),
    ];
    for &(mode, expect) in &cases {
        for s in 0..8u64 {
            seed_both(DEFAULT_SEED ^ s as usize);
            let mut rng = Rng::new(0x8200u64.wrapping_add(s).wrapping_add(mode as i64 as u64));
            // string.mode 1/2/3 keep a real `char *` in the element; every other
            // value falls into the `switch` default and memcpy's the key *text*
            // there — see `str_table_run` for why lookups are then fatal.
            let key_is_ptr = matches!(expect, 1 | 2 | 3);
            let cfg = if key_is_ptr { str_cfg(16) } else { bin_cfg(16, 8) };
            let mut p = Pair::new(format!("shmode={mode} s={s}"), cfg);
            p.shmode(mode);
            assert_eq!(
                p.string_mode(0),
                Some(expect),
                "shmode_func({mode}) -> (unsigned char) truncation"
            );
            assert_eq!(p.string_mode(1), Some(expect));
            p.check(&format!("shmode={mode} fresh"));
            let keys = distinct_keys(&mut rng, 8, 8, 16);
            let mut ptrs = Vec::new();
            for kb in &keys {
                let k = p.intern_cstr(kb) as *mut c_void;
                ptrs.push(k);
                assert!(p.put(k, STBDS_HM_STRING, rng.next_u64()) >= 0);
                p.check(&format!("shmode={mode} put"));
            }
            if key_is_ptr {
                for &k in &ptrs {
                    assert!(p.get(k, STBDS_HM_STRING) >= 0);
                }
                p.check(&format!("shmode={mode} gets"));
                for i in (0..ptrs.len()).rev() {
                    assert_eq!(p.del(ptrs[i], 0, STBDS_HM_STRING), 1);
                    p.check(&format!("shmode={mode} del {i}"));
                }
            }
            p.free();
        }
    }
}

#[test]
fn cfg_shmode_elemsize_matrix() {
    // Row 69: shmode_func over elemsize x mode.
    for &elemsize in &[8usize, 12, 16, 24, 32] {
        for mode in 0..4i32 {
            seed_both(DEFAULT_SEED);
            let key_is_ptr = matches!(mode, 1 | 2 | 3) && elemsize >= 16;
            let cfg = if key_is_ptr {
                str_cfg(elemsize)
            } else {
                bin_cfg(elemsize, 8.min(elemsize))
            };
            let mut p = Pair::new(format!("shmode_es es={elemsize} m={mode}"), cfg);
            p.shmode(mode);
            p.check(&format!("shmode_func({elemsize},{mode})"));
            assert_eq!(p.snapshot(0).length, 1, "sentinel only");
            assert!(p.snapshot(0).has_table, "index must exist immediately");
            p.free();
        }
    }
}

// ===========================================================================
// Rows 64-65 — the global hash seed
// ===========================================================================

#[test]
fn cfg_seed_sweep_workload() {
    // Row 64: the same workload under many `stbds_rand_seed` values — the
    // per-table seed, every bucket hash and the LCG advance must all match.
    let mut meta = Rng::new(0x9000);
    let mut seeds: Vec<usize> = vec![0, 1, 2, DEFAULT_SEED, usize::MAX, usize::MAX - 1];
    for _ in 0..8 {
        seeds.push(meta.next_usize());
    }
    for &hs in &seeds {
        seed_both(hs);
        let mut rng = Rng::new(0x9100 ^ hs as u64);
        let mut p = Pair::new(format!("seed={hs:#x}"), bin_cfg(16, 8));
        let mut keys = Vec::new();
        for i in 0..200 {
            let kb = rng.bytes(8);
            let k = intern_key(&mut p, &kb);
            keys.push(k);
            assert!(p.put(k, STBDS_HM_BINARY, rng.next_u64()) >= 0);
            if i % 11 == 0 {
                p.check(&format!("seed={hs:#x} put {i}"));
            }
        }
        p.check(&format!("seed={hs:#x} populated"));
        assert_eq!(
            p.snapshot(0).table.as_ref().unwrap().seed,
            p.snapshot(1).table.as_ref().unwrap().seed
        );
        for (i, &k) in keys.iter().enumerate() {
            assert!(p.get(k, STBDS_HM_BINARY) >= 0, "seed={hs:#x} key {i}");
        }
        for i in 0..100 {
            assert_eq!(p.del(keys[i], 0, STBDS_HM_BINARY), 1);
            if i % 11 == 0 {
                p.check(&format!("seed={hs:#x} del {i}"));
            }
        }
        p.check(&format!("seed={hs:#x} after deletes"));
        p.free();
    }
}

#[test]
fn cfg_seed_lcg_sequence() {
    // Row 65: build many tables in a row *without* re-seeding — every
    // `stbds_make_hash_index(_, NULL)` advances the global seed by
    // `seed = seed*A + B`, so the Nth table's seed depends on the whole history.
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0x9200);
    let mut seeds_c = Vec::new();
    let mut seeds_r = Vec::new();
    for round in 0..32 {
        let mut p = Pair::new(format!("lcg round={round}"), bin_cfg(16, 8));
        let kb = rng.bytes(8);
        let k = intern_key(&mut p, &kb);
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
        p.check(&format!("lcg round={round}"));
        seeds_c.push(p.snapshot(0).table.as_ref().unwrap().seed);
        seeds_r.push(p.snapshot(1).table.as_ref().unwrap().seed);
        p.free();
    }
    assert_eq!(seeds_c, seeds_r, "per-table seed sequence must match");
    // and the sequence must actually be advancing
    assert!(
        seeds_c.windows(2).all(|w| w[0] != w[1]),
        "the LCG should produce a fresh seed per table: {seeds_c:?}"
    );
    // sh_new_* / rehash paths advance it too
    seed_both(12345);
    let mut sh_c = Vec::new();
    let mut sh_r = Vec::new();
    for round in 0..16 {
        let mut p = Pair::new(format!("lcg sh round={round}"), str_cfg(16));
        p.shmode(STBDS_SH_STRDUP);
        p.check(&format!("lcg sh round={round}"));
        sh_c.push(p.snapshot(0).table.as_ref().unwrap().seed);
        sh_r.push(p.snapshot(1).table.as_ref().unwrap().seed);
        p.free();
    }
    assert_eq!(sh_c, sh_r);
}

// ===========================================================================
// Rows 66-68 — hmput_default / hmfree_func
// ===========================================================================

#[test]
fn cfg_hmput_default_matrix() {
    // Row 66: hmput_default on NULL / fresh / populated, then a lookup (which
    // takes the `hash_table == NULL` branch on a default-only map).
    for &elemsize in &[8usize, 16, 24] {
        for stage in 0..3 {
            seed_both(DEFAULT_SEED);
            let mut rng = Rng::new(0x9300 + elemsize as u64 * 7 + stage);
            let mut p = Pair::new(
                format!("hmput_default es={elemsize} stage={stage}"),
                bin_cfg(elemsize, 8.min(elemsize)),
            );
            p.put_default();
            p.check("after first hmput_default");
            assert_eq!(p.snapshot(0).length, 1);
            assert!(!p.snapshot(0).has_table, "no index yet");
            if stage >= 1 {
                // idempotent: length > 0 so the pointer comes back unchanged
                let before = p.t;
                p.put_default();
                assert_eq!(before, p.t, "hmput_default must be idempotent");
                p.check("after second hmput_default");
            }
            if stage >= 2 {
                // a lookup on a table-less map -> temp == -1, no hashing at all
                let kb = rng.bytes(8.min(elemsize));
                let k = intern_key(&mut p, &kb);
                assert_eq!(p.get(k, STBDS_HM_BINARY), -1);
                p.check("get on table-less map");
                assert_eq!(p.get_ts(k, STBDS_HM_BINARY), -1);
                p.check("get_ts on table-less map");
                assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 0);
                p.check("del on table-less map");
            }
            p.free();
        }
    }
}

#[test]
fn cfg_hmput_default_then_inserts() {
    // Row 67: the default (sentinel) element must survive later inserts.
    for s in 0..32u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0x9400 + s);
        let mut p = Pair::new(format!("default+insert s={s}"), bin_cfg(16, 8));
        p.put_default();
        // write the "default value" the way `hmdefault` does: t[-1].value = v
        let dv = rng.bytes(8);
        for i in 0..2 {
            unsafe {
                let e = (p.t[i] as *mut u8).sub(16).add(8);
                std::ptr::copy_nonoverlapping(dv.as_ptr(), e, 8);
            }
        }
        p.check("default written");
        let mut keys = Vec::new();
        for i in 0..12 {
            let kb = rng.bytes(8);
            let k = intern_key(&mut p, &kb);
            keys.push(k);
            assert!(p.put(k, STBDS_HM_BINARY, rng.next_u64()) >= 0);
            p.check(&format!("default+insert put {i}"));
        }
        // the sentinel is raw element 0 and is part of every snapshot already
        for &k in &keys {
            assert!(p.get(k, STBDS_HM_BINARY) >= 0);
        }
        p.check("default+insert gets");
        p.free();
    }
}

#[test]
fn cfg_hmfree_matrix() {
    // Row 68: teardown over string.mode x element count x with/without index.
    // Any divergence in the free() *sequence* (double free, freeing a
    // non-strdup key, forgetting the arena chain) makes glibc abort the process,
    // so surviving this loop with identical snapshots up to the free is the
    // observable contract.
    for &shmode in &[-1i32, 0, 1, 2, 3, 255] {
        for &n in &[0usize, 1, 5, 100] {
            for with_index in [true, false] {
                seed_both(DEFAULT_SEED);
                let mut rng = Rng::new(
                    0x9500u64
                        .wrapping_add(n as u64)
                        .wrapping_add((shmode as i64 as u64).wrapping_mul(13)),
                );
                let smode = shmode as i64 as u8;
                let key_is_ptr = with_index && matches!(smode, 1 | 2 | 3);
                let cfg = if key_is_ptr {
                    str_cfg(16)
                } else {
                    bin_cfg(16, 8)
                };
                let mut p = Pair::new(format!("hmfree sh={shmode} n={n} idx={with_index}"), cfg);
                if with_index {
                    p.shmode(shmode);
                } else {
                    p.put_default();
                }
                if with_index {
                    // >= 8 byte keys: for the memcpy (switch default) string
                    // modes the 8 byte copy must stay inside the key buffer.
                    let keys = distinct_keys(&mut rng, n, 8, 20);
                    for kb in &keys {
                        let k = p.intern_cstr(kb) as *mut c_void;
                        assert!(p.put(k, STBDS_HM_STRING, rng.next_u64()) >= 0);
                    }
                }
                p.check(&format!("hmfree sh={shmode} n={n} idx={with_index} before free"));
                p.free();
                p.check("after free (both NULL)");
            }
        }
    }
}

// ===========================================================================
// Rows 70-72 — composed pipelines
// ===========================================================================

#[test]
fn cfg_pipeline_strdup() {
    // Row 70: shmode(STRDUP) -> 500 put -> 250 del -> 250 put -> get all -> free
    for s in 0..4u64 {
        seed_both(DEFAULT_SEED ^ (s as usize * 31337));
        let mut rng = Rng::new(0xA000 + s);
        let mut p = Pair::new(format!("pipe_strdup s={s}"), str_cfg(24));
        p.shmode(STBDS_SH_STRDUP);
        let keys = distinct_keys(&mut rng, 750, 0, 28);
        let mut ptrs: Vec<*mut c_void> = Vec::new();
        for kb in &keys[..500] {
            let k = p.intern_cstr(kb) as *mut c_void;
            ptrs.push(k);
            assert!(p.put(k, STBDS_HM_STRING, rng.next_u64()) >= 0);
        }
        p.check("500 inserted");
        for i in 0..250 {
            assert_eq!(p.del(ptrs[i * 2], 0, STBDS_HM_STRING), 1);
            if i % 23 == 0 {
                p.check(&format!("pipe del {i}"));
            }
        }
        p.check("250 deleted");
        for kb in &keys[500..] {
            let k = p.intern_cstr(kb) as *mut c_void;
            ptrs.push(k);
            assert!(p.put(k, STBDS_HM_STRING, rng.next_u64()) >= 0);
        }
        p.check("250 re-inserted");
        for i in 0..500 {
            let expect_present = i % 2 == 1;
            let t = p.get(ptrs[i], STBDS_HM_STRING);
            assert_eq!(t >= 0, expect_present, "pipeline key {i}");
        }
        for i in 500..750 {
            assert!(p.get(ptrs[i], STBDS_HM_STRING) >= 0);
        }
        p.check("pipeline lookups");
        p.free();
    }
}

#[test]
fn cfg_pipeline_binary() {
    // Row 71: binary map: 500 put -> 400 del (shrink + rebuild) -> 300 put ->
    // get all -> free
    for s in 0..4u64 {
        seed_both(DEFAULT_SEED ^ (s as usize * 65599));
        let mut rng = Rng::new(0xB000 + s);
        let mut p = Pair::new(format!("pipe_bin s={s}"), bin_cfg(16, 8));
        let mut ptrs = Vec::new();
        for _ in 0..500 {
            let kb = rng.bytes(8);
            let k = intern_key(&mut p, &kb);
            ptrs.push(k);
            assert!(p.put(k, STBDS_HM_BINARY, rng.next_u64()) >= 0);
        }
        p.check("500 inserted");
        for i in 0..400 {
            assert_eq!(p.del(ptrs[i], 0, STBDS_HM_BINARY), 1);
            if i % 29 == 0 {
                p.check(&format!("pipe_bin del {i}"));
            }
        }
        p.check("400 deleted");
        for _ in 0..300 {
            let kb = rng.bytes(8);
            let k = intern_key(&mut p, &kb);
            ptrs.push(k);
            assert!(p.put(k, STBDS_HM_BINARY, rng.next_u64()) >= 0);
        }
        p.check("300 re-inserted");
        for i in 0..400 {
            assert_eq!(p.get(ptrs[i], STBDS_HM_BINARY), -1, "deleted key {i}");
        }
        for i in 400..ptrs.len() {
            assert!(p.get(ptrs[i], STBDS_HM_BINARY) >= 0, "live key {i}");
        }
        p.check("pipe_bin lookups");
        p.free();
    }
}

#[test]
fn cfg_pipeline_arena() {
    // Row 72: arena-backed string map with key lengths that cross the arena
    // blocksize, then deletes, then teardown (which walks the block chain).
    for s in 0..2u64 {
        seed_both(DEFAULT_SEED ^ (s as usize * 7));
        let mut rng = Rng::new(0xC000 + s);
        let mut p = Pair::new(format!("pipe_arena s={s}"), str_cfg(16));
        p.shmode(STBDS_SH_ARENA);
        let mut ptrs = Vec::new();
        let mut used = std::collections::HashSet::new();
        while ptrs.len() < 400 {
            let len = match rng.below(8) {
                0 => rng.range(500, 700),
                1 => rng.range(1, 4),
                _ => rng.range(4, 60),
            };
            let kb = rng.ascii(len);
            if !used.insert(kb.clone()) {
                continue;
            }
            let k = p.intern_cstr(&kb) as *mut c_void;
            ptrs.push(k);
            assert!(p.put(k, STBDS_HM_STRING, rng.next_u64()) >= 0);
            if ptrs.len() % 19 == 0 {
                p.check(&format!("pipe_arena put {}", ptrs.len()));
            }
        }
        p.check("arena populated");
        for i in 0..200 {
            assert_eq!(p.del(ptrs[i * 2], 0, STBDS_HM_STRING), 1);
            if i % 17 == 0 {
                p.check(&format!("pipe_arena del {i}"));
            }
        }
        p.check("arena deletes done");
        for i in 0..400 {
            let expect = i % 2 == 1;
            assert_eq!(p.get(ptrs[i], STBDS_HM_STRING) >= 0, expect);
        }
        p.check("arena lookups");
        p.free();
    }
}
