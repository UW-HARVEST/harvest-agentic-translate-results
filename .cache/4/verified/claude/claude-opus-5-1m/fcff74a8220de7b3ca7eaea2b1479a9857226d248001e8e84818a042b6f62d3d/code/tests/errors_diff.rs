//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input / boundary condition, calls
//! BOTH `.so` files and asserts they return the same sentinel / take the same
//! branch.
mod common;

use common::*;
use std::collections::HashSet;
use std::ffi::{c_char, c_int, c_void};

fn distinct_bin_keys(rng: &mut Rng, keysize: usize, n: usize) -> Vec<Vec<u8>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let k = rng.bytes(keysize);
        if seen.insert(k.clone()) {
            out.push(k);
        }
    }
    out
}

fn distinct_str_keys(rng: &mut Rng, n: usize, minlen: usize, maxlen: usize) -> Vec<Vec<u8>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let len = minlen + rng.below(maxlen - minlen + 1);
        let k = rng.cstring(len);
        if seen.insert(k.clone()) {
            out.push(k);
        }
    }
    out
}

unsafe fn raw_hdr(a: *mut c_void) -> ArrayHeader {
    *(a as *mut ArrayHeader).sub(1)
}

fn dump_raw(a: *mut c_void) -> Vec<String> {
    unsafe {
        if a.is_null() {
            return vec!["NULL".into()];
        }
        let h = raw_hdr(a);
        vec![
            format!("length={}", h.length),
            format!("capacity={}", h.capacity),
            format!("temp={}", h.temp),
            format!("has_table={}", !h.hash_table.is_null()),
        ]
    }
}

// ===========================================================================
// E1..E5, G5 — stbds_arrgrowf boundaries
// ===========================================================================

/// E1 — `min_cap <= arrcap(a)`: returns `a` unchanged, no realloc.
#[test]
fn err_arrgrowf_no_grow_returns_same_ptr() {
    let l = libs();
    unsafe {
        // fresh array with capacity 4
        let mut ca = (l.c.arrgrowf)(std::ptr::null_mut(), 8, 0, 1);
        let mut ra = (l.r.arrgrowf)(std::ptr::null_mut(), 8, 0, 1);
        assert_same("setup", &dump_raw(ca), &dump_raw(ra));
        assert_eq!(raw_hdr(ca).capacity, 4);
        for (addlen, min_cap) in [(0usize, 0usize), (0, 1), (0, 4), (1, 0), (4, 0), (2, 3)] {
            let cb = (l.c.arrgrowf)(ca, 8, addlen, min_cap);
            let rb = (l.r.arrgrowf)(ra, 8, addlen, min_cap);
            assert_eq!(cb, ca, "C must return the same pointer ({addlen},{min_cap})");
            assert_eq!(rb, ra, "RUST must return the same pointer ({addlen},{min_cap})");
            assert_same(
                &format!("no-grow ({addlen},{min_cap})"),
                &dump_raw(cb),
                &dump_raw(rb),
            );
            ca = cb;
            ra = rb;
        }
        // and `a == NULL` with min_cap 0 returns NULL (0 <= arrcap(NULL) == 0)
        let cn = (l.c.arrgrowf)(std::ptr::null_mut(), 8, 0, 0);
        let rn = (l.r.arrgrowf)(std::ptr::null_mut(), 8, 0, 0);
        assert!(cn.is_null(), "C: arrgrowf(NULL,8,0,0) must return NULL");
        assert!(rn.is_null(), "RUST: arrgrowf(NULL,8,0,0) must return NULL");
        (l.c.arrfreef)(ca);
        (l.r.arrfreef)(ra);
    }
}

/// E2 / E3 — `a == NULL` header initialisation and the `min_cap < 4` bump.
#[test]
fn err_arrgrowf_null_input() {
    let l = libs();
    unsafe {
        for (elemsize, addlen, min_cap) in [
            (8usize, 0usize, 1usize),
            (8, 1, 0),
            (8, 0, 2),
            (8, 0, 3),
            (8, 0, 4),
            (8, 0, 5),
            (1, 0, 1),
            (64, 3, 3),
        ] {
            let ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
            let ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
            assert_same(
                &format!("arrgrowf(NULL,{elemsize},{addlen},{min_cap})"),
                &dump_raw(ca),
                &dump_raw(ra),
            );
            let h = raw_hdr(ca);
            assert_eq!(h.length, 0);
            assert_eq!(h.temp, 0);
            assert!(h.hash_table.is_null());
            (l.c.arrfreef)(ca);
            (l.r.arrfreef)(ra);
        }
    }
}

/// E3 — exactly the `min_cap < 4` boundary.
#[test]
fn err_arrgrowf_min_cap_below_4() {
    let l = libs();
    unsafe {
        for min_cap in [1usize, 2, 3, 4, 5] {
            let ca = (l.c.arrgrowf)(std::ptr::null_mut(), 8, 0, min_cap);
            let ra = (l.r.arrgrowf)(std::ptr::null_mut(), 8, 0, min_cap);
            let want = if min_cap < 4 { 4 } else { min_cap };
            assert_eq!(raw_hdr(ca).capacity, want, "C capacity for min_cap={min_cap}");
            assert_eq!(
                raw_hdr(ra).capacity,
                want,
                "RUST capacity for min_cap={min_cap}"
            );
            assert_same(
                &format!("min_cap={min_cap}"),
                &dump_raw(ca),
                &dump_raw(ra),
            );
            (l.c.arrfreef)(ca);
            (l.r.arrfreef)(ra);
        }
    }
}

/// E4 / G5 — `elemsize * min_cap + sizeof(header)` wraps around `size_t`
/// (unsigned overflow is well defined in C and must wrap in Rust too).
#[test]
fn err_arrgrowf_size_overflow_wraps() {
    let l = libs();
    unsafe {
        // elemsize = SIZE_MAX/2, min_cap becomes 4 => 4*(2^63-1)+32 == 28
        let elemsize = usize::MAX / 2;
        let ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
        let ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
        assert_same("arrgrowf size overflow", &dump_raw(ca), &dump_raw(ra));
        assert_eq!(raw_hdr(ca).capacity, 4);
        assert_eq!(raw_hdr(ra).capacity, 4);
        (l.c.arrfreef)(ca);
        (l.r.arrfreef)(ra);
    }
}

/// E5 — `arrlen(a) + addlen` wraps.
#[test]
fn err_arrgrowf_addlen_overflow() {
    let l = libs();
    unsafe {
        // elemsize 1, addlen SIZE_MAX => min_cap = SIZE_MAX, size = SIZE_MAX+32 = 31
        let ca = (l.c.arrgrowf)(std::ptr::null_mut(), 1, usize::MAX, 0);
        let ra = (l.r.arrgrowf)(std::ptr::null_mut(), 1, usize::MAX, 0);
        assert_same("arrgrowf addlen overflow", &dump_raw(ca), &dump_raw(ra));
        assert_eq!(raw_hdr(ca).capacity, usize::MAX);
        assert_eq!(raw_hdr(ra).capacity, usize::MAX);
        (l.c.arrfreef)(ca);
        (l.r.arrfreef)(ra);
    }
}

// ===========================================================================
// E6 / E7 — stbds_hmfree_func
// ===========================================================================

