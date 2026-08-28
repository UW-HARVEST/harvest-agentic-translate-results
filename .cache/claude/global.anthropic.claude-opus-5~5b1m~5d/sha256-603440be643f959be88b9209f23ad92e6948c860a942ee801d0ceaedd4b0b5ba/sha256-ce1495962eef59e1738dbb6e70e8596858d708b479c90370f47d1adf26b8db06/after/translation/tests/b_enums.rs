//! Phase B/C — CONFIGS.md rows C61..C65 and ERRORS.md rows R20..R24:
//! **out-of-range enum values passed across the FFI boundary**.
//!
//! `STBDS_HM_BINARY`/`STBDS_HM_STRING` are `#define`d ints and `STBDS_SH_*` is a
//! plain C `enum`, so the exported functions accept *any* `int`.  The C code
//! tests them with `mode >= STBDS_HM_STRING`, `mode == STBDS_HM_STRING`,
//! `(unsigned char) mode` and a `switch` with a `default:` — four different
//! notions of "valid" that all have to be reproduced exactly.

mod common;
use common::*;
use std::ffi::c_int;

/// `mode` values that make `mode >= STBDS_HM_STRING` true but are not the
/// `STBDS_HM_STRING` enumerator.
const STRINGISH_MODES: &[c_int] = &[2, 3, 5, 42, 255, 256, 65536, c_int::MAX, c_int::MAX - 1];
/// `mode` values that make `mode >= STBDS_HM_STRING` false.
const BINARYISH_MODES: &[c_int] = &[-1, -2, -255, -256, c_int::MIN, c_int::MIN + 1];
/// Every `STBDS_SH_*` value plus out-of-range ones, with the `(unsigned char)`
/// truncation the C performs.
const SH_MODES: &[c_int] = &[
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    127,
    128,
    254,
    255,
    256,
    257,
    511,
    512,
    -1,
    -2,
    -255,
    c_int::MIN,
    c_int::MAX,
];

