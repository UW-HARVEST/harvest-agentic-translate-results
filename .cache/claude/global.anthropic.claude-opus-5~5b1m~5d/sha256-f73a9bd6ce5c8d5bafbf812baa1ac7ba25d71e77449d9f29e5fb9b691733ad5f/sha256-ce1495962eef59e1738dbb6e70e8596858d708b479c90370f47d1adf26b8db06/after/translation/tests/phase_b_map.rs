//! Phase B — CONFIGS.md rows 15-30: binary-keyed hash maps, driven through the
//! low-level `stbds_hm*_key` entry points exactly as the `hmput`/`hmget`/
//! `hmdel`/`hmdefault`/`hmfree` macros do.

mod common;

use common::*;

/// little-endian key bytes of the requested width
fn k(v: u64, keysize: usize) -> Vec<u8> {
    let b = v.to_le_bytes();
    let mut out = vec![0u8; keysize];
    for i in 0..keysize.min(8) {
        out[i] = b[i];
    }
    out
}

/// payload derived from the value
fn pay(v: u64) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

/// row 15 — `stbds_hmput_default` on NULL, on a fresh default map (no-op path)
/// and on a map created by `hmget_key_ts(NULL, ..)`.
#[test]
fn put_default_paths() {
    for &(es, ks) in &[(8usize, 4usize), (16, 8), (4, 4), (1, 1), (12, 4)] {
        let cfg = MapCfg::binary(es, ks);
        diff_script(
            "put_default: NULL then no-op then again",
            DEFAULT_SEED,
            cfg,
            &[
                Op::PutDefault { payload: pay(0xdead_beef) },
                Op::PutDefault { payload: pay(0x1234_5678) },
                Op::PutDefault { payload: pay(0x1111_2222) },
                get(&k(1, ks)),
                Op::PutDefault { payload: pay(0x3333_4444) },
                Op::Free,
            ],
        );
        diff_script(
            "put_default after get_key_ts(NULL)",
            DEFAULT_SEED,
            cfg,
            &[
                get_ts(&k(7, ks)),
                Op::PutDefault { payload: pay(0xaaaa_bbbb) },
                get_ts(&k(7, ks)),
                put(&k(7, ks), &pay(9)),
                Op::PutDefault { payload: pay(0xcccc_dddd) },
                get(&k(7, ks)),
                Op::Free,
            ],
        );
    }
}

/// row 16 — get/get_ts on a map that has no hash table at all
#[test]
fn get_without_table() {
    let cfg = MapCfg::binary(8, 4);
    diff_script(
        "get without table",
        DEFAULT_SEED,
        cfg,
        &[
            Op::PutDefault { payload: pay(0x2222) },
            get(&k(0, 4)),
            get_ts(&k(0, 4)),
            get(&k(0xffff_ffff, 4)),
            get_ts(&k(1, 4)),
            del(&k(1, 4)),
            Op::Free,
        ],
    );
    // and the same starting from hmget_key(NULL) (which allocates but leaves
    // hash_table == NULL)
    diff_script(
        "get(NULL) then get again",
        DEFAULT_SEED,
        cfg,
        &[get(&k(1, 4)), get(&k(1, 4)), get_ts(&k(2, 4)), del(&k(1, 4)), Op::Free],
    );
}

/// row 17 — int→int map, element counts around `used_count_threshold` (6 for
/// `slot_count == 8`, so the 7th insert grows the table to 16)
#[test]
fn binary_int_counts() {
    let ks = 4;
    let cfg = MapCfg::binary(8, ks);
    for &count in &[0usize, 1, 2, 5, 6, 7, 8, 12, 13, 20] {
        for seed in [DEFAULT_SEED, 0, 1, usize::MAX] {
            let mut ops = vec![Op::PutDefault { payload: pay(0xffff_ffff_ffff_fffe) }];
            for i in 0..count {
                ops.push(put(&k(i as u64 * 2, ks), &pay(i as u64 * 5)));
            }
            for i in 0..(count * 2 + 2) {
                ops.push(get(&k(i as u64, ks)));
                ops.push(get_ts(&k(i as u64, ks)));
            }
            ops.push(Op::Free);
            diff_script(&format!("binary count={count}"), seed, cfg, &ops);
        }
    }
}