/// E6 — `a == NULL` returns immediately.
#[test]
fn err_hmfree_null() {
    let l = libs();
    unsafe {
        for elemsize in [0usize, 1, 8, 16, usize::MAX] {
            (l.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (l.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

/// E7 — map whose `hash_table` is NULL: the strdup loop and `strreset` are
/// skipped, `free(NULL)` + `free(header)` still happen.
#[test]
fn err_hmfree_no_table() {
    let _g = seed_lock();
    let l = libs();
    let shape = Shape::bin(16, 8);
    unsafe {
        // an array with no hash index, exactly as `hmget_key` leaves it
        let mut temp: isize = 0;
        let mut key = [1u8; 8];
        let cp = (l.c.hmget_key_ts)(
            std::ptr::null_mut(),
            shape.elemsize,
            key.as_mut_ptr() as *mut c_void,
            shape.keysize,
            &mut temp,
            HM_BINARY,
        );
        let ctemp = temp;
        let rp = (l.r.hmget_key_ts)(
            std::ptr::null_mut(),
            shape.elemsize,
            key.as_mut_ptr() as *mut c_void,
            shape.keysize,
            &mut temp,
            HM_BINARY,
        );
        assert_eq!(ctemp, temp);
        assert!((*header_of(cp, shape.elemsize)).hash_table.is_null());
        assert!((*header_of(rp, shape.elemsize)).hash_table.is_null());
        assert_same(
            "map without index",
            &fingerprint(cp, shape),
            &fingerprint(rp, shape),
        );
        (l.c.hmfree_func)(
            (cp as *mut u8).sub(shape.elemsize) as *mut c_void,
            shape.elemsize,
        );
        (l.r.hmfree_func)(
            (rp as *mut u8).sub(shape.elemsize) as *mut c_void,
            shape.elemsize,
        );
    }
}

// ===========================================================================
// E8 / E12 — misses return -1
// ===========================================================================

/// E8 / E12 — a missing key returns `STBDS_INDEX_EMPTY (-1)` (the empty slot is
/// found while scanning the upper half of a bucket).
#[test]
fn err_get_missing_key_returns_minus1() {
    let _g = seed_lock();
    let mut rng = Rng::new(0xE8_0000);
    for shape in [Shape::bin(16, 8), Shape::bin(8, 4)] {
        for n in [0usize, 1, 3, 6, 7, 20, 100] {
            seed_both(0x3141_5926);
            let mut m = MapPair::empty(shape);
            m.ctx = format!("miss n={n}");
            let keys = distinct_bin_keys(&mut rng, shape.keysize, n + 50);
            unsafe {
                for k in keys.iter().take(n) {
                    let v = rng.bytes(8);
                    m.put(k, HM_BINARY, &v, "setup");
                }
                for (i, k) in keys.iter().skip(n).enumerate() {
                    assert_eq!(
                        m.get(k, HM_BINARY, &format!("miss#{i}")),
                        INDEX_EMPTY,
                        "miss must be -1"
                    );
                    assert_eq!(
                        m.get_ts(k, HM_BINARY, &format!("miss_ts#{i}")),
                        INDEX_EMPTY
                    );
                }
                m.free();
            }
        }
    }
}

/// E9 — the miss is detected in the *wrapped* half of the bucket
/// (`for (i = 0; i < limit; ++i)` at lib.c L615-623).
///
/// Forced deterministically by crafting the bucket contents identically in both
/// libraries: every slot from `pos & 7` upwards is occupied by a non-matching,
/// non-empty hash, and the first slot of the wrapped scan range is EMPTY.
#[test]
fn err_get_missing_key_wrapped_half() {
    let _g = seed_lock();
    let l = libs();
    let mut rng = Rng::new(0xE9_0000);
    let shape = Shape::bin(16, 8);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "wrapped-half miss".into();
    unsafe {
        // one real entry so the array/index exist
        let k0 = rng.bytes(8);
        m.put(&k0, HM_BINARY, &[7u8; 8], "setup");
        let seed = (*m.c.table()).seed;
        assert_eq!(seed, (*m.r.table()).seed);
        assert_eq!((*m.c.table()).slot_count, 8);

        // pick a probe key whose start position is >= 2 so the wrapped range
        // (0..pos) has at least two slots
        let mut probe;
        let mut pos;
        loop {
            probe = rng.bytes(8);
            let mut h = (l.c.hash_bytes)(probe.as_ptr() as *mut c_void, 8, seed);
            assert_eq!(h, (l.r.hash_bytes)(probe.as_ptr() as *mut c_void, 8, seed));
            if h < 2 {
                h += 2;
            }
            pos = h & 7;
            if pos >= 2 && probe != k0 {
                break;
            }
        }
        // craft: slots [pos..8) busy with fake hashes, slot 0 busy, slot 1 EMPTY
        // (so the wrapped scan 0..pos hits an empty slot and returns -1)
        let mut slots = [(0usize, INDEX_EMPTY); 8];
        for (j, s) in slots.iter_mut().enumerate() {
            *s = if j >= pos || j == 0 {
                (0xF000_0000_0000_0000usize + j as usize, 0isize)
            } else if j == 1 {
                (0, INDEX_EMPTY)
            } else {
                (0xE000_0000_0000_0000usize + j as usize, 0isize)
            };
        }
        for t in [m.c.table(), m.r.table()] {
            let bucket = (*t).storage;
            for (j, (h, i)) in slots.iter().enumerate() {
                (*bucket).hash[j] = *h;
                (*bucket).index[j] = *i;
            }
        }
        m.check("crafted bucket");
        assert_eq!(
            m.get(&probe, HM_BINARY, "wrapped-half miss"),
            INDEX_EMPTY,
            "a miss found in the wrapped half must still be -1"
        );
        assert_eq!(
            m.get_ts(&probe, HM_BINARY, "wrapped-half miss (ts)"),
            INDEX_EMPTY
        );
        m.free();
    }
}

// ===========================================================================
// E10 / E11 / E13 — the get entry points on degenerate maps
// ===========================================================================

/// E10 — `stbds_hmget_key_ts(NULL, ...)`: allocates, `*temp = -1`.
#[test]
fn err_hmget_ts_null_map() {
    let _g = seed_lock();
    let l = libs();
    for shape in [Shape::bin(16, 8), Shape::bin(1, 1), Shape::str_ptr(16)] {
        let mode = if shape.key == KeyRepr::CStr {
            HM_STRING
        } else {
            HM_BINARY
        };
        unsafe {
            let mut key = [b'x' as u8; 16];
            let mut ctemp: isize = 0x1234;
            let mut rtemp: isize = 0x1234;
            let cp = (l.c.hmget_key_ts)(
                std::ptr::null_mut(),
                shape.elemsize,
                key.as_mut_ptr() as *mut c_void,
                shape.keysize,
                &mut ctemp,
                mode,
            );
            let rp = (l.r.hmget_key_ts)(
                std::ptr::null_mut(),
                shape.elemsize,
                key.as_mut_ptr() as *mut c_void,
                shape.keysize,
                &mut rtemp,
                mode,
            );
            assert_eq!(ctemp, INDEX_EMPTY, "C *temp");
            assert_eq!(rtemp, INDEX_EMPTY, "RUST *temp");
            assert!(!cp.is_null() && !rp.is_null());
            assert_eq!((*header_of(cp, shape.elemsize)).length, 1);
            assert_eq!((*header_of(cp, shape.elemsize)).capacity, 4);
            assert_same(
                &format!("hmget_key_ts(NULL) {shape:?}"),
                &fingerprint(cp, shape),
                &fingerprint(rp, shape),
            );
            (l.c.hmfree_func)(
                (cp as *mut u8).sub(shape.elemsize) as *mut c_void,
                shape.elemsize,
            );
            (l.r.hmfree_func)(
                (rp as *mut u8).sub(shape.elemsize) as *mut c_void,
                shape.elemsize,
            );
        }
    }
}

/// E11 — map with `hash_table == 0`: `*temp = -1` and `a` returned unchanged.
#[test]
fn err_hmget_ts_no_table() {
    let _g = seed_lock();
    let l = libs();
    let shape = Shape::bin(16, 8);
    unsafe {
        let mut key = [3u8; 8];
        let mut t: isize = 0;
        let mut cp = (l.c.hmget_key_ts)(
            std::ptr::null_mut(),
            shape.elemsize,
            key.as_mut_ptr() as *mut c_void,
            shape.keysize,
            &mut t,
            HM_BINARY,
        );
        let mut rp = (l.r.hmget_key_ts)(
            std::ptr::null_mut(),
            shape.elemsize,
            key.as_mut_ptr() as *mut c_void,
            shape.keysize,
            &mut t,
            HM_BINARY,
        );
        // now the array exists but has no index
        for round in 0..3 {
            let mut ctemp: isize = 0x7777;
            let mut rtemp: isize = 0x7777;
            let cp2 = (l.c.hmget_key_ts)(
                cp,
                shape.elemsize,
                key.as_mut_ptr() as *mut c_void,
                shape.keysize,
                &mut ctemp,
                HM_BINARY,
            );
            let rp2 = (l.r.hmget_key_ts)(
                rp,
                shape.elemsize,
                key.as_mut_ptr() as *mut c_void,
                shape.keysize,
                &mut rtemp,
                HM_BINARY,
            );
            assert_eq!(cp2, cp, "C must return `a` unchanged (round {round})");
            assert_eq!(rp2, rp, "RUST must return `a` unchanged (round {round})");
            assert_eq!(ctemp, -1);
            assert_eq!(rtemp, -1);
            assert_same(
                &format!("no-table get round {round}"),
                &fingerprint(cp, shape),
                &fingerprint(rp, shape),
            );
            cp = cp2;
            rp = rp2;
        }
        (l.c.hmfree_func)(
            (cp as *mut u8).sub(shape.elemsize) as *mut c_void,
            shape.elemsize,
        );
        (l.r.hmfree_func)(
            (rp as *mut u8).sub(shape.elemsize) as *mut c_void,
            shape.elemsize,
        );
    }
}

/// E13 — `stbds_hmget_key` writes `temp` into the header, `_ts` does not.
#[test]
fn err_hmget_key_writes_temp() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x13_1313);
    let shape = Shape::bin(16, 8);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "hmget_key vs _ts temp".into();
    let keys = distinct_bin_keys(&mut rng, 8, 5);
    unsafe {
        for k in &keys {
            let v = rng.bytes(8);
            m.put(k, HM_BINARY, &v, "setup");
        }
        for (i, k) in keys.iter().enumerate() {
            // poison the header temp in both libraries
            (*header_of(m.c.p, shape.elemsize)).temp = 0x1234;
            (*header_of(m.r.p, shape.elemsize)).temp = 0x1234;
            let ts = m.get_ts(k, HM_BINARY, &format!("ts#{i}"));
            assert_eq!(ts, i as isize);
            assert_eq!(
                (*header_of(m.c.p, shape.elemsize)).temp,
                0x1234,
                "hmget_key_ts must not touch hdr.temp"
            );
            assert_eq!((*header_of(m.r.p, shape.elemsize)).temp, 0x1234);
            let g = m.get(k, HM_BINARY, &format!("get#{i}"));
            assert_eq!(g, i as isize);
            assert_eq!(
                (*header_of(m.c.p, shape.elemsize)).temp,
                i as isize,
                "hmget_key must write hdr.temp"
            );
            assert_eq!((*header_of(m.r.p, shape.elemsize)).temp, i as isize);
        }
        m.free();
    }
}

// ===========================================================================
// E14 / E15 / E16 — stbds_hmput_default
// ===========================================================================

/// E14 / E15 / E16 — all three branches of `stbds_hmput_default`.
#[test]
fn err_hmput_default_branches() {
    let _g = seed_lock();
    let l = libs();
    for shape in [Shape::bin(16, 8), Shape::bin(1, 1), Shape::bin(64, 8)] {
        unsafe {
            // E14: a == NULL
            let cp = (l.c.hmput_default)(std::ptr::null_mut(), shape.elemsize);
            let rp = (l.r.hmput_default)(std::ptr::null_mut(), shape.elemsize);
            assert_same(
                &format!("hmput_default(NULL) {shape:?}"),
                &fingerprint(cp, shape),
                &fingerprint(rp, shape),
            );
            assert_eq!((*header_of(cp, shape.elemsize)).length, 1);
            assert_eq!((*header_of(cp, shape.elemsize)).capacity, 4);
            // element 0 must be all zero
            let z = vec![0u8; shape.elemsize];
            let cb = std::slice::from_raw_parts(
                (cp as *const u8).sub(shape.elemsize),
                shape.elemsize,
            );
            assert_eq!(cb, &z[..], "C default element must be zeroed");
            let rb = std::slice::from_raw_parts(
                (rp as *const u8).sub(shape.elemsize),
                shape.elemsize,
            );
            assert_eq!(rb, &z[..], "RUST default element must be zeroed");

            // E16: a != NULL, length != 0 -> unchanged
            let cp2 = (l.c.hmput_default)(cp, shape.elemsize);
            let rp2 = (l.r.hmput_default)(rp, shape.elemsize);
            assert_eq!(cp2, cp, "C: no-op must return the same pointer");
            assert_eq!(rp2, rp, "RUST: no-op must return the same pointer");

            // E15: a != NULL, length == 0
            (*header_of(cp, shape.elemsize)).length = 0;
            (*header_of(rp, shape.elemsize)).length = 0;
            // dirty the element so the memset is observable
            std::ptr::write_bytes((cp as *mut u8).sub(shape.elemsize), 0xAB, shape.elemsize);
            std::ptr::write_bytes((rp as *mut u8).sub(shape.elemsize), 0xAB, shape.elemsize);
            let cp3 = (l.c.hmput_default)(cp, shape.elemsize);
            let rp3 = (l.r.hmput_default)(rp, shape.elemsize);
            assert_same(
                &format!("hmput_default length==0 {shape:?}"),
                &fingerprint(cp3, shape),
                &fingerprint(rp3, shape),
            );
            assert_eq!((*header_of(cp3, shape.elemsize)).length, 1);
            let cb = std::slice::from_raw_parts(
                (cp3 as *const u8).sub(shape.elemsize),
                shape.elemsize,
            );
            assert_eq!(cb, &z[..], "C must re-zero the default element");
            let rb = std::slice::from_raw_parts(
                (rp3 as *const u8).sub(shape.elemsize),
                shape.elemsize,
            );
            assert_eq!(rb, &z[..], "RUST must re-zero the default element");

            (l.c.hmfree_func)(
                (cp3 as *mut u8).sub(shape.elemsize) as *mut c_void,
                shape.elemsize,
            );
            (l.r.hmfree_func)(
                (rp3 as *mut u8).sub(shape.elemsize) as *mut c_void,
                shape.elemsize,
            );
        }
    }
}

// ===========================================================================
// E17 / E18 / E19 / G8 — stbds_hmput_key entry conditions
// ===========================================================================

/// E17 — `stbds_hmput_key(NULL, ...)` creates the array *and* the index.
#[test]
fn err_hmput_key_null_map() {
    let _g = seed_lock();
    let l = libs();
    for shape in [Shape::bin(16, 8), Shape::bin(1, 1), Shape::bin(40, 3)] {
        seed_both(0x3141_5926);
        unsafe {
            let mut key = [0x5Au8; 8];
            let cp = (l.c.hmput_key)(
                std::ptr::null_mut(),
                shape.elemsize,
                key.as_mut_ptr() as *mut c_void,
                shape.keysize,
                HM_BINARY,
            );
            let rp = (l.r.hmput_key)(
                std::ptr::null_mut(),
                shape.elemsize,
                key.as_mut_ptr() as *mut c_void,
                shape.keysize,
                HM_BINARY,
            );
            assert_eq!((*header_of(cp, shape.elemsize)).temp, 0);
            assert_eq!((*header_of(rp, shape.elemsize)).temp, 0);
            assert_eq!((*header_of(cp, shape.elemsize)).length, 2);
            assert_eq!((*header_of(rp, shape.elemsize)).length, 2);
            // the value region is untouched by hmput_key, so compare only the
            // header + index + key bytes
            let s = Shape {
                elemsize: shape.elemsize,
                keysize: shape.keysize,
                key: KeyRepr::Bytes,
            };
            let cf: Vec<String> = fingerprint(cp, s)
                .into_iter()
                .filter(|l| !l.contains(".value="))
                .collect();
            let rf: Vec<String> = fingerprint(rp, s)
                .into_iter()
                .filter(|l| !l.contains(".value="))
                .collect();
            assert_same(&format!("hmput_key(NULL) {shape:?}"), &cf, &rf);
            (l.c.hmfree_func)(
                (cp as *mut u8).sub(shape.elemsize) as *mut c_void,
                shape.elemsize,
            );
            (l.r.hmfree_func)(
                (rp as *mut u8).sub(shape.elemsize) as *mut c_void,
                shape.elemsize,
            );
        }
    }
}

/// E18 / E21 / E22 — the `mode >= STBDS_HM_STRING` test decides `string.mode`
/// when the index is created (`SH_DEFAULT` vs `0`).
#[test]
fn err_hmput_key_mode_selects_string_mode() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x18_1818);
    for mode in [
        c_int::MIN,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        44,
        c_int::MAX,
    ] {
        let is_string = mode >= HM_STRING;
        let shape = if is_string {
            Shape::str_ptr(16)
        } else {
            Shape::bin(16, 8)
        };
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("mode={mode}");
        unsafe {
            let keys = if is_string {
                distinct_str_keys(&mut rng, 12, 1, 20)
            } else {
                distinct_bin_keys(&mut rng, 8, 12)
            };
            for (i, k) in keys.iter().enumerate() {
                let v = rng.bytes(8);
                let idx = m.put(k, mode, &v, &format!("key#{i}"));
                assert_eq!(idx, i as isize);
            }
            let want = if is_string { SH_DEFAULT } else { 0 };
            assert_eq!(
                (*m.c.table()).string.mode as c_int,
                want,
                "C string.mode for mode={mode}"
            );
            assert_eq!(
                (*m.r.table()).string.mode as c_int,
                want,
                "RUST string.mode for mode={mode}"
            );
            for (i, k) in keys.iter().enumerate() {
                assert_eq!(m.get(k, mode, &format!("get#{i}")), i as isize);
                assert_eq!(m.get_ts(k, mode, &format!("get_ts#{i}")), i as isize);
            }
            m.free();
        }
    }
}

/// E21 — out-of-enum "string" modes behave exactly like `STBDS_HM_STRING`
/// for hashing / comparison / `temp_key`.
#[test]
fn err_mode_out_of_range_string_side() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x21_2121);
    let shape = Shape::str_ptr(16);
    for mode in [1, 2, 3, 4, 44, 1000, c_int::MAX] {
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("string-side mode={mode}");
        let keys = distinct_str_keys(&mut rng, 20, 1, 30);
        unsafe {
            for (i, k) in keys.iter().enumerate() {
                let v = rng.bytes(8);
                m.put_check_temp_key(k, mode, &v, &format!("key#{i}"));
            }
            // a duplicate put must find the key by strcmp
            for (i, k) in keys.iter().enumerate() {
                let v = rng.bytes(8);
                assert_eq!(
                    m.put_check_temp_key(k, mode, &v, &format!("dup#{i}")),
                    i as isize
                );
            }
            // and the STRING hash must be identical to mode 1's
            let h1 = (libs().c.hash_string)(keys[0].as_ptr() as *mut c_char, (*m.c.table()).seed);
            let h2 = (libs().r.hash_string)(keys[0].as_ptr() as *mut c_char, (*m.r.table()).seed);
            assert_eq!(h1, h2);
            m.free();
        }
    }
}

