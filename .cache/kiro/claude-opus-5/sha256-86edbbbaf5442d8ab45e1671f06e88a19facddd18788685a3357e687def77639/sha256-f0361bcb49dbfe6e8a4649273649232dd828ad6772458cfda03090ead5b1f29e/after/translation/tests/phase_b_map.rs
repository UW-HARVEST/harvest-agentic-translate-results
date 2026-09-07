//! Phase B — hash-map entry points driven exactly the way the `hmput`/`hmget`/
//! `hmdel`/`shput`/`shget`/`shdel` macros drive them, at the lowest level:
//! `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`,
//! `stbds_hmput_default`, `stbds_hmdel_key`, `stbds_shmode_func`,
//! `stbds_hmfree_func`.
//!
//! CONFIGS.md rows 12, 20-57, 67-69.

mod common;
use common::*;
use std::ffi::c_int;

const SEED: u64 = 0xC0FFEE;
const PIN: usize = 0x3141_5926;

const HM_BINARY: c_int = 0;
const HM_STRING: c_int = 1;

const SH_NONE: c_int = 0;
const SH_DEFAULT: c_int = 1;
const SH_STRDUP: c_int = 2;
const SH_ARENA: c_int = 3;

fn key4(v: u32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

/// CONFIGS rows 20-22: `stbds_hmput_default`.
#[test]
fn row20_22_hmput_default() {
    let _g = lock();
    // row 20: a == NULL
    let mut m = MapPair::new(8, 4, KeyKind::Bytes);
    m.put_default();
    let (a, _) = m.snap();
    assert!(!a.ptr_null);
    assert_eq!(a.length, 1, "hmput_default(NULL) length");
    assert!(!a.has_table, "hmput_default must not create a table");

    // row 22: a != NULL, length != 0 -> unchanged
    let before = m.snap().0;
    m.put_default();
    assert_eq!(m.snap().0, before, "hmput_default on length!=0 must be a no-op");

    // row 21: a != NULL, length == 0 -> re-grow + re-zero
    unsafe {
        let ch = (m.c as *mut u8).sub(m.elemsize).sub(HEADER_SIZE) as *mut RawArrayHeader;
        let rh = (m.r as *mut u8).sub(m.elemsize).sub(HEADER_SIZE) as *mut RawArrayHeader;
        (*ch).length = 0;
        (*rh).length = 0;
    }
    m.put_default();
    let (a, _) = m.snap();
    assert_eq!(a.length, 1, "hmput_default(length==0) must restore length 1");
    m.free();
}

/// CONFIGS rows 56-57: `stbds_shmode_func` incl. out-of-range `mode`.
#[test]
fn row56_57_shmode_func() {
    let _g = lock();
    let (c, r) = libs();
    let modes: [c_int; 12] = [0, 1, 2, 3, 4, 5, 255, 256, 259, -1, c_int::MIN, c_int::MAX];
    for elemsize in [8usize, 16, 24] {
        for mode in modes {
            unsafe {
                (c.rand_seed)(PIN);
                (r.rand_seed)(PIN);
                let ct = (c.shmode_func)(elemsize, mode);
                let rt = (r.shmode_func)(elemsize, mode);
                let a = snapshot_map(ct, elemsize, 0, KeyKind::Bytes);
                let b = snapshot_map(rt, elemsize, 0, KeyKind::Bytes);
                assert_eq!(a, b, "shmode_func({elemsize}, {mode})");
                assert_eq!(
                    a.string_mode,
                    (mode as u32 & 0xFF) as u8,
                    "shmode_func truncating cast for mode={mode}"
                );
                assert_eq!(a.length, 1);
                assert_eq!(a.slot_count, 8);
                (c.hmfree_func)((ct as *mut u8).sub(elemsize) as *mut _, elemsize);
                (r.hmfree_func)((rt as *mut u8).sub(elemsize) as *mut _, elemsize);
            }
        }
    }
}

/// CONFIGS row 12: the global seed sequence — each fresh table consumes and
/// advances `stbds_hash_seed`, so the Nth table gets a different seed.
#[test]
fn row12_global_seed_sequence() {
    let _g = lock();
    let (c, r) = libs();
    for start in [0usize, 1, PIN, usize::MAX, usize::MAX >> 1] {
        unsafe {
            (c.rand_seed)(start);
            (r.rand_seed)(start);
            let mut cseeds = Vec::new();
            let mut rseeds = Vec::new();
            let mut alive = Vec::new();
            for _ in 0..12 {
                let ct = (c.shmode_func)(16, SH_NONE);
                let rt = (r.shmode_func)(16, SH_NONE);
                cseeds.push(snapshot_map(ct, 16, 0, KeyKind::Bytes).seed);
                rseeds.push(snapshot_map(rt, 16, 0, KeyKind::Bytes).seed);
                alive.push((ct, rt));
            }
            assert_eq!(cseeds, rseeds, "seed sequence from rand_seed({start:#x})");
            // and the sequence must actually advance (not a constant)
            assert!(cseeds.windows(2).any(|w| w[0] != w[1]), "seed never advanced");
            for (ct, rt) in alive {
                (c.hmfree_func)((ct as *mut u8).sub(16) as *mut _, 16);
                (r.hmfree_func)((rt as *mut u8).sub(16) as *mut _, 16);
            }
        }
    }
}

/// CONFIGS rows 23-25: binary mode, 1 / 5 / 6 elements (6 crosses the
/// `used_count_threshold` of 6 at `slot_count` 8 and forces a grow + rehash).
#[test]
fn row23_25_binary_small_and_grow() {
    let _g = lock();
    for n in [1usize, 2, 5, 6, 7, 8, 9] {
        let mut m = MapPair::new(8, 4, KeyKind::Bytes);
        m.pin_seed(PIN);
        for i in 0..n {
            m.put(&key4(i as u32 * 7 + 1), HM_BINARY, 0x40 + i as u8);
        }
        let (a, _) = m.snap();
        assert_eq!(a.length, n + 1, "n={n} length (index 0 is the default slot)");
        assert_eq!(a.used_count, n, "n={n} used_count");
        // The grow test happens *before* the insert, so with
        // used_count_threshold == 6 the 7th put is the one that grows.
        if n >= 7 {
            assert!(a.slot_count > 8, "n={n} must have grown past slot_count 8");
        } else {
            assert_eq!(a.slot_count, 8, "n={n} must not have grown");
        }
        for i in 0..n {
            let t = m.get(&key4(i as u32 * 7 + 1), HM_BINARY);
            assert!(t >= 0, "n={n} key {i} must be found");
        }
        m.free();
    }
}

/// CONFIGS rows 26-27: many randomized binary keys, with duplicates driving
/// the update-in-place path.
#[test]
fn row26_27_binary_many_and_duplicates() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x101);
    for &n in &[100usize, 500] {
        let mut m = MapPair::new(8, 4, KeyKind::Bytes);
        m.pin_seed(PIN);
        let mut keys = Vec::new();
        for i in 0..n {
            // ~25% duplicates once there is something to duplicate
            let k = if !keys.is_empty() && rng.below(4) == 0 {
                keys[rng.below(keys.len())]
            } else {
                let k = rng.next_u32();
                keys.push(k);
                k
            };
            m.put(&key4(k), HM_BINARY, (i & 0xFF) as u8);
        }
        // every distinct key must be findable, absent keys must report -1
        for &k in &keys {
            assert!(m.get(&key4(k), HM_BINARY) >= 0, "key {k} lost");
        }
        for _ in 0..200 {
            let probe = rng.next_u32();
            if !keys.contains(&probe) {
                assert_eq!(m.get(&key4(probe), HM_BINARY), -1, "absent key {probe}");
            }
        }
        m.free();
    }
}

