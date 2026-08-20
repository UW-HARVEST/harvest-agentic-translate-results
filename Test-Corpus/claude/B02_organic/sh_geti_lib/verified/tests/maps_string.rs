//! Phase B — CONFIGS.md rows 43..57: string-keyed maps, every `string.mode`.
mod common;

use common::*;
use core::ffi::{c_char, c_int, c_void};
use std::collections::BTreeSet;

/// `n` distinct NUL-terminated keys at stable addresses (identical pointers for
/// both libraries), plus `n_extra` disjoint "absent" keys.
fn string_keys(
    rng: &mut Rng,
    keys: &mut Keys,
    n: usize,
    n_extra: usize,
    len_lo: usize,
    len_hi: usize,
) -> (Vec<*mut c_void>, Vec<*mut c_void>, Vec<Vec<u8>>) {
    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    let mut ptrs = Vec::new();
    let mut texts = Vec::new();
    let mut guard = 0;
    while ptrs.len() < n + n_extra {
        guard += 1;
        assert!(guard < 1_000_000, "string key generation stalled");
        let len = rng.range(len_lo, len_hi);
        let s = rng.nz_bytes(len);
        if !seen.insert(s.clone()) {
            continue;
        }
        ptrs.push(keys.cstr(&s));
        texts.push(s);
    }
    let extra = ptrs.split_off(n);
    texts.truncate(n);
    (ptrs, extra, texts)
}

fn cmp_for(string_mode: c_int) -> ElemCmp {
    match string_mode {
        x if x == SH_STRDUP || x == SH_ARENA => ElemCmp::StrKey,
        _ => ElemCmp::Raw,
    }
}

/// The `char*` the library stored for hash-space element `idx`.
unsafe fn stored_ptr(m: &Map, idx: isize) -> *mut c_char {
    *(elem(m.t, m.elemsize, idx) as *mut *mut c_char)
}

unsafe fn stored_text(m: &Map, idx: isize) -> Option<Vec<u8>> {
    cstr_bytes(stored_ptr(m, idx))
}

/// Full string-map life-cycle for one `string.mode`.
///
/// `bootstrap == None` means "let `stbds_hmput_key` create the table"
/// (=> `string.mode` becomes `STBDS_SH_DEFAULT`); otherwise
/// `stbds_shmode_func` is used to set the mode explicitly.
unsafe fn string_lifecycle(
    c: &Lib,
    r: &Lib,
    elemsize: usize,
    bootstrap: Option<c_int>,
    mode: c_int,
    n: usize,
    seed: usize,
    len_lo: usize,
    len_hi: usize,
    rng: &mut Rng,
) {
    let string_mode = bootstrap.unwrap_or(SH_DEFAULT);
    (c.rand_seed)(seed);
    (r.rand_seed)(seed);
    let mut keys = Keys::new();
    let (ks, absent, texts) = string_keys(rng, &mut keys, n, 8, len_lo, len_hi);
    let label = format!(
        "str/es{elemsize}/sm{string_mode}/mode{mode}/n{n}/seed{seed}/len{len_lo}-{len_hi}"
    );
    let mut p = Pair::new(c, r, elemsize, 8, cmp_for(string_mode), label);

    match bootstrap {
        Some(sm) => p.shmode(sm),
        None => {}
    }
    p.set_default(0xDEFA);

    for (i, &k) in ks.iter().enumerate() {
        let idx = p.put(k, mode, i as u64);
        assert_eq!(idx, i as isize, "insertion index");
        assert_eq!(p.len(), i as isize + 1);
        // the library must have stored the key's text
        let tk = p.temp_key();
        assert_eq!(
            tk.as_deref(),
            Some(texts[i].as_slice()),
            "stbds_temp_key after put"
        );
        assert_eq!(stored_text(&p.c, idx).as_deref(), Some(texts[i].as_slice()));
        assert_eq!(stored_text(&p.r, idx).as_deref(), Some(texts[i].as_slice()));
        // STRDUP/ARENA must copy; DEFAULT must alias the caller's pointer
        match string_mode {
            x if x == SH_DEFAULT => {
                assert_eq!(stored_ptr(&p.c, idx) as *mut c_void, k);
                assert_eq!(stored_ptr(&p.r, idx) as *mut c_void, k);
            }
            x if x == SH_STRDUP || x == SH_ARENA => {
                assert_ne!(stored_ptr(&p.c, idx) as *mut c_void, k);
                assert_ne!(stored_ptr(&p.r, idx) as *mut c_void, k);
            }
            _ => {}
        }
    }
    for (i, &k) in ks.iter().enumerate() {
        assert_eq!(p.geti(k, mode), i as isize);
        assert_eq!(p.geti_ts(k, mode), i as isize);
        p.value_of(i as isize);
    }
    for &k in &absent {
        assert_eq!(p.geti(k, mode), -1, "absent key must miss");
        assert_eq!(p.geti_ts(k, mode), -1);
    }
    // delete every key (interior + tail, so the memmove + re-find path runs)
    let mut order: Vec<usize> = (0..ks.len()).collect();
    for i in (1..order.len()).rev() {
        let j = rng.below(i + 1);
        order.swap(i, j);
    }
    for &i in &order {
        assert_eq!(p.del(ks[i], 0, mode), 1, "delete of a live key");
    }
    assert_eq!(p.len(), 0);
    for &k in &ks {
        assert_eq!(p.geti(k, mode), -1);
    }
    p.free();
}

