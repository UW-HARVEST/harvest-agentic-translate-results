//! Phase B — CONFIGS.md rows 31-40: `char *`-keyed maps across all four
//! `STBDS_SH_*` arena modes, plus the cross-mode combinations.

mod common;

use common::*;

fn s(text: &str) -> Vec<u8> {
    let mut v = text.as_bytes().to_vec();
    v.push(0);
    v
}

fn pay(v: u64) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

/// A pool of keys with shared prefixes, duplicates, empty and long strings.
fn key_pool(rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    let mut keys = vec![
        s(""),
        s("a"),
        s("ab"),
        s("abc"),
        s("abcd"),
        s("test_0"),
        s("test_1"),
        s("test_10"),
        s("test_100"),
        s("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ];
    // a couple of high-bit keys
    keys.push(vec![0x80, 0x81, 0xff, 0xfe, 0]);
    keys.push(vec![0xff; 9].into_iter().chain(std::iter::once(0)).collect());
    while keys.len() < n {
        let l = rng.below(20) as usize;
        keys.push(rng.nul_free(l));
    }
    keys.truncate(n);
    keys
}

/// row 31 — implicit creation: `hmput_key(NULL, .., mode=STRING)` sets
/// `string.mode = STBDS_SH_DEFAULT` and stores the caller's pointer.
#[test]
fn implicit_default() {
    for &es in &[8usize, 16, 24] {
        let cfg = MapCfg::string(es, HM_STRING);
        let mut rng = Rng::new(0x5111_0000 ^ es as u64);
        for &count in &[1usize, 5, 6, 7, 20] {
            let keys = key_pool(&mut rng, count);
            let mut ops = Vec::new();
            for (i, kk) in keys.iter().enumerate() {
                ops.push(put(kk, &pay(i as u64)));
            }
            for kk in &keys {
                ops.push(get(kk));
                ops.push(get_ts(kk));
            }
            ops.push(get(&s("definitely-absent")));
            ops.push(Op::Free);
            diff_script(&format!("implicit default es={es} n={count}"), DEFAULT_SEED, cfg, &ops);
        }
    }
}

/// row 32 — explicit `STBDS_SH_DEFAULT` through `stbds_shmode_func`
#[test]
fn sh_default() {
    for seed in [DEFAULT_SEED, 0, 1, usize::MAX] {
        for &es in &[8usize, 16] {
            let cfg = MapCfg::string(es, HM_STRING);
            let mut rng = Rng::new(0x5222_0000 ^ seed as u64 ^ es as u64);
            let keys = key_pool(&mut rng, 24);
            let mut ops = vec![Op::ShMode { sh_mode: SH_DEFAULT }];
            for (i, kk) in keys.iter().enumerate() {
                ops.push(put(kk, &pay(i as u64)));
                ops.push(get(kk));
            }
            // duplicates (update path)
            for (i, kk) in keys.iter().enumerate() {
                ops.push(put(kk, &pay(1000 + i as u64)));
            }
            // misses
            for i in 0..5 {
                ops.push(get(&s(&format!("miss-{i}"))));
                ops.push(get_ts(&s(&format!("miss-{i}"))));
                ops.push(del(&s(&format!("miss-{i}"))));
            }
            // delete every key from the front (interior + re-index path)
            for kk in keys.iter() {
                ops.push(del(kk));
                ops.push(get(kk));
            }
            ops.push(Op::Free);
            diff_script(&format!("sh_default es={es}"), seed, cfg, &ops);
        }
    }
}

/// row 33 — `STBDS_SH_STRDUP`: keys duplicated on insert, freed on delete and
/// by `hmfree_func`
#[test]
fn sh_strdup() {
    for seed in [DEFAULT_SEED, 0, 0x777] {
        for &es in &[8usize, 16, 24] {
            let cfg = MapCfg::string(es, HM_STRING);
            let mut rng = Rng::new(0x5333_0000 ^ seed as u64 ^ es as u64);
            let keys = key_pool(&mut rng, 20);
            let mut ops = vec![Op::ShMode { sh_mode: SH_STRDUP }];
            for (i, kk) in keys.iter().enumerate() {
                ops.push(put(kk, &pay(i as u64)));
            }
            for kk in &keys {
                ops.push(get(kk));
                ops.push(get_ts(kk));
            }
            // duplicate put: must NOT re-duplicate the key
            for (i, kk) in keys.iter().enumerate() {
                ops.push(put(kk, &pay(500 + i as u64)));
            }
            // deletes free the strdup'd key (mode == STBDS_HM_STRING exactly)
            for kk in keys.iter().take(10) {
                ops.push(del(kk));
                ops.push(get(kk));
            }
            // re-insert (fresh strdup into tombstoned slots)
            for (i, kk) in keys.iter().take(10).enumerate() {
                ops.push(put(kk, &pay(900 + i as u64)));
            }
            ops.push(Op::Free); // frees every remaining strdup'd key
            diff_script(&format!("sh_strdup es={es}"), seed, cfg, &ops);
        }
    }
}

/// row 34 — `STBDS_SH_ARENA`: keys copied into the string arena; long keys and
/// many keys force several blocks (`block` 0→n) and the oversized-block path.
#[test]
fn sh_arena() {
    for seed in [DEFAULT_SEED, 0, 0x555] {
        for &es in &[8usize, 16] {
            let cfg = MapCfg::string(es, HM_STRING);
            let mut rng = Rng::new(0x5444_0000 ^ seed as u64 ^ es as u64);
            let mut ops = vec![Op::ShMode { sh_mode: SH_ARENA }];
            let mut keys: Vec<Vec<u8>> = Vec::new();
            // short keys first (fill the 512-byte block, then grow)
            for i in 0..60u64 {
                let kk = s(&format!("arena-key-{i:04}"));
                keys.push(kk.clone());
                ops.push(put(&kk, &pay(i)));
            }
            // one huge key: len > blocksize -> dedicated block
            let big = rng.nul_free(1500);
            keys.push(big.clone());
            ops.push(put(&big, &pay(0xbeef)));
            // then more short keys (remaining is unchanged after the big block)
            for i in 60..80u64 {
                let kk = s(&format!("arena-key-{i:04}"));
                keys.push(kk.clone());
                ops.push(put(&kk, &pay(i)));
            }
            for kk in &keys {
                ops.push(get(kk));
            }
            for kk in keys.iter().take(20) {
                ops.push(del(kk));
            }
            for kk in &keys {
                ops.push(get_ts(kk));
            }
            ops.push(Op::Free); // strreset frees every block
            diff_script(&format!("sh_arena es={es}"), seed, cfg, &ops);
        }
    }
}

/// row 35 — `STBDS_SH_NONE` with `mode = STBDS_HM_STRING`: the `switch`
/// `default` arm `memcpy`s `keysize` bytes *of the string* into the element, so
/// the stored "key pointer" is really string bytes.  Only distinct keys and
/// absent lookups are exercised: any `stbds_is_key_equal` call in this state
/// would dereference string bytes as a pointer (the C would crash too).
#[test]
fn sh_none_string() {
    for &es in &[8usize, 16] {
        let cfg = MapCfg::string(es, HM_STRING);
        let mut ops = vec![Op::ShMode { sh_mode: SH_NONE }];
        let mut keys = Vec::new();
        for i in 0..12u64 {
            // distinct 8+ byte keys so the memcpy of 8 bytes is in bounds
            let kk = s(&format!("none-key-{i:04}"));
            keys.push(kk.clone());
            ops.push(put(&kk, &pay(i)));
        }
        for i in 0..6u64 {
            ops.push(get(&s(&format!("absent-key-{i:04}"))));
            ops.push(get_ts(&s(&format!("absent-key-{i:04}"))));
        }
        ops.push(Op::Free);
        // snapshot must not interpret the raw bytes as a pointer here
        let cfg = MapCfg { kind: KeyKind::Binary, ..cfg };
        diff_script(&format!("sh_none_string es={es}"), DEFAULT_SEED, cfg, &ops);
    }
}

/// row 36 — key shapes: empty, 1 char, shared prefixes, 511/512/513 bytes,
/// high-bit bytes, duplicates
#[test]
fn key_shapes() {
    let mut rng = Rng::new(0x5666_0000);
    let mut keys: Vec<Vec<u8>> = vec![
        s(""),
        s("x"),
        s("xy"),
        s("prefix"),
        s("prefix1"),
        s("prefix12"),
        s("prefix123"),
        vec![0x80, 0],
        vec![0xff, 0x80, 0x7f, 0],
    ];
    for l in [510usize, 511, 512, 513, 1000] {
        keys.push(rng.nul_free(l));
    }
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let cfg = MapCfg::string(16, HM_STRING);
        let mut ops = vec![Op::ShMode { sh_mode: mode }];
        for (i, kk) in keys.iter().enumerate() {
            ops.push(put(kk, &pay(i as u64)));
            ops.push(put(kk, &pay(i as u64 + 1)));
            ops.push(get(kk));
        }
        for kk in &keys {
            ops.push(get_ts(kk));
        }
        for kk in keys.iter().rev() {
            ops.push(del(kk));
        }
        ops.push(Op::Free);
        diff_script(&format!("key shapes sh={mode}"), DEFAULT_SEED, cfg, &ops);
    }
}

/// row 37 — `mode` out of the `STBDS_HM_*` enum but `>= STBDS_HM_STRING`.
///
/// Hashing/compare behave exactly like `mode == 1`, but `stbds_hmdel_key`'s
/// `mode == STBDS_HM_STRING` tests are false, so only *last element* deletes
/// are exercised (an interior delete would re-hash the raw pointer bytes,
/// which is genuinely address-dependent in the C as well).
#[test]
fn mode_out_of_range_valid() {
    for mode in [2i32, 3, 4, 7, 255, 1000, i32::MAX] {
        for sh in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            let cfg = MapCfg::string(16, mode);
            let mut ops = vec![Op::ShMode { sh_mode: sh }];
            let mut keys = Vec::new();
            for i in 0..10u64 {
                let kk = s(&format!("oor-{i}"));
                keys.push(kk.clone());
                ops.push(put(&kk, &pay(i)));
                ops.push(get(&kk));
                ops.push(get_ts(&kk));
            }
            for i in 0..3u64 {
                ops.push(get(&s(&format!("oor-absent-{i}"))));
                ops.push(del(&s(&format!("oor-absent-{i}"))));
            }
            // delete the last inserted key repeatedly (old_index == final_index)
            for kk in keys.iter().rev() {
                ops.push(del(kk));
            }
            ops.push(Op::Free);
            diff_script(&format!("mode={mode} sh={sh}"), DEFAULT_SEED, cfg, &ops);
        }
    }
}