/// CONFIGS row 28: `keysize` / `elemsize` shape matrix.
#[test]
fn row28_keysize_elemsize_matrix() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x202);
    for keysize in [1usize, 2, 3, 4, 5, 8, 12, 16] {
        for pad in [0usize, 4, 8] {
            let elemsize = keysize + pad;
            if elemsize == 0 {
                continue;
            }
            let mut m = MapPair::new(elemsize, keysize, KeyKind::Bytes);
            m.pin_seed(PIN);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..40usize {
                let k = if !keys.is_empty() && rng.below(5) == 0 {
                    keys[rng.below(keys.len())].clone()
                } else {
                    let k = rng.bytes(keysize);
                    keys.push(k.clone());
                    k
                };
                m.put(&k, HM_BINARY, (i * 3) as u8);
            }
            for k in &keys {
                assert!(m.get(k, HM_BINARY) >= 0, "keysize={keysize} elemsize={elemsize} lost {k:?}");
            }
            m.free();
        }
    }
}

/// CONFIGS row 29 (and ERRORS row 27, binary half): every `mode < 1` must take
/// the identical binary path.
#[test]
fn row29_binary_mode_aliases() {
    let _g = lock();
    let modes: [c_int; 5] = [0, -1, -2, c_int::MIN, -12345];
    let mut refsnap = None;
    for mode in modes {
        let mut m = MapPair::new(8, 4, KeyKind::Bytes);
        m.pin_seed(PIN);
        for i in 0..20u32 {
            m.put(&key4(i * 13 + 5), mode, i as u8);
        }
        for i in 0..20u32 {
            assert!(m.get(&key4(i * 13 + 5), mode) >= 0);
        }
        assert_eq!(m.get(&key4(999999), mode), -1);
        let s = m.snap().0;
        if let Some(prev) = &refsnap {
            assert_eq!(&s, prev, "mode={mode} must behave exactly like mode=0");
        } else {
            refsnap = Some(s);
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// String modes
// ---------------------------------------------------------------------------

/// Build a string-mode map for the given `string.mode`, using the right
/// construction path. `SH_DEFAULT` is what `hmput_key` sets implicitly on a
/// fresh NULL map in string mode; the rest need `shmode_func`.
fn string_map(sh_mode: c_int, elemsize: usize, implicit: bool) -> MapPair {
    if implicit {
        let mut m = MapPair::new(elemsize, 8, KeyKind::StrPtr);
        m.pin_seed(PIN);
        m
    } else {
        MapPair::with_shmode(elemsize, 8, KeyKind::StrPtr, sh_mode, PIN)
    }
}

/// CONFIGS rows 30-36: the three *pointer-storing* `string.mode`s × put/get,
/// with randomized strings, duplicates and enough elements to force grows.
/// Both `mode == 1` and the out-of-enum `mode == 2` are exercised: they behave
/// identically for put/get and differ only inside `hmdel_key`
/// (see `phase_c_errors.rs`).
///
/// `string.mode == SH_NONE` combined with a string `mode` is covered separately
/// in `row34_string_mode_with_sh_none`: that combination stores the key bytes
/// inline via the `default:` memcpy branch, so any `is_key_equal` call would
/// dereference those bytes as a `char *` (C undefined behaviour).
#[test]
fn row30_36_string_modes() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x303);
    // (sh_mode, use implicit SH_DEFAULT path)
    let variants: [(c_int, bool); 4] = [
        (SH_DEFAULT, true),
        (SH_DEFAULT, false),
        (SH_STRDUP, false),
        (SH_ARENA, false),
    ];
    for (sh, implicit) in variants {
        for &n in &[1usize, 5, 6, 20, 100] {
            for mode in [HM_STRING, 2 as c_int] {
                let mut m = string_map(sh, 16, implicit);
                let mut keys: Vec<Vec<u8>> = Vec::new();
                for i in 0..n {
                    let k = if !keys.is_empty() && rng.below(4) == 0 {
                        keys[rng.below(keys.len())].clone()
                    } else {
                        let k = { let n = 1 + rng.below(24); rng.cstring(n) };
                        if keys.iter().any(|e| e == &k) {
                            keys[0].clone()
                        } else {
                            keys.push(k.clone());
                            k
                        }
                    };
                    m.put(&k, mode, (i & 0xFF) as u8);
                }
                for k in &keys {
                    assert!(
                        m.get(k, mode) >= 0,
                        "sh={sh} implicit={implicit} n={n} mode={mode}: lost {:?}",
                        String::from_utf8_lossy(k)
                    );
                }
                // absent keys
                for _ in 0..20 {
                    let mut probe = rng.cstring(30);
                    probe[0] = b'Z';
                    if !keys.iter().any(|e| e == &probe) {
                        assert_eq!(m.get(&probe, mode), -1, "sh={sh}: absent key found");
                    }
                }
                m.free();
            }
        }
    }
}

/// CONFIGS row 34: string hashing (`mode >= 1`) on a table whose
/// `string.mode` is `SH_NONE`, so `hmput_key` takes the `default:`
/// `memcpy(elem, key, keysize)` branch instead of storing a pointer.
///
/// Only *puts of distinct keys into a fresh table* are performed: on that path
/// every probe terminates on an EMPTY slot, so `is_key_equal` is never called
/// and the behaviour is well defined. A get, or a duplicate put, would
/// dereference the inlined key bytes as a `char *` — C undefined behaviour,
/// deliberately not exercised.
#[test]
fn row34_string_mode_with_sh_none() {
    let _g = lock();
    for mode in [HM_STRING, 2 as c_int] {
        let mut m = MapPair::with_shmode(16, 8, KeyKind::Bytes, SH_NONE, PIN);
        for (i, k) in [
            &b"alpha\0\0\0"[..],
            b"bravo\0\0\0",
            b"charlie\0",
            b"delta\0\0\0",
            b"echo\0\0\0\0",
        ]
        .iter()
        .enumerate()
        {
            m.put(k, mode, 0x30 + i as u8);
        }
        let (a, _) = m.snap();
        assert_eq!(a.string_mode, SH_NONE as u8);
        assert_eq!(a.length, 6, "5 puts + default slot");
        m.free();
    }
}

/// CONFIGS row 32/62: `SH_ARENA` with strings long enough to drive the arena
/// block progression (`512 << (block>>1)`) and the oversize-block branch.
#[test]
fn row32_62_arena_block_progression() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x404);
    for &maxlen in &[8usize, 64, 600, 5000] {
        let mut m = MapPair::with_shmode(24, 8, KeyKind::StrPtr, SH_ARENA, PIN);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..80usize {
            let k = { let n = 1 + rng.below(maxlen); rng.cstring(n) };
            if keys.iter().any(|e| e == &k) {
                continue;
            }
            keys.push(k.clone());
            m.put(&k, HM_STRING, (i & 0xFF) as u8);
        }
        for k in &keys {
            assert!(m.get(k, HM_STRING) >= 0, "maxlen={maxlen}: lost key");
        }
        let (a, _) = m.snap();
        assert_eq!(a.string_mode, SH_ARENA as u8);
        m.free();
    }
}