/// Row 43 — table bootstrapped by `stbds_hmput_key` itself (`SH_DEFAULT`).
#[test]
fn cfg43_bootstrapped_default_mode() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4343_4343);
    unsafe {
        for n in 1..=40usize {
            let seed = rng.next_u64() as usize;
            string_lifecycle(&c, &r, 16, None, HM_STRING, n, seed, 1, 20, &mut rng);
        }
        for &seed in &[0usize, 1, 0x31415926, usize::MAX] {
            for n in [1usize, 5, 6, 7, 12, 13, 24, 25] {
                string_lifecycle(&c, &r, 24, None, HM_STRING, n, seed, 1, 40, &mut rng);
            }
        }
    }
}

/// Row 45 — explicit `stbds_shmode_func(elemsize, STBDS_SH_DEFAULT)`.
#[test]
fn cfg45_sh_default() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4545_4545);
    unsafe {
        for n in 1..=40usize {
            let seed = rng.next_u64() as usize;
            string_lifecycle(
                &c,
                &r,
                16,
                Some(SH_DEFAULT),
                HM_STRING,
                n,
                seed,
                1,
                30,
                &mut rng,
            );
        }
    }
}

/// Row 46 — `STBDS_SH_STRDUP`.
#[test]
fn cfg46_sh_strdup() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4646_4646);
    unsafe {
        for n in 1..=40usize {
            let seed = rng.next_u64() as usize;
            string_lifecycle(
                &c,
                &r,
                16,
                Some(SH_STRDUP),
                HM_STRING,
                n,
                seed,
                1,
                30,
                &mut rng,
            );
        }
        for &elemsize in &[24usize, 32] {
            for &seed in &[0usize, 1, usize::MAX] {
                string_lifecycle(
                    &c,
                    &r,
                    elemsize,
                    Some(SH_STRDUP),
                    HM_STRING,
                    25,
                    seed,
                    1,
                    64,
                    &mut rng,
                );
            }
        }
    }
}

/// Row 47 — `STBDS_SH_ARENA` (the arena's `remaining`/`block` evolution is part
/// of the compared snapshot).
#[test]
fn cfg47_sh_arena() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4747_4747);
    unsafe {
        for n in 1..=40usize {
            let seed = rng.next_u64() as usize;
            string_lifecycle(
                &c,
                &r,
                16,
                Some(SH_ARENA),
                HM_STRING,
                n,
                seed,
                1,
                30,
                &mut rng,
            );
        }
        for &elemsize in &[24usize, 32] {
            for &seed in &[0usize, 1, usize::MAX] {
                string_lifecycle(
                    &c,
                    &r,
                    elemsize,
                    Some(SH_ARENA),
                    HM_STRING,
                    25,
                    seed,
                    1,
                    64,
                    &mut rng,
                );
            }
        }
    }
}

