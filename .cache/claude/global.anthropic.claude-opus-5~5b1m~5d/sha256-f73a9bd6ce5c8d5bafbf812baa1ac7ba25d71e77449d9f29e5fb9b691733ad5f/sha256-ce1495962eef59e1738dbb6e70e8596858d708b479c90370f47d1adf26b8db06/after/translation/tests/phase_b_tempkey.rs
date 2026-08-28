//! Phase B — CONFIGS.md row 52: `stbds_temp_key` on the *update* path.
//!
//! `stbds_hmput_key` writes `stbds_temp_key(a)` when it finds an existing key
//! in the first probe loop (`i = pos&7 .. 8`) but **not** in the wrap-around
//! loop (`i = 0 .. pos`).  This asymmetry is a quirk of the original that the
//! translation must reproduce.
//!
//! The scripts keep `used_count` below `used_count_threshold` (6 for
//! `slot_count == 8`) so the table is never rebuilt — `stbds_make_hash_index`
//! leaves `temp_key` uninitialised, so it may only be read when a previous
//! insert has written it.

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

#[test]
fn temp_key_on_update() {
    // many seeds: the probe position of each key changes with the table seed,
    // so both the first-loop hit (temp_key updated) and the wrap-around hit
    // (temp_key left stale) are exercised.
    let mut rng = Rng::new(0x7E_9000);
    let mut seeds: Vec<usize> = vec![DEFAULT_SEED, 0, 1, 2, 3, usize::MAX];
    for _ in 0..30 {
        seeds.push(rng.next_u64() as usize);
    }
    for sh in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &seed in &seeds {
            for nkeys in 1usize..=5 {
                let cfg = MapCfg::string(16, HM_STRING).always_temp_key();
                let keys: Vec<Vec<u8>> =
                    (0..nkeys).map(|i| s(&format!("tk-{i}-{}", "x".repeat(i)))).collect();
                let mut ops = vec![Op::ShMode { sh_mode: sh }];
                for (i, k) in keys.iter().enumerate() {
                    ops.push(put(k, &pay(i as u64)));
                }
                // duplicate puts in every order: each one is an "existing key"
                // hit through one of the two probe loops
                for round in 0..3u64 {
                    for (i, k) in keys.iter().enumerate() {
                        ops.push(put(k, &pay(round * 10 + i as u64)));
                    }
                    for (i, k) in keys.iter().enumerate().rev() {
                        ops.push(put(k, &pay(round * 20 + i as u64)));
                    }
                }
                ops.push(Op::Free);
                diff_script(&format!("temp_key sh={sh} nkeys={nkeys}"), seed, cfg, &ops);
            }
        }
    }
}

/// Same, but with `mode = HM_BINARY` on a `SH_STRDUP`/`SH_ARENA` table: the
/// `switch` still writes `temp_key` on insert while the `mode >= HM_STRING`
/// guard in the found-existing branch does not.
#[test]
fn temp_key_binary_mode() {
    for sh in [SH_STRDUP, SH_ARENA] {
        for seed in [DEFAULT_SEED, 0, 1, 0xfeed] {
            let mut cfg = MapCfg::string(16, HM_BINARY).always_temp_key();
            cfg.keysize = 8;
            let keys: Vec<Vec<u8>> = (0..4).map(|i| s(&format!("tkbin-key-{i}"))).collect();
            let mut ops = vec![Op::ShMode { sh_mode: sh }];
            for (i, k) in keys.iter().enumerate() {
                ops.push(put(k, &pay(i as u64)));
            }
            ops.push(Op::Free);
            diff_script(&format!("temp_key binary sh={sh}"), seed, cfg, &ops);
        }
    }
}