/// CONFIGS rows 37-40: `hmget_key` / `hmget_key_ts` including the `a == NULL`
/// allocating branch (ERRORS rows 6-9).
#[test]
fn row37_40_get_paths() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x505);

    // row 40 / ERRORS row 6: get on a NULL map allocates and reports -1
    for mode in [HM_BINARY, HM_STRING] {
        let mut m = MapPair::new(16, if mode == HM_BINARY { 4 } else { 8 }, KeyKind::Bytes);
        m.pin_seed(PIN);
        let t = m.get_ts(b"abcd\0\0\0\0", mode);
        assert_eq!(t, -1, "get_ts on NULL map must yield -1 (mode={mode})");
        let (a, _) = m.snap();
        assert!(!a.ptr_null, "get_ts on NULL map must allocate");
        assert_eq!(a.length, 1);
        assert!(!a.has_table, "no table is created by get");
        // ERRORS row 7: table == 0 on a non-NULL map
        assert_eq!(m.get_ts(b"efgh\0\0\0\0", mode), -1);
        assert_eq!(m.get(b"efgh\0\0\0\0", mode), -1);
        m.free();
    }

    // rows 37-39 on populated tables of several sizes
    for &n in &[0usize, 1, 5, 6, 20, 100] {
        let mut m = MapPair::new(8, 4, KeyKind::Bytes);
        m.pin_seed(PIN);
        let mut keys = Vec::new();
        for i in 0..n {
            let k = rng.next_u32();
            keys.push(k);
            m.put(&key4(k), HM_BINARY, i as u8);
        }
        for &k in &keys {
            let a = m.get(&key4(k), HM_BINARY);
            let b = m.get_ts(&key4(k), HM_BINARY);
            assert_eq!(a, b, "hmget_key and hmget_key_ts disagree for {k}");
            assert!(a >= 0);
        }
        for _ in 0..50 {
            let p = rng.next_u32();
            if !keys.contains(&p) {
                assert_eq!(m.get(&key4(p), HM_BINARY), -1);
                assert_eq!(m.get_ts(&key4(p), HM_BINARY), -1);
            }
        }
        m.free();
    }
}