/// Row 48 — key lengths at every arena block-size boundary, under STRDUP and
/// ARENA.
#[test]
fn cfg48_key_length_boundaries() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4848_4848);
    const LENS: &[usize] = &[0, 1, 2, 7, 8, 9, 63, 64, 127, 255, 256, 505, 510, 511, 512, 513, 1023, 1024, 4096];
    unsafe {
        for &sm in &[SH_STRDUP, SH_ARENA, SH_DEFAULT] {
            for &seed in &[1usize, 0x31415926] {
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let mut keys = Keys::new();
                let mut ptrs = Vec::new();
                let mut texts = Vec::new();
                for (i, &len) in LENS.iter().enumerate() {
                    // make each key unique even at the same length
                    let mut s = rng.nz_bytes(len);
                    if len > 0 {
                        s[0] = (b'a' as usize + i % 26) as u8;
                    }
                    ptrs.push(keys.cstr(&s));
                    texts.push(s);
                }
                let mut p = Pair::new(
                    &c,
                    &r,
                    16,
                    8,
                    cmp_for(sm),
                    format!("lens/sm{sm}/seed{seed}"),
                );
                p.shmode(sm);
                p.set_default(1);
                for (i, &k) in ptrs.iter().enumerate() {
                    let idx = p.put(k, HM_STRING, i as u64);
                    assert_eq!(
                        p.temp_key().as_deref(),
                        Some(texts[i].as_slice()),
                        "temp_key for len {}",
                        LENS[i]
                    );
                    assert_eq!(stored_text(&p.c, idx).as_deref(), Some(texts[i].as_slice()));
                }
                for (i, &k) in ptrs.iter().enumerate() {
                    assert_eq!(p.geti(k, HM_STRING), i as isize, "len {}", LENS[i]);
                }
                for &k in &ptrs {
                    p.del(k, 0, HM_STRING);
                }
                p.free();
            }
        }
    }
}

/// Row 49 — many arena keys, so `string.block` increments repeatedly
/// (512, 512, 1024, 1024, 2048, ...).
#[test]
fn cfg49_arena_block_growth() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4949_4949);
    unsafe {
        for &seed in &[1usize, 7, 0x31415926] {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut keys = Keys::new();
            let (ks, _, texts) = string_keys(&mut rng, &mut keys, 400, 0, 20, 200);
            let mut p = Pair::new(&c, &r, 16, 8, ElemCmp::StrKey, format!("blocks/{seed}"));
            p.shmode(SH_ARENA);
            p.set_default(1);
            for (i, &k) in ks.iter().enumerate() {
                let idx = p.put(k, HM_STRING, i as u64);
                assert_eq!(stored_text(&p.c, idx).as_deref(), Some(texts[i].as_slice()));
            }
            let snap = p.c.snap(ElemCmp::StrKey);
            assert!(
                snap.str_block >= 3,
                "arena block counter should have advanced, got {}",
                snap.str_block
            );
            for (i, &k) in ks.iter().enumerate() {
                assert_eq!(p.geti(k, HM_STRING), i as isize);
            }
            p.free();
        }
    }
}

