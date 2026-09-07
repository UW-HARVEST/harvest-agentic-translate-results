//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1–E52), plus the generic FFI boundaries:
//! null pointers, zero and oversized lengths, values one past a valid range,
//! and out-of-enum-range `int` values crossing the FFI boundary.
//!
//! Every case asserts the two libraries produce the SAME rejection — the same
//! sentinel (`-1` / `NULL` / `temp == 0`) or the same process-level outcome
//! (SIGABRT from a live `STBDS_ASSERT`, SIGSEGV from the same wild pointer) —
//! not merely "both failed somehow".

mod harness;

use harness::*;
use std::ffi::{c_char, c_int, c_void};

fn bin_key(i: u64, keysize: usize) -> Vec<u8> {
    let mut r = Rng::new(0x1000_0000 ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut v = r.bytes(keysize.max(1));
    for (k, b) in v.iter_mut().enumerate() {
        if k < 8 {
            *b = ((i >> (8 * k)) & 0xff) as u8;
        }
    }
    v.truncate(keysize.max(1));
    v
}

fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

unsafe fn free_map(api: &Api, a: *mut c_void, elemsize: usize) {
    if !a.is_null() {
        unsafe {
            (api.hmfree_func)((a as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize)
        };
    }
}

// ===========================================================================
// E1–E5: stbds_arrgrowf
// ===========================================================================

#[test]
fn e1_arrgrowf_early_out() {
    let b = libs();
    for elemsize in [1usize, 4, 8, 16] {
        unsafe {
            let c = (b.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let r = (b.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            diff_eq!("E1 setup", snap_arr(c), snap_arr(r));
            // Now min_cap <= capacity and min_len <= min_cap: nothing to do.
            for min_cap in [0usize, 1, 5, 9, 10] {
                let cc = (b.c.arrgrowf)(c, elemsize, 0, min_cap);
                let rr = (b.r.arrgrowf)(r, elemsize, 0, min_cap);
                assert_eq!(cc, c, "E1: C must return the identical pointer");
                assert_eq!(rr, r, "E1: Rust must return the identical pointer");
                diff_eq!(format!("E1 min_cap={min_cap}"), snap_arr(cc), snap_arr(rr));
            }
            (b.c.arrfreef)(c);
            (b.r.arrfreef)(r);
        }
    }
}

#[test]
fn e2_arrgrowf_from_null_initialises_header() {
    let b = libs();
    for elemsize in [1usize, 4, 8, 16, 64] {
        for min_cap in [1usize, 2, 3, 4, 5, 100] {
            unsafe {
                let c = (b.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                let r = (b.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                let cs = snap_arr(c);
                diff_eq!(format!("E2 es={elemsize} mc={min_cap}"), cs.clone(), snap_arr(r));
                let h = cs.header.expect("E2: expected an allocated header");
                assert_eq!(h.length, 0, "E2: length must start at 0");
                assert!(!h.has_hash_table, "E2: hash_table must start NULL");
                assert_eq!(h.temp, 0, "E2: temp must start at 0");
                (b.c.arrfreef)(c);
                (b.r.arrfreef)(r);
            }
        }
    }
}

#[test]
fn e3_arrgrowf_degenerate_returns_null() {
    let b = libs();
    for elemsize in [0usize, 1, 4, 8, 1024] {
        unsafe {
            let c = (b.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let r = (b.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            diff_eq!(format!("E3 es={elemsize}"), c as usize, r as usize);
            assert!(
                c.is_null(),
                "E3: C returns the NULL it was given (early-out), got {c:?}"
            );
            assert!(r.is_null(), "E3: Rust must also return NULL");
        }
    }
}

#[test]
fn e4_arrgrowf_elemsize_zero() {
    let b = libs();
    for min_cap in [0usize, 1, 4, 7, 1000] {
        for addlen in [0usize, 1, 9] {
            unsafe {
                let c = (b.c.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
                let r = (b.r.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
                diff_eq!(
                    format!("E4 mc={min_cap} addlen={addlen}"),
                    snap_arr(c),
                    snap_arr(r)
                );
                if (c as usize) >= 4096 {
                    (b.c.arrfreef)(c);
                    (b.r.arrfreef)(r);
                }
            }
        }
    }
}

/// E5 wrap scenario, isolated so it can run in a forked child.
///
/// With `addlen = SIZE_MAX - 2` and `length = 2`, `min_len` becomes `SIZE_MAX`,
/// so `min_cap = SIZE_MAX` and `elemsize * min_cap + sizeof(header)` wraps to
/// **24** — `realloc` therefore SHRINKS the block below the 32-byte header the
/// function then writes, corrupting the heap. Identical in both libraries, but
/// it cannot be observed in-process.
fn e5_wrap_scenario(api: &Api, addlen: usize) {
    unsafe {
        let a = (api.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        (*((a as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader)).length = 2;
        let b = (api.arrgrowf)(a, 8, addlen, 0);
        let h = *((b as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader);
        std::hint::black_box((h.length, h.capacity, h.temp));
    }
}

#[test]
fn e5_arrgrowf_size_arithmetic_wraps() {
    let b = libs();
    unsafe {
        // (a) `min_len = arrlen(a) + addlen` wraps back into the early-out range.
        let c = (b.c.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        let r = (b.r.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        (*((c as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader)).length = 2;
        (*((r as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader)).length = 2;
        for addlen in [usize::MAX, usize::MAX - 1] {
            let cc = (b.c.arrgrowf)(c, 8, addlen, 0);
            let rr = (b.r.arrgrowf)(r, 8, addlen, 0);
            diff_eq!(format!("E5a addlen={addlen}"), snap_arr(cc), snap_arr(rr));
            assert_eq!(cc, c, "E5a: wrapped min_len must take the early-out path");
            assert_eq!(rr, r, "E5a: wrapped min_len must take the early-out path");
        }
        (b.c.arrfreef)(c);
        (b.r.arrfreef)(r);
    }
    // (a') `elemsize * min_cap + sizeof(header)` wraps to a value SMALLER than
    // the header, so `realloc` shrinks the block. Compared as a process outcome.
    for addlen in [usize::MAX - 2, usize::MAX - 3, usize::MAX - 5] {
        let co = run_in_child(|| e5_wrap_scenario(&b.c, addlen));
        let ro = run_in_child(|| e5_wrap_scenario(&b.r, addlen));
        diff_eq!(format!("E5a' addlen={addlen} outcome"), co, ro);
    }
    unsafe {
        // (b) `elemsize * min_cap` wraps to exactly 0, so only the header is
        // allocated — this one is well-behaved and fully comparable in-process.
        for elemsize in [1usize << 62, 1usize << 63, usize::MAX / 4 + 1] {
            let c = (b.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let r = (b.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let cs = snap_arr(c);
            diff_eq!(format!("E5b es={elemsize:#x}"), cs.clone(), snap_arr(r));
            assert_eq!(cs.header.unwrap().capacity, 4);
            (b.c.arrfreef)(c);
            (b.r.arrfreef)(r);
        }
    }
}

// ===========================================================================
// E6–E7: stbds_hmfree_func
// ===========================================================================

#[test]
fn e6_hmfree_null() {
    let b = libs();
    for elemsize in [0usize, 1, 8, 16, usize::MAX] {
        unsafe {
            // Must return immediately without touching anything.
            (b.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (b.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

#[test]
fn e7_hmfree_no_hash_table() {
    let b = libs();
    for elemsize in [1usize, 4, 8, 16, 24] {
        unsafe {
            // An array grown but never hashed: `hash_table == NULL`, so the
            // strdup loop and `stbds_strreset` are both skipped.
            let c = (b.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let r = (b.r.hmput_default)(std::ptr::null_mut(), elemsize);
            let cs = snap_map(c, elemsize, KeyRepr::Bytes, elemsize, 0);
            diff_eq!(
                format!("E7 es={elemsize}"),
                cs.clone(),
                snap_map(r, elemsize, KeyRepr::Bytes, elemsize, 0)
            );
            assert!(
                !cs.header.unwrap().has_hash_table,
                "E7: expected hash_table == NULL"
            );
            free_map(&b.c, c, elemsize);
            free_map(&b.r, r, elemsize);
        }
    }
}

// ===========================================================================
// E8–E9: stbds_hm_find_slot returning -1
// ===========================================================================

#[test]
fn e8_e9_find_slot_miss_both_scans() {
    let b = libs();
    // A miss is reported as -1 regardless of whether the empty slot is found in
    // the first `pos&7 .. 8` scan or in the wrapped `0 .. pos&7` scan. Sweeping
    // many table sizes and many random absent keys hits both.
    for n in [0u64, 1, 3, 6, 7, 8, 20, 60, 200] {
        reset_seed(&b, 0x3141_5926);
        let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
        for i in 0..n {
            let mut k = bin_key(i, 8);
            let v = bin_key(i ^ 0xF0F0, 8);
            m.put(&mut k, &v, &format!("E8 n={n} put#{i}"));
        }
        for j in 0..300u64 {
            let mut k = bin_key(0x7000_0000 + j, 8);
            let idx = m.geti_ts(&mut k, &format!("E8/E9 n={n} miss#{j}"));
            assert_eq!(idx, -1, "E8/E9: absent key must report -1, got {idx}");
        }
        m.free();
    }
}

// ===========================================================================
// E10–E13: stbds_hmget_key / _ts miss sentinels
// ===========================================================================

#[test]
fn e10_hmget_key_ts_null_map() {
    let b = libs();
    for elemsize in [1usize, 4, 8, 16, 24] {
        for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, -1, 2] {
            unsafe {
                let mut ct: isize = 0x1234;
                let mut rt: isize = 0x1234;
                let mut k = cstr("anything");
                let kp = k.as_mut_ptr() as *mut c_void;
                let c = (b.c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    kp,
                    elemsize.min(8),
                    &mut ct,
                    mode,
                );
                let r = (b.r.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    kp,
                    elemsize.min(8),
                    &mut rt,
                    mode,
                );
                diff_eq!(format!("E10 es={elemsize} mode={mode} temp"), ct, rt);
                assert_eq!(ct, -1, "E10: NULL map must yield temp == -1");
                let cs = snap_map(c, elemsize, KeyRepr::Bytes, elemsize, 0);
                diff_eq!(
                    format!("E10 es={elemsize} mode={mode} state"),
                    cs.clone(),
                    snap_map(r, elemsize, KeyRepr::Bytes, elemsize, 0)
                );
                let h = cs.header.unwrap();
                assert_eq!(h.length, 1, "E10: a 1-element array must be created");
                assert!(!h.has_hash_table, "E10: no hash table is created");
                free_map(&b.c, c, elemsize);
                free_map(&b.r, r, elemsize);
            }
        }
    }
}

#[test]
fn e11_hmget_key_ts_no_hash_table() {
    let b = libs();
    for elemsize in [4usize, 8, 16] {
        for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2, -7] {
            unsafe {
                let c0 = (b.c.hmput_default)(std::ptr::null_mut(), elemsize);
                let r0 = (b.r.hmput_default)(std::ptr::null_mut(), elemsize);
                let mut ct: isize = 0x1234;
                let mut rt: isize = 0x1234;
                let mut k = cstr("anything");
                let kp = k.as_mut_ptr() as *mut c_void;
                let c = (b.c.hmget_key_ts)(c0, elemsize, kp, 4, &mut ct, mode);
                let r = (b.r.hmget_key_ts)(r0, elemsize, kp, 4, &mut rt, mode);
                diff_eq!(format!("E11 es={elemsize} mode={mode} temp"), ct, rt);
                assert_eq!(ct, -1, "E11: un-hashed map must yield temp == -1");
                assert_eq!(c, c0, "E11: the pointer must be returned unchanged");
                assert_eq!(r, r0, "E11: the pointer must be returned unchanged");
                diff_eq!(
                    format!("E11 es={elemsize} mode={mode} state"),
                    snap_map(c, elemsize, KeyRepr::Bytes, elemsize, 0),
                    snap_map(r, elemsize, KeyRepr::Bytes, elemsize, 0)
                );
                free_map(&b.c, c, elemsize);
                free_map(&b.r, r, elemsize);
            }
        }
    }
}

#[test]
fn e12_hmget_key_ts_absent_key() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    for i in 0..50u64 {
        let mut k = bin_key(i, 8);
        let v = bin_key(i ^ 0x99, 8);
        m.put(&mut k, &v, &format!("E12 put#{i}"));
    }
    for j in 0..200u64 {
        let mut k = bin_key(0x8000_0000 + j, 8);
        let idx = m.geti_ts(&mut k, &format!("E12 absent#{j}"));
        assert_eq!(idx, -1, "E12: absent key must report STBDS_INDEX_EMPTY");
    }
    m.free();
}

#[test]
fn e13_hmget_key_writes_temp_on_miss() {
    let b = libs();
    // The non-`_ts` wrapper copies the sentinel into `stbds_temp(raw_a)`.
    for elemsize in [4usize, 8, 16] {
        unsafe {
            // (a) NULL map
            let mut k = cstr("nope");
            let kp = k.as_mut_ptr() as *mut c_void;
            let c = (b.c.hmget_key)(std::ptr::null_mut(), elemsize, kp, 4, STBDS_HM_BINARY);
            let r = (b.r.hmget_key)(std::ptr::null_mut(), elemsize, kp, 4, STBDS_HM_BINARY);
            let cs = snap_map(c, elemsize, KeyRepr::Bytes, elemsize, 0);
            diff_eq!(
                format!("E13a es={elemsize}"),
                cs.clone(),
                snap_map(r, elemsize, KeyRepr::Bytes, elemsize, 0)
            );
            assert_eq!(cs.header.unwrap().temp, -1, "E13a: temp must be -1");
            // (b) un-hashed map
            let c2 = (b.c.hmget_key)(c, elemsize, kp, 4, STBDS_HM_BINARY);
            let r2 = (b.r.hmget_key)(r, elemsize, kp, 4, STBDS_HM_BINARY);
            let cs2 = snap_map(c2, elemsize, KeyRepr::Bytes, elemsize, 0);
            diff_eq!(
                format!("E13b es={elemsize}"),
                cs2.clone(),
                snap_map(r2, elemsize, KeyRepr::Bytes, elemsize, 0)
            );
            assert_eq!(cs2.header.unwrap().temp, -1, "E13b: temp must be -1");
            free_map(&b.c, c2, elemsize);
            free_map(&b.r, r2, elemsize);
        }
    }
    // (c) populated map, absent key — covered by MapPair::geti's own comparison.
    reset_seed(&b, 0x3141_5926);
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    for i in 0..30u64 {
        let mut k = bin_key(i, 8);
        let v = bin_key(i, 8);
        m.put(&mut k, &v, &format!("E13c put#{i}"));
    }
    for j in 0..50u64 {
        let mut k = bin_key(0x9000_0000 + j, 8);
        let idx = m.geti(&mut k, &format!("E13c absent#{j}"));
        assert_eq!(idx, -1);
    }
    m.free();
}

// ===========================================================================
// E14–E15: out-of-enum-range `mode` across the FFI boundary
// ===========================================================================

/// Every `int` value a C caller can legally pass as `mode`, including values
/// with no corresponding enum variant.
const MODES: [c_int; 12] = [
    c_int::MIN,
    -1000,
    -2,
    -1,
    0,
    1,
    2,
    3,
    127,
    255,
    1000,
    c_int::MAX,
];

#[test]
fn e14_e15_mode_out_of_range_get() {
    let b = libs();
    for &mode in &MODES {
        // Build the table with the SAME out-of-range mode used for lookups, so
        // the map is self-consistent and only the >=1 / <1 split is exercised.
        let string_like = mode >= STBDS_HM_STRING;
        let repr = if string_like {
            KeyRepr::Ptr
        } else {
            KeyRepr::Bytes
        };
        reset_seed(&b, 0x3141_5926);
        let mut owned: Vec<Vec<u8>> = (0..20).map(|i| cstr(&format!("key_{i}"))).collect();
        let mut m = MapPair::new(&b, 16, 8, 8, repr, mode);
        for i in 0..20 {
            let kp: &mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(owned[i].as_mut_ptr(), owned[i].len())
            };
            let v = bin_key(i as u64, 8);
            m.put(kp, &v, &format!("E14/E15 mode={mode} put#{i}"));
        }
        for i in 0..20 {
            let kp: &mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(owned[i].as_mut_ptr(), owned[i].len())
            };
            let idx = m.geti(kp, &format!("E14/E15 mode={mode} get#{i}"));
            assert!(idx >= 0, "E14/E15 mode={mode}: key {i} should be found");
            m.geti_ts(kp, &format!("E14/E15 mode={mode} get_ts#{i}"));
        }
        for i in 0..20 {
            let mut k = cstr(&format!("absent_{i}"));
            let idx = m.geti_ts(&mut k, &format!("E14/E15 mode={mode} absent#{i}"));
            assert_eq!(idx, -1, "E14/E15 mode={mode}: absent key reported present");
        }
        m.free();
        drop(owned);
    }
}

// ===========================================================================
// E16–E18: stbds_hmput_default
// ===========================================================================

#[test]
fn e16_e17_e18_hmput_default_branches() {
    let b = libs();
    for elemsize in [1usize, 4, 8, 16, 24, 32] {
        unsafe {
            // E16: a == NULL
            let mut c = (b.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let mut r = (b.r.hmput_default)(std::ptr::null_mut(), elemsize);
            let cs = snap_map(c, elemsize, KeyRepr::Bytes, elemsize, 0);
            diff_eq!(
                format!("E16 es={elemsize}"),
                cs.clone(),
                snap_map(r, elemsize, KeyRepr::Bytes, elemsize, 0)
            );
            assert_eq!(cs.header.unwrap().length, 1);
            assert_eq!(cs.elems.unwrap(), vec![0u8; elemsize], "E16: must be zeroed");

            // E18: length != 0 -> returned unchanged, NOT re-zeroed
            for k in 0..elemsize {
                *(c as *mut u8).wrapping_sub(elemsize).wrapping_add(k) = 0x5C;
                *(r as *mut u8).wrapping_sub(elemsize).wrapping_add(k) = 0x5C;
            }
            let c2 = (b.c.hmput_default)(c, elemsize);
            let r2 = (b.r.hmput_default)(r, elemsize);
            assert_eq!(c2, c, "E18: pointer must be returned unchanged");
            assert_eq!(r2, r, "E18: pointer must be returned unchanged");
            let cs2 = snap_map(c2, elemsize, KeyRepr::Bytes, elemsize, 0);
            diff_eq!(
                format!("E18 es={elemsize}"),
                cs2.clone(),
                snap_map(r2, elemsize, KeyRepr::Bytes, elemsize, 0)
            );
            assert_eq!(
                cs2.elems.unwrap(),
                vec![0x5Cu8; elemsize],
                "E18: must NOT re-zero"
            );

            // E17: length == 0 -> re-grow and zero
            (*((c as *mut u8)
                .wrapping_sub(elemsize)
                .wrapping_sub(HEADER_SIZE) as *mut ArrHeader))
                .length = 0;
            (*((r as *mut u8)
                .wrapping_sub(elemsize)
                .wrapping_sub(HEADER_SIZE) as *mut ArrHeader))
                .length = 0;
            c = (b.c.hmput_default)(c, elemsize);
            r = (b.r.hmput_default)(r, elemsize);
            let cs3 = snap_map(c, elemsize, KeyRepr::Bytes, elemsize, 0);
            diff_eq!(
                format!("E17 es={elemsize}"),
                cs3.clone(),
                snap_map(r, elemsize, KeyRepr::Bytes, elemsize, 0)
            );
            assert_eq!(cs3.header.unwrap().length, 1);
            assert_eq!(cs3.elems.unwrap(), vec![0u8; elemsize], "E17: must re-zero");

            free_map(&b.c, c, elemsize);
            free_map(&b.r, r, elemsize);
        }
    }
}

// ===========================================================================
// E19–E23, E25–E26: stbds_hmput_key
// ===========================================================================

#[test]
fn e19_hmput_key_null_bootstrap() {
    let b = libs();
    for elemsize in [8usize, 16, 24] {
        for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2, -1] {
            reset_seed(&b, 0x3141_5926);
            unsafe {
                let mut k = cstr("bootstrap");
                let kp = k.as_mut_ptr() as *mut c_void;
                let c = (b.c.hmput_key)(std::ptr::null_mut(), elemsize, kp, 8, mode);
                let r = (b.r.hmput_key)(std::ptr::null_mut(), elemsize, kp, 8, mode);
                let repr = if mode >= STBDS_HM_STRING {
                    KeyRepr::Ptr
                } else {
                    KeyRepr::Bytes
                };
                let cs = snap_map(c, elemsize, repr, 8, 0);
                diff_eq!(
                    format!("E19 es={elemsize} mode={mode}"),
                    cs.clone(),
                    snap_map(r, elemsize, repr, 8, 0)
                );
                let h = cs.header.unwrap();
                assert_eq!(h.length, 2, "E19: default slot + the new element");
                assert_eq!(h.temp, 0, "E19: the first key lands at index 0");
                free_map(&b.c, c, elemsize);
                free_map(&b.r, r, elemsize);
            }
        }
    }
}

#[test]
fn e20_e21_hmput_key_duplicate() {
    let b = libs();
    // Re-putting keys many times over a large table forces duplicate hits in
    // both the first and the wrapped bucket scans.
    for &n in &[1u64, 7, 40, 150] {
        reset_seed(&b, 0x3141_5926);
        let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
        for i in 0..n {
            let mut k = bin_key(i, 8);
            let v = bin_key(i, 8);
            m.put(&mut k, &v, &format!("E20 n={n} put#{i}"));
        }
        let len_before = {
            let s = m.snap_c();
            s.header.unwrap().length
        };
        for round in 0..4u64 {
            for i in 0..n {
                let mut k = bin_key(i, 8);
                let v = bin_key(i ^ (round << 32), 8);
                m.put(&mut k, &v, &format!("E20/E21 n={n} r={round} re-put#{i}"));
            }
        }
        let len_after = {
            let s = m.snap_c();
            s.header.unwrap().length
        };
        assert_eq!(
            len_before, len_after,
            "E20/E21: duplicate puts must not grow the map"
        );
        m.free();
    }
}

#[test]
fn e22_hmput_key_load_factor_grow() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    let mut seen_counts: Vec<usize> = Vec::new();
    for i in 0..400u64 {
        let mut k = bin_key(i, 8);
        let v = bin_key(i, 8);
        m.put(&mut k, &v, &format!("E22 put#{i}"));
        let sc = m.snap_c().index.unwrap().slot_count;
        if seen_counts.last() != Some(&sc) {
            seen_counts.push(sc);
        }
    }
    assert_eq!(seen_counts[0], 8, "E22: tables start at 8 slots");
    assert!(
        seen_counts.len() >= 7,
        "E22: expected several growths, got {seen_counts:?}"
    );
    assert!(
        seen_counts.windows(2).all(|w| w[1] == w[0] * 2),
        "E22: slot_count must exactly double each time, got {seen_counts:?}"
    );
    m.free();
}

#[test]
fn e23_hmput_key_tombstone_reuse() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    // Delete then re-insert the same key repeatedly: the re-insert must land on
    // the tombstone the delete left, decrementing `tombstone_count`.
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    for i in 0..40u64 {
        let mut k = bin_key(i, 8);
        let v = bin_key(i, 8);
        m.put(&mut k, &v, &format!("E23 seed#{i}"));
    }
    for round in 0..60u64 {
        let i = round % 40;
        let mut k = bin_key(i, 8);
        m.del(&mut k, 0, &format!("E23 r={round} del#{i}"));
        let v = bin_key(i ^ round, 8);
        m.put(&mut k, &v, &format!("E23 r={round} reput#{i}"));
    }
    m.free();
}

#[test]
fn e25_hmput_key_string_mode_no_valid_variant() {
    let b = libs();
    // `stbds_shmode_func(-1)` truncates to `string.mode == 255`, which has no
    // enum variant, so the `switch` takes `default:` -> `memcpy(elem, key,
    // keysize)`, i.e. an inline byte copy rather than a strdup/arena copy.
    for sh in [-1i32, 4, 5, 200, 255, 256, 1000] {
        for hm_mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2] {
            reset_seed(&b, 0x3141_5926);
            unsafe {
                let mut c = (b.c.shmode_func)(16, sh);
                let mut r = (b.r.shmode_func)(16, sh);
                let effective = (sh as u32 & 0xff) as u8;
                let repr = if (1..=3).contains(&effective) {
                    KeyRepr::Ptr
                } else {
                    KeyRepr::Bytes
                };
                let mut owned: Vec<Vec<u8>> =
                    (0..4).map(|i| cstr(&format!("dflt_{i}"))).collect();
                for i in 0..4 {
                    let kp = owned[i].as_mut_ptr() as *mut c_void;
                    c = (b.c.hmput_key)(c, 16, kp, 8, hm_mode);
                    r = (b.r.hmput_key)(r, 16, kp, 8, hm_mode);
                    diff_eq!(
                        format!("E25 sh={sh} hm={hm_mode} put#{i}"),
                        snap_map(c, 16, repr, 8, 0),
                        snap_map(r, 16, repr, 8, 0)
                    );
                }
                free_map(&b.c, c, 16);
                free_map(&b.r, r, 16);
                drop(owned);
            }
        }
    }
}

#[test]
fn e26_hmput_key_keysize_zero() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    // `keysize == 0`: every key hashes identically and `memcmp(.., 0) == 0`
    // always matches, so after the first insert every put is a duplicate hit.
    let mut m = MapPair::new(&b, 8, 0, 4, KeyRepr::Bytes, STBDS_HM_BINARY);
    for i in 0..50u64 {
        let mut k = bin_key(i, 1);
        let v = bin_key(i, 4);
        m.put(&mut k, &v, &format!("E26 put#{i}"));
    }
    let len = m.snap_c().header.unwrap().length;
    assert_eq!(len, 2, "E26: keysize 0 must collapse to a single element");
    for i in 0..10u64 {
        let mut k = bin_key(i, 1);
        let idx = m.geti_ts(&mut k, &format!("E26 get#{i}"));
        assert_eq!(idx, 0, "E26: every key must resolve to index 0");
    }
    m.free();
}

// ===========================================================================
// E27–E31, E36–E38: stbds_hmdel_key
// ===========================================================================

#[test]
fn e27_hmdel_key_null_returns_null() {
    let b = libs();
    for elemsize in [0usize, 1, 8, 16] {
        for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2, -1] {
            for keyoffset in [0usize, 8] {
                unsafe {
                    let mut k = cstr("whatever");
                    let kp = k.as_mut_ptr() as *mut c_void;
                    let c = (b.c.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        kp,
                        8,
                        keyoffset,
                        mode,
                    );
                    let r = (b.r.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        kp,
                        8,
                        keyoffset,
                        mode,
                    );
                    diff_eq!(
                        format!("E27 es={elemsize} mode={mode} ko={keyoffset}"),
                        c as usize,
                        r as usize
                    );
                    assert!(c.is_null(), "E27: C must return NULL");
                    assert!(r.is_null(), "E27: Rust must return NULL");
                }
            }
        }
    }
}

#[test]
fn e28_hmdel_key_no_hash_table() {
    let b = libs();
    for elemsize in [4usize, 8, 16] {
        unsafe {
            let c0 = (b.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let r0 = (b.r.hmput_default)(std::ptr::null_mut(), elemsize);
            // Pre-poison `temp` so we can see it being cleared to 0.
            (*((c0 as *mut u8)
                .wrapping_sub(elemsize)
                .wrapping_sub(HEADER_SIZE) as *mut ArrHeader))
                .temp = 77;
            (*((r0 as *mut u8)
                .wrapping_sub(elemsize)
                .wrapping_sub(HEADER_SIZE) as *mut ArrHeader))
                .temp = 77;
            let mut k = cstr("nope");
            let kp = k.as_mut_ptr() as *mut c_void;
            let c = (b.c.hmdel_key)(c0, elemsize, kp, 4, 0, STBDS_HM_BINARY);
            let r = (b.r.hmdel_key)(r0, elemsize, kp, 4, 0, STBDS_HM_BINARY);
            assert_eq!(c, c0, "E28: pointer must come back unchanged");
            assert_eq!(r, r0, "E28: pointer must come back unchanged");
            let cs = snap_map(c, elemsize, KeyRepr::Bytes, elemsize, 0);
            diff_eq!(
                format!("E28 es={elemsize}"),
                cs.clone(),
                snap_map(r, elemsize, KeyRepr::Bytes, elemsize, 0)
            );
            assert_eq!(cs.header.unwrap().temp, 0, "E28: temp must be reset to 0");
            free_map(&b.c, c, elemsize);
            free_map(&b.r, r, elemsize);
        }
    }
}

#[test]
fn e29_e30_hmdel_key_absent_and_double_delete() {
    let b = libs();
    for &n in &[1u64, 7, 40, 150] {
        reset_seed(&b, 0x3141_5926);
        let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
        for i in 0..n {
            let mut k = bin_key(i, 8);
            let v = bin_key(i, 8);
            m.put(&mut k, &v, &format!("E29 n={n} put#{i}"));
        }
        // E29: never-inserted keys
        for j in 0..60u64 {
            let mut k = bin_key(0xA000_0000 + j, 8);
            let before = m.snap_c().header.unwrap().length;
            let t = m.del(&mut k, 0, &format!("E29 n={n} absent#{j}"));
            let after = m.snap_c().header.unwrap().length;
            assert_eq!(t, 0, "E29: deleting an absent key must report temp == 0");
            assert_eq!(before, after, "E29: length must not change");
        }
        // E30: delete twice
        for i in 0..n {
            let mut k = bin_key(i, 8);
            let t1 = m.del(&mut k, 0, &format!("E30 n={n} del#{i} first"));
            assert_eq!(t1, 1, "E30: first delete must report 1");
            let t2 = m.del(&mut k, 0, &format!("E30 n={n} del#{i} second"));
            assert_eq!(t2, 0, "E30: second delete must report 0");
            let t3 = m.del(&mut k, 0, &format!("E30 n={n} del#{i} third"));
            assert_eq!(t3, 0, "E30: third delete must report 0");
        }
        m.free();
    }
}

/// Reproduce ERRORS.md E31 / E33 / E34 in a child process.
fn e31_scenario(api: &Api, mode: c_int, n: usize) {
    unsafe {
        let mut owned: Vec<Vec<u8>> = (0..n).map(|i| cstr(&format!("mm_{i}"))).collect();
        let mut a = (api.shmode_func)(16, STBDS_SH_STRDUP);
        for i in 0..n {
            let kp = owned[i].as_mut_ptr() as *mut c_void;
            a = (api.hmput_key)(a, 16, kp, 8, mode);
        }
        let kp = owned[0].as_mut_ptr() as *mut c_void;
        a = (api.hmdel_key)(a, 16, kp, 8, 0, mode);
        std::hint::black_box(a);
        std::mem::forget(owned);
    }
}

#[test]
fn e31_hmdel_key_mode_two_asymmetry() {
    let b = libs();
    // n == 1: `old_index == final_index`, so the STRDUP free is skipped
    // (`mode != STBDS_HM_STRING`) and the re-find is never reached. Both
    // libraries must complete and report the same sentinel.
    for mode in [2i32, 3, 7, 1000, c_int::MAX] {
        reset_seed(&b, 0x3141_5926);
        let mut owned = vec![cstr("solo")];
        let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Ptr, mode).with_shmode(STBDS_SH_STRDUP);
        let kp: &mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(owned[0].as_mut_ptr(), owned[0].len()) };
        let v = bin_key(0, 8);
        m.put(kp, &v, &format!("E31 mode={mode} put"));
        let t = m.del(kp, 0, &format!("E31 mode={mode} del"));
        assert_eq!(t, 1, "E31: the delete itself still succeeds");
        m.free();
        drop(owned);
    }
    // n >= 2: the re-find is reached with a BINARY-style key argument even
    // though `find_slot` hashes it as a string, so it misses and the live
    // `STBDS_ASSERT(slot >= 0)` aborts. Both sides must abort identically.
    for mode in [2i32, 3, 7, 1000, c_int::MAX] {
        for n in [2usize, 9] {
            reset_seed(&b, 0x3141_5926);
            let co = run_in_child(|| e31_scenario(&b.c, mode, n));
            reset_seed(&b, 0x3141_5926);
            let ro = run_in_child(|| e31_scenario(&b.r, mode, n));
            diff_eq!(format!("E31/E33/E34 mode={mode} n={n}"), co, ro);
            assert_eq!(
                co,
                Outcome::Signaled(6),
                "E31: expected SIGABRT from the live STBDS_ASSERT(slot >= 0)"
            );
        }
    }
}

#[test]
fn e36_hmdel_key_shrink() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    for i in 0..300u64 {
        let mut k = bin_key(i, 8);
        let v = bin_key(i, 8);
        m.put(&mut k, &v, &format!("E36 put#{i}"));
    }
    let mut counts: Vec<usize> = vec![m.snap_c().index.unwrap().slot_count];
    for i in 0..300u64 {
        let mut k = bin_key(i, 8);
        m.del(&mut k, 0, &format!("E36 del#{i}"));
        let sc = m.snap_c().index.unwrap().slot_count;
        if counts.last() != Some(&sc) {
            counts.push(sc);
        }
    }
    assert!(
        counts.len() > 1 && counts.last() == Some(&8),
        "E36: expected shrinking down to 8, got {counts:?}"
    );
    assert!(
        counts.windows(2).all(|w| w[1] <= w[0]),
        "E36: slot_count must only shrink here, got {counts:?}"
    );
    m.free();
}

#[test]
fn e37_hmdel_key_tombstone_rebuild() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    // Keep `used_count` high (so shrinking is not triggered) while churning
    // through throwaway keys so tombstones cross
    // `tombstone_count_threshold = (slot_count>>3) + (slot_count>>4)`.
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    for i in 0..60u64 {
        let mut k = bin_key(i, 8);
        let v = bin_key(i, 8);
        m.put(&mut k, &v, &format!("E37 live#{i}"));
    }
    let mut saw_reset = false;
    let mut prev_tomb = m.snap_c().index.unwrap().tombstone_count;
    for round in 0..200u64 {
        let victim = 0xB000_0000 + round;
        let mut k = bin_key(victim, 8);
        let v = bin_key(victim, 8);
        m.put(&mut k, &v, &format!("E37 r={round} put"));
        m.del(&mut k, 0, &format!("E37 r={round} del"));
        let tomb = m.snap_c().index.unwrap().tombstone_count;
        if tomb < prev_tomb {
            saw_reset = true;
        }
        prev_tomb = tomb;
    }
    assert!(saw_reset, "E37: expected at least one tombstone rebuild");
    for i in 0..60u64 {
        let mut k = bin_key(i, 8);
        let idx = m.geti(&mut k, &format!("E37 survivor#{i}"));
        assert!(idx >= 0, "E37: live key {i} lost");
    }
    m.free();
}

/// The E38 scenario, isolated so it can be run in a forked child.
fn e38_scenario(api: &Api, keyoffset: usize, n: u64) {
    unsafe {
        let mut a: *mut c_void = std::ptr::null_mut();
        for i in 0..n {
            let mut k = bin_key(i, 8);
            a = (api.hmput_key)(a, 16, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
            let t = (*((a as *mut u8).wrapping_sub(16).wrapping_sub(HEADER_SIZE)
                as *mut ArrHeader))
                .temp;
            let v = bin_key(i, 8);
            std::ptr::copy_nonoverlapping(
                v.as_ptr(),
                (a as *mut u8).wrapping_offset(16 * t).wrapping_add(8),
                8,
            );
        }
        for i in 0..n {
            let mut k = bin_key(i, 8);
            a = (api.hmdel_key)(
                a,
                16,
                k.as_mut_ptr() as *mut c_void,
                8,
                keyoffset,
                STBDS_HM_BINARY,
            );
            if a.is_null() {
                return;
            }
        }
        (api.hmfree_func)((a as *mut u8).wrapping_sub(16) as *mut c_void, 16);
    }
}

#[test]
fn e38_hmdel_key_wrong_keyoffset() {
    let b = libs();
    // `stbds_hmput_key` hard-codes `keyoffset = 0`, so passing a non-zero
    // `keyoffset` to `stbds_hmdel_key` compares the wrong bytes. Depending on the
    // element layout that either misses (temp 0, `return a`) or — when the bytes
    // at the wrong offset happen to match — corrupts the slot re-find enough to
    // trip the live `STBDS_ASSERT(slot >= 0)`. Both libraries must take the same
    // route, so the process-level outcome is compared first, and only scenarios
    // that terminate normally are then re-run inline for a full state diff.
    for keyoffset in [1usize, 2, 4, 8, 12, 15] {
        for &n in &[1u64, 7, 40] {
            reset_seed(&b, 0x3141_5926);
            let co = run_in_child(|| e38_scenario(&b.c, keyoffset, n));
            reset_seed(&b, 0x3141_5926);
            let ro = run_in_child(|| e38_scenario(&b.r, keyoffset, n));
            diff_eq!(format!("E38 ko={keyoffset} n={n} outcome"), co, ro);

            if co != Outcome::Exited(0) {
                continue;
            }
            // Safe to run in-process: compare full state after every call.
            reset_seed(&b, 0x3141_5926);
            let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
            for i in 0..n {
                let mut k = bin_key(i, 8);
                let v = bin_key(i, 8);
                m.put(&mut k, &v, &format!("E38 ko={keyoffset} n={n} put#{i}"));
            }
            let before = m.snap_c().header.unwrap().length;
            for i in 0..n {
                let mut k = bin_key(i, 8);
                m.del(
                    &mut k,
                    keyoffset,
                    &format!("E38 ko={keyoffset} n={n} del#{i}"),
                );
            }
            let after = m.snap_c().header.unwrap().length;
            assert!(after <= before);
            m.free();
        }
    }
}

// ===========================================================================
// E24, E32, E35, E39: asserts that are unreachable via the public API
// ===========================================================================

#[test]
fn e24_e32_e35_e39_unreachable_asserts_never_fire() {
    let b = libs();
    // These four `STBDS_ASSERT`s are analysed as unreachable:
    //  E24 `hmput_key`:        (size_t)i+1 <= arrcap(a)  — guaranteed by the grow above it
    //  E32 `hmdel_key`:        slot < slot_count         — find_slot masks pos with slot_count-1
    //  E35 `hmdel_key`:        used_count >= 0           — `used_count` is size_t
    //  E39 `make_hash_index`:  used_thr + tomb_thr < slot_count — slot_count is a power of two >= 8
    // The Rust translation contains all four (as `abort()` calls), so the check
    // is that a heavy mixed workload terminates NORMALLY on both sides, and
    // identically.
    fn workload(api: &Api) {
        unsafe {
            let mut rng = Rng::new(0xA55A);
            for elemsize in [8usize, 16, 24] {
                for sh in [None, Some(STBDS_SH_STRDUP), Some(STBDS_SH_ARENA)] {
                    let mode = if sh.is_some() {
                        STBDS_HM_STRING
                    } else {
                        STBDS_HM_BINARY
                    };
                    let mut a: *mut c_void = match sh {
                        Some(sm) => (api.shmode_func)(elemsize, sm),
                        None => std::ptr::null_mut(),
                    };
                    let mut owned: Vec<Vec<u8>> =
                        (0..300).map(|i| cstr(&format!("wl_{i}"))).collect();
                    for _ in 0..3000 {
                        let i = rng.below(300);
                        let kp = owned[i].as_mut_ptr() as *mut c_void;
                        if rng.below(3) < 2 {
                            a = (api.hmput_key)(a, elemsize, kp, 8, mode);
                        } else {
                            a = (api.hmdel_key)(a, elemsize, kp, 8, 0, mode);
                        }
                        if a.is_null() {
                            break;
                        }
                    }
                    if !a.is_null() {
                        (api.hmfree_func)(
                            (a as *mut u8).wrapping_sub(elemsize) as *mut c_void,
                            elemsize,
                        );
                    }
                    drop(owned);
                }
            }
        }
    }
    reset_seed(&b, 0x3141_5926);
    let co = run_in_child(|| workload(&b.c));
    reset_seed(&b, 0x3141_5926);
    let ro = run_in_child(|| workload(&b.r));
    diff_eq!("E24/E32/E35/E39 outcome", co, ro);
    assert_eq!(
        co,
        Outcome::Exited(0),
        "E24/E32/E35/E39: no assert should fire on a valid workload"
    );
}

// ===========================================================================
// E40: make_hash_index shrink-threshold clamp
// ===========================================================================

#[test]
fn e40_slot_count_eight_disables_shrinking() {
    let b = libs();
    reset_seed(&b, 0x3141_5926);
    unsafe {
        let c = (b.c.shmode_func)(16, STBDS_SH_DEFAULT);
        let r = (b.r.shmode_func)(16, STBDS_SH_DEFAULT);
        let cs = snap_map(c, 16, KeyRepr::Bytes, 8, 0);
        diff_eq!("E40", cs.clone(), snap_map(r, 16, KeyRepr::Bytes, 8, 0));
        let ix = cs.index.unwrap();
        assert_eq!(ix.slot_count, 8);
        assert_eq!(
            ix.used_count_shrink_threshold, 0,
            "E40: shrinking must be disabled at slot_count 8"
        );
        free_map(&b.c, c, 16);
        free_map(&b.r, r, 16);
    }
    // And for slot_count > 8 it must be slot_count>>2.
    reset_seed(&b, 0x3141_5926);
    let mut m = MapPair::new(&b, 16, 8, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
    for i in 0..100u64 {
        let mut k = bin_key(i, 8);
        let v = bin_key(i, 8);
        m.put(&mut k, &v, &format!("E40 put#{i}"));
        let ix = m.snap_c().index.unwrap();
        let want = if ix.slot_count <= 8 {
            0
        } else {
            ix.slot_count >> 2
        };
        assert_eq!(ix.used_count_shrink_threshold, want);
    }
    m.free();
}

// ===========================================================================
// E41–E43: hash function degenerate inputs
// ===========================================================================

#[test]
fn e41_hash_bytes_len_zero() {
    let b = libs();
    for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
        unsafe {
            let c = (b.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let r = (b.r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            diff_eq!(format!("E41 null seed={seed:#x}"), c, r);
            let mut buf = [0xAAu8; 64];
            let p = buf.as_mut_ptr() as *mut c_void;
            let c2 = (b.c.hash_bytes)(p, 0, seed);
            let r2 = (b.r.hash_bytes)(p, 0, seed);
            diff_eq!(format!("E41 ptr seed={seed:#x}"), c2, r2);
            assert_eq!(c, c2, "E41: len 0 must not depend on the pointer");
        }
    }
}

#[test]
fn e42_hash_bytes_partial_tail_sign_extension() {
    let b = libs();
    // The tail `switch` builds `data` with `d[3] << 24` computed in `int`, so a
    // byte >= 0x80 at offset 3 sign-extends into the upper 32 bits of `size_t`.
    let mut rng = Rng::new(TEST_SEED ^ 0x42);
    for len in 1..24usize {
        if len % 8 == 0 {
            continue;
        }
        for _ in 0..200 {
            let mut buf = rng.bytes(len);
            for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
                unsafe {
                    let p = buf.as_mut_ptr() as *mut c_void;
                    let c = (b.c.hash_bytes)(p, len, seed);
                    let r = (b.r.hash_bytes)(p, len, seed);
                    diff_eq!(format!("E42 len={len} seed={seed:#x}"), c, r);
                }
            }
        }
        // Explicit sign-bit-at-offset-3 and offset-7 patterns.
        for hot in 0..len.min(8) {
            let mut buf = vec![0u8; len];
            buf[hot] = 0xFF;
            for seed in [0usize, 0x3141_5926] {
                unsafe {
                    let p = buf.as_mut_ptr() as *mut c_void;
                    let c = (b.c.hash_bytes)(p, len, seed);
                    let r = (b.r.hash_bytes)(p, len, seed);
                    diff_eq!(format!("E42 len={len} hot={hot}"), c, r);
                }
            }
        }
    }
}

#[test]
fn e43_hash_string_empty() {
    let b = libs();
    let mut empty = vec![0u8];
    for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
        unsafe {
            let p = empty.as_mut_ptr() as *mut c_char;
            let c = (b.c.hash_string)(p, seed);
            let r = (b.r.hash_string)(p, seed);
            diff_eq!(format!("E43 seed={seed:#x}"), c, r);
        }
    }
}

// ===========================================================================
// E44–E49: string arena edge cases
// ===========================================================================

unsafe fn arena_obs(api: &Api, a: *mut StringArena, s: &mut Vec<u8>) -> (Vec<u8>, usize, u8, u8, usize) {
    unsafe {
        let ret = (api.stralloc)(a, s.as_mut_ptr() as *mut c_char);
        let (rem, blk, md, chain) = snap_arena(&*a);
        (read_cstr(ret), rem, blk, md, chain)
    }
}

#[test]
fn e44_stralloc_oversize_null_storage() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 0x44);
    for len in [512usize, 513, 1000, 65536] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let mut s = rng.ascii(len);
        unsafe {
            let co = arena_obs(&b.c, &mut ca, &mut s);
            let ro = arena_obs(&b.r, &mut ra, &mut s);
            diff_eq!(format!("E44 len={len}"), co.clone(), ro);
            assert_eq!(co.1, 0, "E44: remaining must be reset to 0");
            assert_eq!(co.4, 1, "E44: exactly one block in the chain");
            (b.c.strreset)(&mut ca);
            (b.r.strreset)(&mut ra);
        }
    }
}

#[test]
fn e45_stralloc_oversize_splices_after_head() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 0x45);
    for big in [513usize, 900, 40_000] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        unsafe {
            let mut small = rng.ascii(8);
            let c1 = arena_obs(&b.c, &mut ca, &mut small);
            let r1 = arena_obs(&b.r, &mut ra, &mut small);
            diff_eq!(format!("E45 big={big} seed alloc"), c1.clone(), r1);
            let rem_before = c1.1;
            let mut s = rng.ascii(big);
            let co = arena_obs(&b.c, &mut ca, &mut s);
            let ro = arena_obs(&b.r, &mut ra, &mut s);
            diff_eq!(format!("E45 big={big}"), co.clone(), ro);
            assert_eq!(
                co.1, rem_before,
                "E45: `remaining` must NOT be reset when storage != NULL"
            );
            assert_eq!(co.4, 2, "E45: the oversize block is spliced after the head");
            (b.c.strreset)(&mut ca);
            (b.r.strreset)(&mut ra);
        }
    }
}

#[test]
fn e46_stralloc_block_clamps_at_max() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 0x46);
    // `blocksize = 512 << (block>>1)`; it stops incrementing once
    // `blocksize >= 1<<20`, i.e. at block == 22.
    for block in [18u8, 19, 20, 21, 22, 23] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        ca.block = block;
        ra.block = block;
        let mut s = rng.ascii(4);
        unsafe {
            let co = arena_obs(&b.c, &mut ca, &mut s);
            let ro = arena_obs(&b.r, &mut ra, &mut s);
            diff_eq!(format!("E46 block={block}"), co.clone(), ro);
            let expect = if 512usize << (block >> 1) < (1 << 20) {
                block + 1
            } else {
                block
            };
            assert_eq!(co.2, expect, "E46: block clamp wrong for {block}");
            (b.c.strreset)(&mut ca);
            (b.r.strreset)(&mut ra);
        }
    }
}

#[test]
fn e47_stralloc_block_out_of_library_range() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 0x47);
    // Only values whose masked shift keeps `blocksize` allocatable (or wraps it
    // to 0, taking the oversize path) are observable; the rest make BOTH
    // libraries `realloc` a multi-gigabyte block, fail, and crash identically.
    for block in [
        24u8, 26, 28, 30, 31, 110, 112, 120, 126, 127, 128, 130, 132, 140, 150, 240, 250, 254, 255,
    ] {
        for len in [1usize, 40, 700] {
            let mut ca = StringArena::zeroed();
            let mut ra = StringArena::zeroed();
            ca.block = block;
            ra.block = block;
            let mut s = rng.ascii(len - 1);
            unsafe {
                let co = arena_obs(&b.c, &mut ca, &mut s);
                let ro = arena_obs(&b.r, &mut ra, &mut s);
                diff_eq!(format!("E47 block={block} len={len}"), co, ro);
                (b.c.strreset)(&mut ca);
                (b.r.strreset)(&mut ra);
            }
        }
    }
}

#[test]
fn e48_stralloc_empty_string_fresh_arena() {
    let b = libs();
    let mut empty = vec![0u8];
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    unsafe {
        let co = arena_obs(&b.c, &mut ca, &mut empty);
        let ro = arena_obs(&b.r, &mut ra, &mut empty);
        diff_eq!("E48", co.clone(), ro);
        assert_eq!(co.0, Vec::<u8>::new(), "E48: the empty string round-trips");
        assert_eq!(co.1, 511, "E48: 512-byte block minus the 1-byte NUL");
        assert_eq!(co.2, 1, "E48: block must advance to 1");
        (b.c.strreset)(&mut ca);
        (b.r.strreset)(&mut ra);
    }
}

#[test]
fn e49_strreset_empty_arena() {
    let b = libs();
    for (block, mode, remaining) in [(0u8, 0u8, 0usize), (7, 3, 123), (255, 255, usize::MAX)] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        ca.block = block;
        ca.mode = mode;
        ca.remaining = remaining;
        ra.block = block;
        ra.mode = mode;
        ra.remaining = remaining;
        unsafe {
            (b.c.strreset)(&mut ca);
            (b.r.strreset)(&mut ra);
            diff_eq!(
                format!("E49 block={block} mode={mode}"),
                snap_arena(&ca),
                snap_arena(&ra)
            );
            assert_eq!(snap_arena(&ca), (0, 0, 0, 0), "E49: arena must be zeroed");
        }
    }
}

// ===========================================================================
// E50: stbds_arrfreef(NULL) — free() of a wild pointer
// ===========================================================================

#[test]
fn e50_arrfreef_null_is_undefined_but_identical() {
    let b = libs();
    // `stbds_arrfreef(NULL)` becomes `free((char *)NULL - 32)`. That is UB in C,
    // and the `arrfree` macro guards against it, but both libraries must still
    // do the SAME thing with the same wild pointer. Run each in a child and
    // compare how it terminates.
    let co = run_in_child(|| unsafe { (b.c.arrfreef)(std::ptr::null_mut()) });
    let ro = run_in_child(|| unsafe { (b.r.arrfreef)(std::ptr::null_mut()) });
    diff_eq!("E50 outcome", co, ro);
}

// ===========================================================================
// E51–E52: strkey / arr_del
// ===========================================================================

#[test]
fn e51_strkey_extremes() {
    let b = libs();
    for n in [
        0,
        1,
        -1,
        9,
        -9,
        99999,
        -99999,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
    ] {
        unsafe {
            let c = read_cstr((b.c.strkey)(n));
            let r = read_cstr((b.r.strkey)(n));
            diff_eq!(format!("E51 n={n}"), c.clone(), r);
            assert_eq!(
                c,
                format!("test_{n}").into_bytes(),
                "E51: unexpected formatting"
            );
        }
    }
}

#[test]
fn e52_arr_del_extremes() {
    let b = libs();
    for n in [0, 1, -1, 2, 3, 4, 5, c_int::MAX, c_int::MIN] {
        unsafe {
            (b.c.arr_del)(n);
            (b.r.arr_del)(n);
        }
    }
    // And confirm neither library aborts or crashes on a long run.
    let co = run_in_child(|| unsafe {
        for n in 0..20000 {
            (b.c.arr_del)(n);
        }
    });
    let ro = run_in_child(|| unsafe {
        for n in 0..20000 {
            (b.r.arr_del)(n);
        }
    });
    diff_eq!("E52 outcome", co, ro);
    assert_eq!(co, Outcome::Exited(0));
}

// ===========================================================================
// Generic FFI boundaries (required even though not in ERRORS.md)
// ===========================================================================

#[test]
fn g1_null_pointer_arguments() {
    let b = libs();
    unsafe {
        // arrgrowf(NULL, ..) is the normal bootstrap path.
        let c = (b.c.arrgrowf)(std::ptr::null_mut(), 8, 0, 1);
        let r = (b.r.arrgrowf)(std::ptr::null_mut(), 8, 0, 1);
        diff_eq!("G1 arrgrowf", snap_arr(c), snap_arr(r));
        (b.c.arrfreef)(c);
        (b.r.arrfreef)(r);

        // hmfree_func(NULL, ..)
        (b.c.hmfree_func)(std::ptr::null_mut(), 8);
        (b.r.hmfree_func)(std::ptr::null_mut(), 8);

        // hmput_default(NULL, ..)
        let c = (b.c.hmput_default)(std::ptr::null_mut(), 8);
        let r = (b.r.hmput_default)(std::ptr::null_mut(), 8);
        diff_eq!(
            "G1 hmput_default",
            snap_map(c, 8, KeyRepr::Bytes, 8, 0),
            snap_map(r, 8, KeyRepr::Bytes, 8, 0)
        );
        free_map(&b.c, c, 8);
        free_map(&b.r, r, 8);

        // hmdel_key(NULL, ..) -> NULL
        let mut k = cstr("k");
        let kp = k.as_mut_ptr() as *mut c_void;
        let c = (b.c.hmdel_key)(std::ptr::null_mut(), 8, kp, 1, 0, STBDS_HM_BINARY);
        let r = (b.r.hmdel_key)(std::ptr::null_mut(), 8, kp, 1, 0, STBDS_HM_BINARY);
        diff_eq!("G1 hmdel_key", c.is_null(), r.is_null());
        assert!(c.is_null());

        // hash_bytes(NULL, 0, seed)
        let ch = (b.c.hash_bytes)(std::ptr::null_mut(), 0, 7);
        let rh = (b.r.hash_bytes)(std::ptr::null_mut(), 0, 7);
        diff_eq!("G1 hash_bytes", ch, rh);
    }
}

#[test]
fn g2_zero_lengths() {
    let b = libs();
    unsafe {
        // elemsize 0
        for min_cap in [0usize, 1, 4] {
            let c = (b.c.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            let r = (b.r.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            diff_eq!(format!("G2 arrgrowf es=0 mc={min_cap}"), snap_arr(c), snap_arr(r));
            if (c as usize) >= 4096 {
                (b.c.arrfreef)(c);
                (b.r.arrfreef)(r);
            }
        }
        // hash with len 0 and 1
        let mut buf = [7u8; 8];
        for len in [0usize, 1] {
            let c = (b.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 3);
            let r = (b.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 3);
            diff_eq!(format!("G2 hash len={len}"), c, r);
        }
    }
    // keysize 0 map — see E26.
    reset_seed(&b, 0x3141_5926);
    let mut m = MapPair::new(&b, 8, 0, 4, KeyRepr::Bytes, STBDS_HM_BINARY);
    let mut k = vec![1u8];
    m.put(&mut k, &[0, 0, 0, 0], "G2 keysize0 put");
    m.geti(&mut k, "G2 keysize0 get");
    m.del(&mut k, 0, "G2 keysize0 del");
    m.free();
}

#[test]
fn g3_large_lengths() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 0xA3);
    // Large but in-bounds hash lengths.
    let mut buf = rng.bytes(65536);
    for len in [
        1usize, 63, 64, 65, 1023, 1024, 1025, 4095, 4096, 32767, 65535, 65536,
    ] {
        unsafe {
            let p = buf.as_mut_ptr() as *mut c_void;
            let c = (b.c.hash_bytes)(p, len, 0x3141_5926);
            let r = (b.r.hash_bytes)(p, len, 0x3141_5926);
            diff_eq!(format!("G3 hash_bytes len={len}"), c, r);
        }
    }
    // Long strings.
    let mut s = rng.latin1(65535);
    unsafe {
        let p = s.as_mut_ptr() as *mut c_char;
        let c = (b.c.hash_string)(p, 0x3141_5926);
        let r = (b.r.hash_string)(p, 0x3141_5926);
        diff_eq!("G3 hash_string 65535", c, r);
    }
    // Large elemsize / keysize.
    for elemsize in [256usize, 4096, 65536] {
        reset_seed(&b, 0x3141_5926);
        let mut m = MapPair::new(&b, elemsize, 128, 8, KeyRepr::Bytes, STBDS_HM_BINARY);
        for i in 0..20u64 {
            let mut k = bin_key(i, 128);
            let v = bin_key(i, 8);
            m.put(&mut k, &v, &format!("G3 es={elemsize} put#{i}"));
        }
        for i in 0..20u64 {
            let mut k = bin_key(i, 128);
            assert!(m.geti(&mut k, &format!("G3 es={elemsize} get#{i}")) >= 0);
        }
        m.free();
    }
}

#[test]
fn g4_one_past_valid_range() {
    let b = libs();
    // `mode`: STBDS_HM_BINARY = 0, STBDS_HM_STRING = 1, so 2 and -1 are one
    // step past each end. Covered functionally by E14/E15/E31; here we check the
    // exact sentinels on a miss for each of them.
    for mode in [-1i32, 0, 1, 2] {
        reset_seed(&b, 0x3141_5926);
        let repr = if mode >= 1 { KeyRepr::Ptr } else { KeyRepr::Bytes };
        let mut owned: Vec<Vec<u8>> = (0..8).map(|i| cstr(&format!("g4_{i}"))).collect();
        let mut m = MapPair::new(&b, 16, 8, 8, repr, mode);
        for i in 0..8 {
            let kp: &mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(owned[i].as_mut_ptr(), owned[i].len())
            };
            let v = bin_key(i as u64, 8);
            m.put(kp, &v, &format!("G4 mode={mode} put#{i}"));
        }
        let mut absent = cstr("g4_absent");
        let idx = m.geti_ts(&mut absent, &format!("G4 mode={mode} absent"));
        assert_eq!(idx, -1, "G4 mode={mode}: miss must be -1");
        let t = m.del(&mut absent, 0, &format!("G4 mode={mode} del absent"));
        assert_eq!(t, 0, "G4 mode={mode}: deleting an absent key must be 0");
        m.free();
        drop(owned);
    }

    // `sh_mode`: the enum runs 0..=3, so 4 and -1 are one step past each end.
    for sh in [-1i32, 0, 1, 2, 3, 4] {
        reset_seed(&b, 0x3141_5926);
        unsafe {
            let c = (b.c.shmode_func)(16, sh);
            let r = (b.r.shmode_func)(16, sh);
            let cs = snap_map(c, 16, KeyRepr::Bytes, 8, 0);
            diff_eq!(
                format!("G4 sh={sh}"),
                cs.clone(),
                snap_map(r, 16, KeyRepr::Bytes, 8, 0)
            );
            assert_eq!(
                cs.index.unwrap().arena_mode,
                (sh as u32 & 0xff) as u8,
                "G4: sh_mode must be truncated to unsigned char"
            );
            free_map(&b.c, c, 16);
            free_map(&b.r, r, 16);
        }
    }
}

