//! Phase B — CONFIGS.md rows 76..79
//! Long randomized op sequences that cross every axis at once.  The *entire*
//! observable state (array header, every element, hash-index fields, every
//! bucket `hash[]`/`index[]`) is compared after EVERY operation.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

/// Exact model of `stbds_arrgrowf` — `None` means "returns the input pointer".
fn grow_model(arrlen: usize, arrcap: usize, addlen: usize, min_cap: usize) -> Option<usize> {
    let mut mc = min_cap;
    let min_len = arrlen + addlen;
    if min_len > mc {
        mc = min_len;
    }
    if mc <= arrcap {
        return None;
    }
    if mc < 2 * arrcap {
        mc = 2 * arrcap;
    } else if mc < 4 {
        mc = 4;
    }
    Some(mc)
}

// -------------------------------------------------------------------- row 76
#[test]
fn c76_binary_op_sequences() {
    let mut rng = Rng::new(0x7676);
    for seq in 0..3000 {
        let elemsize = *rng.pick(&[8usize, 12, 16, 24, 32, 40, 64]);
        let keysize = *rng.pick(&[0usize, 1, 2, 3, 4, 8, 16]).min(&elemsize);
        let mode = *rng.pick(&[0i32, -1, c_int::MIN, -7]);
        let universe = 1 + rng.below(40);
        let _g = seed_guard(if seq % 3 == 0 {
            DEFAULT_SEED
        } else {
            rng.next_u64() as usize
        });
        unsafe {
            let mut mp = MapPair::new(
                elemsize,
                KeyRepr::Raw,
                format!("fuzz-bin {} es={} ks={} mode={}", seq, elemsize, keysize, mode),
            )
            .with_value_offset(keysize);
            let keys: Vec<Vec<u8>> = (0..universe)
                .map(|i| {
                    let mut v = rng.bytes(keysize.max(1));
                    v[0] = i as u8;
                    v
                })
                .collect();
            let mut present: std::collections::HashSet<usize> = std::collections::HashSet::new();
            for _ in 0..200 {
                let pick = rng.below(keys.len());
                let mut k = keys[pick].clone();
                match rng.below(24) {
                    0..=9 => {
                        let idx = same_idx("fuzz-bin", mp.put(&mut k, keysize, mode));
                        assert!(idx >= 0);
                        if keysize > 0 {
                            present.insert(pick);
                        }
                    }
                    10..=15 => {
                        let t = same_idx("fuzz-bin", mp.del(&mut k, keysize, 0, mode));
                        if keysize > 0 {
                            assert_eq!(
                                t,
                                if present.remove(&pick) { 1 } else { 0 },
                                "delete result for key {}",
                                pick
                            );
                        }
                    }
                    16..=19 => {
                        let idx = same_idx("fuzz-bin", mp.get(&mut k, keysize, mode));
                        if keysize > 0 {
                            assert_eq!(idx >= 0, present.contains(&pick));
                        }
                    }
                    20..=21 => {
                        same_idx("fuzz-bin", mp.get_ts(&mut k, keysize, mode));
                    }
                    22 => mp.put_default(),
                    _ => {
                        // free and start over from NULL
                        mp.free();
                        present.clear();
                        mp.check("after free");
                    }
                }
            }
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 77
#[test]
fn c77_string_op_sequences() {
    let mut rng = Rng::new(0x7777);
    for seq in 0..3000 {
        let elemsize = *rng.pick(&[8usize, 16, 24, 32, 64]);
        let str_mode = *rng.pick(&[SH_DEFAULT, SH_STRDUP, SH_ARENA]);
        let mode: c_int = 1; // deletes require mode == STBDS_HM_STRING (see c70)
        let universe = 1 + rng.below(40);
        let _g = seed_guard(if seq % 3 == 0 {
            DEFAULT_SEED
        } else {
            rng.next_u64() as usize
        });
        unsafe {
            let mut mp = MapPair::new(
                elemsize,
                KeyRepr::Pointer,
                format!("fuzz-str {} es={} sm={}", seq, elemsize, str_mode),
            )
            .with_value_offset(8.min(elemsize));
            mp.shmode(str_mode as c_int);
            mp.put_default();

            // key shapes: empty, short, long (>512 => arena oversized path),
            // high bytes, shared prefixes
            let keys: Vec<Vec<u8>> = (0..universe)
                .map(|i| match i % 5 {
                    0 => vec![0u8],
                    1 => {
                        let l = 1 + rng.below(20);
                        let mut v = rng.cstring(l, ASCII);
                        v.insert(0, b'p');
                        v
                    }
                    2 => {
                        let l = 500 + rng.below(1200);
                        rng.cstring(l, ASCII)
                    }
                    3 => {
                        let l = 1 + rng.below(30);
                        rng.cstring(l, HIGHBYTES)
                    }
                    _ => format!("test_{}\0", i).into_bytes(),
                })
                .collect();
            // de-duplicate so the presence model stays exact
            let mut uniq: Vec<Vec<u8>> = Vec::new();
            for k in keys {
                if !uniq.contains(&k) {
                    uniq.push(k);
                }
            }
            // NOTE: with `string.mode == SH_DEFAULT` the map stores the
            // CALLER's key pointer, so the buffers must outlive the map — a
            // temporary clone would leave a dangling pointer behind.
            let mut keys = uniq;
            let mut present: std::collections::HashSet<usize> = std::collections::HashSet::new();

            for _ in 0..120 {
                let pick = rng.below(keys.len());
                match rng.below(22) {
                    0..=9 => {
                        // `temp_key` is only written on the insert path and on
                        // the upper-half duplicate hit, so it is NOT a
                        // well-defined observable here (see ERRORS.md #21).
                        // It is compared under controlled conditions in
                        // errors.rs::e20/e21 instead.
                        let idx = same_idx("fuzz-str", mp.put(&mut keys[pick], 8, mode));
                        assert!(idx >= 0);
                        present.insert(pick);
                    }
                    10..=14 => {
                        let t = same_idx("fuzz-str", mp.del(&mut keys[pick], 8, 0, mode));
                        assert_eq!(
                            t,
                            if present.remove(&pick) { 1 } else { 0 },
                            "delete result"
                        );
                    }
                    15..=18 => {
                        let idx = same_idx("fuzz-str", mp.get(&mut keys[pick], 8, mode));
                        assert_eq!(idx >= 0, present.contains(&pick));
                    }
                    19..=20 => {
                        same_idx("fuzz-str", mp.get_ts(&mut keys[pick], 8, mode));
                    }
                    _ => mp.put_default(),
                }
            }
            mp.free();
        }
    }
}

/// Put-only sequences for the `string.mode` values that store raw key bytes
/// (`default:` branch of the `switch`).  Look-ups would reinterpret those bytes
/// as a `char *`, which is undefined behaviour in the C original.
#[test]
fn c77b_string_memcpy_storage_sequences() {
    let mut rng = Rng::new(0x77B0);
    for seq in 0..1000 {
        let elemsize = *rng.pick(&[16usize, 24, 32, 64]);
        let str_mode = *rng.pick(&[SH_NONE, 4u8, 200u8, 255u8]);
        let mode = *rng.pick(&[1i32, 2, 9, c_int::MAX]);
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(
                elemsize,
                KeyRepr::Raw,
                format!("fuzz-memcpy {} es={} sm={} mode={}", seq, elemsize, str_mode, mode),
            )
            .with_value_offset(8);
            mp.shmode(str_mode as c_int);
            mp.put_default();
            for i in 0..20 {
                let mut k = format!("memcpy_key_{}_{}\0", seq, i).into_bytes();
                assert!(same_idx("fuzz-memcpy", mp.put(&mut k, 8, mode)) >= 0);
            }
            mp.free();
        }
    }
}

// -------------------------------------------------------------------- row 78
#[test]
fn c78_array_op_sequences() {
    let p = pair();
    let mut rng = Rng::new(0x7878);
    for seq in 0..3000 {
        let elemsize = *rng.pick(&[0usize, 1, 2, 3, 4, 8, 12, 16, 24, 32, 64]);
        unsafe {
            let mut ac: *mut c_void = std::ptr::null_mut();
            let mut ar: *mut c_void = std::ptr::null_mut();
            let mut len = 0usize;
            for step in 0..40 {
                let addlen = *rng.pick(&[0usize, 1, 2, 3, 5, 8, 17, 64]);
                let min_cap = *rng.pick(&[0usize, 1, 2, 4, 7, 9, 33, 100]);
                let oc = ac;
                let or = ar;
                let prev_cap = if ac.is_null() {
                    0
                } else {
                    (*((ac as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).capacity
                };
                ac = (p.c.arrgrowf)(ac, elemsize, addlen, min_cap);
                ar = (p.r.arrgrowf)(ar, elemsize, addlen, min_cap);
                // A *reallocating* call returns an allocator-chosen address, so
                // only the DECISION to return the input unchanged is comparable.
                let expect = grow_model(len, prev_cap, addlen, min_cap);
                if expect.is_none() {
                    assert!(
                        ac == oc && ar == or,
                        "seq {} step {} es={} addlen={} min_cap={}: both must return the input",
                        seq,
                        step,
                        elemsize,
                        addlen,
                        min_cap
                    );
                }
                if ac.is_null() {
                    assert!(ar.is_null());
                    continue;
                }
                let cap = (*((ac as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).capacity;
                if let Some(m) = expect {
                    assert_eq!(cap, m, "seq {} step {} capacity model", seq, step);
                    assert_eq!(
                        (*((ar as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).capacity,
                        m,
                        "seq {} step {} Rust capacity model",
                        seq,
                        step
                    );
                }
                len = (len + addlen).min(cap);
                for a in [ac, ar] {
                    (*((a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length = len;
                }
                if elemsize > 0 && len > 0 {
                    let bytes = rng.bytes(len * elemsize);
                    for a in [ac, ar] {
                        std::ptr::copy_nonoverlapping(bytes.as_ptr(), a as *mut u8, bytes.len());
                    }
                }
                let sc = snap_raw(ac, elemsize, KeyRepr::Raw);
                let sr = snap_raw(ar, elemsize, KeyRepr::Raw);
                assert!(
                    sc == sr,
                    "seq {} step {} es={}: DIVERGENCE\n C={:#?}\n R={:#?}",
                    seq,
                    step,
                    elemsize,
                    sc,
                    sr
                );
            }
            if !ac.is_null() {
                (p.c.arrfreef)(ac);
            }
            if !ar.is_null() {
                (p.r.arrfreef)(ar);
            }
        }
    }
}

// -------------------------------------------------------------------- row 79
#[test]
fn c79_arena_op_sequences() {
    let p = pair();
    let mut rng = Rng::new(0x7979);
    for seq in 0..3000 {
        unsafe {
            let mut sc = StringArena::zeroed();
            let mut sr = StringArena::zeroed();
            let mut live: Vec<(*mut c_char, *mut c_char, Vec<u8>)> = Vec::new();
            for step in 0..30 {
                if rng.below(16) == 0 {
                    (p.c.strreset)(&mut sc);
                    (p.r.strreset)(&mut sr);
                    live.clear();
                } else {
                    let len = match rng.below(8) {
                        0 => 0,
                        1 => 500 + rng.below(3000),
                        _ => rng.below(150),
                    };
                    let alphabet: &[u8] = if rng.below(2) == 0 { ASCII } else { HIGHBYTES };
                    let mut s = rng.cstring(len, alphabet);
                    let pc = (p.c.stralloc)(&mut sc, s.as_mut_ptr() as *mut c_char);
                    let pr = (p.r.stralloc)(&mut sr, s.as_mut_ptr() as *mut c_char);
                    let want = s[..s.len() - 1].to_vec();
                    assert_eq!(read_cstr(pc), want, "seq {} step {} C content", seq, step);
                    assert_eq!(read_cstr(pr), want, "seq {} step {} Rust content", seq, step);
                    live.push((pc, pr, want));
                }
                assert_eq!(
                    (sc.remaining, sc.block, sc.mode, sc.storage.is_null()),
                    (sr.remaining, sr.block, sr.mode, sr.storage.is_null()),
                    "seq {} step {}: arena field divergence",
                    seq,
                    step
                );
                for (i, (pc, pr, want)) in live.iter().enumerate() {
                    assert_eq!(&read_cstr(*pc), want, "seq {} live {} corrupted in C", seq, i);
                    assert_eq!(
                        &read_cstr(*pr),
                        want,
                        "seq {} live {} corrupted in Rust",
                        seq,
                        i
                    );
                }
            }
            (p.c.strreset)(&mut sc);
            (p.r.strreset)(&mut sr);
        }
    }
}
