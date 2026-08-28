//! Phase B — CONFIGS.md rows 14..15 and 34..75
//! `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmget_key_ts` /
//! `stbds_hmput_default` / `stbds_hmdel_key` / `stbds_shmode_func` /
//! `stbds_hmfree_func`

mod common;
use common::*;
use std::ffi::c_int;

/// String modes for which an element's first 8 bytes are a `char *`.
fn repr_for(str_mode: u8) -> KeyRepr {
    match str_mode {
        SH_DEFAULT | SH_STRDUP | SH_ARENA => KeyRepr::Pointer,
        _ => KeyRepr::Raw,
    }
}

// ===========================================================================
// rows 14..15 — seed handling
// ===========================================================================

#[test]
fn c14_table_seed_and_evolution() {
    let mut rng = Rng::new(0x1414);
    let mut seeds = vec![0usize, 1, DEFAULT_SEED, usize::MAX, 2];
    for _ in 0..32 {
        seeds.push(rng.next_u64() as usize);
    }
    for s in seeds {
        let _g = seed_guard(s);
        let p = pair();
        let mut expect = s;
        let mut ts = Vec::new();
        unsafe {
            for i in 0..16 {
                let tc = (p.c.shmode_func)(16, SH_STRDUP as c_int);
                let tr = (p.r.shmode_func)(16, SH_STRDUP as c_int);
                let sc = snap_map(tc, 16, KeyRepr::Pointer);
                let sr = snap_map(tr, 16, KeyRepr::Pointer);
                assert_eq!(sc, sr, "table {} for seed {:#x}", i, s);
                let got = sc.table.as_ref().unwrap().seed;
                assert_eq!(got, expect, "table {} seed for start {:#x}", i, s);
                expect = next_hash_seed(expect);
                ts.push((tc, tr));
            }
            for (tc, tr) in ts {
                (p.c.hmfree_func)((tc as *mut u8).sub(16) as *mut _, 16);
                (p.r.hmfree_func)((tr as *mut u8).sub(16) as *mut _, 16);
            }
        }
    }
}

#[test]
fn c15_reseed_mid_sequence() {
    let p = pair();
    let mut rng = Rng::new(0x1515);
    for _ in 0..32 {
        let s0 = rng.next_u64() as usize;
        let s1 = rng.next_u64() as usize;
        let _g = seed_guard(s0);
        unsafe {
            let a = (p.c.shmode_func)(16, SH_ARENA as c_int);
            let ar = (p.r.shmode_func)(16, SH_ARENA as c_int);
            assert_eq!(
                snap_map(a, 16, KeyRepr::Pointer),
                snap_map(ar, 16, KeyRepr::Pointer)
            );
            // reseed both and observe the next table picking up the new value
            reseed(s1);
            let b = (p.c.shmode_func)(16, SH_ARENA as c_int);
            let br = (p.r.shmode_func)(16, SH_ARENA as c_int);
            let sb = snap_map(b, 16, KeyRepr::Pointer);
            assert_eq!(sb, snap_map(br, 16, KeyRepr::Pointer));
            assert_eq!(sb.table.as_ref().unwrap().seed, s1);
            for t in [a, b] {
                (p.c.hmfree_func)((t as *mut u8).sub(16) as *mut _, 16);
            }
            for t in [ar, br] {
                (p.r.hmfree_func)((t as *mut u8).sub(16) as *mut _, 16);
            }
        }
    }
}

// ===========================================================================
// BINARY-mode helpers
// ===========================================================================

/// Insert `n` distinct random keys, verify state after every step, then look
/// every key up (present) plus `n` absent keys, then delete in `order`.
#[allow(clippy::too_many_arguments)]
unsafe fn binary_scenario(
    elemsize: usize,
    keysize: usize,
    mode: c_int,
    n: usize,
    rng: &mut Rng,
    ctx: &str,
    delete_order: Option<&[usize]>,
) {
    let _g = seed_guard(DEFAULT_SEED);
    let mut mp = MapPair::new(elemsize, KeyRepr::Raw, ctx).with_value_offset(keysize);
    mp.check("initial (NULL)");

    let mut keys: Vec<Vec<u8>> = Vec::new();
    let mut seen: std::collections::HashSet<Vec<u8>> = std::collections::HashSet::new();
    while keys.len() < n {
        let k = rng.bytes(keysize.max(1));
        if keysize > 0 && !seen.insert(k[..keysize].to_vec()) {
            continue;
        }
        keys.push(k);
        if keysize == 0 {
            break; // every key is "equal", one is enough for distinctness
        }
    }

    for k in keys.iter_mut() {
        let idx = same_idx(ctx, mp.put(k, keysize, mode));
        assert!(idx >= 0, "put must return a valid index");
    }

    // every key must be found, at the same index in both
    for (i, k) in keys.iter_mut().enumerate() {
        let idx = same_idx(ctx, mp.get(k, keysize, mode));
        assert!(idx >= 0, "key {} must be found", i);
        let ts = same_idx(ctx, mp.get_ts(k, keysize, mode));
        assert_eq!(ts, idx, "hmget_key and hmget_key_ts must agree");
    }

    // absent keys must all miss
    if keysize > 0 {
        for _ in 0..n.max(8) {
            let mut k = rng.bytes(keysize.max(1));
            if seen.contains(&k[..keysize].to_vec()) {
                continue;
            }
            let idx = same_idx(ctx, mp.get(&mut k, keysize, mode));
            assert_eq!(idx, -1, "absent key must miss");
        }
    }

    if let Some(order) = delete_order {
        for &i in order {
            let mut k = keys[i].clone();
            same_idx(ctx, mp.del(&mut k, keysize, 0, mode));
        }
    }

    mp.free();
}