/// CONFIGS rows 41-43: binary deletes — non-last element (memmove + slot
/// re-find), last element, delete-all forwards and backwards.
#[test]
fn row41_43_binary_delete() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x606);

    // non-last and last element
    for n in [2usize, 3, 8, 20] {
        for target in 0..n {
            let mut m = MapPair::new(8, 4, KeyKind::Bytes);
            m.pin_seed(PIN);
            let keys: Vec<u32> = (0..n as u32).map(|i| i * 31 + 3).collect();
            for (i, &k) in keys.iter().enumerate() {
                m.put(&key4(k), HM_BINARY, i as u8);
            }
            let t = m.del(&key4(keys[target]), HM_BINARY);
            assert_eq!(t, 1, "n={n} target={target}: delete must report 1");
            assert_eq!(m.get(&key4(keys[target]), HM_BINARY), -1, "deleted key still present");
            for (i, &k) in keys.iter().enumerate() {
                if i != target {
                    assert!(m.get(&key4(k), HM_BINARY) >= 0, "n={n} target={target}: lost key {k}");
                }
            }
            m.free();
        }
    }

    // delete-all, forwards and reverse
    for reverse in [false, true] {
        let mut m = MapPair::new(8, 4, KeyKind::Bytes);
        m.pin_seed(PIN);
        let mut keys: Vec<u32> = (0..20u32).map(|_| rng.next_u32()).collect();
        keys.dedup();
        for (i, &k) in keys.iter().enumerate() {
            m.put(&key4(k), HM_BINARY, i as u8);
        }
        let order: Vec<u32> = if reverse {
            keys.iter().rev().copied().collect()
        } else {
            keys.clone()
        };
        for &k in &order {
            assert_eq!(m.del(&key4(k), HM_BINARY), 1, "delete-all reverse={reverse} key={k}");
        }
        let (a, _) = m.snap();
        assert_eq!(a.length, 1, "after delete-all only the default slot remains");
        assert_eq!(a.used_count, 0);
        m.free();
    }
}