/// row 18 — many keys: repeated table growth 8→16→32→…
#[test]
fn binary_int_many() {
    let ks = 4;
    for &count in &[100usize, 1000] {
        let cfg = MapCfg::binary(8, ks).digested();
        let mut rng = Rng::new(0xC0FFEE ^ count as u64);
        let mut ops = vec![Op::PutDefault { payload: pay(0xdeadbeef) }];
        let mut keys = Vec::new();
        for i in 0..count {
            let key = rng.next_u64() & 0xffff;
            keys.push(key);
            ops.push(put(&k(key, ks), &pay(i as u64)));
        }
        for kk in &keys {
            ops.push(get(&k(*kk, ks)));
        }
        for i in 0..count {
            ops.push(get_ts(&k(i as u64, ks)));
        }
        ops.push(Op::Free);
        diff_script(&format!("binary many={count}"), DEFAULT_SEED, cfg, &ops);
    }
}

/// row 19 — `elemsize=16`, `keysize=8`
#[test]
fn binary_e16_k8() {
    let cfg = MapCfg::binary(16, 8);
    let mut rng = Rng::new(0x1616_0808);
    for seed in [DEFAULT_SEED, 0, 7] {
        let mut ops = vec![Op::PutDefault { payload: pay(0x9999) }];
        for i in 0..40u64 {
            let key = rng.next_u64();
            ops.push(put(&k(key, 8), &pay(i)));
            ops.push(get(&k(key, 8)));
        }
        ops.push(Op::Free);
        diff_script("binary e16 k8", seed, cfg, &ops);
    }
}

/// row 20 — `elemsize=16`, `keysize=4` (12-byte payload, keysize < elemsize)
#[test]
fn binary_e16_k4() {
    let cfg = MapCfg::binary(16, 4);
    let mut rng = Rng::new(0x1616_0404);
    for seed in [DEFAULT_SEED, 3] {
        let mut ops = vec![Op::PutDefault { payload: pay(0x8888) }];
        for i in 0..40u64 {
            let key = rng.below(30);
            ops.push(put(&k(key, 4), &pay(i.wrapping_mul(0x0101_0101))));
            ops.push(get_ts(&k(key, 4)));
        }
        for i in 0..30 {
            ops.push(del(&k(i, 4)));
            ops.push(get(&k(i, 4)));
        }
        ops.push(Op::Free);
        diff_script("binary e16 k4", seed, cfg, &ops);
    }
}

/// row 21 — odd element/key sizes
#[test]
fn binary_odd_shapes() {
    for &(es, ks) in &[(1usize, 1usize), (3, 3), (5, 2), (2, 1), (64, 16), (12, 3), (7, 5)] {
        let cfg = MapCfg::binary(es, ks);
        let mut rng = Rng::new(0x0DD5_4A9E ^ ((es as u64) << 8) ^ ks as u64);
        let mut ops = vec![Op::PutDefault { payload: pay(0x55) }];
        for i in 0..25u64 {
            let key = rng.below(16);
            ops.push(put(&k(key, ks), &pay(i)));
        }
        for i in 0..16u64 {
            ops.push(get(&k(i, ks)));
        }
        for i in 0..8u64 {
            ops.push(del(&k(i, ks)));
        }
        for i in 0..16u64 {
            ops.push(get_ts(&k(i, ks)));
        }
        ops.push(Op::Free);
        diff_script(&format!("odd shape es={es} ks={ks}"), DEFAULT_SEED, cfg, &ops);
    }
}

/// row 22 — duplicate keys: update path (existing index returned, `used_count`
/// unchanged)
#[test]
fn binary_duplicates() {
    let cfg = MapCfg::binary(8, 4);
    let mut ops = vec![Op::PutDefault { payload: pay(0x7777) }];
    for round in 0..4u64 {
        for i in 0..10u64 {
            ops.push(put(&k(i, 4), &pay(round * 100 + i)));
            ops.push(get(&k(i, 4)));
        }
    }
    ops.push(Op::Free);
    for seed in [DEFAULT_SEED, 0, 1, 0xabcd] {
        diff_script("binary duplicates", seed, cfg, &ops);
    }
}