// -------------------------------------------------------------------- row 34
#[test]
fn c34_binary_small_counts() {
    let mut rng = Rng::new(0x3434);
    for n in [1usize, 2, 3, 4, 5, 6, 7, 8, 9, 12, 13] {
        for rep in 0..6 {
            unsafe {
                binary_scenario(
                    8,
                    4,
                    HM_BINARY,
                    n,
                    &mut rng,
                    &format!("bin n={} rep={}", n, rep),
                    None,
                );
            }
        }
    }
}

// -------------------------------------------------------------------- row 35
#[test]
fn c35_binary_two_hundred() {
    let mut rng = Rng::new(0x3535);
    for rep in 0..4 {
        unsafe {
            binary_scenario(16, 8, HM_BINARY, 200, &mut rng, &format!("bin200 {}", rep), None);
        }
    }
}

// -------------------------------------------------------------------- row 36
#[test]
fn c36_binary_elemsize_keysize_cross_product() {
    let mut rng = Rng::new(0x3636);
    for &elemsize in &[8usize, 12, 16, 24, 32, 40, 64] {
        for &keysize in &[1usize, 2, 3, 4, 8, 16] {
            if keysize > elemsize {
                continue;
            }
            unsafe {
                binary_scenario(
                    elemsize,
                    keysize,
                    HM_BINARY,
                    40,
                    &mut rng,
                    &format!("es={} ks={}", elemsize, keysize),
                    None,
                );
            }
        }
    }
}

// -------------------------------------------------------------------- row 37
#[test]
fn c37_binary_duplicates() {
    let mut rng = Rng::new(0x3737);
    for rep in 0..8 {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(16, KeyRepr::Raw, format!("dups {}", rep)).with_value_offset(8);
            let mut keys: Vec<Vec<u8>> = (0..30).map(|_| rng.bytes(8)).collect();
            let mut first_idx = std::collections::HashMap::new();
            for round in 0..3 {
                for (i, k) in keys.iter_mut().enumerate() {
                    let idx = same_idx("dups", mp.put(k, 8, HM_BINARY));
                    if round == 0 {
                        first_idx.insert(i, idx);
                    } else {
                        assert_eq!(
                            first_idx[&i], idx,
                            "re-putting an existing key must return its old index"
                        );
                    }
                }
            }
            assert_eq!(mp.len_from_c(), 31, "30 distinct keys + default element");
            mp.free();
        }
    }
}

impl MapPair {
    fn len_from_c(&self) -> usize {
        unsafe {
            if self.tc.is_null() {
                0
            } else {
                (*(((self.tc as *mut u8).sub(self.elemsize)).sub(HEADER_SIZE) as *mut ArrayHeader))
                    .length
            }
        }
    }
}

// -------------------------------------------------------------------- row 38
#[test]
fn c38_binary_zero_keysize() {
    let mut rng = Rng::new(0x3838);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "keysize=0").with_value_offset(0);
        // memcmp(...,0) == 0, so every key compares equal to the first one
        let mut idxs = Vec::new();
        for _ in 0..20 {
            let mut k = rng.bytes(8);
            idxs.push(same_idx("ks0", mp.put(&mut k, 0, HM_BINARY)));
        }
        assert!(idxs.iter().all(|&i| i == idxs[0]), "all keys collapse: {:?}", idxs);
        assert_eq!(mp.len_from_c(), 2, "default element + exactly one entry");
        let mut probe = rng.bytes(8);
        assert_eq!(same_idx("ks0", mp.get(&mut probe, 0, HM_BINARY)), idxs[0]);
        same_idx("ks0", mp.del(&mut probe, 0, 0, HM_BINARY));
        assert_eq!(mp.len_from_c(), 1);
        mp.free();
    }
}

// -------------------------------------------------------------------- row 39
#[test]
fn c39_negative_modes_are_binary() {
    let mut rng = Rng::new(0x3939);
    for &mode in &[c_int::MIN, -1, -7, 0] {
        unsafe {
            binary_scenario(
                16,
                8,
                mode,
                40,
                &mut rng,
                &format!("mode={}", mode),
                None,
            );
        }
    }
}

// ------------------------------------------------------------- rows 40,41,42
#[test]
fn c40_c41_get_present_and_absent() {
    let mut rng = Rng::new(0x4040);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "get-mix").with_value_offset(8);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..60 {
            let mut k = rng.bytes(8);
            keys.push(k.clone());
            let idx = same_idx("get-mix", mp.put(&mut k, 8, HM_BINARY));
            let _ = idx;
            // after every insert, all previous keys must still resolve
            for (j, kk) in keys.iter_mut().enumerate() {
                let a = same_idx("get-mix", mp.get(kk, 8, HM_BINARY));
                assert!(a >= 0, "key {} lost after inserting {}", j, i);
                let b = same_idx("get-mix", mp.get_ts(kk, 8, HM_BINARY));
                assert_eq!(a, b);
            }
            // and a fresh random key must miss
            let mut miss = rng.bytes(8);
            assert_eq!(same_idx("get-mix", mp.get(&mut miss, 8, HM_BINARY)), -1);
            assert_eq!(same_idx("get-mix", mp.get_ts(&mut miss, 8, HM_BINARY)), -1);
        }
        mp.free();
    }
}