/// CONFIGS row 44: shrink path (`used_count < used_count_shrink_threshold &&
/// slot_count > 8`).
#[test]
fn row44_delete_shrink() {
    let _g = lock();
    let mut m = MapPair::new(8, 4, KeyKind::Bytes);
    m.pin_seed(PIN);
    let keys: Vec<u32> = (0..60u32).map(|i| i * 977 + 11).collect();
    for (i, &k) in keys.iter().enumerate() {
        m.put(&key4(k), HM_BINARY, i as u8);
    }
    let big = m.snap().0.slot_count;
    assert!(big >= 64, "expected several grows, got slot_count={big}");
    let mut shrank = false;
    for &k in &keys {
        m.del(&key4(k), HM_BINARY);
        if m.snap().0.slot_count < big {
            shrank = true;
        }
    }
    assert!(shrank, "shrink path was never taken");
    let (a, _) = m.snap();
    assert_eq!(a.slot_count, 8, "should have shrunk all the way back to 8");
    m.free();
}

/// CONFIGS rows 45-46: tombstone rebuild at constant `slot_count`, and
/// tombstone reuse by a subsequent put.
#[test]
fn row45_46_tombstones() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x707);
    for round in 0..8usize {
        let mut m = MapPair::new(8, 4, KeyKind::Bytes);
        m.pin_seed(PIN.wrapping_add(round));
        // fill to a large table, then churn: delete one + insert one, keeping
        // used_count high so the shrink path is not taken and tombstones
        // accumulate until tombstone_count_threshold triggers a rebuild.
        let mut live: Vec<u32> = Vec::new();
        for i in 0..80u32 {
            let k = i * 7919 + 17;
            live.push(k);
            m.put(&key4(k), HM_BINARY, i as u8);
        }
        let mut saw_rebuild = false;
        let mut saw_tombstone_reuse = false;
        for i in 0..400usize {
            let idx = rng.below(live.len());
            let victim = live.swap_remove(idx);
            let before = m.snap().0;
            m.del(&key4(victim), HM_BINARY);
            let after = m.snap().0;
            if after.tombstone_count < before.tombstone_count && after.slot_count == before.slot_count {
                saw_rebuild = true;
            }
            let fresh = 1_000_000 + i as u32 * 13;
            let b2 = m.snap().0;
            m.put(&key4(fresh), HM_BINARY, (i & 0xFF) as u8);
            let a2 = m.snap().0;
            if a2.tombstone_count < b2.tombstone_count {
                saw_tombstone_reuse = true;
            }
            live.push(fresh);
            for &k in live.iter().take(8) {
                assert!(m.get(&key4(k), HM_BINARY) >= 0, "churn lost key {k}");
            }
        }
        if round == 0 {
            assert!(saw_rebuild, "tombstone rebuild path never taken");
            assert!(saw_tombstone_reuse, "tombstone reuse path never taken");
        }
        m.free();
    }
}