/// E22 — negative modes take the BINARY side.
#[test]
fn err_mode_negative_is_binary() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x22_2222);
    let shape = Shape::bin(16, 8);
    for mode in [c_int::MIN, -1000, -2, -1, 0] {
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("binary-side mode={mode}");
        let keys = distinct_bin_keys(&mut rng, 8, 20);
        unsafe {
            for (i, k) in keys.iter().enumerate() {
                let v = rng.bytes(8);
                m.put(k, mode, &v, &format!("key#{i}"));
            }
            assert_eq!((*m.c.table()).string.mode, 0);
            assert_eq!((*m.r.table()).string.mode, 0);
            for (i, k) in keys.iter().enumerate() {
                assert_eq!(m.get(k, mode, &format!("get#{i}")), i as isize);
            }
            // the stored key must be the raw bytes (memcpy branch)
            let elem = (m.c.p as *const u8).add(shape.elemsize * 3);
            assert_eq!(std::slice::from_raw_parts(elem, 8), &keys[3][..]);
            let elem = (m.r.p as *const u8).add(shape.elemsize * 3);
            assert_eq!(std::slice::from_raw_parts(elem, 8), &keys[3][..]);
            m.free();
        }
    }
}

/// E19 / G8 — growth happens when `used_count >= used_count_threshold`, i.e.
/// one insert *past* the threshold, not at it.
#[test]
fn err_hmput_grow_at_threshold() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x19_1919);
    let shape = Shape::bin(16, 8);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "growth at threshold".into();
    let keys = distinct_bin_keys(&mut rng, 8, 16);
    unsafe {
        let mut trace = Vec::new();
        for (i, k) in keys.iter().enumerate() {
            let v = rng.bytes(8);
            m.put(k, HM_BINARY, &v, &format!("key#{i}"));
            let t = m.c.table();
            let rt = m.r.table();
            assert_eq!((*t).slot_count, (*rt).slot_count);
            assert_eq!((*t).used_count, (*rt).used_count);
            trace.push(((*t).slot_count, (*t).used_count, (*t).used_count_threshold));
        }
        // 8 slots, threshold 6: inserts 1..6 keep 8 slots (used_count 1..6),
        // the 7th insert sees used_count == 6 >= 6 and doubles to 16.
        assert_eq!(trace[5], (8, 6, 6), "after the 6th insert: {:?}", trace[5]);
        assert_eq!(trace[6], (16, 7, 12), "after the 7th insert: {:?}", trace[6]);
        assert_eq!(trace[11], (16, 12, 12), "after the 12th insert");
        assert_eq!(trace[12], (32, 13, 24), "after the 13th insert");
        m.free();
    }
}