#[test]
fn c42_get_on_table_less_map() {
    let _g = seed_guard(DEFAULT_SEED);
    let p = pair();
    unsafe {
        for &elemsize in &[8usize, 16, 24] {
            let mut mp = MapPair::new(elemsize, KeyRepr::Raw, format!("no-table es={}", elemsize));
            mp.put_default(); // creates the array but NOT the hash index
            let sc = snap_map(mp.tc, elemsize, KeyRepr::Raw);
            assert!(!sc.has_table, "hmput_default must not create a hash index");
            let mut k = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
            assert_eq!(same_idx("no-table", mp.get(&mut k, 8, HM_BINARY)), -1);
            assert_eq!(same_idx("no-table", mp.get_ts(&mut k, 8, HM_BINARY)), -1);
            // hmdel on a table-less map sets temp = 0 and returns a unchanged
            let before = mp.tc;
            let (dc, dr) = mp.del(&mut k, 8, 0, HM_BINARY);
            assert_eq!(dc, 0);
            assert_eq!(dr, 0);
            assert_eq!(mp.tc, before);
            let _ = p;
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 43
#[test]
fn c43_keysize_equals_elemsize() {
    let mut rng = Rng::new(0x4343);
    for &elemsize in &[8usize, 16, 24, 32] {
        unsafe {
            binary_scenario(
                elemsize,
                elemsize,
                HM_BINARY,
                40,
                &mut rng,
                &format!("ks==es={}", elemsize),
                None,
            );
        }
    }
}

// ===========================================================================
// STRING-mode helpers
// ===========================================================================

/// `str_mode == None` means "let `stbds_hmput_key` create the table implicitly"
/// (which yields `SH_DEFAULT` for `mode >= STBDS_HM_STRING`).
unsafe fn string_scenario(
    elemsize: usize,
    mode: c_int,
    str_mode: Option<u8>,
    n: usize,
    key_len: impl Fn(usize, &mut Rng) -> usize,
    alphabet: &[u8],
    rng: &mut Rng,
    ctx: &str,
    do_lookups: bool,
) {
    let _g = seed_guard(DEFAULT_SEED);
    let effective_mode = str_mode.unwrap_or(SH_DEFAULT);
    let repr = repr_for(effective_mode);
    let mut mp = MapPair::new(elemsize, repr, ctx).with_value_offset(8.min(elemsize));
    if let Some(m) = str_mode {
        mp.shmode(m as c_int);
    }
    mp.put_default();

    // key storage must stay put for the whole scenario (SH_DEFAULT keeps the
    // caller's pointers)
    let mut keys: Vec<Vec<u8>> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut guard = 0;
    while keys.len() < n {
        guard += 1;
        assert!(guard < 100_000, "could not generate enough distinct keys");
        let len = key_len(keys.len(), rng);
        let k = rng.cstring(len, alphabet);
        if !seen.insert(k.clone()) {
            continue;
        }
        keys.push(k);
    }

    for k in keys.iter_mut() {
        let idx = same_idx(ctx, mp.put(k, 8, mode));
        assert!(idx >= 0);
        // `table->temp_key` is only *written* by the SH_DEFAULT / SH_STRDUP /
        // SH_ARENA branches of `stbds_hmput_key`.  For every other
        // `string.mode` the field stays whatever `malloc` left there, so it is
        // not an observable value and must not be dereferenced.
        if repr == KeyRepr::Pointer {
            mp.check_temp_key("put");
        }
    }

    if do_lookups {
        for (i, k) in keys.iter_mut().enumerate() {
            let idx = same_idx(ctx, mp.get(k, 8, mode));
            assert!(idx >= 0, "key {} must be found", i);
            let ts = same_idx(ctx, mp.get_ts(k, 8, mode));
            assert_eq!(ts, idx);
        }
        // misses
        for j in 0..8 {
            let mut k = format!("__absent_{}__\0", j).into_bytes();
            assert_eq!(same_idx(ctx, mp.get(&mut k, 8, mode)), -1);
        }
    }

    mp.free();
}

// -------------------------------------------------------------------- row 44
#[test]
fn c44_string_implicit_default_mode() {
    let mut rng = Rng::new(0x4444);
    for n in [1usize, 2, 6, 7, 8, 50] {
        unsafe {
            string_scenario(
                16,
                HM_STRING,
                None,
                n,
                |_, r| 1 + r.below(20),
                ASCII,
                &mut rng,
                &format!("implicit n={}", n),
                true,
            );
        }
    }
}

// -------------------------------------------------------------------- row 45
#[test]
fn c45_string_strdup_mode() {
    let mut rng = Rng::new(0x4545);
    unsafe {
        string_scenario(
            16,
            HM_STRING,
            Some(SH_STRDUP),
            100,
            |_, r| 1 + r.below(40),
            ASCII,
            &mut rng,
            "strdup",
            true,
        );
    }
}

// -------------------------------------------------------------------- row 46
#[test]
fn c46_string_arena_mode() {
    let mut rng = Rng::new(0x4646);
    unsafe {
        string_scenario(
            16,
            HM_STRING,
            Some(SH_ARENA),
            100,
            |_, r| 1 + r.below(40),
            ASCII,
            &mut rng,
            "arena",
            true,
        );
        // keys long enough to take the arena's oversized-block path
        string_scenario(
            24,
            HM_STRING,
            Some(SH_ARENA),
            40,
            |i, r| if i % 4 == 0 { 600 + r.below(2000) } else { 1 + r.below(30) },
            ASCII,
            &mut rng,
            "arena-oversized",
            true,
        );
    }
}

// -------------------------------------------------------------------- row 47
#[test]
fn c47_string_default_mode_explicit() {
    let mut rng = Rng::new(0x4747);
    unsafe {
        string_scenario(
            16,
            HM_STRING,
            Some(SH_DEFAULT),
            100,
            |_, r| 1 + r.below(40),
            ASCII,
            &mut rng,
            "default-explicit",
            true,
        );
    }
}

// --------------------------------------------------------------- rows 48, 49
#[test]
fn c48_c49_string_hash_with_memcpy_storage() {
    // `string.mode` outside {SH_DEFAULT, SH_STRDUP, SH_ARENA} makes
    // `stbds_hmput_key` take the `default:` branch and `memcpy` the raw key
    // bytes.  Look-ups would then reinterpret those bytes as a `char *`
    // (undefined behaviour in the C original), so only inserts are exercised.
    let mut rng = Rng::new(0x4849);
    for &sm in &[SH_NONE, 4u8, 255u8] {
        unsafe {
            string_scenario(
                24,
                HM_STRING,
                Some(sm),
                12,
                |_, r| 8 + r.below(20),
                ASCII,
                &mut rng,
                &format!("memcpy-storage sm={}", sm),
                false,
            );
        }
    }
}

// -------------------------------------------------------------------- row 50
#[test]
fn c50_mode_two_behaves_like_string() {
    let mut rng = Rng::new(0x5050);
    for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        unsafe {
            string_scenario(
                16,
                2, // out-of-range but >= STBDS_HM_STRING
                Some(sm),
                60,
                |_, r| 1 + r.below(30),
                ASCII,
                &mut rng,
                &format!("mode2 sm={}", sm),
                true,
            );
            string_scenario(
                16,
                c_int::MAX,
                Some(sm),
                40,
                |_, r| 1 + r.below(30),
                ASCII,
                &mut rng,
                &format!("modeMAX sm={}", sm),
                true,
            );
        }
    }
}

// -------------------------------------------------------------------- row 51
#[test]
fn c51_string_duplicate_keys() {
    let mut rng = Rng::new(0x5151);
    for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp =
                MapPair::new(16, KeyRepr::Pointer, format!("dupstr sm={}", sm)).with_value_offset(8);
            mp.shmode(sm as c_int);
            mp.put_default();
            let mut keys: Vec<Vec<u8>> = (0..40)
                .map(|_| {
                    let l = 1 + rng.below(20);
                    rng.cstring(l, ASCII)
                })
                .collect();
            keys.dedup();
            let mut first = std::collections::HashMap::new();
            for round in 0..3 {
                for (i, k) in keys.iter_mut().enumerate() {
                    let idx = same_idx("dupstr", mp.put(k, 8, HM_STRING));
                    if round == 0 {
                        first.insert(i, idx);
                    } else if let Some(&f) = first.get(&i) {
                        assert_eq!(f, idx, "duplicate key must map to its old index");
                    }
                }
            }
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 52
#[test]
fn c52_string_prefix_and_last_byte_keys() {
    for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp =
                MapPair::new(16, KeyRepr::Pointer, format!("prefix sm={}", sm)).with_value_offset(8);
            mp.shmode(sm as c_int);
            mp.put_default();
            let mut keys: Vec<Vec<u8>> = Vec::new();
            // prefixes of one another
            let base = b"abcdefghijklmnopqrstuvwxyz";
            for l in 0..=base.len() {
                let mut k = base[..l].to_vec();
                k.push(0);
                keys.push(k);
            }
            // differing only in the last byte
            for c in b'a'..=b'z' {
                let mut k = b"prefix_key_".to_vec();
                k.push(c);
                k.push(0);
                keys.push(k);
            }
            for k in keys.iter_mut() {
                assert!(same_idx("prefix", mp.put(k, 8, HM_STRING)) >= 0);
            }
            for k in keys.iter_mut() {
                assert!(same_idx("prefix", mp.get(k, 8, HM_STRING)) >= 0);
            }
            for k in keys.iter_mut() {
                same_idx("prefix", mp.del(k, 8, 0, HM_STRING));
            }
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 53
#[test]
fn c53_string_strkey_shaped_keys() {
    for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp =
                MapPair::new(16, KeyRepr::Pointer, format!("strkey sm={}", sm)).with_value_offset(8);
            mp.shmode(sm as c_int);
            mp.put_default();
            let mut keys: Vec<Vec<u8>> =
                (0..500).map(|i| format!("test_{}\0", i).into_bytes()).collect();
            for k in keys.iter_mut() {
                assert!(same_idx("strkey", mp.put(k, 8, HM_STRING)) >= 0);
            }
            for k in keys.iter_mut() {
                assert!(same_idx("strkey", mp.get(k, 8, HM_STRING)) >= 0);
            }
            // delete every other one, then verify the rest
            for i in (0..500).step_by(2) {
                let mut k = keys[i].clone();
                same_idx("strkey", mp.del(&mut k, 8, 0, HM_STRING));
            }
            for i in 0..500 {
                let mut k = keys[i].clone();
                let idx = same_idx("strkey", mp.get(&mut k, 8, HM_STRING));
                if i % 2 == 0 {
                    assert_eq!(idx, -1, "test_{} should be gone", i);
                } else {
                    assert!(idx >= 0, "test_{} should still be present", i);
                }
            }
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 54
#[test]
fn c54_string_empty_key() {
    for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp =
                MapPair::new(16, KeyRepr::Pointer, format!("empty sm={}", sm)).with_value_offset(8);
            mp.shmode(sm as c_int);
            mp.put_default();
            let mut empty = vec![0u8];
            let i1 = same_idx("empty", mp.put(&mut empty, 8, HM_STRING));
            let i2 = same_idx("empty", mp.put(&mut empty, 8, HM_STRING));
            assert_eq!(i1, i2, "\"\" must be a normal key");
            let mut other = b"x\0".to_vec();
            same_idx("empty", mp.put(&mut other, 8, HM_STRING));
            assert_eq!(same_idx("empty", mp.get(&mut empty, 8, HM_STRING)), i1);
            same_idx("empty", mp.del(&mut empty, 8, 0, HM_STRING));
            assert_eq!(same_idx("empty", mp.get(&mut empty, 8, HM_STRING)), -1);
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 55
#[test]
fn c55_string_elemsize_variants() {
    let mut rng = Rng::new(0x5555);
    for &elemsize in &[8usize, 16, 24, 32, 64] {
        for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            unsafe {
                string_scenario(
                    elemsize,
                    HM_STRING,
                    Some(sm),
                    40,
                    |_, r| 1 + r.below(25),
                    ASCII,
                    &mut rng,
                    &format!("es={} sm={}", elemsize, sm),
                    true,
                );
            }
        }
    }
}

// ===========================================================================
// rows 56..60 — stbds_hmput_default
// ===========================================================================

#[test]
fn c56_c57_put_default_null_and_twice() {
    for &elemsize in &[8usize, 12, 16, 24, 32, 64] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(elemsize, KeyRepr::Raw, format!("pd es={}", elemsize))
                .with_value_offset(elemsize);
            mp.put_default();
            assert_eq!(mp.len_from_c(), 1);
            let before = mp.tc;
            mp.put_default();
            assert_eq!(mp.tc, before, "second call must be a no-op");
            assert_eq!(mp.len_from_c(), 1);
            mp.put_default();
            assert_eq!(mp.len_from_c(), 1);
            mp.free();
        }
    }
}

#[test]
fn c58_put_default_after_puts() {
    let mut rng = Rng::new(0x5858);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        // binary
        let mut mp = MapPair::new(16, KeyRepr::Raw, "pd-after-bin").with_value_offset(8);
        for _ in 0..20 {
            let mut k = rng.bytes(8);
            assert!(same_idx("pd", mp.put(&mut k, 8, HM_BINARY)) >= 0);
            let before = mp.tc;
            mp.put_default();
            assert_eq!(mp.tc, before, "no-op while length != 0");
        }
        mp.free();

        // string
        let mut mp = MapPair::new(16, KeyRepr::Pointer, "pd-after-str").with_value_offset(8);
        mp.shmode(SH_STRDUP as c_int);
        let mut keys: Vec<Vec<u8>> = (0..20).map(|i| format!("k{}\0", i).into_bytes()).collect();
        for k in keys.iter_mut() {
            assert!(same_idx("pd", mp.put(k, 8, HM_STRING)) >= 0);
            mp.put_default();
        }
        mp.free();
    }
}

#[test]
fn c59_put_default_on_len0_array() {
    let p = pair();
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        for &elemsize in &[8usize, 16, 24] {
            // build an array with capacity but length 0, then hand it to
            // hmput_default as a "hash map pointer"
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let tc0 = (ac as *mut u8).add(elemsize) as *mut _;
            let tr0 = (ar as *mut u8).add(elemsize) as *mut _;
            let tc = (p.c.hmput_default)(tc0, elemsize);
            let tr = (p.r.hmput_default)(tr0, elemsize);
            let sc = snap_map(tc, elemsize, KeyRepr::Raw);
            let sr = snap_map(tr, elemsize, KeyRepr::Raw);
            assert_eq!(sc, sr, "hmput_default on a length-0 array");
            assert_eq!(sc.length, 1);
            (p.c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut _, elemsize);
            (p.r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut _, elemsize);
        }
    }
}

#[test]
fn c60_default_element_is_returned_for_misses() {
    let mut rng = Rng::new(0x6060);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "default-elem").with_value_offset(8);
        mp.put_default();
        // the stb_ds `hmdefault(t, v)` idiom writes into t[-1]
        mp.write_elem(-1, 8, &(-2i64).to_le_bytes());
        mp.check("hmdefault");
        for _ in 0..40 {
            let mut k = rng.bytes(8);
            assert!(same_idx("default-elem", mp.put(&mut k, 8, HM_BINARY)) >= 0);
        }
        for _ in 0..40 {
            let mut k = rng.bytes(8);
            let idx = same_idx("default-elem", mp.get(&mut k, 8, HM_BINARY));
            assert_eq!(idx, -1);
            // t[-1].value == -2 in BOTH
            let vc = std::slice::from_raw_parts(
                (mp.tc as *const u8).offset(-(16isize)).add(8),
                8,
            )
            .to_vec();
            let vr = std::slice::from_raw_parts(
                (mp.tr as *const u8).offset(-(16isize)).add(8),
                8,
            )
            .to_vec();
            assert_eq!(vc, vr);
            assert_eq!(i64::from_le_bytes(vc.try_into().unwrap()), -2);
        }
        mp.free();
    }
}