/// row 38 — string-mode delete paths: interior delete re-finds through
/// `*(char**)`, plus shrink and rebuild
#[test]
fn string_del_paths() {
    for sh in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for n in [1usize, 2, 7, 13, 30] {
            let cfg = MapCfg::string(16, HM_STRING);
            let mut ops = vec![Op::ShMode { sh_mode: sh }];
            let keys: Vec<Vec<u8>> = (0..n).map(|i| s(&format!("del-{i:03}"))).collect();
            for (i, kk) in keys.iter().enumerate() {
                ops.push(put(kk, &pay(i as u64)));
            }
            // front-first deletes: always the memmove + re-find + re-index path
            for kk in &keys {
                ops.push(del(kk));
                for other in &keys {
                    ops.push(get(other));
                }
            }
            ops.push(Op::Free);
            diff_script(&format!("string del sh={sh} n={n}"), DEFAULT_SEED, cfg, &ops);
        }
    }
}

/// row 39 — table created with `SH_STRDUP` but used with `mode = HM_BINARY`:
/// binary hash/compare (over the first 8 bytes of the key string) *and*
/// `stbds_strdup` on insert, so identical keys never match and accumulate.
#[test]
fn cross_strdup_binary() {
    for &es in &[8usize, 16] {
        let mut cfg = MapCfg::string(es, HM_BINARY);
        cfg.keysize = 8;
        let mut ops = vec![Op::ShMode { sh_mode: SH_STRDUP }];
        // keys must be >= 8 bytes: hash_bytes/memcmp read exactly keysize bytes
        let keys: Vec<Vec<u8>> = (0..12u64).map(|i| s(&format!("cross-key-{i:04}"))).collect();
        for (i, kk) in keys.iter().enumerate() {
            ops.push(put(kk, &pay(i as u64)));
        }
        for kk in &keys {
            ops.push(put(kk, &pay(0xaa))); // never "found": appends again
            ops.push(get(kk));
            ops.push(get_ts(kk));
            ops.push(del(kk)); // never found either
        }
        ops.push(Op::Free); // still frees every strdup'd key
        diff_script(&format!("cross strdup/binary es={es}"), DEFAULT_SEED, cfg, &ops);
    }
}

/// row 40 — table created with `SH_ARENA` but used with `mode = HM_BINARY`
#[test]
fn cross_arena_binary() {
    for &es in &[8usize, 16] {
        let mut cfg = MapCfg::string(es, HM_BINARY);
        cfg.keysize = 8;
        let mut ops = vec![Op::ShMode { sh_mode: SH_ARENA }];
        let keys: Vec<Vec<u8>> = (0..40u64).map(|i| s(&format!("cross-arena-key-{i:04}"))).collect();
        for (i, kk) in keys.iter().enumerate() {
            ops.push(put(kk, &pay(i as u64)));
            ops.push(get(kk));
        }
        for kk in keys.iter().take(10) {
            ops.push(del(kk));
        }
        ops.push(Op::Free);
        diff_script(&format!("cross arena/binary es={es}"), DEFAULT_SEED, cfg, &ops);
    }
}