/// E20 — the `if (hash < 2) hash += 2` fixup keeps EMPTY(0)/DELETED(1) out of
/// the stored hash values.  A real 64-bit siphash of 0 or 1 cannot be produced
/// on purpose (2^-63), so the row is pinned by its *observable* invariant,
/// checked on both libraries over many randomized maps: an occupied slot never
/// stores hash 0 or 1, and hash 0 / 1 always mean EMPTY / DELETED.
#[test]
fn err_hash_below_2_invariant() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x20_2020);
    for round in 0..40 {
        let shape = if round % 2 == 0 {
            Shape::bin(16, 8)
        } else {
            Shape::str_ptr(16)
        };
        let mode = if shape.key == KeyRepr::CStr {
            HM_STRING
        } else {
            HM_BINARY
        };
        seed_both(rng.next_u64() as usize);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("hash<2 invariant round {round}");
        unsafe {
            let keys = if mode == HM_STRING {
                distinct_str_keys(&mut rng, 40, 0, 20)
            } else {
                distinct_bin_keys(&mut rng, 8, 40)
            };
            for k in &keys {
                let v = rng.bytes(8);
                m.put(k, mode, &v, "put");
            }
            for k in keys.iter().step_by(3) {
                m.del(k, mode, "del");
            }
            for (name, t) in [("C", m.c.table()), ("RUST", m.r.table())] {
                let n = (*t).slot_count >> 3;
                for b in 0..n {
                    let bucket = (*t).storage.add(b);
                    for s in 0..BUCKET_LENGTH {
                        let h = (*bucket).hash[s];
                        let i = (*bucket).index[s];
                        match h {
                            0 => assert_eq!(i, INDEX_EMPTY, "{name}: hash 0 must be EMPTY"),
                            1 => assert_eq!(i, INDEX_DELETED, "{name}: hash 1 must be DELETED"),
                            _ => assert!(
                                i >= 0,
                                "{name}: hash {h:#x} with index {i} (must be in use)"
                            ),
                        }
                    }
                }
            }
            m.free();
        }
    }
    seed_both(0x3141_5926);
}

// ===========================================================================
// E23 / E34 / G7 — string.mode out of range -> the switch `default:` branch
// ===========================================================================