/// CONFIGS rows 47-50: string-mode deletes for every `string.mode`, plus the
/// `mode == 1` vs `mode >= 2` split inside `hmdel_key`.
#[test]
fn row47_50_string_delete() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x808);
    // NOTE: only `mode == STBDS_HM_STRING` (1) is delete-safe. `hmdel_key`
    // splits on `mode == STBDS_HM_STRING` *exactly*, so with mode >= 2 the
    // post-memmove slot re-find passes the raw element bytes instead of the
    // stored `char *`, the lookup fails and the C aborts on
    // `STBDS_ASSERT(slot >= 0)`. That is ERRORS.md rows 20-21 and is verified
    // for abort parity in `phase_c_errors.rs`.
    for sh in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for mode in [HM_STRING] {
            for n in [1usize, 2, 8, 30] {
                let mut m = MapPair::with_shmode(24, 8, KeyKind::StrPtr, sh, PIN);
                let mut keys: Vec<Vec<u8>> = Vec::new();
                while keys.len() < n {
                    let k = { let n = 1 + rng.below(20); rng.cstring(n) };
                    if !keys.iter().any(|e| e == &k) {
                        keys.push(k);
                    }
                }
                for (i, k) in keys.iter().enumerate() {
                    m.put(k, mode, i as u8);
                }
                // delete every key, checking state after each step
                for (i, k) in keys.iter().enumerate() {
                    let t = m.del(k, mode);
                    assert_eq!(t, 1, "sh={sh} mode={mode} n={n} i={i}: delete must report 1");
                    assert_eq!(m.get(k, mode), -1, "deleted string key still present");
                }
                let (a, _) = m.snap();
                assert_eq!(a.length, 1);
                m.free();
            }
        }
    }
}

/// CONFIGS rows 52-55: `stbds_hmfree_func` for every `string.mode`, plus the
/// no-table case (ERRORS row 2).
#[test]
fn row52_55_hmfree_all_modes() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0x909);
    // populated tables in every string mode (freeing must not crash and must
    // leave identical state up to the free)
    for sh in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        // SH_NONE inlines the key (default: memcpy), so it must be driven with
        // a binary `mode`; the pointer-storing modes need a string `mode`.
        let (kind, mode) = if sh == SH_NONE {
            (KeyKind::Bytes, HM_BINARY)
        } else {
            (KeyKind::StrPtr, HM_STRING)
        };
        let mut m = MapPair::with_shmode(24, 8, kind, sh, PIN);
        let mut seen: Vec<Vec<u8>> = Vec::new();
        for i in 0..30usize {
            let k = if mode == HM_BINARY {
                rng.bytes(8)
            } else {
                let n = 1 + rng.below(40);
                rng.cstring(n)
            };
            if seen.iter().any(|e| e == &k) {
                continue;
            }
            seen.push(k.clone());
            m.put(&k, mode, i as u8);
        }
        m.assert_same("before hmfree");
        m.free();
    }
    // binary map
    let mut m = MapPair::new(8, 4, KeyKind::Bytes);
    m.pin_seed(PIN);
    for i in 0..30u32 {
        m.put(&key4(i * 101 + 7), HM_BINARY, i as u8);
    }
    m.free();

    // ERRORS row 1: hmfree_func(NULL, elemsize) is a no-op
    let (c, r) = libs();
    unsafe {
        (c.hmfree_func)(std::ptr::null_mut(), 8);
        (r.hmfree_func)(std::ptr::null_mut(), 8);
    }

    // ERRORS row 2: array with no hash table (from hmput_default)
    let mut m = MapPair::new(8, 4, KeyKind::Bytes);
    m.put_default();
    let (a, _) = m.snap();
    assert!(!a.has_table);
    m.free();
}

/// CONFIGS rows 67 + 69: randomized binary op stream over many shapes, with a
/// full state comparison after every single operation.
#[test]
fn row67_69_random_pipeline_binary() {
    let _g = lock();
    for start_seed in [0usize, 1, PIN, usize::MAX] {
        let mut rng = Rng::new(SEED ^ 0xA0A ^ start_seed as u64);
        for (elemsize, keysize) in [(8usize, 4usize), (4, 4), (16, 8), (5, 1), (12, 3), (16, 16), (24, 12), (9, 8)] {
            let mut m = MapPair::new(elemsize, keysize, KeyKind::Bytes);
            m.pin_seed(start_seed);
            let mut live: Vec<Vec<u8>> = Vec::new();
            for step in 0..250usize {
                let op = rng.below(10);
                match op {
                    0..=4 => {
                        // put (sometimes a duplicate)
                        let k = if !live.is_empty() && rng.below(3) == 0 {
                            live[rng.below(live.len())].clone()
                        } else {
                            let k = rng.bytes(keysize);
                            if !live.iter().any(|e| e == &k) {
                                live.push(k.clone());
                            }
                            k
                        };
                        m.put(&k, HM_BINARY, (step & 0xFF) as u8);
                    }
                    5..=6 => {
                        let k = if !live.is_empty() && rng.below(2) == 0 {
                            live[rng.below(live.len())].clone()
                        } else {
                            rng.bytes(keysize)
                        };
                        m.get(&k, HM_BINARY);
                    }
                    7 => {
                        let k = if !live.is_empty() && rng.below(2) == 0 {
                            live[rng.below(live.len())].clone()
                        } else {
                            rng.bytes(keysize)
                        };
                        m.get_ts(&k, HM_BINARY);
                    }
                    8 => {
                        if !live.is_empty() {
                            let i = rng.below(live.len());
                            let k = live.swap_remove(i);
                            m.del(&k, HM_BINARY);
                        }
                    }
                    _ => {
                        // delete an absent key
                        let k = rng.bytes(keysize);
                        if !live.iter().any(|e| e == &k) {
                            m.del(&k, HM_BINARY);
                        }
                    }
                }
            }
            m.free();
        }
    }
}