/// One `(sh_mode, mode)` combination of the G5 sweep, isolated so it can be run
/// in a forked child (many combinations are inherently crash-prone: e.g.
/// `sh_mode = STBDS_SH_NONE` stores the key bytes INLINE while `mode >= 1` makes
/// the lookup `strcmp` those bytes *as a pointer*).
fn g5_scenario(api: &Api, sh: c_int, mode: c_int) {
    unsafe {
        let mut owned: Vec<Vec<u8>> = (0..3).map(|i| cstr(&format!("g5_{i}"))).collect();
        let mut a = (api.shmode_func)(16, sh);
        for i in 0..3 {
            let kp = owned[i].as_mut_ptr() as *mut c_void;
            a = (api.hmput_key)(a, 16, kp, 8, mode);
        }
        let kp = owned[2].as_mut_ptr() as *mut c_void;
        let mut t: isize = 0;
        a = (api.hmget_key_ts)(a, 16, kp, 8, &mut t, mode);
        a = (api.hmdel_key)(a, 16, kp, 8, 0, mode);
        std::hint::black_box(t);
        if !a.is_null() {
            (api.hmfree_func)((a as *mut u8).wrapping_sub(16) as *mut c_void, 16);
        }
        std::mem::forget(owned);
    }
}

#[test]
fn g5_all_out_of_range_enum_values() {
    let b = libs();
    // A C enum accepts any `int`. Sweep a wide set through both `mode` (hash-map
    // comparison mode) and `sh_mode` (string storage mode) and require the two
    // libraries to agree on the outcome of every combination — and, for the
    // combinations that terminate normally, on the full resulting table state.
    let vals: [c_int; 16] = [
        c_int::MIN,
        c_int::MIN + 1,
        -70000,
        -256,
        -255,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        255,
        256,
        70000,
        c_int::MAX,
    ];
    let mut normal = 0usize;
    let mut crashed = 0usize;
    for &sh in &vals {
        for &mode in &vals {
            reset_seed(&b, 0x3141_5926);
            let co = run_in_child(|| g5_scenario(&b.c, sh, mode));
            reset_seed(&b, 0x3141_5926);
            let ro = run_in_child(|| g5_scenario(&b.r, sh, mode));
            diff_eq!(format!("G5 sh={sh} mode={mode} outcome"), co, ro);
            if co != Outcome::Exited(0) {
                crashed += 1;
                continue;
            }
            normal += 1;

            reset_seed(&b, 0x3141_5926);
            unsafe {
                let mut c = (b.c.shmode_func)(16, sh);
                let mut r = (b.r.shmode_func)(16, sh);
                let effective = (sh as u32 & 0xff) as u8;
                let repr = if (1..=3).contains(&effective) {
                    KeyRepr::Ptr
                } else {
                    KeyRepr::Bytes
                };
                let cs = snap_map(c, 16, repr, 8, 0);
                diff_eq!(
                    format!("G5 sh={sh} mode={mode} fresh"),
                    cs.clone(),
                    snap_map(r, 16, repr, 8, 0)
                );
                assert_eq!(
                    cs.index.unwrap().arena_mode,
                    effective,
                    "G5: sh_mode must be truncated to unsigned char"
                );
                let mut owned: Vec<Vec<u8>> =
                    (0..3).map(|i| cstr(&format!("g5_{i}"))).collect();
                for i in 0..3 {
                    let kp = owned[i].as_mut_ptr() as *mut c_void;
                    c = (b.c.hmput_key)(c, 16, kp, 8, mode);
                    r = (b.r.hmput_key)(r, 16, kp, 8, mode);
                    diff_eq!(
                        format!("G5 sh={sh} mode={mode} put#{i}"),
                        snap_map(c, 16, repr, 8, 0),
                        snap_map(r, 16, repr, 8, 0)
                    );
                }
                let kp = owned[2].as_mut_ptr() as *mut c_void;
                let mut ct: isize = 0;
                let mut rt: isize = 0;
                c = (b.c.hmget_key_ts)(c, 16, kp, 8, &mut ct, mode);
                r = (b.r.hmget_key_ts)(r, 16, kp, 8, &mut rt, mode);
                diff_eq!(format!("G5 sh={sh} mode={mode} get temp"), ct, rt);
                c = (b.c.hmdel_key)(c, 16, kp, 8, 0, mode);
                r = (b.r.hmdel_key)(r, 16, kp, 8, 0, mode);
                diff_eq!(
                    format!("G5 sh={sh} mode={mode} del"),
                    snap_map(c, 16, repr, 8, 0),
                    snap_map(r, 16, repr, 8, 0)
                );
                free_map(&b.c, c, 16);
                free_map(&b.r, r, 16);
                drop(owned);
            }
        }
    }
    assert!(
        normal > 0 && crashed > 0,
        "G5: expected a mix of normal and crashing combinations, got {normal}/{crashed}"
    );
    eprintln!("G5: {normal} combinations completed normally, {crashed} crashed identically");
}