/// E34 / G7 — `stbds_shmode_func` truncates `mode` to `unsigned char`.
#[test]
fn err_shmode_out_of_range() {
    let _g = seed_lock();
    let l = libs();
    for mode in [
        0i32,
        1,
        2,
        3,
        4,
        5,
        44,
        127,
        128,
        255,
        256,
        257,
        300,
        -1,
        -256,
        c_int::MIN,
        c_int::MAX,
    ] {
        for elemsize in [16usize, 24] {
            seed_both(0x3141_5926);
            let shape = Shape::bin(elemsize, 8);
            unsafe {
                let cp = (l.c.shmode_func)(elemsize, mode);
                let rp = (l.r.shmode_func)(elemsize, mode);
                let ct = (*header_of(cp, elemsize)).hash_table as *mut HashIndex;
                let rt = (*header_of(rp, elemsize)).hash_table as *mut HashIndex;
                let want = (mode as u32 & 0xff) as u8;
                assert_eq!((*ct).string.mode, want, "C string.mode for mode={mode}");
                assert_eq!((*rt).string.mode, want, "RUST string.mode for mode={mode}");
                assert_same(
                    &format!("shmode_func({elemsize},{mode})"),
                    &fingerprint(cp, shape),
                    &fingerprint(rp, shape),
                );
                (l.c.hmfree_func)((cp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (l.r.hmfree_func)((rp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

/// E23 — a `string.mode` outside `{SH_STRDUP, SH_ARENA, SH_DEFAULT}` falls into
/// the `default:` case and `memcpy`s `keysize` raw bytes of the key.
#[test]
fn err_string_mode_default_branch() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x23_2323);
    // 0 (SH_NONE), 4, 44, 255 and 256->0 all hit `default:`
    for mode in [0i32, 4, 44, 255, 256, 300, -1] {
        let shape = Shape::bin(24, 8);
        seed_both(0x3141_5926);
        let mut m = MapPair::shmode(shape, mode);
        m.ctx = format!("default: branch shmode={mode}");
        // keys are >= 8 bytes long so the memcpy only reads defined bytes
        let keys = distinct_str_keys(&mut rng, 12, 10, 24);
        unsafe {
            for (i, k) in keys.iter().enumerate() {
                let v = rng.bytes(8);
                let idx = m.put(k, HM_STRING, &v, &format!("key#{i}"));
                assert_eq!(idx, i as isize);
                for (name, p) in [("C", m.c.p), ("RUST", m.r.p)] {
                    let elem = (p as *const u8).add(shape.elemsize * idx as usize);
                    assert_eq!(
                        std::slice::from_raw_parts(elem, 8),
                        &k[..8],
                        "{name}: default: branch must memcpy the key bytes (mode={mode})"
                    );
                }
            }
            m.free();
        }
    }
}

// ===========================================================================
// E24 — the `temp_key` asymmetry between the two halves of the bucket scan
// ===========================================================================

/// E24 — a duplicate key found in the *wrapped* half (lib.c L747-751) sets
/// `stbds_temp` but — unlike the upper half (L729-735) — does **not** refresh
/// `table->temp_key`.
///
/// Both bucket layouts are crafted identically in the two libraries so the
/// branch is taken deterministically.
#[test]
fn err_dup_key_wrapped_half_no_temp_key() {
    let _g = seed_lock();
    let l = libs();
    let mut rng = Rng::new(0x24_2424);
    let shape = Shape::str_ptr(16);

    for wrapped in [true, false] {
        let mut done = false;
        for _attempt in 0..500 {
            seed_both(0x3141_5926);
            let keys = distinct_str_keys(&mut rng, 3, 4, 12);
            let mut m = MapPair::empty(shape);
            m.ctx = format!("temp_key asymmetry wrapped={wrapped}");
            unsafe {
                for (i, k) in keys.iter().enumerate() {
                    let v = rng.bytes(8);
                    m.put_check_temp_key(k, HM_STRING, &v, &format!("setup#{i}"));
                }
                let seed = (*m.c.table()).seed;
                assert_eq!(seed, (*m.r.table()).seed);
                assert_eq!((*m.c.table()).slot_count, 8);
                let mut h0 = (l.c.hash_string)(keys[0].as_ptr() as *mut c_char, seed);
                assert_eq!(
                    h0,
                    (l.r.hash_string)(keys[0].as_ptr() as *mut c_char, seed)
                );
                if h0 < 2 {
                    h0 += 2;
                }
                let pos = h0 & 7;
                // the wrapped variant needs at least 2 slots below `pos`
                if (wrapped && pos < 2) || (!wrapped && pos > 6) {
                    m.free();
                    continue;
                }
                // craft bucket 0: no empty slot before the match
                let target = if wrapped { pos - 1 } else { pos };
                let mut slots = [(0usize, 0isize); 8];
                for (j, s) in slots.iter_mut().enumerate() {
                    *s = if j == target {
                        (h0, 0isize) // keys[0] lives at element index 0
                    } else {
                        (0xF000_0000_0000_0000usize + j, 0isize)
                    };
                }
                for t in [m.c.table(), m.r.table()] {
                    let bucket = (*t).storage;
                    for (j, (h, i)) in slots.iter().enumerate() {
                        (*bucket).hash[j] = *h;
                        (*bucket).index[j] = *i;
                    }
                }
                m.check("crafted bucket");

                // re-put keys[0]: must resolve to index 0
                let v = rng.bytes(8);
                let idx = m.put(&keys[0], HM_STRING, &v, "crafted dup put");
                assert_eq!(idx, 0, "the duplicate must resolve to element index 0");

                let k0_str = format!("{:02x?}", &keys[0][..keys[0].len() - 1]);
                let k2_str = format!("{:02x?}", &keys[2][..keys[2].len() - 1]);
                let ctk = m.c.temp_key_str();
                let rtk = m.r.temp_key_str();
                assert_eq!(ctk, rtk, "temp_key must match between the libraries");
                if wrapped {
                    assert_eq!(
                        ctk, k2_str,
                        "the wrapped-half hit must NOT refresh temp_key \
                         (it must still hold the last inserted key)"
                    );
                } else {
                    assert_eq!(
                        ctk, k0_str,
                        "the upper-half hit MUST refresh temp_key"
                    );
                }
                m.check("after crafted dup put");
                m.free();
                done = true;
            }
            break;
        }
        assert!(done, "could not craft the wrapped={wrapped} case");
    }
}

// ===========================================================================
// E25 — insertion reuses a tombstone slot
// ===========================================================================

/// E25 — when a tombstone is seen before the empty slot, the insert goes into
/// the tombstone and `tombstone_count` is decremented.
#[test]
fn err_insert_reuses_tombstone() {
    let _g = seed_lock();
    let l = libs();
    let mut rng = Rng::new(0x25_2525);
    let shape = Shape::bin(16, 8);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "tombstone reuse".into();
    let keys = distinct_bin_keys(&mut rng, 8, 5);
    unsafe {
        for (i, k) in keys.iter().enumerate() {
            let v = rng.bytes(8);
            m.put(k, HM_BINARY, &v, &format!("setup#{i}"));
        }
        assert_eq!((*m.c.table()).slot_count, 8);
        assert_eq!(m.del(&keys[2], HM_BINARY, "make a tombstone"), 1);
        assert_eq!((*m.c.table()).tombstone_count, 1);
        assert_eq!((*m.r.table()).tombstone_count, 1);

        // locate the tombstone slot (identical in both libraries)
        let seed = (*m.c.table()).seed;
        let mut slot_t = usize::MAX;
        for s in 0..8usize {
            if (*(*m.c.table()).storage).hash[s] == 1 {
                assert_eq!((*(*m.c.table()).storage).index[s], INDEX_DELETED);
                assert_eq!((*(*m.r.table()).storage).hash[s], 1);
                slot_t = s;
            }
        }
        assert!(slot_t != usize::MAX, "no tombstone found");

        // find a fresh key that probes straight into the tombstone slot
        let mut newkey = Vec::new();
        for _ in 0..100_000 {
            let k = rng.bytes(8);
            if keys.contains(&k) {
                continue;
            }
            let mut h = (l.c.hash_bytes)(k.as_ptr() as *mut c_void, 8, seed);
            assert_eq!(h, (l.r.hash_bytes)(k.as_ptr() as *mut c_void, 8, seed));
            if h < 2 {
                h += 2;
            }
            if h & 7 == slot_t as usize {
                newkey = k;
                break;
            }
        }
        assert!(!newkey.is_empty(), "no key probing into the tombstone slot");

        let v = rng.bytes(8);
        let idx = m.put(&newkey, HM_BINARY, &v, "insert into tombstone");
        assert_eq!(idx, 4, "the new element index");
        for (name, t) in [("C", m.c.table()), ("RUST", m.r.table())] {
            assert_eq!(
                (*t).tombstone_count,
                0,
                "{name}: the tombstone must have been consumed"
            );
            assert_eq!((*t).used_count, 5, "{name}: used_count");
            assert_eq!(
                (*(*t).storage).index[slot_t],
                4,
                "{name}: the insert must land in the tombstone slot"
            );
        }
        m.free();
    }
}

// ===========================================================================
// E26..E33 — stbds_hmdel_key
// ===========================================================================

/// E26 — `stbds_hmdel_key(NULL, ...)` returns `0` (NULL).
#[test]
fn err_hmdel_null_map() {
    let l = libs();
    unsafe {
        let mut key = [1u8; 16];
        for elemsize in [1usize, 8, 16, 64] {
            for keysize in [0usize, 1, 8, 16] {
                for mode in [c_int::MIN, -1, 0, 1, 2, 44, c_int::MAX] {
                    let cp = (l.c.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.as_mut_ptr() as *mut c_void,
                        keysize,
                        0,
                        mode,
                    );
                    let rp = (l.r.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.as_mut_ptr() as *mut c_void,
                        keysize,
                        0,
                        mode,
                    );
                    assert!(cp.is_null(), "C hmdel_key(NULL) must return NULL");
                    assert!(rp.is_null(), "RUST hmdel_key(NULL) must return NULL");
                }
            }
        }
    }
}

/// E27 — map without a hash index: `temp = 0`, `a` returned unchanged.
#[test]
fn err_hmdel_no_table() {
    let _g = seed_lock();
    let l = libs();
    let shape = Shape::bin(16, 8);
    unsafe {
        let mut key = [9u8; 8];
        let mut t: isize = 0;
        let cp = (l.c.hmget_key_ts)(
            std::ptr::null_mut(),
            shape.elemsize,
            key.as_mut_ptr() as *mut c_void,
            shape.keysize,
            &mut t,
            HM_BINARY,
        );
        let rp = (l.r.hmget_key_ts)(
            std::ptr::null_mut(),
            shape.elemsize,
            key.as_mut_ptr() as *mut c_void,
            shape.keysize,
            &mut t,
            HM_BINARY,
        );
        // poison temp so the write to 0 is observable
        (*header_of(cp, shape.elemsize)).temp = 0x3333;
        (*header_of(rp, shape.elemsize)).temp = 0x3333;
        let cp2 = (l.c.hmdel_key)(
            cp,
            shape.elemsize,
            key.as_mut_ptr() as *mut c_void,
            shape.keysize,
            0,
            HM_BINARY,
        );
        let rp2 = (l.r.hmdel_key)(
            rp,
            shape.elemsize,
            key.as_mut_ptr() as *mut c_void,
            shape.keysize,
            0,
            HM_BINARY,
        );
        assert_eq!(cp2, cp);
        assert_eq!(rp2, rp);
        assert_eq!((*header_of(cp, shape.elemsize)).temp, 0, "C temp must be 0");
        assert_eq!((*header_of(rp, shape.elemsize)).temp, 0, "RUST temp must be 0");
        assert_same(
            "hmdel without index",
            &fingerprint(cp, shape),
            &fingerprint(rp, shape),
        );
        (l.c.hmfree_func)(
            (cp as *mut u8).sub(shape.elemsize) as *mut c_void,
            shape.elemsize,
        );
        (l.r.hmfree_func)(
            (rp as *mut u8).sub(shape.elemsize) as *mut c_void,
            shape.elemsize,
        );
    }
}

/// E28 / E29 / E31 — missing key -> `temp = 0`; found key -> `temp = 1` plus
/// the tombstone marks; deleting the last element skips the `memmove`.
#[test]
fn err_hmdel_missing_found_and_last() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x28_2828);
    let shape = Shape::bin(16, 8);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "hmdel missing/found/last".into();
    let keys = distinct_bin_keys(&mut rng, 8, 5);
    let absent = distinct_bin_keys(&mut rng, 8, 5);
    unsafe {
        for (i, k) in keys.iter().enumerate() {
            let v = rng.bytes(8);
            m.put(k, HM_BINARY, &v, &format!("setup#{i}"));
        }
        // E28: missing key
        for (i, k) in absent.iter().enumerate() {
            if keys.contains(k) {
                continue;
            }
            // everything except `hdr.temp` (which the delete sets to 0) must
            // be untouched
            let strip = |v: Vec<String>| -> Vec<String> {
                v.into_iter().filter(|l| !l.starts_with("hdr.temp=")).collect()
            };
            let before_c = strip(m.c.fingerprint());
            assert_eq!(
                m.del(k, HM_BINARY, &format!("missing#{i}")),
                0,
                "a missing key must set temp = 0"
            );
            assert_eq!(
                before_c,
                strip(m.c.fingerprint()),
                "a missing key must leave the map untouched"
            );
        }
        // E29 / E31: delete the last element (old_index == final_index)
        let last = keys.len() - 1;
        let before = m.c.fingerprint();
        assert_eq!(m.del(&keys[last], HM_BINARY, "last"), 1);
        let after = m.c.fingerprint();
        assert_ne!(before, after);
        for (name, t) in [("C", m.c.table()), ("RUST", m.r.table())] {
            assert_eq!((*t).used_count, 4, "{name} used_count");
            assert_eq!((*t).tombstone_count, 1, "{name} tombstone_count");
            let mut deleted = 0;
            for s in 0..8 {
                if (*(*t).storage).hash[s] == 1 {
                    assert_eq!((*(*t).storage).index[s], INDEX_DELETED);
                    deleted += 1;
                }
            }
            assert_eq!(deleted, 1, "{name}: exactly one tombstone");
        }
        assert_eq!(m.c.hmlen(), 4);
        // the surviving elements must be untouched (no memmove happened)
        for (i, k) in keys.iter().take(last).enumerate() {
            assert_eq!(m.get(k, HM_BINARY, &format!("survivor#{i}")), i as isize);
        }
        m.free();
    }
}

/// E30 / G6 — the `strdup` free in `hmdel_key` is guarded by
/// `mode == STBDS_HM_STRING` **exactly**: with `mode = 2` (still "string" for
/// hashing) the key is *not* freed.
#[test]
fn err_hmdel_strdup_free_only_mode_1() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x30_3030);
    let shape = Shape::str_ptr(16);
    seed_both(0x3141_5926);
    let mut m = MapPair::shmode(shape, SH_STRDUP);
    m.ctx = "strdup free only for mode==1".into();
    // long keys so that a stray free() would be visible (glibc writes tcache
    // bookkeeping into the first 16 bytes of a freed chunk)
    let keys = distinct_str_keys(&mut rng, 4, 40, 48);
    unsafe {
        for (i, k) in keys.iter().enumerate() {
            let v = rng.bytes(8);
            m.put(k, HM_STRING, &v, &format!("setup#{i}"));
        }
        // delete the LAST element with mode = 2: no memmove (old == final) and
        // no free of the strdup'ed key
        let last = keys.len() - 1;
        assert_eq!(m.del(&keys[last], 2, "mode 2 delete of the last element"), 1);
        // the vacated slot still holds the (unfreed) key pointer
        let cstr_c = {
            let p = *((m.c.p as *mut u8).add(shape.elemsize * last) as *mut *mut c_char);
            cstr(p)
        };
        let cstr_r = {
            let p = *((m.r.p as *mut u8).add(shape.elemsize * last) as *mut *mut c_char);
            cstr(p)
        };
        assert_eq!(
            cstr_c, cstr_r,
            "mode 2 must NOT free the strdup'ed key in either library"
        );
        assert_eq!(
            cstr_c,
            format!("{:02x?}", &keys[last][..keys[last].len() - 1]),
            "the key contents must still be intact"
        );
        // now delete another last element with mode == 1: the key IS freed
        assert_eq!(m.del(&keys[last - 1], HM_STRING, "mode 1 delete"), 1);
        m.check("after the mode-1 delete");
        m.free();
    }
}

/// E32 / G9 — shrink: `used_count < used_count_shrink_threshold` (and
/// `slot_count > 8`) halves the index.
#[test]
fn err_hmdel_shrink() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x32_3232);
    let shape = Shape::bin(16, 8);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "shrink".into();
    let keys = distinct_bin_keys(&mut rng, 8, 7);
    unsafe {
        for (i, k) in keys.iter().enumerate() {
            let v = rng.bytes(8);
            m.put(k, HM_BINARY, &v, &format!("setup#{i}"));
        }
        assert_eq!((*m.c.table()).slot_count, 16);
        assert_eq!((*m.c.table()).used_count_shrink_threshold, 4);
        let mut trace = Vec::new();
        for (i, k) in keys.iter().enumerate() {
            m.del(k, HM_BINARY, &format!("del#{i}"));
            let t = m.c.table();
            trace.push(((*t).slot_count, (*t).used_count, (*t).tombstone_count));
        }
        // 7 -> 6 -> 5 -> 4 (no shrink yet, 4 !< 4) -> 3 shrinks to 8 slots
        assert_eq!(trace[2].0, 16, "still 16 slots after 3 deletes: {trace:?}");
        assert_eq!(trace[3], (8, 3, 0), "the 4th delete must shrink: {trace:?}");
        // 3 -> 2 (tomb 1, 1 > 1 false) -> 1 (tomb 2 > 1 -> rebuild, tomb 0)
        //   -> 0 (tomb 1 again)
        assert_eq!(trace[4], (8, 2, 1), "{trace:?}");
        assert_eq!(trace[5], (8, 1, 0), "rebuild on the 6th delete: {trace:?}");
        assert_eq!(*trace.last().unwrap(), (8, 0, 1), "{trace:?}");
        m.free();
    }
}