/// CONFIGS rows 68 + 69: the same randomized stream in string mode, for every
/// `string.mode` and both `mode == 1` and `mode == 2`.
#[test]
fn row68_69_random_pipeline_string() {
    let _g = lock();
    for start_seed in [0usize, PIN, usize::MAX] {
        let mut rng = Rng::new(SEED ^ 0xB0B ^ start_seed as u64);
        for sh in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for mode in [HM_STRING, 2 as c_int] {
                for elemsize in [16usize, 24] {
                    let mut m = MapPair::with_shmode(elemsize, 8, KeyKind::StrPtr, sh, start_seed);
                    let mut live: Vec<Vec<u8>> = Vec::new();
                    for step in 0..150usize {
                        let op = rng.below(10);
                        match op {
                            0..=4 => {
                                let k = if !live.is_empty() && rng.below(3) == 0 {
                                    live[rng.below(live.len())].clone()
                                } else {
                                    let k = { let n = 1 + rng.below(30); rng.cstring(n) };
                                    if !live.iter().any(|e| e == &k) {
                                        live.push(k.clone());
                                    }
                                    k
                                };
                                m.put(&k, mode, (step & 0xFF) as u8);
                            }
                            5..=7 => {
                                let k = if !live.is_empty() && rng.below(2) == 0 {
                                    live[rng.below(live.len())].clone()
                                } else {
                                    { let n = 1 + rng.below(30); rng.cstring(n) }
                                };
                                if rng.below(2) == 0 {
                                    m.get(&k, mode);
                                } else {
                                    m.get_ts(&k, mode);
                                }
                            }
                            8 => {
                                // deletes are only well defined for mode == 1
                                // (see ERRORS.md rows 20-21)
                                if mode == HM_STRING && !live.is_empty() {
                                    let i = rng.below(live.len());
                                    let k = live.swap_remove(i);
                                    m.del(&k, mode);
                                }
                            }
                            _ => {
                                let k = { let n = 1 + rng.below(30); rng.cstring(n) };
                                if mode == HM_STRING && !live.iter().any(|e| e == &k) {
                                    m.del(&k, mode);
                                }
                            }
                        }
                    }
                    m.free();
                }
            }
        }
    }
}