/// row 23 — delete the last element (`old_index == final_index`, no memmove)
#[test]
fn binary_del_last() {
    let cfg = MapCfg::binary(8, 4);
    for n in [1usize, 2, 5, 6, 7, 10] {
        let mut ops = vec![Op::PutDefault { payload: pay(0x1) }];
        for i in 0..n as u64 {
            ops.push(put(&k(i, 4), &pay(i * 3)));
        }
        // delete in reverse insertion order: always the last element
        for i in (0..n as u64).rev() {
            ops.push(del(&k(i, 4)));
            ops.push(get(&k(i, 4)));
        }
        ops.push(Op::Free);
        diff_script(&format!("del last n={n}"), DEFAULT_SEED, cfg, &ops);
    }
}

/// row 24 — delete interior elements (`old_index != final_index`: memmove +
/// re-find + re-index of the moved element)
#[test]
fn binary_del_interior() {
    let cfg = MapCfg::binary(8, 4);
    for n in [2usize, 3, 6, 7, 12, 30] {
        let mut ops = vec![Op::PutDefault { payload: pay(0x2) }];
        for i in 0..n as u64 {
            ops.push(put(&k(i, 4), &pay(i * 7)));
        }
        // always delete the front element: forces the memmove/re-index path
        for i in 0..n as u64 {
            ops.push(del(&k(i, 4)));
            for j in 0..n as u64 {
                ops.push(get(&k(j, 4)));
            }
        }
        ops.push(Op::Free);
        diff_script(&format!("del interior n={n}"), DEFAULT_SEED, cfg, &ops);
    }
}

/// row 25 — `tombstone_count > tombstone_count_threshold` ⇒ rebuild at the same
/// `slot_count` (threshold is 1 for slot_count 8, 3 for 16, 6 for 32)
#[test]
fn binary_tombstone_rebuild() {
    let cfg = MapCfg::binary(8, 4);
    // slot_count stays 8: 5 inserts, then delete/insert churn
    let mut ops = vec![Op::PutDefault { payload: pay(0x3) }];
    for i in 0..5u64 {
        ops.push(put(&k(i, 4), &pay(i)));
    }
    for round in 0..6u64 {
        ops.push(del(&k(round % 5, 4)));
        ops.push(put(&k(100 + round, 4), &pay(round)));
        for j in 0..106u64 {
            ops.push(get_ts(&k(j, 4)));
        }
    }
    ops.push(Op::Free);
    diff_script("tombstone rebuild sc=8", DEFAULT_SEED, cfg, &ops);

    // slot_count 32: insert 25, then delete 8 (tombstone_threshold = 6)
    let mut ops = vec![Op::PutDefault { payload: pay(0x4) }];
    for i in 0..25u64 {
        ops.push(put(&k(i, 4), &pay(i)));
    }
    for i in 0..8u64 {
        ops.push(del(&k(i * 3, 4)));
    }
    for j in 0..30u64 {
        ops.push(get(&k(j, 4)));
    }
    ops.push(Op::Free);
    diff_script("tombstone rebuild sc=32", DEFAULT_SEED, cfg, &ops);
}

/// row 26 — `used_count < used_count_shrink_threshold && slot_count > 8` ⇒
/// shrink to `slot_count>>1`
#[test]
fn binary_shrink() {
    let cfg = MapCfg::binary(8, 4);
    for n in [7usize, 13, 25, 50] {
        let mut ops = vec![Op::PutDefault { payload: pay(0x5) }];
        for i in 0..n as u64 {
            ops.push(put(&k(i, 4), &pay(i)));
        }
        // delete everything: walks every shrink step back down to slot_count 8
        for i in 0..n as u64 {
            ops.push(del(&k(i, 4)));
            ops.push(get(&k(i, 4)));
        }
        for i in 0..n as u64 {
            ops.push(get_ts(&k(i, 4)));
        }
        ops.push(Op::Free);
        diff_script(&format!("shrink n={n}"), DEFAULT_SEED, cfg, &ops);
    }
}