/// E33 / G9 — rebuild: `tombstone_count > tombstone_count_threshold`
/// (== 1 for an 8-slot index) rebuilds at the same size.
#[test]
fn err_hmdel_rebuild_on_tombstones() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x33_3333);
    let shape = Shape::bin(16, 8);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "tombstone rebuild".into();
    let keys = distinct_bin_keys(&mut rng, 8, 5);
    unsafe {
        for (i, k) in keys.iter().enumerate() {
            let v = rng.bytes(8);
            m.put(k, HM_BINARY, &v, &format!("setup#{i}"));
        }
        assert_eq!((*m.c.table()).slot_count, 8);
        assert_eq!((*m.c.table()).tombstone_count_threshold, 1);
        assert_eq!((*m.c.table()).used_count_shrink_threshold, 0); // no shrinking at 8
        m.del(&keys[0], HM_BINARY, "1st delete");
        for (name, t) in [("C", m.c.table()), ("RUST", m.r.table())] {
            assert_eq!((*t).tombstone_count, 1, "{name}: one tombstone, 1 > 1 is false");
            assert_eq!((*t).slot_count, 8, "{name}");
        }
        m.del(&keys[1], HM_BINARY, "2nd delete -> rebuild");
        for (name, t) in [("C", m.c.table()), ("RUST", m.r.table())] {
            assert_eq!(
                (*t).tombstone_count,
                0,
                "{name}: the 2nd tombstone (2 > 1) must trigger a rebuild"
            );
            assert_eq!((*t).slot_count, 8, "{name}: rebuild keeps the size");
            assert_eq!((*t).used_count, 3, "{name}");
        }
        for (i, k) in keys.iter().enumerate().skip(2) {
            assert!(m.get(k, HM_BINARY, &format!("survivor#{i}")) >= 0);
        }
        m.free();
    }
}