/// Row 50 — duplicate put under STRDUP must NOT re-strdup: the stored pointer
/// stays the same and `temp_key` is refreshed from the stored key.
#[test]
fn cfg50_strdup_duplicate_put() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x5050_5050);
    let mut first_loop_hits = 0usize;
    let mut wrap_loop_hits = 0usize;
    unsafe {
        for &sm in &[SH_STRDUP, SH_ARENA, SH_DEFAULT] {
            for &seed in &[1usize, 3, 0x31415926] {
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let mut keys = Keys::new();
                let (ks, _, texts) = string_keys(&mut rng, &mut keys, 10, 0, 1, 30);
                let mut p =
                    Pair::new(&c, &r, 16, 8, cmp_for(sm), format!("dup/sm{sm}/{seed}"));
                p.shmode(sm);
                p.set_default(1);
                for (i, &k) in ks.iter().enumerate() {
                    p.put(k, HM_STRING, i as u64);
                }
                let ptrs_c: Vec<_> = (0..ks.len()).map(|i| stored_ptr(&p.c, i as isize)).collect();
                let ptrs_r: Vec<_> = (0..ks.len()).map(|i| stored_ptr(&p.r, i as isize)).collect();
                let arena_before = p.c.snap(cmp_for(sm)).str_remaining;
                for round in 0..5u64 {
                    for (i, &k) in ks.iter().enumerate() {
                        let idx = p.put(k, HM_STRING, 500 + round);
                        assert_eq!(idx, i as isize);
                        // `stbds_temp_key` is only refreshed when the FIRST
                        // probe loop matched; the wrap-around loop in
                        // stbds_hmput_key omits it (an stb quirk). So all that
                        // can be required here is that C and Rust agree — which
                        // is exactly what Pair::temp_key() asserts.
                        let tk = p.temp_key();
                        if tk.as_deref() == Some(texts[i].as_slice()) {
                            first_loop_hits += 1;
                        } else {
                            wrap_loop_hits += 1;
                        }
                        assert_eq!(
                            stored_ptr(&p.c, idx),
                            ptrs_c[i],
                            "duplicate put must not re-allocate the key (C)"
                        );
                        assert_eq!(
                            stored_ptr(&p.r, idx),
                            ptrs_r[i],
                            "duplicate put must not re-allocate the key (Rust)"
                        );
                    }
                }
                if sm == SH_ARENA {
                    assert_eq!(
                        p.c.snap(cmp_for(sm)).str_remaining,
                        arena_before,
                        "duplicate puts must not consume arena space"
                    );
                }
                p.free();
            }
        }
        // both branches of the found-key path must have been exercised
        assert!(first_loop_hits > 0, "the first probe loop never matched");
        assert!(
            wrap_loop_hits > 0,
            "the wrap-around probe loop (which skips the temp_key update) never matched"
        );
    }
}

/// Rows 51, 52, 53 — deleting under each string mode: STRDUP frees the stored
/// key, ARENA and DEFAULT do not; all three memmove the tail element and re-find
/// its slot through `*(char**)`.
#[test]
fn cfg51_53_string_delete_modes() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x5153_5153);
    unsafe {
        for &sm in &[SH_STRDUP, SH_ARENA, SH_DEFAULT] {
            for n in 1..=20usize {
                for victim in 0..n {
                    let seed = rng.next_u64() as usize;
                    (c.rand_seed)(seed);
                    (r.rand_seed)(seed);
                    let mut keys = Keys::new();
                    let (ks, _, _) = string_keys(&mut rng, &mut keys, n, 0, 1, 24);
                    let mut p = Pair::new(
                        &c,
                        &r,
                        16,
                        8,
                        cmp_for(sm),
                        format!("sdel/sm{sm}/n{n}/v{victim}"),
                    );
                    p.shmode(sm);
                    p.set_default(1);
                    for (i, &k) in ks.iter().enumerate() {
                        p.put(k, HM_STRING, i as u64);
                    }
                    assert_eq!(p.del(ks[victim], 0, HM_STRING), 1);
                    assert_eq!(p.len(), n as isize - 1);
                    assert_eq!(p.geti(ks[victim], HM_STRING), -1);
                    for (i, &k) in ks.iter().enumerate() {
                        if i == victim {
                            continue;
                        }
                        let idx = p.geti(k, HM_STRING);
                        assert!(idx >= 0, "survivor {i} lost");
                    }
                    // DEFAULT mode must leave the caller's buffers untouched
                    if sm == SH_DEFAULT {
                        for (i, &k) in ks.iter().enumerate() {
                            if i == victim {
                                continue;
                            }
                            let idx = p.geti(k, HM_STRING);
                            assert_eq!(stored_ptr(&p.c, idx) as *mut c_void, k);
                            assert_eq!(stored_ptr(&p.r, idx) as *mut c_void, k);
                        }
                    }
                    p.free();
                }
            }
        }
    }
}