// ===========================================================================
// rows 61..72 — stbds_hmdel_key
// ===========================================================================

/// Build a binary map with `n` distinct 8-byte keys.  Returns the keys in
/// insertion order (their t[] index equals their position).
unsafe fn build_binary(
    mp: &mut MapPair,
    n: usize,
    rng: &mut Rng,
    mode: c_int,
) -> Vec<Vec<u8>> {
    let mut keys: Vec<Vec<u8>> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    while keys.len() < n {
        let k = rng.bytes(8);
        if !seen.insert(k.clone()) {
            continue;
        }
        keys.push(k);
    }
    for (i, k) in keys.iter_mut().enumerate() {
        let idx = same_idx("build", mp.put(k, 8, mode));
        assert_eq!(idx, i as isize, "insertion index must be sequential");
    }
    keys
}

// -------------------------------------------------------------------- row 61
#[test]
fn c61_delete_last_element() {
    let mut rng = Rng::new(0x6161);
    for n in [1usize, 2, 3, 7, 8, 20, 60] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(16, KeyRepr::Raw, format!("del-last n={}", n))
                .with_value_offset(8);
            let mut keys = build_binary(&mut mp, n, &mut rng, HM_BINARY);
            // repeatedly remove the highest index: old_index == final_index
            for i in (0..n).rev() {
                let before_len = mp.len_from_c();
                let t = same_idx("del-last", mp.del(&mut keys[i], 8, 0, HM_BINARY));
                assert_eq!(t, 1, "successful delete sets temp = 1");
                assert_eq!(mp.len_from_c(), before_len - 1);
                assert_eq!(same_idx("del-last", mp.get(&mut keys[i], 8, HM_BINARY)), -1);
            }
            assert_eq!(mp.len_from_c(), 1, "only the default element is left");
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 62
#[test]
fn c62_delete_interior_element() {
    let mut rng = Rng::new(0x6262);
    for n in [2usize, 3, 8, 20, 60] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(24, KeyRepr::Raw, format!("del-mid n={}", n))
                .with_value_offset(8);
            let mut keys = build_binary(&mut mp, n, &mut rng, HM_BINARY);
            // always delete index 0 => swap-with-last + slot re-point
            let mut alive: Vec<usize> = (0..n).collect();
            while !alive.is_empty() {
                let victim = alive[0];
                let t = same_idx("del-mid", mp.del(&mut keys[victim], 8, 0, HM_BINARY));
                assert_eq!(t, 1);
                alive.remove(0);
                assert_eq!(same_idx("del-mid", mp.get(&mut keys[victim], 8, HM_BINARY)), -1);
                for &j in &alive {
                    let mut k = keys[j].clone();
                    assert!(
                        same_idx("del-mid", mp.get(&mut k, 8, HM_BINARY)) >= 0,
                        "surviving key {} lost",
                        j
                    );
                }
            }
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 63
#[test]
fn c63_delete_all_orders() {
    let mut rng = Rng::new(0x6363);
    for n in [1usize, 2, 8, 50, 200] {
        for which in 0..3 {
            let _g = seed_guard(DEFAULT_SEED);
            unsafe {
                let mut mp = MapPair::new(16, KeyRepr::Raw, format!("del-all n={} o={}", n, which))
                    .with_value_offset(8);
                let mut keys = build_binary(&mut mp, n, &mut rng, HM_BINARY);
                let mut order: Vec<usize> = (0..n).collect();
                match which {
                    0 => {}
                    1 => order.reverse(),
                    _ => {
                        for i in (1..n).rev() {
                            let j = rng.below(i + 1);
                            order.swap(i, j);
                        }
                    }
                }
                for &i in &order {
                    let t = same_idx("del-all", mp.del(&mut keys[i], 8, 0, HM_BINARY));
                    assert_eq!(t, 1);
                }
                assert_eq!(mp.len_from_c(), 1);
                for k in keys.iter_mut() {
                    assert_eq!(same_idx("del-all", mp.get(k, 8, HM_BINARY)), -1);
                    // deleting again must be a no-op with temp = 0
                    assert_eq!(same_idx("del-all", mp.del(k, 8, 0, HM_BINARY)), 0);
                }
                mp.free();
            }
        }
    }
}

// -------------------------------------------------------------------- row 64
#[test]
fn c64_randomized_delete_reinsert() {
    let mut rng = Rng::new(0x6464);
    for seq in 0..256 {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let elemsize = *rng.pick(&[8usize, 16, 24, 32]);
            let keysize = *rng.pick(&[1usize, 2, 4, 8]).min(&elemsize);
            let mut mp = MapPair::new(elemsize, KeyRepr::Raw, format!("churn {}", seq))
                .with_value_offset(keysize);
            // a small key universe so collisions / reinserts actually happen
            let universe: Vec<Vec<u8>> = (0..24u8)
                .map(|i| {
                    let mut v = vec![0u8; keysize.max(1)];
                    v[0] = i;
                    v
                })
                .collect();
            let mut present = std::collections::HashSet::new();
            for _ in 0..80 {
                let pick = rng.below(universe.len());
                let mut k = universe[pick].clone();
                match rng.below(10) {
                    0..=4 => {
                        same_idx("churn", mp.put(&mut k, keysize, HM_BINARY));
                        if keysize > 0 {
                            present.insert(pick);
                        }
                    }
                    5..=7 => {
                        let t = same_idx("churn", mp.del(&mut k, keysize, 0, HM_BINARY));
                        if keysize > 0 {
                            let expect = if present.remove(&pick) { 1 } else { 0 };
                            assert_eq!(t, expect, "hmdel temp for pick {}", pick);
                        }
                    }
                    8 => {
                        let idx = same_idx("churn", mp.get(&mut k, keysize, HM_BINARY));
                        if keysize > 0 {
                            let expect_present = present.contains(&pick);
                            assert_eq!(idx >= 0, expect_present, "presence for pick {}", pick);
                        }
                    }
                    _ => {
                        same_idx("churn", mp.get_ts(&mut k, keysize, HM_BINARY));
                    }
                }
            }
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 65
#[test]
fn c65_delete_triggers_shrink() {
    let mut rng = Rng::new(0x6565);
    for n in [7usize, 13, 25, 60, 200] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp =
                MapPair::new(16, KeyRepr::Raw, format!("shrink n={}", n)).with_value_offset(8);
            let mut keys = build_binary(&mut mp, n, &mut rng, HM_BINARY);
            let big = snap_map(mp.tc, 16, KeyRepr::Raw).table.unwrap().slot_count;
            assert!(big >= 16, "expected a grown table, got {}", big);
            let mut smallest = big;
            for k in keys.iter_mut() {
                same_idx("shrink", mp.del(k, 8, 0, HM_BINARY));
                let sc = snap_map(mp.tc, 16, KeyRepr::Raw).table.unwrap().slot_count;
                smallest = smallest.min(sc);
            }
            assert!(
                smallest < big,
                "the shrink branch was never taken (slot_count stayed {})",
                big
            );
            assert_eq!(smallest, 8, "shrinking must bottom out at STBDS_BUCKET_LENGTH");
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 66
#[test]
fn c66_delete_triggers_tombstone_rebuild() {
    let mut rng = Rng::new(0x6666);
    {
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        // 8 slots: tombstone_count_threshold == 1 and the shrink branch is
        // disabled (used_count_shrink_threshold forced to 0), so churn must go
        // through the tombstone rebuild.  Two deletes in a row are needed to
        // push tombstone_count past 1.
        let mut mp = MapPair::new(16, KeyRepr::Raw, "tombstones-8").with_value_offset(8);
        let mut keys = build_binary(&mut mp, 4, &mut rng, HM_BINARY);
        let mut saw_reset = false;
        for round in 0..30 {
            let a = round % keys.len();
            let b = (round + 1) % keys.len();
            same_idx("tomb", mp.del(&mut keys[a], 8, 0, HM_BINARY));
            let t1 = snap_map(mp.tc, 16, KeyRepr::Raw).table.unwrap();
            assert_eq!(t1.slot_count, 8, "slot_count must stay at the minimum");
            same_idx("tomb", mp.del(&mut keys[b], 8, 0, HM_BINARY));
            let t2 = snap_map(mp.tc, 16, KeyRepr::Raw).table.unwrap();
            assert_eq!(t2.slot_count, 8);
            if t1.tombstone_count > 0 && t2.tombstone_count == 0 {
                saw_reset = true;
            }
            same_idx("tomb", mp.put(&mut keys[a], 8, HM_BINARY));
            same_idx("tomb", mp.put(&mut keys[b], 8, HM_BINARY));
        }
        assert!(saw_reset, "the tombstone rebuild branch was never taken (8 slots)");
        mp.free();
    }
    } // drop the seed guard before re-acquiring it below

    // and once more on a grown table, where the same-size rebuild happens
    // before the shrink branch becomes eligible
    let _g2 = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "tombstones-32").with_value_offset(8);
        let mut keys = build_binary(&mut mp, 20, &mut rng, HM_BINARY);
        let start = snap_map(mp.tc, 16, KeyRepr::Raw).table.unwrap();
        assert_eq!(start.slot_count, 32);
        assert_eq!(start.tombstone_count_threshold, 6);
        let mut saw_rebuild_same_size = false;
        let mut prev = start;
        for k in keys.iter_mut() {
            same_idx("tomb32", mp.del(k, 8, 0, HM_BINARY));
            let cur = snap_map(mp.tc, 16, KeyRepr::Raw).table.unwrap();
            if cur.slot_count == prev.slot_count
                && prev.tombstone_count > 0
                && cur.tombstone_count == 0
            {
                saw_rebuild_same_size = true;
            }
            prev = cur;
        }
        assert!(saw_rebuild_same_size, "no same-size tombstone rebuild observed");
        mp.free();
    }
}

// ------------------------------------------------------------- rows 67,68,69
#[test]
fn c67_c68_c69_delete_string_modes() {
    let mut rng = Rng::new(0x6768);
    for &sm in &[SH_STRDUP, SH_ARENA, SH_DEFAULT] {
        for n in [1usize, 2, 8, 40, 120] {
            let _g = seed_guard(DEFAULT_SEED);
            unsafe {
                let mut mp = MapPair::new(16, KeyRepr::Pointer, format!("delstr sm={} n={}", sm, n))
                    .with_value_offset(8);
                mp.shmode(sm as c_int);
                mp.put_default();
                let mut keys: Vec<Vec<u8>> = Vec::new();
                let mut seen = std::collections::HashSet::new();
                while keys.len() < n {
                    let l = 1 + rng.below(30);
                    let k = rng.cstring(l, ASCII);
                    if !seen.insert(k.clone()) {
                        continue;
                    }
                    keys.push(k);
                }
                for k in keys.iter_mut() {
                    assert!(same_idx("delstr", mp.put(k, 8, HM_STRING)) >= 0);
                }
                // delete every key, checking survivors after each step
                for i in 0..n {
                    let mut k = keys[i].clone();
                    let t = same_idx("delstr", mp.del(&mut k, 8, 0, HM_STRING));
                    assert_eq!(t, 1);
                    for j in (i + 1)..n {
                        let mut kk = keys[j].clone();
                        assert!(
                            same_idx("delstr", mp.get(&mut kk, 8, HM_STRING)) >= 0,
                            "survivor {} lost",
                            j
                        );
                    }
                }
                assert_eq!(mp.len_from_c(), 1);
                // reinsert everything after the mass delete
                for k in keys.iter_mut() {
                    assert!(same_idx("delstr", mp.put(k, 8, HM_STRING)) >= 0);
                }
                mp.free();
            }
        }
    }
}

// -------------------------------------------------------------------- row 70
#[test]
fn c70_delete_mode_two_skips_the_free() {
    // `if (mode == STBDS_HM_STRING && string.mode == SH_STRDUP) free(key)` uses
    // `==`, so mode 2 leaks the strdup'ed key instead of freeing it.
    //
    // Careful: with `mode != STBDS_HM_STRING` the *swap-with-last* path also
    // takes the `else` branch of the re-lookup and passes the ADDRESS of the
    // element instead of the stored `char *`, which makes
    // `STBDS_ASSERT(slot >= 0)` fire.  That abort is covered by
    // `errors.rs::e33_hmdel_mode_two_no_free` in a subprocess; here we delete
    // strictly last-to-first so `old_index == final_index` and no memmove
    // happens.
    for &m in &[2, 7, c_int::MAX] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp =
                MapPair::new(16, KeyRepr::Pointer, format!("mode{}-strdup", m)).with_value_offset(8);
            mp.shmode(SH_STRDUP as c_int);
            mp.put_default();
            let mut keys: Vec<Vec<u8>> =
                (0..40).map(|i| format!("m2_key_{}\0", i).into_bytes()).collect();
            for k in keys.iter_mut() {
                assert!(same_idx("m2", mp.put(k, 8, m)) >= 0);
            }
            for i in (0..keys.len()).rev() {
                assert_eq!(same_idx("m2", mp.del(&mut keys[i], 8, 0, m)), 1);
            }
            assert_eq!(mp.len_from_c(), 1);
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 71
#[test]
fn c71_delete_keyoffset_variants() {
    let mut rng = Rng::new(0x7171);
    for &keyoffset in &[0usize, 4, 8] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(24, KeyRepr::Raw, format!("keyoff={}", keyoffset))
                .with_value_offset(8);
            let mut keys = build_binary(&mut mp, 30, &mut rng, HM_BINARY);
            let len_before = mp.len_from_c();
            let mut deleted = 0;
            for k in keys.iter_mut() {
                let t = same_idx("keyoff", mp.del(k, 8, keyoffset, HM_BINARY));
                if t == 1 {
                    deleted += 1;
                }
            }
            if keyoffset == 0 {
                assert_eq!(deleted, 30, "keyoffset 0 must delete every key");
                assert_eq!(mp.len_from_c(), 1);
            } else {
                // the comparison reads the wrong bytes, so nothing is found
                assert_eq!(deleted, 0, "keyoffset {} must not match anything", keyoffset);
                assert_eq!(mp.len_from_c(), len_before);
            }
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 72
#[test]
fn c72_delete_edge_cases() {
    let p = pair();
    let mut rng = Rng::new(0x7272);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        // a == NULL -> returns NULL
        let mut k = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let rc = (p.c.hmdel_key)(
            std::ptr::null_mut(),
            16,
            k.as_mut_ptr() as *mut _,
            8,
            0,
            HM_BINARY,
        );
        let rr = (p.r.hmdel_key)(
            std::ptr::null_mut(),
            16,
            k.as_mut_ptr() as *mut _,
            8,
            0,
            HM_BINARY,
        );
        assert!(rc.is_null() && rr.is_null(), "hmdel_key(NULL) must return NULL");

        // table == NULL
        let mut mp = MapPair::new(16, KeyRepr::Raw, "del-no-table").with_value_offset(8);
        mp.put_default();
        assert_eq!(same_idx("del-no-table", mp.del(&mut k, 8, 0, HM_BINARY)), 0);
        mp.free();

        // absent key on a populated map
        let mut mp = MapPair::new(16, KeyRepr::Raw, "del-absent").with_value_offset(8);
        let keys = build_binary(&mut mp, 30, &mut rng, HM_BINARY);
        let len_before = mp.len_from_c();
        for _ in 0..40 {
            let mut miss = rng.bytes(8);
            if keys.contains(&miss) {
                continue;
            }
            assert_eq!(same_idx("del-absent", mp.del(&mut miss, 8, 0, HM_BINARY)), 0);
        }
        assert_eq!(mp.len_from_c(), len_before);
        mp.free();
    }
}

// ===========================================================================
// rows 73..75 — stbds_hmfree_func
// ===========================================================================

#[test]
fn c73_hmfree_all_string_modes() {
    let mut rng = Rng::new(0x7373);
    for &sm in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA, 4u8, 255u8] {
        for n in [0usize, 1, 7, 99] {
            let _g = seed_guard(DEFAULT_SEED);
            let repr = repr_for(sm);
            unsafe {
                let mut mp = MapPair::new(24, repr, format!("free sm={} n={}", sm, n))
                    .with_value_offset(8);
                mp.shmode(sm as c_int);
                mp.put_default();
                let mut keys: Vec<Vec<u8>> = Vec::new();
                let mut seen = std::collections::HashSet::new();
                while keys.len() < n {
                    let l = 8 + rng.below(600);
                    let k = rng.cstring(l, ASCII);
                    if !seen.insert(k.clone()) {
                        continue;
                    }
                    keys.push(k);
                }
                for k in keys.iter_mut() {
                    assert!(same_idx("free", mp.put(k, 8, HM_STRING)) >= 0);
                }
                mp.free();
                // for SH_DEFAULT the keys belong to the caller and must survive
                if sm == SH_DEFAULT {
                    for (i, k) in keys.iter().enumerate() {
                        assert_eq!(
                            read_cstr(k.as_ptr() as *const _),
                            k[..k.len() - 1].to_vec(),
                            "caller key {} was freed",
                            i
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn c74_c75_hmfree_null_and_table_less() {
    let p = pair();
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        // a == NULL must be a no-op in both
        (p.c.hmfree_func)(std::ptr::null_mut(), 16);
        (p.r.hmfree_func)(std::ptr::null_mut(), 16);
        for &elemsize in &[8usize, 16, 24, 64] {
            (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (p.r.hmfree_func)(std::ptr::null_mut(), elemsize);
            // hash_table == NULL: only the header is freed
            let tc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let tr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
            assert!(!snap_map(tc, elemsize, KeyRepr::Raw).has_table);
            assert!(!snap_map(tr, elemsize, KeyRepr::Raw).has_table);
            (p.c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut _, elemsize);
            (p.r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut _, elemsize);
        }
    }
}

#[test]
fn c58b_hmfree_strdup_with_only_default_element() {
    // ERRORS.md #58: the `for (i=1; i<length; ++i)` key-free loop must not run
    let _g = seed_guard(DEFAULT_SEED);
    let p = pair();
    unsafe {
        for &elemsize in &[8usize, 16, 24] {
            let tc = (p.c.shmode_func)(elemsize, SH_STRDUP as c_int);
            let tr = (p.r.shmode_func)(elemsize, SH_STRDUP as c_int);
            let sc = snap_map(tc, elemsize, KeyRepr::Pointer);
            assert_eq!(sc.length, 1);
            assert_eq!(sc, snap_map(tr, elemsize, KeyRepr::Pointer));
            (p.c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut _, elemsize);
            (p.r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut _, elemsize);
        }
    }
}