// ===========================================================================
// E35..E38 — stbds_stralloc boundaries
// ===========================================================================

fn arena_dump(a: &StringArena, p: *mut c_char, len: usize) -> Vec<String> {
    unsafe {
        let mut out = vec![
            format!("remaining={}", a.remaining),
            format!("block={}", a.block),
            format!("mode={}", a.mode),
            format!("storage_null={}", a.storage.is_null()),
        ];
        let mut n = 0usize;
        let mut b = a.storage as *mut *mut c_void;
        while !b.is_null() && n < 10_000 {
            n += 1;
            b = *b as *mut *mut c_void;
        }
        out.push(format!("chain_len={n}"));
        if p.is_null() {
            out.push("p=NULL".into());
            return out;
        }
        let head = a.storage as *mut u8;
        out.push(format!(
            "p_loc={}",
            if p as *mut u8 == head.wrapping_add(8 + a.remaining) {
                "head+8+remaining"
            } else {
                "separate_block"
            }
        ));
        out.push(format!(
            "content={:02x?}",
            std::slice::from_raw_parts(p as *const u8, len)
        ));
        out
    }
}

/// E35 / E36 — the dedicated-block path (`len > blocksize`), on an empty arena
/// (becomes the head, `remaining = 0`) and on a populated one (spliced after
/// the head).
#[test]
fn err_stralloc_oversized_string() {
    let l = libs();
    let mut rng = Rng::new(0x35_3535);
    for populated in [false, true] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        unsafe {
            if populated {
                let mut small = rng.cstring(4);
                let cp = (l.c.stralloc)(&mut ca, small.as_mut_ptr() as *mut c_char);
                let rp = (l.r.stralloc)(&mut ra, small.as_mut_ptr() as *mut c_char);
                assert_same(
                    "oversized setup",
                    &arena_dump(&ca, cp, 5),
                    &arena_dump(&ra, rp, 5),
                );
            }
            // 512 << 0 == 512, so 512 bytes of payload (len 513) is oversized
            for len in [512usize, 513, 4096] {
                let mut s = rng.cstring(len);
                let cp = (l.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
                let rp = (l.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
                assert_same(
                    &format!("oversized len={len} populated={populated}"),
                    &arena_dump(&ca, cp, len + 1),
                    &arena_dump(&ra, rp, len + 1),
                );
                assert_eq!(
                    std::slice::from_raw_parts(cp as *const u8, len + 1),
                    &s[..],
                    "the C copy must equal the input"
                );
            }
            if !populated {
                assert_eq!(ca.remaining, 0, "an empty arena keeps remaining = 0");
                assert_eq!(ra.remaining, 0);
            }
            (l.c.strreset)(&mut ca);
            (l.r.strreset)(&mut ra);
        }
    }
}

/// E36 — oversized string into a completely empty arena.
#[test]
fn err_stralloc_oversized_empty_arena() {
    let l = libs();
    let mut rng = Rng::new(0x36_3636);
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    unsafe {
        let mut s = rng.cstring(1000);
        let cp = (l.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
        let rp = (l.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
        assert_same(
            "oversized into empty arena",
            &arena_dump(&ca, cp, 1001),
            &arena_dump(&ra, rp, 1001),
        );
        assert!(!ca.storage.is_null() && !ra.storage.is_null());
        assert_eq!((ca.remaining, ca.block), (0, 1));
        assert_eq!((ra.remaining, ra.block), (0, 1));
        (l.c.strreset)(&mut ca);
        (l.r.strreset)(&mut ra);
    }
}

/// E37 — `a->block` stops growing once `blocksize >= 1<<20`.
#[test]
fn err_stralloc_block_saturates() {
    let l = libs();
    let mut rng = Rng::new(0x37_3737);
    for block in [0u8, 1, 2, 19, 20, 21, 22, 23, 24, 25] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        ca.block = block;
        ra.block = block;
        unsafe {
            let mut s = rng.cstring(8);
            let cp = (l.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
            let rp = (l.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
            assert_same(
                &format!("block saturation block={block}"),
                &arena_dump(&ca, cp, 9),
                &arena_dump(&ra, rp, 9),
            );
            let blocksize = 512usize << (block >> 1);
            let want = if blocksize < (1 << 20) { block + 1 } else { block };
            assert_eq!(ca.block, want, "C block for start={block}");
            assert_eq!(ra.block, want, "RUST block for start={block}");
            (l.c.strreset)(&mut ca);
            (l.r.strreset)(&mut ra);
        }
    }
}

/// E38 — `a->block >= 110` makes `512 << (block>>1)` overflow to 0 and
/// `block >= 128` makes the shift count exceed 63, which x86-64 `shl %cl`
/// evaluates modulo 64 (verified in the C disassembly).
#[test]
fn err_stralloc_huge_block_masked_shift() {
    let l = libs();
    let mut rng = Rng::new(0x38_3838);
    // Only blocks whose `512 << ((block>>1) & 63)` is either 0 (i.e.
    // `(block>>1)&63 >= 55`) or small are usable: for the values in between
    // (e.g. block 109 -> 2^63, block 200 -> 2^45) the C `malloc` fails and the
    // very next line dereferences the NULL result, which crashes *both*
    // libraries (documented in ERRORS.md E38, not executable).
    for block in [110u8, 111, 126, 127, 128, 129, 130, 131, 250, 254, 255] {
        for len in [1usize, 8, 600] {
            let mut ca = StringArena::zeroed();
            let mut ra = StringArena::zeroed();
            ca.block = block;
            ra.block = block;
            unsafe {
                let mut s = rng.cstring(len);
                let cp = (l.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
                let rp = (l.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
                assert_same(
                    &format!("masked shift block={block} len={len}"),
                    &arena_dump(&ca, cp, len + 1),
                    &arena_dump(&ra, rp, len + 1),
                );
                assert_eq!(
                    std::slice::from_raw_parts(cp as *const u8, len + 1),
                    &s[..]
                );
                (l.c.strreset)(&mut ca);
                (l.r.strreset)(&mut ra);
            }
        }
    }
}

// ===========================================================================
// E39..E41, G2 — the hash functions on degenerate input
// ===========================================================================

/// E39 / G2 — `stbds_hash_bytes(NULL, 0, seed)` reads nothing.
#[test]
fn err_hash_bytes_null_zero_len() {
    let l = libs();
    let mut rng = Rng::new(0x39_3939);
    let mut seeds = vec![0usize, 1, 2, 0x3141_5926, usize::MAX, usize::MAX / 2];
    for _ in 0..64 {
        seeds.push(rng.next_u64() as usize);
    }
    for seed in seeds {
        unsafe {
            let a = (l.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let b = (l.r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(a, b, "hash_bytes(NULL,0,{seed:#x})");
            // and a non-NULL pointer with len 0 must give the same value
            let mut buf = [0xAAu8; 8];
            let c = (l.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            let d = (l.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(a, c, "len 0 must not depend on the pointer (C)");
            assert_eq!(b, d, "len 0 must not depend on the pointer (RUST)");
        }
    }
}

/// E40 — the `switch (len - i)` tail, including the sign-extending
/// `data |= (d[3] << 24)` case.
#[test]
fn err_hash_bytes_tail_sign_extension() {
    let l = libs();
    for len in 1..=23usize {
        for pos in 0..len {
            for byte in [0x80u8, 0xff, 0x01, 0x7f] {
                let mut buf = vec![0u8; len];
                buf[pos] = byte;
                for seed in [0usize, 0x3141_5926, usize::MAX] {
                    unsafe {
                        let a = (l.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                        let b = (l.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                        assert_eq!(
                            a, b,
                            "tail len={len} pos={pos} byte={byte:#x} seed={seed:#x}"
                        );
                    }
                }
            }
        }
    }
}

/// E41 — `stbds_hash_string("")`.
#[test]
fn err_hash_string_empty() {
    let l = libs();
    let mut rng = Rng::new(0x41_4141);
    let mut seeds = vec![0usize, 1, 2, 0x3141_5926, usize::MAX, usize::MAX - 1];
    for _ in 0..64 {
        seeds.push(rng.next_u64() as usize);
    }
    let mut empty = [0u8; 1];
    for seed in seeds {
        unsafe {
            let a = (l.c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            let b = (l.r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(a, b, "hash_string(\"\",{seed:#x})");
        }
    }
}

// ===========================================================================
// G1 / G3 / G4 / G6 — generic FFI boundary rows
// ===========================================================================

/// G1 — a NULL map pointer into every entry point that accepts one.
/// (`stbds_arrfreef(NULL)` is excluded: it computes `free((char*)NULL - 32)`,
/// which is an invalid free in *both* libraries — see ERRORS.md E35 note.)
#[test]
fn err_all_null_map_entry_points() {
    let _g = seed_lock();
    let l = libs();
    let elemsize = 16usize;
    let keysize = 8usize;
    let shape = Shape::bin(elemsize, keysize);
    let mut key = [0x11u8; 8];
    unsafe {
        seed_both(0x3141_5926);
        // arrgrowf
        let ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
        let ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
        assert_same("arrgrowf(NULL)", &dump_raw(ca), &dump_raw(ra));
        (l.c.arrfreef)(ca);
        (l.r.arrfreef)(ra);

        // hmfree_func(NULL)
        (l.c.hmfree_func)(std::ptr::null_mut(), elemsize);
        (l.r.hmfree_func)(std::ptr::null_mut(), elemsize);

        // hmdel_key(NULL) -> NULL
        assert!((l.c.hmdel_key)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            0,
            HM_BINARY
        )
        .is_null());
        assert!((l.r.hmdel_key)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            0,
            HM_BINARY
        )
        .is_null());

        // hmput_default(NULL)
        seed_both(0x3141_5926);
        let cd = (l.c.hmput_default)(std::ptr::null_mut(), elemsize);
        let rd = (l.r.hmput_default)(std::ptr::null_mut(), elemsize);
        assert_same(
            "hmput_default(NULL)",
            &fingerprint(cd, shape),
            &fingerprint(rd, shape),
        );
        (l.c.hmfree_func)((cd as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (l.r.hmfree_func)((rd as *mut u8).sub(elemsize) as *mut c_void, elemsize);

        // hmget_key(NULL)
        seed_both(0x3141_5926);
        let cg = (l.c.hmget_key)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            HM_BINARY,
        );
        let rg = (l.r.hmget_key)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            HM_BINARY,
        );
        assert_same(
            "hmget_key(NULL)",
            &fingerprint(cg, shape),
            &fingerprint(rg, shape),
        );
        assert_eq!((*header_of(cg, elemsize)).temp, INDEX_EMPTY);
        assert_eq!((*header_of(rg, elemsize)).temp, INDEX_EMPTY);
        (l.c.hmfree_func)((cg as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (l.r.hmfree_func)((rg as *mut u8).sub(elemsize) as *mut c_void, elemsize);

        // hmput_key(NULL)
        seed_both(0x3141_5926);
        let cp = (l.c.hmput_key)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            HM_BINARY,
        );
        let rp = (l.r.hmput_key)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            HM_BINARY,
        );
        let strip = |v: Vec<String>| -> Vec<String> {
            v.into_iter().filter(|l| !l.contains(".value=")).collect()
        };
        assert_same(
            "hmput_key(NULL)",
            &strip(fingerprint(cp, shape)),
            &strip(fingerprint(rp, shape)),
        );
        (l.c.hmfree_func)((cp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (l.r.hmfree_func)((rp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// G3 — `keysize == 0`: `memcmp(a,b,0) == 0`, so every key compares equal and
/// the map can only ever hold one entry.
#[test]
fn err_keysize_zero() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x03_0303);
    for elemsize in [8usize, 16, 32] {
        let shape = Shape::bin(elemsize, 0);
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("keysize=0 elemsize={elemsize}");
        unsafe {
            for i in 0..10 {
                let k = rng.bytes(8);
                let v = rng.bytes(8);
                let idx = m.put(&k, HM_BINARY, &v, &format!("put#{i}"));
                assert_eq!(idx, 0, "every key must map to element 0");
            }
            assert_eq!(m.c.hmlen(), 1);
            assert_eq!(m.r.hmlen(), 1);
            assert_eq!((*m.c.table()).used_count, 1);
            // any key is a hit
            for i in 0..5 {
                let k = rng.bytes(8);
                assert_eq!(m.get(&k, HM_BINARY, &format!("get#{i}")), 0);
            }
            // and one delete empties the map
            let k = rng.bytes(8);
            assert_eq!(m.del(&k, HM_BINARY, "del"), 1);
            assert_eq!(m.c.hmlen(), 0);
            assert_eq!(m.r.hmlen(), 0);
            m.free();
        }
    }
}

/// G4 — `keysize > elemsize`: the `memcpy` deliberately overruns the element.
/// Limited to two inserts so the write stays inside the initial 4-element
/// allocation (the C would otherwise write past the malloc'ed block).
#[test]
fn err_keysize_gt_elemsize() {
    let _g = seed_lock();
    let l = libs();
    let mut rng = Rng::new(0x04_0404);
    let elemsize = 8usize;
    let keysize = 16usize;
    let shape = Shape::bin(elemsize, keysize);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "keysize > elemsize".into();
    unsafe {
        let k1 = rng.bytes(16);
        let k2 = rng.bytes(16);
        let i1 = m.put_raw(&k1, HM_BINARY, "overrun put 1");
        assert_eq!(i1, 0);
        // the whole 4-element data area (32 bytes) must be identical
        let cd = std::slice::from_raw_parts((m.c.p as *const u8).sub(elemsize), 24);
        let rd = std::slice::from_raw_parts((m.r.p as *const u8).sub(elemsize), 24);
        assert_eq!(cd, rd, "data area after the overrunning memcpy");
        assert_eq!(&cd[8..24], &k1[..], "the key spilled into the next element");

        let i2 = m.put_raw(&k2, HM_BINARY, "overrun put 2");
        assert_eq!(i2, 1);
        let cd = std::slice::from_raw_parts((m.c.p as *const u8).sub(elemsize), 32);
        let rd = std::slice::from_raw_parts((m.r.p as *const u8).sub(elemsize), 32);
        assert_eq!(cd, rd, "data area after the 2nd overrunning memcpy");
        assert_eq!(&cd[16..32], &k2[..]);

        // key 1 is now unfindable (its stored bytes were partly overwritten)
        // and key 2 is findable; both libraries must agree
        let a = m.c.get(&k1, HM_BINARY);
        let b = m.r.get(&k1, HM_BINARY);
        assert_eq!(a, b, "lookup of the clobbered key");
        let a = m.c.get(&k2, HM_BINARY);
        let b = m.r.get(&k2, HM_BINARY);
        assert_eq!(a, b, "lookup of the intact key");
        assert_eq!(a, 1);
        let _ = l;
        m.free();
    }
}

/// G6 — the full out-of-range `mode` matrix through
/// `hmput_key` / `hmget_key` / `hmget_key_ts` / `hmdel_key`.
///
/// Only the *last* element is deleted for the string-side modes: for
/// `mode != 1` the post-`memmove` slot re-lookup would hash the raw key
/// *pointer bytes* (lib.c L842-845), whose value differs between the two
/// processes' heaps, so that path is not comparable (ERRORS.md A5/A6/E42).
#[test]
fn err_mode_matrix_binary() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x06_0606);
    for mode in [
        c_int::MIN,
        -1,
        0,
        1,
        2,
        3,
        4,
        44,
        c_int::MAX,
    ] {
        let is_string = mode >= HM_STRING;
        let shape = if is_string {
            Shape::str_ptr(16)
        } else {
            Shape::bin(16, 8)
        };
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("mode matrix mode={mode}");
        unsafe {
            let keys = if is_string {
                distinct_str_keys(&mut rng, 6, 1, 20)
            } else {
                distinct_bin_keys(&mut rng, 8, 6)
            };
            let absent = if is_string {
                distinct_str_keys(&mut rng, 3, 30, 40)
            } else {
                distinct_bin_keys(&mut rng, 8, 3)
            };
            for (i, k) in keys.iter().enumerate() {
                let v = rng.bytes(8);
                assert_eq!(m.put(k, mode, &v, &format!("put#{i}")), i as isize);
            }
            for (i, k) in keys.iter().enumerate() {
                assert_eq!(m.get(k, mode, &format!("get#{i}")), i as isize);
                assert_eq!(m.get_ts(k, mode, &format!("get_ts#{i}")), i as isize);
            }
            for (i, k) in absent.iter().enumerate() {
                if keys.contains(k) {
                    continue;
                }
                assert_eq!(m.get(k, mode, &format!("miss#{i}")), INDEX_EMPTY);
                assert_eq!(m.del(k, mode, &format!("del miss#{i}")), 0);
            }
            // delete the last element repeatedly (old_index == final_index)
            for i in (0..keys.len()).rev() {
                assert_eq!(m.del(&keys[i], mode, &format!("del last#{i}")), 1);
            }
            assert_eq!(m.c.hmlen(), 0);
            m.free();
        }
    }
}