/// Row 54 — prefix keys and keys differing only in the last byte.
#[test]
fn cfg54_prefix_keys() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        let mut sets: Vec<Vec<Vec<u8>>> = Vec::new();
        sets.push(vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"ab".to_vec(),
            b"abc".to_vec(),
            b"abcd".to_vec(),
            b"abcde".to_vec(),
            b"abcdef".to_vec(),
            b"abcdefg".to_vec(),
            b"abcdefgh".to_vec(),
            b"abcdefghi".to_vec(),
        ]);
        sets.push(
            (0u8..26)
                .map(|i| {
                    let mut v = b"prefix_common_part_".to_vec();
                    v.push(b'a' + i);
                    v
                })
                .collect(),
        );
        sets.push(vec![
            b"test_0".to_vec(),
            b"test_1".to_vec(),
            b"test_10".to_vec(),
            b"test_100".to_vec(),
            b"test_2".to_vec(),
            b"test_20".to_vec(),
        ]);
        for &sm in &[SH_STRDUP, SH_ARENA, SH_DEFAULT] {
            for (si, set) in sets.iter().enumerate() {
                for &seed in &[1usize, 0x31415926, usize::MAX] {
                    (c.rand_seed)(seed);
                    (r.rand_seed)(seed);
                    let mut keys = Keys::new();
                    let ptrs: Vec<_> = set.iter().map(|s| keys.cstr(s)).collect();
                    let mut p = Pair::new(
                        &c,
                        &r,
                        16,
                        8,
                        cmp_for(sm),
                        format!("prefix/sm{sm}/set{si}/{seed}"),
                    );
                    p.shmode(sm);
                    p.set_default(1);
                    for (i, &k) in ptrs.iter().enumerate() {
                        assert_eq!(p.put(k, HM_STRING, i as u64), i as isize);
                    }
                    for (i, &k) in ptrs.iter().enumerate() {
                        assert_eq!(p.geti(k, HM_STRING), i as isize);
                    }
                    for &k in &ptrs {
                        assert_eq!(p.del(k, 0, HM_STRING), 1);
                    }
                    p.free();
                }
            }
        }
    }
}

/// Row 55 — keys with bytes >= 0x80 (the `(unsigned char)` promotion in
/// `stbds_hash_string`).
#[test]
fn cfg55_high_byte_keys() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x5555_5555);
    unsafe {
        for &sm in &[SH_STRDUP, SH_ARENA, SH_DEFAULT] {
            for &seed in &[1usize, 0x31415926, usize::MAX] {
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let mut keys = Keys::new();
                let mut ptrs = Vec::new();
                let mut seen = BTreeSet::new();
                // all-high-byte, alternating, and random-high keys
                for i in 0..30usize {
                    let len = 1 + i % 12;
                    let mut s: Vec<u8> = (0..len)
                        .map(|j| match i % 3 {
                            0 => 0x80 | ((i + j) as u8 & 0x7F),
                            1 => {
                                if j % 2 == 0 {
                                    0xFF
                                } else {
                                    (b'a' as usize + j) as u8
                                }
                            }
                            _ => {
                                let b = rng.next_u8();
                                if b == 0 {
                                    0x80
                                } else {
                                    b
                                }
                            }
                        })
                        .collect();
                    s.push(0x80 | (i as u8 & 0x7F));
                    if !seen.insert(s.clone()) {
                        continue;
                    }
                    ptrs.push(keys.cstr(&s));
                }
                let mut p = Pair::new(&c, &r, 16, 8, cmp_for(sm), format!("hi/sm{sm}/{seed}"));
                p.shmode(sm);
                p.set_default(1);
                for (i, &k) in ptrs.iter().enumerate() {
                    assert_eq!(p.put(k, HM_STRING, i as u64), i as isize);
                }
                for (i, &k) in ptrs.iter().enumerate() {
                    assert_eq!(p.geti(k, HM_STRING), i as isize);
                }
                for &k in &ptrs {
                    assert_eq!(p.del(k, 0, HM_STRING), 1);
                }
                p.free();
            }
        }
    }
}