fn keys(n: usize, salt: u64) -> Vec<Vec<u8>> {
    // distinct (index prefix), NUL-terminated, zero-padded so any
    // `memcpy(_, key, keysize)` with keysize up to 24 is an in-bounds
    // deterministic read.  `salt` varies the tail content.
    let mut rng = Rng::new(0x9999_0000 + salt);
    (0..n)
        .map(|i| {
            let tail: String = (0..(1 + (rng.next_u64() % 20) as usize))
                .map(|_| (b'a' + (rng.next_u64() % 26) as u8) as char)
                .collect();
            padded_key(format!("key{i:03}_{tail}").as_bytes())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// C61 — mode >= 2: string hash/compare on put/get/get_ts (R20)
// ---------------------------------------------------------------------------
#[test]
fn c61_stringish_modes_put_get() {
    let _g = lock();
    let (c, r) = both();
    for &mode in STRINGISH_MODES {
        for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &seed in &[0usize, DEFAULT_SEED] {
                let ks = keys(30, mode as u64 as u64 ^ seed as u64);
                sync_seed(seed);
                let mut ops: Vec<Op> = Vec::new();
                for i in 0..30 {
                    ops.push(Op::Put(i, i as u8));
                    ops.push(Op::Get(i));
                    ops.push(Op::GetTs(i));
                    ops.push(Op::Len);
                }
                for i in 0..30 {
                    ops.push(Op::Put(i, 0x80 + i as u8)); // duplicate path
                }
                run_ops(
                    &format!("stringish mode={mode} sh={sh} seed={seed:#x}"),
                    Drv::shmode(c, 16, 8, mode, sh),
                    Drv::shmode(r, 16, 8, mode, sh),
                    &ks,
                    &ops,
                );
            }
        }
        // and starting from NULL: nt->string.mode = (mode >= 1 ? SH_DEFAULT : 0)
        sync_seed(DEFAULT_SEED);
        let ks = keys(20, 7);
        let mut ops: Vec<Op> = Vec::new();
        for i in 0..20 {
            ops.push(Op::Put(i, i as u8));
            ops.push(Op::Get(i));
        }
        run_ops(
            &format!("stringish-from-null mode={mode}"),
            Drv::empty(c, 16, 8, mode),
            Drv::empty(r, 16, 8, mode),
            &ks,
            &ops,
        );
    }
}

// ---------------------------------------------------------------------------
// C62 — negative mode: binary hash/compare (R21)
// ---------------------------------------------------------------------------
#[test]
fn c62_binaryish_modes_put_get() {
    let _g = lock();
    let (c, r) = both();
    for &mode in BINARYISH_MODES {
        for &sh in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA, 200] {
            for &seed in &[0usize, DEFAULT_SEED] {
                let ks: Vec<Vec<u8>> = (0..25).map(|i| bin_key(i as u64 * 7 + 1, 8)).collect();
                sync_seed(seed);
                let mut ops: Vec<Op> = Vec::new();
                for i in 0..25 {
                    ops.push(Op::Put(i, i as u8));
                    ops.push(Op::Get(i));
                    ops.push(Op::GetTs(i));
                }
                run_ops(
                    &format!("binaryish mode={mode} sh={sh} seed={seed:#x}"),
                    Drv::shmode(c, 16, 8, mode, sh),
                    Drv::shmode(r, 16, 8, mode, sh),
                    &ks,
                    &ops,
                );
            }
        }
        // from NULL: nt->string.mode = 0 => memcpy key path
        sync_seed(DEFAULT_SEED);
        let ks: Vec<Vec<u8>> = (0..25).map(|i| bin_key(i as u64 * 13 + 5, 8)).collect();
        let mut ops: Vec<Op> = Vec::new();
        for i in 0..25 {
            ops.push(Op::Put(i, i as u8));
            ops.push(Op::Get(i));
            ops.push(Op::Del(i, 0));
            ops.push(Op::Get(i));
        }
        run_ops(
            &format!("binaryish-from-null mode={mode}"),
            Drv::empty(c, 16, 8, mode),
            Drv::empty(r, 16, 8, mode),
            &ks,
            &ops,
        );
    }
}

// ---------------------------------------------------------------------------
// C63 — hmdel_key with mode >= 2: string hash for the lookup but
//       `mode == STBDS_HM_STRING` is FALSE, so
//         * a STBDS_SH_STRDUP key is NOT freed, and
//         * the relocation re-lookup uses the *binary* key form.
//
//       The relocation branch then hashes the bytes of the key *pointer* as a
//       string, does not find the moved element and trips the live C
//       `STBDS_ASSERT(slot >= 0)` (ERRORS.md A5).  Only deletes that need no
//       relocation (`old_index == final_index`, i.e. delete-last) are exercised;
//       inserting k0..kn in order and deleting in reverse guarantees that.
// ---------------------------------------------------------------------------
#[test]
fn c63_stringish_delete_last_only() {
    let _g = lock();
    let (c, r) = both();
    for &mode in STRINGISH_MODES {
        for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &seed in &[0usize, 5, DEFAULT_SEED] {
                let ks = keys(12, 31 + seed as u64);
                sync_seed(seed);
                let mut ops: Vec<Op> = (0..12).map(|i| Op::Put(i, i as u8)).collect();
                for i in (0..12).rev() {
                    ops.push(Op::Del(i, 0)); // always the last element
                    ops.push(Op::Len);
                    for j in 0..i {
                        ops.push(Op::Get(j));
                    }
                }
                run_ops(
                    &format!("del-stringish mode={mode} sh={sh} seed={seed:#x}"),
                    Drv::shmode(c, 16, 8, mode, sh),
                    Drv::shmode(r, 16, 8, mode, sh),
                    &ks,
                    &ops,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C64 — hmdel_key with negative mode: fully binary, relocation included
// ---------------------------------------------------------------------------
#[test]
fn c64_binaryish_delete_all() {
    let _g = lock();
    let (c, r) = both();
    for &mode in BINARYISH_MODES {
        for &sh in &[SH_NONE, 200] {
            for &seed in &[0usize, 5, DEFAULT_SEED] {
                let ks: Vec<Vec<u8>> = (0..14).map(|i| bin_key(i as u64 * 101 + 3, 8)).collect();
                sync_seed(seed);
                let mut ops: Vec<Op> = (0..14).map(|i| Op::Put(i, i as u8)).collect();
                for i in 0..14 {
                    ops.push(Op::Del(i, 0)); // relocation on every step
                    ops.push(Op::Len);
                    for j in 0..14 {
                        ops.push(Op::Get(j));
                    }
                }
                run_ops(
                    &format!("del-binaryish mode={mode} sh={sh} seed={seed:#x}"),
                    Drv::shmode(c, 16, 8, mode, sh),
                    Drv::shmode(r, 16, 8, mode, sh),
                    &ks,
                    &ops,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C65 — stbds_shmode_func with every in- and out-of-range mode:
//       `h->string.mode = (unsigned char) mode` (R23) and the resulting
//       `switch (table->string.mode)` behaviour in hmput_key (R24)
// ---------------------------------------------------------------------------
#[test]
fn c65_shmode_func_truncation() {
    let _g = lock();
    let (c, r) = both();
    for &sh in SH_MODES {
        for &es in &[8usize, 16, 24] {
            sync_seed(0x6565);
            unsafe {
                let ct = (c.shmode_func)(es, sh);
                let rt = (r.shmode_func)(es, sh);
                let cm = (*((*header(hash_to_arr(ct, es))).hash_table as *mut HashIndex))
                    .string
                    .mode;
                let rm = (*((*header(hash_to_arr(rt, es))).hash_table as *mut HashIndex))
                    .string
                    .mode;
                assert_eq!(cm, rm, "shmode_func({es}, {sh}): string.mode");
                assert_eq!(cm, sh as u8, "shmode_func({es}, {sh}): (unsigned char) mode");
                eqs(
                    &format!("shmode sh={sh} es={es}"),
                    &snap_map(ct, es, KeyKind::Bin),
                    &snap_map(rt, es, KeyKind::Bin),
                );
                (c.hmfree_func)(hash_to_arr(ct, es), es);
                (r.hmfree_func)(hash_to_arr(rt, es), es);
            }
        }
    }

    // Now drive each truncated string.mode with a full binary put/get/del cycle
    // (binary mode is always safe, whatever the key-storage mode does).
    for &sh in SH_MODES {
        for &seed in &[0usize, DEFAULT_SEED] {
            let ks: Vec<Vec<u8>> = (0..14).map(|i| bin_key(i as u64 * 37 + 9, 8)).collect();
            sync_seed(seed);
            let mut ops: Vec<Op> = Vec::new();
            for i in 0..14 {
                ops.push(Op::Put(i, i as u8));
                ops.push(Op::Get(i));
            }
            for i in 0..14 {
                ops.push(Op::Del(i, 0));
                ops.push(Op::Len);
            }
            run_ops(
                &format!("shmode-binary sh={sh} seed={seed:#x}"),
                Drv::shmode(c, 24, 8, HM_BINARY, sh),
                Drv::shmode(r, 24, 8, HM_BINARY, sh),
                &ks,
                &ops,
            );
        }
    }

    // And the string-key path for the three modes that store a pointer.
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &mode in &[HM_STRING, 2, 99] {
            sync_seed(DEFAULT_SEED);
            let ks = keys(20, sh as u64 * 100 + mode as u64);
            let mut ops: Vec<Op> = Vec::new();
            for i in 0..20 {
                ops.push(Op::Put(i, i as u8));
                ops.push(Op::Get(i));
            }
            run_ops(
                &format!("shmode-string sh={sh} mode={mode}"),
                Drv::shmode(c, 16, 8, mode, sh),
                Drv::shmode(r, 16, 8, mode, sh),
                &ks,
                &ops,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Randomized sweep over (mode, sh) with random operation scripts.
// ---------------------------------------------------------------------------
#[test]
fn c61_c65_randomized_enum_sweep() {
    let _g = lock();
    let (c, r) = both();
    let modes: Vec<c_int> = STRINGISH_MODES
        .iter()
        .chain(BINARYISH_MODES.iter())
        .chain([HM_BINARY, HM_STRING].iter())
        .copied()
        .collect();

    for trial in 0..200u64 {
        let mut rng = Rng::new(0xE0E0_0000 + trial);
        let mode = modes[(rng.below(modes.len() as u64)) as usize];
        let stringish = mode >= HM_STRING;
        // keep the configuration free of C-level UB: a string-ish `mode` needs a
        // pointer-storing `string.mode`, otherwise `strcmp` would dereference raw
        // key bytes on any hash match (see C54).
        let sh = if stringish {
            [SH_DEFAULT, SH_STRDUP, SH_ARENA][rng.below(3) as usize]
        } else {
            SH_MODES[rng.below(SH_MODES.len() as u64) as usize]
        };
        let es = [8usize, 16, 24, 32][rng.below(4) as usize];
        let nkeys = 6 + rng.below(30) as usize;
        let ks: Vec<Vec<u8>> = if stringish {
            keys(nkeys, trial)
        } else {
            (0..nkeys).map(|i| bin_key(i as u64 * 8191 + 1, 8)).collect()
        };
        sync_seed(rng.next_u64() as usize);

        // deletes: relocation is UB for `mode >= 2` (A5), so those trials delete
        // only from the tail.
        let allow_relocating_delete = mode <= HM_STRING;
        let mut ops = Vec::new();
        let mut inserted = 0usize;
        for _ in 0..120 {
            match rng.below(10) {
                0..=5 => {
                    let k = rng.below(nkeys as u64) as usize;
                    ops.push(Op::Put(k, rng.byte()));
                    if k == inserted {
                        inserted += 1;
                    }
                }
                6 | 7 => ops.push(Op::Get(rng.below(nkeys as u64) as usize)),
                8 => ops.push(Op::GetTs(rng.below(nkeys as u64) as usize)),
                _ => {
                    if allow_relocating_delete {
                        ops.push(Op::Del(rng.below(nkeys as u64) as usize, 0));
                    } else {
                        ops.push(Op::Len);
                    }
                }
            }
        }
        run_ops(
            &format!("enum-sweep trial={trial} mode={mode} sh={sh} es={es}"),
            Drv::shmode(c, es, 8, mode, sh),
            Drv::shmode(r, es, 8, mode, sh),
            &ks,
            &ops,
        );
    }
}