/// CONFIGS row 36 (the `temp_key` half): `stbds_hmput_key` writes
/// `stbds_temp_key(a)` — the field the `shputs` / `shgets` macros read back —
/// on three of its four exits, and deliberately NOT on the "found an existing
/// key in the wrap-around half of the probe" exit.
///
/// IMPORTANT: `stbds_make_hash_index` never initialises `temp_key`, so the
/// field is indeterminate in the C original until some put writes it. A put
/// that both grows the table *and* finds an existing key would therefore leave
/// it uninitialised, and reading it would be undefined behaviour. The test
/// keeps `used_count` strictly below `used_count_threshold` before every
/// duplicate put, which guarantees no grow can coincide with a found-key exit,
/// and asserts that the table was not re-created.
#[test]
fn row36_temp_key_parity() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xD0D);
    for sh in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for start in [0usize, 1, PIN, usize::MAX] {
            let elemsize = 24usize;
            let mut m = MapPair::with_shmode(elemsize, 8, KeyKind::StrPtr, sh, start);

            let cmp_temp_key = |m: &MapPair, what: &str| unsafe {
                let ck = temp_key_string(m.c, elemsize);
                let rk = temp_key_string(m.r, elemsize);
                assert_eq!(ck, rk, "sh={sh} start={start:#x}: temp_key differs {what}");
                assert!(ck.is_some(), "temp_key should have been written {what}");
            };

            // Phase 1: distinct keys only. Every one takes the
            // `found_empty_slot` exit, which always writes temp_key (grows are
            // fine here).
            let mut keys: Vec<Vec<u8>> = Vec::new();
            while keys.len() < 120 {
                let n = 1 + rng.below(20);
                let k = rng.cstring(n);
                if keys.iter().any(|e| e == &k) {
                    continue;
                }
                keys.push(k.clone());
                m.put(&k, HM_STRING, (keys.len() & 0xFF) as u8);
                cmp_temp_key(&m, &format!("after distinct put #{}", keys.len()));
            }

            // Phase 2: make sure a duplicate put cannot trigger a grow, then do
            // many duplicate puts to hit the found-key exits (both halves).
            let mut extra = 0u32;
            while {
                let s = m.snap().0;
                s.used_count >= s.used_count_threshold
            } {
                let k = format!("filler_{extra}\0").into_bytes();
                extra += 1;
                if keys.iter().any(|e| e == &k) {
                    continue;
                }
                keys.push(k.clone());
                m.put(&k, HM_STRING, 0x99);
                assert!(extra < 100, "could not get used_count below the threshold");
            }
            for i in 0..400usize {
                let k = keys[rng.below(keys.len())].clone();
                let (ct, rt) = unsafe { (table_ptr(m.c, elemsize), table_ptr(m.r, elemsize)) };
                m.put(&k, HM_STRING, (i & 0xFF) as u8);
                let (ct2, rt2) = unsafe { (table_ptr(m.c, elemsize), table_ptr(m.r, elemsize)) };
                assert_eq!(ct, ct2, "duplicate put unexpectedly re-created the C table");
                assert_eq!(rt, rt2, "duplicate put unexpectedly re-created the Rust table");
                cmp_temp_key(&m, &format!("after duplicate put #{i}"));
            }

            // hmget_key never touches temp_key, so it must stay put.
            for k in keys.iter().take(30) {
                let before = unsafe { temp_key_string(m.c, elemsize) };
                m.get(k, HM_STRING);
                cmp_temp_key(&m, "after get");
                assert_eq!(
                    unsafe { temp_key_string(m.c, elemsize) },
                    before,
                    "hmget_key must not change temp_key"
                );
            }
            m.free();
        }
    }
}

/// CONFIGS rows 26/43/44/45 extension: a large map (2000+ entries, slot_count
/// up to 4096) driven through repeated grow / churn / shrink cycles. This is the
/// only test that reaches `slot_count_log2` > 8 and the deep
/// `pos += step; step += 8` probe continuation across many buckets.
#[test]
fn row26_44_large_map_stress() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xE0E);
    for start in [PIN, 0usize] {
        let mut m = MapPair::new(8, 4, KeyKind::Bytes);
        m.pin_seed(start);
        let mut live: Vec<u32> = Vec::new();
        // grow phase
        for i in 0..2000u32 {
            let k = i.wrapping_mul(2_654_435_761);
            if live.contains(&k) {
                continue;
            }
            live.push(k);
            m.put(&key4(k), HM_BINARY, (i & 0xFF) as u8);
        }
        let peak = m.snap().0;
        assert!(peak.slot_count >= 4096, "expected slot_count >= 4096, got {}", peak.slot_count);
        assert!(peak.slot_count_log2 >= 12);
        // spot-check lookups (full-state compare happens inside every call)
        for &k in live.iter().step_by(37) {
            assert!(m.get(&key4(k), HM_BINARY) >= 0, "large map lost key {k}");
        }
        for i in 0..200u32 {
            let probe = 0xF000_0000u32 ^ i;
            if !live.contains(&probe) {
                assert_eq!(m.get(&key4(probe), HM_BINARY), -1);
            }
        }
        // churn phase: alternate delete/insert to build tombstones at a large
        // slot_count, then drain everything to force repeated shrinks
        for i in 0..300usize {
            let idx = rng.below(live.len());
            let victim = live.swap_remove(idx);
            m.del(&key4(victim), HM_BINARY);
            let fresh = 0xA000_0000u32 + i as u32;
            live.push(fresh);
            m.put(&key4(fresh), HM_BINARY, (i & 0xFF) as u8);
        }
        while let Some(k) = live.pop() {
            m.del(&key4(k), HM_BINARY);
        }
        let (a, _) = m.snap();
        assert_eq!(a.length, 1, "drain must leave only the default slot");
        assert_eq!(a.used_count, 0);
        assert_eq!(a.slot_count, 8, "must shrink all the way back to 8");
        m.free();
    }
}