/// Row 56 — fuzz row: 200 randomized scripts over DEFAULT / STRDUP / ARENA,
/// random `elemsize`, random key lengths, and `mode` drawn from every value
/// that takes the string path.
#[test]
fn cfg56_string_fuzz() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x5656_5656);
    unsafe {
        for script in 0..200u64 {
            let sm = [SH_DEFAULT, SH_STRDUP, SH_ARENA][rng.below(3)];
            let elemsize = [16usize, 24, 32, 40][rng.below(4)];
            let seed = rng.next_u64() as usize;
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut keys = Keys::new();
            let (pool, _, _) = string_keys(&mut rng, &mut keys, 32, 0, 0, 40);
            let mut p = Pair::new(
                &c,
                &r,
                elemsize,
                8,
                cmp_for(sm),
                format!("sfuzz/{script}/sm{sm}/es{elemsize}"),
            );
            // half the scripts bootstrap through hmput_key instead of shmode_func
            let use_shmode = rng.below(2) == 0;
            let effective_sm = if use_shmode {
                p.shmode(sm);
                sm
            } else {
                SH_DEFAULT
            };
            p.cmp = cmp_for(effective_sm);
            if rng.below(2) == 0 {
                p.set_default(script);
            }
            let ops = rng.range(20, 120);
            for op in 0..ops {
                let k = pool[rng.below(pool.len())];
                // any mode >= 1 takes the string path in put/get; hmdel only
                // frees strdup'ed keys when mode == 1 exactly, so restrict
                // delete to 1 here (mode >= 2 delete is ERRORS.md row 19).
                let get_mode = [1 as c_int, 2, 7, 255, c_int::MAX][rng.below(5)];
                match rng.below(10) {
                    0..=4 => {
                        p.put(k, HM_STRING, op as u64);
                    }
                    5..=6 => {
                        p.geti(k, get_mode);
                    }
                    7 => {
                        p.geti_ts(k, get_mode);
                    }
                    8 => {
                        p.del(k, 0, HM_STRING);
                    }
                    _ => {
                        p.len();
                    }
                }
            }
            for &k in &pool {
                p.geti(k, HM_STRING);
            }
            p.free();
        }
    }
}

/// Row 57 — `stbds_hmfree_func` under each `string.mode` (the STRDUP path runs a
/// per-element `free` loop; ARENA runs `stbds_strreset`).
#[test]
fn cfg57_hmfree_string_modes() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x5757_5757);
    unsafe {
        for &sm in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for n in [0usize, 1, 6, 12, 40] {
                for &elemsize in &[16usize, 32] {
                    let seed = rng.next_u64() as usize;
                    (c.rand_seed)(seed);
                    (r.rand_seed)(seed);
                    let mut keys = Keys::new();
                    let (ks, _, _) = string_keys(&mut rng, &mut keys, n, 0, 1, 60);
                    let mut p = Pair::new(
                        &c,
                        &r,
                        elemsize,
                        8,
                        cmp_for(sm),
                        format!("free/sm{sm}/n{n}/es{elemsize}"),
                    );
                    p.shmode(sm);
                    p.set_default(1);
                    for (i, &k) in ks.iter().enumerate() {
                        p.put(k, HM_STRING, i as u64);
                    }
                    p.free();
                    assert!(p.c.t.is_null() && p.r.t.is_null());
                }
            }
        }
    }
}

