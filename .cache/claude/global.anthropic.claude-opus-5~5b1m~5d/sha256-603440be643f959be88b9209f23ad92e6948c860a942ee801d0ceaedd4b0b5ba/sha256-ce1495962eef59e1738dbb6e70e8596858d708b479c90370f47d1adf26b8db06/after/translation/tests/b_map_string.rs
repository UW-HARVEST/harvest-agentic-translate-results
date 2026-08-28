//! Phase B — CONFIGS.md rows C51..C60: the hash map with **string** keys, in
//! every `STBDS_SH_*` key-storage mode, driven through the low-level
//! `stbds_shmode_func` / `stbds_hm*_key` entry points.

mod common;
use common::*;

/// Generate `n` **provably distinct** NUL-terminated keys whose lengths cycle
/// through `lens`, with pseudo-random (but reproducible) filler.
///
/// Distinctness matters: with `STBDS_SH_NONE` + `STBDS_HM_STRING` the element
/// holds raw string bytes, so a hash match would make the C `strcmp` those bytes
/// as a `char *` — undefined behaviour in the original (see C54).
///
/// `upper == true` produces a disjoint "absent" key set (uppercase alphabet).
fn gen_keys(n: usize, lens: &[usize], salt: u64, upper: bool) -> Vec<Vec<u8>> {
    use std::collections::{HashMap, HashSet};
    let mut rng = Rng::new(0x5757_0000 + salt);
    let base = if upper { b'A' } else { b'a' };
    let mut per_len: HashMap<usize, usize> = HashMap::new();
    let mut seen: HashSet<Vec<u8>> = HashSet::new();
    let mut out = Vec::new();
    for i in 0..n {
        let mut l = lens[i % lens.len()];
        if l == 0 && (i != 0 || upper) {
            l = 1; // only one key can be the empty string
        }
        let slot = per_len.entry(l).or_insert(0);
        let idx = *slot;
        *slot += 1;

        // unique prefix: base-26 of `idx`, width min(l, 2..3); random filler after
        let w = if l >= 3 { 3 } else { l };
        let mut v = vec![base; l];
        let mut x = idx;
        for p in (0..w).rev() {
            v[p] = base + (x % 26) as u8;
            x /= 26;
        }
        assert_eq!(x, 0, "cannot encode key #{idx} of length {l} uniquely");
        for p in w..l {
            v[p] = base + (rng.next_u64() % 26) as u8;
        }
        assert!(seen.insert(v.clone()), "duplicate key generated: {v:?}");
        out.push(padded_key(&v));
    }
    out
}

fn skeys(n: usize, lens: &[usize], salt: u64) -> Vec<Vec<u8>> {
    gen_keys(n, lens, salt, false)
}

fn absent_keys(n: usize, lens: &[usize], salt: u64) -> Vec<Vec<u8>> {
    gen_keys(n, lens, salt, true)
}

const LENS: &[usize] = &[6, 1, 7, 8, 9, 15, 16, 17, 31, 63, 120];