/// row 27 — re-insert after deletes: the `tombstone >= 0` slot-reuse branch of
/// `stbds_hmput_key`
#[test]
fn binary_tombstone_reuse() {
    let cfg = MapCfg::binary(8, 4);
    for seed in [DEFAULT_SEED, 0, 0x999] {
        let mut ops = vec![Op::PutDefault { payload: pay(0x6) }];
        for i in 0..12u64 {
            ops.push(put(&k(i, 4), &pay(i)));
        }
        for i in 0..6u64 {
            ops.push(del(&k(i * 2, 4)));
        }
        // re-insert the deleted keys — must land in the tombstoned slots
        for i in 0..6u64 {
            ops.push(put(&k(i * 2, 4), &pay(1000 + i)));
            for j in 0..12u64 {
                ops.push(get(&k(j, 4)));
            }
        }
        ops.push(Op::Free);
        diff_script("tombstone reuse", seed, cfg, &ops);
    }
}

/// row 28 — randomised interleaved scripts (the property-style core of Phase B)
#[test]
fn binary_random_scripts() {
    let shapes = [(8usize, 4usize), (16, 8), (8, 8), (4, 4), (16, 4), (24, 3)];
    for trial in 0..40u64 {
        let (es, ks) = shapes[(trial as usize) % shapes.len()];
        let cfg = MapCfg::binary(es, ks).digested();
        let mut rng = Rng::new(0xD1CE_0000 + trial);
        let mut ops: Vec<Op> = Vec::new();
        if trial % 3 == 0 {
            ops.push(Op::PutDefault { payload: pay(0xfeed) });
        }
        for i in 0..400u64 {
            let key = rng.below(24);
            match rng.below(10) {
                0..=3 => ops.push(put(&k(key, ks), &pay(i))),
                4..=5 => ops.push(get(&k(key, ks))),
                6 => ops.push(get_ts(&k(key, ks))),
                7..=8 => ops.push(del(&k(key, ks))),
                _ => {
                    if rng.below(8) == 0 {
                        ops.push(Op::Free);
                    } else {
                        ops.push(Op::PutDefault { payload: pay(i) });
                    }
                }
            }
        }
        ops.push(Op::Free);
        let seed = [DEFAULT_SEED, 0, 1, usize::MAX][(trial as usize) % 4];
        diff_script(&format!("random script trial={trial}"), seed, cfg, &ops);
    }
}

/// row 29 — `keysize == 0`: every key hashes and compares equal
#[test]
fn binary_keysize0() {
    for &es in &[8usize, 16, 1] {
        let cfg = MapCfg::binary(es, 0);
        let mut ops = vec![Op::PutDefault { payload: pay(0x7) }];
        for i in 0..6u64 {
            ops.push(put(&k(i, 8), &pay(i)));
            ops.push(get(&k(i + 100, 8)));
            ops.push(get_ts(&k(i, 8)));
        }
        ops.push(del(&k(3, 8)));
        ops.push(get(&k(0, 8)));
        ops.push(del(&k(3, 8)));
        ops.push(put(&k(9, 8), &pay(9)));
        ops.push(Op::Free);
        diff_script(&format!("keysize0 es={es}"), DEFAULT_SEED, cfg, &ops);
    }
}

/// row 30 — free then reuse (the tail of `hm_geti`)
#[test]
fn free_and_reuse() {
    let cfg = MapCfg::binary(8, 4);
    let mut ops = Vec::new();
    for round in 0..4u64 {
        ops.push(Op::PutDefault { payload: pay(0xfffe) });
        for i in 0..(6 + round * 5) {
            ops.push(put(&k(i, 4), &pay(i * 3)));
        }
        for i in 0..(6 + round * 5) {
            ops.push(get(&k(i, 4)));
        }
        ops.push(Op::Free);
        ops.push(get(&k(1, 4)));
        ops.push(Op::Free);
    }
    for seed in [DEFAULT_SEED, 0, 42] {
        diff_script("free and reuse", seed, cfg, &ops);
    }
}