/// Row 44 — `STBDS_SH_NONE` with `mode == STBDS_HM_STRING`: the `switch` falls
/// through to `default:` and `memcpy`s `keysize` bytes of the key's *text* into
/// the element, where later lookups will reinterpret them as a `char*`.
///
/// Inserting distinct keys is well defined (no `stbds_is_key_equal` call happens
/// unless the full 64-bit hashes collide); the *lookup* of an existing key
/// dereferences the copied text as a pointer and is therefore fatal — that half
/// is covered by `errors.rs::fatal_sh_none_string_lookup`.
#[test]
fn cfg44_sh_none_string_insert_only() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4444_4444);
    unsafe {
        for &seed in &[1usize, 3, 0x31415926, usize::MAX] {
            for n in 1..=12usize {
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let mut keys = Keys::new();
                let (ks, _, texts) = string_keys(&mut rng, &mut keys, n, 0, 9, 24);
                let mut p = Pair::new(
                    &c,
                    &r,
                    16,
                    8,
                    ElemCmp::Raw,
                    format!("shnone/n{n}/seed{seed}"),
                );
                p.shmode(SH_NONE);
                p.set_default(1);
                for (i, &k) in ks.iter().enumerate() {
                    let idx = p.put(k, HM_STRING, i as u64);
                    assert_eq!(idx, i as isize);
                    // the element now holds the first 8 bytes of the key TEXT
                    let raw = core::slice::from_raw_parts(elem(p.c.t, 16, idx), 8);
                    assert_eq!(raw, &texts[i][..8], "SH_NONE memcpy of key text");
                }
                p.free();
            }
        }
    }
}

/// Extra (deep-growth soak, string modes) — 800 distinct string keys per
/// `string.mode`, driven to 1024+ slots and back down, with the full state
/// (including the arena's `remaining`/`block` and every stored key's text)
/// compared periodically.
#[test]
fn cfg_string_deep_growth_soak() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x50A4_5748_1111_2222);
    unsafe {
        for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &seed in &[1usize, 0x31415926] {
                (c.rand_seed)(seed);
                (r.rand_seed)(seed);
                let elemsize = 16usize;
                let n = 800usize;
                let mut keys = Keys::new();
                let (ks, _, texts) = string_keys(&mut rng, &mut keys, n, 0, 1, 60);
                let cmp = cmp_for(sm);
                let mut cm = Map::new(&c, elemsize, 8);
                let mut rm = Map::new(&r, elemsize, 8);
                cm.shmode(sm);
                rm.shmode(sm);
                cm.set_default(1);
                rm.set_default(1);
                let chk = |cm: &Map, rm: &Map, what: &str, i: usize| {
                    let a = cm.snap(cmp);
                    let b = rm.snap(cmp);
                    if a != b {
                        panic!(
                            "[soak/sm{sm}] {what} at op {i} diverged\n{}",
                            diff_snaps(&a, &b)
                        );
                    }
                    a
                };
                let mut peak = 0usize;
                for (i, &k) in ks.iter().enumerate() {
                    let ta = cm.put(k, HM_STRING, i as u64);
                    let tb = rm.put(k, HM_STRING, i as u64);
                    assert_eq!(ta, tb);
                    assert_eq!(ta, i as isize);
                    assert_eq!(stored_text(&cm, ta).as_deref(), Some(texts[i].as_slice()));
                    assert_eq!(stored_text(&rm, ta).as_deref(), Some(texts[i].as_slice()));
                    if i < 150 || i >= n - 150 || i % 31 == 0 {
                        peak = peak.max(chk(&cm, &rm, "insert", i).slot_count);
                    }
                }
                assert!(peak >= 1024, "expected deep growth, peak = {peak}");
                for (i, &k) in ks.iter().enumerate() {
                    assert_eq!(cm.geti(k, HM_STRING), i as isize);
                    assert_eq!(rm.geti(k, HM_STRING), i as isize);
                }
                let mut order: Vec<usize> = (0..n).collect();
                for i in (1..order.len()).rev() {
                    let j = rng.below(i + 1);
                    order.swap(i, j);
                }
                for (step, &i) in order.iter().enumerate() {
                    let ta = cm.del(ks[i], 0, HM_STRING);
                    let tb = rm.del(ks[i], 0, HM_STRING);
                    assert_eq!(ta, tb, "del at step {step}");
                    assert_eq!(ta, 1);
                    if step < 150 || step >= n - 150 || step % 31 == 0 {
                        chk(&cm, &rm, "delete", step);
                    }
                }
                let end = chk(&cm, &rm, "final", n);
                assert_eq!(end.slot_count, BUCKET_LENGTH);
                assert_eq!(end.length, 1);
                cm.free();
                rm.free();
            }
        }
    }
}