// ---------------------------------------------------------------------------
// C51 — mode = STBDS_HM_STRING on a NULL map: string.mode becomes SH_DEFAULT
// C55 — the same reached explicitly through shmode_func(_, SH_DEFAULT)
// ---------------------------------------------------------------------------
#[test]
fn c51_c55_string_default_mode() {
    let _g = lock();
    let (c, r) = both();
    for &es in &[8usize, 16, 24, 32] {
        for &seed in &[0usize, 1, DEFAULT_SEED, 0xfeed_face] {
            let keys = skeys(40, LENS, seed as u64);

            // implicit SH_DEFAULT (map starts as NULL)
            sync_seed(seed);
            let mut ops = Vec::new();
            for i in 0..40 {
                ops.push(Op::Put(i, i as u8));
                ops.push(Op::Get(i));
                ops.push(Op::Len);
            }
            for i in 0..40 {
                ops.push(Op::GetTs(i));
            }
            run_ops(
                &format!("implicit-default es={es} seed={seed:#x}"),
                Drv::empty(c, es, 8, HM_STRING),
                Drv::empty(r, es, 8, HM_STRING),
                &keys,
                &ops,
            );

            // explicit SH_DEFAULT
            sync_seed(seed);
            run_ops(
                &format!("explicit-default es={es} seed={seed:#x}"),
                Drv::shmode(c, es, 8, HM_STRING, SH_DEFAULT),
                Drv::shmode(r, es, 8, HM_STRING, SH_DEFAULT),
                &keys,
                &ops,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C52 — SH_STRDUP: keys are malloc-copied; hmdel(mode==1) frees the copy;
//       hmfree_func sweeps every element (R4/R5 free path)
// C53 — SH_ARENA: keys are arena-copied, table->string advances, strreset on free
// ---------------------------------------------------------------------------
#[test]
fn c52_c53_strdup_and_arena() {
    let _g = lock();
    let (c, r) = both();
    for &sh in &[SH_STRDUP, SH_ARENA] {
        for &es in &[8usize, 16, 24] {
            for &seed in &[0usize, 1, DEFAULT_SEED, 0x1234_5678] {
                let keys = skeys(40, LENS, 100 + seed as u64);
                sync_seed(seed);
                let mut ops = Vec::new();
                for i in 0..40 {
                    ops.push(Op::Put(i, i as u8));
                    ops.push(Op::Get(i));
                }
                // re-put every key (duplicate path)
                for i in 0..40 {
                    ops.push(Op::Put(i, 0x80 + i as u8));
                }
                // delete in reverse (delete-last) then forward (relocation)
                for i in (20..40).rev() {
                    ops.push(Op::Del(i, 0));
                    ops.push(Op::Len);
                }
                for i in 0..20 {
                    ops.push(Op::Del(i, 0));
                    ops.push(Op::Len);
                    for j in 0..20 {
                        ops.push(Op::Get(j));
                    }
                }
                run_ops(
                    &format!("sh={sh} es={es} seed={seed:#x}"),
                    Drv::shmode(c, es, 8, HM_STRING, sh),
                    Drv::shmode(r, es, 8, HM_STRING, sh),
                    &keys,
                    &ops,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C54 — SH_NONE + mode = STBDS_HM_STRING: the `switch` falls through to
//       `memcpy(dst, key, keysize)`, so the element holds *string data*, not a
//       pointer (R24).
//
//       NOTE: with this combination `stbds_is_key_equal` would do
//       `strcmp(key, *(char **) element)` — i.e. dereference string bytes as a
//       pointer — so ANY hash match is undefined behaviour in the C original.
//       Only operations that cannot produce a hash match are exercised:
//       inserts of distinct keys and lookups/deletes of absent keys.
// ---------------------------------------------------------------------------
#[test]
fn c54_sh_none_with_string_mode() {
    let _g = lock();
    let (c, r) = both();
    for &es in &[8usize, 16, 24] {
        for &ks in &[1usize, 2, 4, 8, 16] {
            if ks > es {
                continue;
            }
            for &seed in &[0usize, DEFAULT_SEED, 0xabcd] {
                let present = skeys(20, LENS, 200 + seed as u64);
                let absent = absent_keys(20, LENS, 999_000 + seed as u64);
                let mut keys = present.clone();
                keys.extend(absent);
                sync_seed(seed);
                let mut ops = Vec::new();
                for i in 0..20 {
                    ops.push(Op::Put(i, i as u8)); // distinct => no hash match
                    ops.push(Op::Len);
                }
                for i in 20..40 {
                    ops.push(Op::Get(i)); // absent
                    ops.push(Op::GetTs(i));
                    ops.push(Op::Del(i, 0));
                }
                run_ops(
                    &format!("sh_none-string es={es} ks={ks} seed={seed:#x}"),
                    Drv::shmode(c, es, ks, HM_STRING, SH_NONE),
                    Drv::shmode(r, es, ks, HM_STRING, SH_NONE),
                    &keys,
                    &ops,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C56 — all four SH_* modes, 1..64 distinct keys (full grow chain) and keys of
//       length 0/1/7/8/9/600 (the 600-byte one forces the arena's oversized
//       block path *inside* the map)
// ---------------------------------------------------------------------------
#[test]
fn c56_all_modes_grow_chain_and_key_lengths() {
    let _g = lock();
    let (c, r) = both();
    let lens: &[usize] = &[0, 1, 2, 7, 8, 9, 16, 33, 64, 200, 600, 1500];
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &seed in &[0usize, DEFAULT_SEED, 0x5eed] {
            let keys = skeys(64, lens, 300 + seed as u64);
            sync_seed(seed);
            let mut ops = Vec::new();
            for i in 0..64 {
                ops.push(Op::Put(i, i as u8));
                ops.push(Op::Len);
                ops.push(Op::Get(i));
                ops.push(Op::Get(i / 3));
            }
            run_ops(
                &format!("grow-chain sh={sh} seed={seed:#x}"),
                Drv::shmode(c, 16, 8, HM_STRING, sh),
                Drv::shmode(r, 16, 8, HM_STRING, sh),
                &keys,
                &ops,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C57 — present/absent lookups through both getter entry points
// ---------------------------------------------------------------------------
#[test]
fn c57_string_lookups() {
    let _g = lock();
    let (c, r) = both();
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &seed in &[0usize, 7, DEFAULT_SEED] {
            let mut keys = skeys(30, LENS, 400 + seed as u64);
            keys.extend(absent_keys(30, LENS, 400_000 + seed as u64)); // absent set
            sync_seed(seed);
            let mut ops = Vec::new();
            for i in 0..30 {
                ops.push(Op::Put(i, i as u8));
            }
            for i in 0..60 {
                ops.push(Op::Get(i));
                ops.push(Op::GetTs(i));
            }
            run_ops(
                &format!("lookups sh={sh} seed={seed:#x}"),
                Drv::shmode(c, 24, 8, HM_STRING, sh),
                Drv::shmode(r, 24, 8, HM_STRING, sh),
                &keys,
                &ops,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C58 — duplicate put: `temp` reuse AND `stbds_temp_key` propagation, which is
//       what the `shputs` macro (and therefore `sh_puts`) depends on.
//
//       `temp_key` is only assigned by the found-empty-slot path and the
//       *first* probe scan's duplicate path, and `stbds_make_hash_index` never
//       initialises it, so it is only read here in a table that never grows or
//       rebuilds (<= 6 keys, no deletes).
// ---------------------------------------------------------------------------
#[test]
fn c58_temp_key_propagation() {
    let _g = lock();
    let (c, r) = both();
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &seed in &[0usize, 1, 2, 3, 17, DEFAULT_SEED, 0xbeef] {
            // 5 keys: `used_count` stays at 5, below the 8-slot table's
            // `used_count_threshold` of 6, so the grow check at the top of
            // `stbds_hmput_key` never fires and `temp_key` therefore stays valid
            // for the whole sequence (it is uninitialised in a fresh table).
            let keys = skeys(5, LENS, 500 + seed as u64);
            sync_seed(seed);
            let mut cd = Drv::shmode(c, 16, 8, HM_STRING, sh);
            let mut rd = Drv::shmode(r, 16, 8, HM_STRING, sh);
            unsafe {
                for round in 0..4u8 {
                    for (i, k) in keys.iter().enumerate() {
                        let a = cd.put(k, round * 32 + i as u8);
                        let b = rd.put(k, round * 32 + i as u8);
                        assert_eq!(a, b, "sh={sh} seed={seed} round={round} key#{i}: temp");
                        assert_eq!(
                            cd.temp_key_str(),
                            rd.temp_key_str(),
                            "sh={sh} seed={seed} round={round} key#{i}: temp_key"
                        );
                        eqs(
                            &format!("temp_key sh={sh} seed={seed} r{round} #{i}"),
                            &cd.snap(),
                            &rd.snap(),
                        );
                    }
                }
                // the table must never have grown (used_count stays <= 6)
                assert_eq!((*cd.table()).slot_count, 8);
                assert_eq!((*rd.table()).slot_count, 8);
                cd.free();
                rd.free();
            }
        }
    }
}

/// `shputs`: write the whole struct then re-read the key from `temp_key` —
/// exactly the sequence `sh_puts` performs.
#[test]
fn c58_shputs_macro_shape() {
    let _g = lock();
    let (c, r) = both();
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &seed in &[0usize, 1, DEFAULT_SEED] {
            // 5 keys => no grow => `temp_key` stays valid (see c58 above);
            // `shputs` stores the key member *from* `temp_key`.
            let keys = skeys(5, LENS, 600 + seed as u64);
            sync_seed(seed);
            let mut ops = Vec::new();
            for round in 0..3u8 {
                for i in 0..5 {
                    ops.push(Op::PutStruct(i, round * 16 + i as u8));
                    ops.push(Op::Get(i));
                    ops.push(Op::Len);
                }
            }
            run_ops(
                &format!("shputs sh={sh} seed={seed:#x}"),
                Drv::shmode(c, 16, 8, HM_STRING, sh),
                Drv::shmode(r, 16, 8, HM_STRING, sh),
                &keys,
                &ops,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C59/C60 — deletes in string mode: strdup free, relocation, tombstone rebuild,
//           shrink
// ---------------------------------------------------------------------------
#[test]
fn c59_c60_string_deletes() {
    let _g = lock();
    let (c, r) = both();
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &seed in &[0usize, 3, 11, DEFAULT_SEED, 0xc0ffee] {
            for n in [4usize, 7, 13, 26] {
                let keys = skeys(n, LENS, 700 + seed as u64 + n as u64 * 31);
                sync_seed(seed);
                let mut ops: Vec<Op> = (0..n).map(|i| Op::Put(i, i as u8)).collect();
                // forward deletes => relocation on every step but the last
                for i in 0..n {
                    ops.push(Op::Del(i, 0));
                    ops.push(Op::Len);
                    for j in 0..n {
                        ops.push(Op::Get(j));
                    }
                }
                // re-insert into the shrunk/rebuilt table
                for i in 0..n {
                    ops.push(Op::Put(i, 0x40 + i as u8));
                }
                run_ops(
                    &format!("del sh={sh} n={n} seed={seed:#x}"),
                    Drv::shmode(c, 16, 8, HM_STRING, sh),
                    Drv::shmode(r, 16, 8, HM_STRING, sh),
                    &keys,
                    &ops,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Long randomized walks across all safe string configurations.
// ---------------------------------------------------------------------------
#[test]
fn c51_c60_randomized_walks() {
    let _g = lock();
    let (c, r) = both();
    let shs = [SH_DEFAULT, SH_STRDUP, SH_ARENA];
    for trial in 0..120u64 {
        let mut rng = Rng::new(0x5AA0_0000 + trial);
        let sh = shs[(trial as usize) % shs.len()];
        let es = [8usize, 16, 24, 32][(trial as usize / 3) % 4];
        let nkeys = 6 + rng.below(60) as usize;
        let keys = skeys(nkeys, LENS, 800 + trial);
        sync_seed(rng.next_u64() as usize);

        let mut ops = Vec::new();
        for _ in 0..150 {
            let k = rng.below(nkeys as u64) as usize;
            ops.push(match rng.below(12) {
                0..=4 => Op::Put(k, rng.byte()),
                5 | 6 => Op::Get(k),
                7 => Op::GetTs(k),
                8..=10 => Op::Del(k, 0),
                _ => Op::Len,
            });
        }
        run_ops(
            &format!("walk trial={trial} sh={sh} es={es} nkeys={nkeys}"),
            Drv::shmode(c, es, 8, HM_STRING, sh),
            Drv::shmode(r, es, 8, HM_STRING, sh),
            &keys,
            &ops,
        );
    }
}

/// Same, but starting from a NULL map (implicit `STBDS_SH_DEFAULT`).
#[test]
fn c51_randomized_walks_from_null() {
    let _g = lock();
    let (c, r) = both();
    for trial in 0..80u64 {
        let mut rng = Rng::new(0x5BB0_0000 + trial);
        let es = [8usize, 16, 24][(trial as usize) % 3];
        let nkeys = 6 + rng.below(50) as usize;
        let keys = skeys(nkeys, LENS, 900 + trial);
        sync_seed(rng.next_u64() as usize);
        let mut ops = Vec::new();
        for _ in 0..150 {
            let k = rng.below(nkeys as u64) as usize;
            ops.push(match rng.below(11) {
                0..=4 => Op::Put(k, rng.byte()),
                5 | 6 => Op::Get(k),
                7 => Op::GetTs(k),
                8 | 9 => Op::Del(k, 0),
                _ => Op::Len,
            });
        }
        run_ops(
            &format!("null-walk trial={trial} es={es} nkeys={nkeys}"),
            Drv::empty(c, es, 8, HM_STRING),
            Drv::empty(r, es, 8, HM_STRING),
            &keys,
            &ops,
        );
    }
}
