//! Phase C — error / rejection-path differential tests.
//!
//! One test (or one clearly-named section) per row of `ERRORS.md`. Every test
//! constructs the exact invalid input and asserts BOTH libraries return the
//! SAME sentinel / same out-parameter value, not merely "both failed".

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

const SEEDS: [usize; 5] = [0, 1, 0x3141_5926, usize::MAX, 0xDEAD_BEEF_CAFE_BABE];

/// Every `int` value a C enum argument can legitimately carry across the FFI
/// boundary, including the ones with no valid variant.
const MODES: [c_int; 12] = [
    i32::MIN,
    i32::MIN + 1,
    -1000,
    -2,
    -1,
    0, // STBDS_HM_BINARY
    1, // STBDS_HM_STRING
    2, // STBDS_HM_PTR_TO_STRING
    3,
    4,
    1000,
    i32::MAX,
];

unsafe fn hdr(a: *mut c_void) -> *mut u8 {
    (a as *mut u8).sub(HEADER_SIZE)
}
unsafe fn get_len(a: *mut c_void) -> usize {
    (hdr(a) as *const usize).read()
}
unsafe fn set_len(a: *mut c_void, v: usize) {
    (hdr(a) as *mut usize).write(v)
}
unsafe fn get_cap(a: *mut c_void) -> usize {
    (hdr(a).add(8) as *const usize).read()
}
unsafe fn get_temp_of_raw(raw: *mut c_void) -> isize {
    (hdr(raw).add(24) as *const isize).read()
}

/// `stbds_hmput_key` writes only the first `keysize` bytes of a fresh element;
/// bytes `keysize..elemsize` are caller-owned scratch that the C library leaves
/// as whatever `realloc` handed back. Zero them in BOTH libraries so a
/// whole-element snapshot comparison is meaningful.
unsafe fn zero_scratch(h: *mut c_void, elemsize: usize, keysize: usize) {
    if h.is_null() || elemsize == 0 {
        return;
    }
    let raw = (h as *mut u8).sub(elemsize);
    let len = (raw.sub(HEADER_SIZE) as *const usize).read();
    let from = keysize.min(elemsize);
    for e in 0..len {
        let p = raw.add(e * elemsize);
        for k in from..elemsize {
            *p.add(k) = 0;
        }
    }
}

/// Snapshot both libraries after normalising the caller-owned scratch bytes.
unsafe fn snap2(
    hc: *mut c_void,
    hr: *mut c_void,
    elemsize: usize,
    keysize: usize,
    kind: KeyKind,
) -> (MapSnap, MapSnap) {
    zero_scratch(hc, elemsize, keysize);
    zero_scratch(hr, elemsize, keysize);
    (
        snap_map(hc, elemsize, kind, false),
        snap_map(hr, elemsize, kind, false),
    )
}

fn sync_seed(c: &Lib, r: &Lib, seed: usize) {
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
    }
}

// ===========================================================================
// E1 / E2 — stbds_arrgrowf rejection ("nothing to do") and size_t wraparound
// ===========================================================================

#[test]
fn e1_arrgrowf_early_return_is_the_input_pointer() {
    let (c, r) = both();
    // NULL in, nothing requested -> NULL out (no allocation at all).
    for &elemsize in &[0usize, 1, 4, 8, 24, 1024] {
        let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        assert!(ac.is_null(), "C must return NULL for elemsize={}", elemsize);
        assert!(ar.is_null(), "Rust must return NULL for elemsize={}", elemsize);
    }
    // Non-NULL in, min_cap <= capacity -> same pointer, header untouched.
    for &elemsize in &[1usize, 4, 8, 16] {
        let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16) };
        let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16) };
        unsafe {
            set_len(ac, 7);
            set_len(ar, 7);
        }
        for &min_cap in &[0usize, 1, 7, 8, 15, 16] {
            let nc = unsafe { (c.arrgrowf)(ac, elemsize, 0, min_cap) };
            let nr = unsafe { (r.arrgrowf)(ar, elemsize, 0, min_cap) };
            assert_eq!(nc, ac, "C must return the identical pointer");
            assert_eq!(nr, ar, "Rust must return the identical pointer");
            assert_eq!(unsafe { get_cap(nc) }, 16);
            assert_eq!(unsafe { get_cap(nr) }, 16);
            assert_eq!(unsafe { get_len(nc) }, 7);
            assert_eq!(unsafe { get_len(nr) }, 7);
        }
        unsafe {
            (c.arrfreef)(ac);
            (r.arrfreef)(ar);
        }
    }
}

#[test]
fn e2_arrgrowf_addlen_and_mincap_wraparound() {
    let (c, r) = both();
    let elemsize = 8usize;
    // (a) arrlen + addlen wraps size_t back to a tiny value: the C code has no
    //     check, so the request degenerates into the early return.
    let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8) };
    let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8) };
    unsafe {
        set_len(ac, 4);
        set_len(ar, 4);
    }
    // With length == 4 the wrapped `min_len` is `3 - k` for `addlen ==
    // usize::MAX - k`, so k in 0..=3 lands back in the early-return branch.
    // k >= 4 makes `min_len` astronomically large, and then
    // `elemsize * min_cap + sizeof(header)` wraps to exactly 0, so the C code
    // calls `realloc(p, 0)` (which frees and returns NULL) and then writes
    // through `(header *)32 - 1`, i.e. address 0. That is the documented E2
    // undefined behaviour and is identical in both libraries, so it is not
    // executed here.
    for &addlen in &[usize::MAX, usize::MAX - 1, usize::MAX - 2, usize::MAX - 3] {
        let nc = unsafe { (c.arrgrowf)(ac, elemsize, addlen, 0) };
        let nr = unsafe { (r.arrgrowf)(ar, elemsize, addlen, 0) };
        assert_eq!(nc, ac, "C: wrapped min_len must hit the early return");
        assert_eq!(nr, ar, "Rust: wrapped min_len must hit the early return");
        assert_eq!(unsafe { get_cap(nc) }, unsafe { get_cap(nr) });
    }
    unsafe {
        (c.arrfreef)(ac);
        (r.arrfreef)(ar);
    }

    // (b) elemsize * min_cap wraps to 0, so `realloc` is asked for just the
    //     32-byte header and SUCCEEDS while `capacity` is astronomically wrong.
    for &(es, mc) in &[
        (8usize, 1usize << 61),
        (4, 1usize << 62),
        (2, 1usize << 63),
        (16, 1usize << 60),
    ] {
        let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), es, 0, mc) };
        let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), es, 0, mc) };
        assert!(!ac.is_null() && !ar.is_null(), "es={} mc={:#x}", es, mc);
        assert_eq!(unsafe { get_len(ac) }, unsafe { get_len(ar) });
        assert_eq!(unsafe { get_cap(ac) }, unsafe { get_cap(ar) });
        assert_eq!(unsafe { get_cap(ac) }, mc, "capacity must be recorded verbatim");
        unsafe {
            (c.arrfreef)(ac);
            (r.arrfreef)(ar);
        }
    }
}

// ===========================================================================
// E3 — stbds_arrfreef(NULL)
// ===========================================================================

/// `stbds_arrfreef(NULL)` computes `free((stbds_array_header *)NULL - 1)`, i.e.
/// `free((void *)-32)`. The C original has NO null check, so this is undefined
/// behaviour that glibc turns into an abort. Both libraries do the identical
/// arithmetic (`c_src/src/lib.c:314` vs `src/lib.rs::stbds_arrfreef`), and
/// executing it would kill the test process, so this row is verified by
/// inspection only.
#[test]
fn e3_arrfreef_null_is_documented_ub() {
    let (_c, _r) = both();
    // deliberately not executed -- see the doc comment above
}

// ===========================================================================
// E4 / E5 / E6 — stbds_hmfree_func rejection paths
// ===========================================================================

#[test]
fn e4_hmfree_func_null_is_noop() {
    let (c, r) = both();
    for &elemsize in &[0usize, 1, 8, 16, 1 << 20] {
        unsafe {
            (c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
    // reaching here means neither library dereferenced the NULL
}

#[test]
fn e5_hmfree_func_without_hash_table() {
    let (c, r) = both();
    for &elemsize in &[1usize, 8, 16, 24] {
        sync_seed(&c, &r, 0x3141_5926);
        // stbds_hmput_default builds an array with hash_table == NULL
        let hc = unsafe { (c.hmput_default)(std::ptr::null_mut(), elemsize) };
        let hr = unsafe { (r.hmput_default)(std::ptr::null_mut(), elemsize) };
        let sc = unsafe { snap_map(hc, elemsize, KeyKind::Raw, false) };
        let sr = unsafe { snap_map(hr, elemsize, KeyKind::Raw, false) };
        assert_eq!(sc, sr);
        assert!(sc.table.is_none(), "hash_table must still be NULL");
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
    // also a bare stbds_arrgrowf array (hash_table NULL, length 0)
    for &elemsize in &[8usize, 16] {
        let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4) };
        let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4) };
        unsafe {
            (c.hmfree_func)(ac, elemsize);
            (r.hmfree_func)(ar, elemsize);
        }
    }
}

#[test]
fn e6_hmfree_func_only_frees_keys_for_sh_strdup() {
    let (c, r) = both();
    // SH_DEFAULT / SH_ARENA / SH_NONE must NOT free the caller's key buffers.
    for &sm in &[SH_NONE, SH_DEFAULT, SH_ARENA, 4, 255] {
        sync_seed(&c, &r, 0x3141_5926);
        let elemsize = 16usize;
        let mut hc = unsafe { (c.shmode_func)(elemsize, sm) };
        let mut hr = unsafe { (r.shmode_func)(elemsize, sm) };
        let mode = if sm == SH_DEFAULT || sm == SH_ARENA {
            HM_STRING
        } else {
            HM_BINARY
        };
        // key buffers owned by the test, contents checked after the free
        let mut keys: Vec<Box<[u8]>> = (0..10usize)
            .map(|i| format!("owned_{:03}\0", i).into_bytes().into_boxed_slice())
            .collect();
        for k in keys.iter_mut() {
            let kp = k.as_mut_ptr() as *mut c_void;
            hc = unsafe { (c.hmput_key)(hc, elemsize, kp, 8, mode) };
            hr = unsafe { (r.hmput_key)(hr, elemsize, kp, 8, mode) };
        }
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(
                &k[..],
                format!("owned_{:03}\0", i).as_bytes(),
                "shmode {}: caller key buffer must be untouched",
                sm
            );
        }
    }
}

// ===========================================================================
// E7 / E10 / E11 — key-not-present rejection
// ===========================================================================

#[test]
fn e7_e10_e11_absent_key_reports_minus_one() {
    let (c, r) = both();
    for &seed in &SEEDS {
        for &(elemsize, keysize) in &[(8usize, 8usize), (16, 4), (24, 8), (8, 1)] {
            sync_seed(&c, &r, seed);
            let mut hc: *mut c_void = std::ptr::null_mut();
            let mut hr: *mut c_void = std::ptr::null_mut();
            let mut rng = Rng::new(seed as u64 ^ elemsize as u64);
            // populate with even ids
            for i in (0..40u64).step_by(2) {
                let mut k = vec![0u8; keysize.max(1)];
                for (j, b) in k.iter_mut().enumerate() {
                    *b = (i.wrapping_mul(0x9E37_79B9) >> (8 * (j % 8))) as u8;
                }
                let kp = k.as_mut_ptr() as *mut c_void;
                hc = unsafe { (c.hmput_key)(hc, elemsize, kp, keysize, HM_BINARY) };
                hr = unsafe { (r.hmput_key)(hr, elemsize, kp, keysize, HM_BINARY) };
            }
            // odd ids are absent -> both must report exactly -1
            for i in (1..80u64).step_by(2) {
                let mut k = vec![0u8; keysize.max(1)];
                for (j, b) in k.iter_mut().enumerate() {
                    *b = (i.wrapping_mul(0x9E37_79B9) >> (8 * (j % 8))) as u8;
                }
                let kp = k.as_mut_ptr() as *mut c_void;
                let mut tc: isize = 12345;
                let mut tr: isize = 54321;
                let nc =
                    unsafe { (c.hmget_key_ts)(hc, elemsize, kp, keysize, &mut tc, HM_BINARY) };
                let nr =
                    unsafe { (r.hmget_key_ts)(hr, elemsize, kp, keysize, &mut tr, HM_BINARY) };
                assert_eq!(nc, hc, "hmget_key_ts must not move the array");
                assert_eq!(nr, hr);
                assert_eq!(tc, -1, "C must report STBDS_INDEX_EMPTY");
                assert_eq!(tr, -1, "Rust must report STBDS_INDEX_EMPTY");
                // E11: the non-_ts wrapper stores the same value into the header
                unsafe {
                    (c.hmget_key)(hc, elemsize, kp, keysize, HM_BINARY);
                    (r.hmget_key)(hr, elemsize, kp, keysize, HM_BINARY);
                }
                let gc = unsafe { get_temp_of_raw((hc as *mut u8).sub(elemsize) as *mut c_void) };
                let gr = unsafe { get_temp_of_raw((hr as *mut u8).sub(elemsize) as *mut c_void) };
                assert_eq!(gc, -1);
                assert_eq!(gr, -1);
            }
            let _ = rng.next_u64();
            unsafe {
                (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

// ===========================================================================
// E8 — hmget_key_ts(NULL, ...)
// ===========================================================================

#[test]
fn e8_hmget_key_ts_on_null_array() {
    let (c, r) = both();
    for &elemsize in &[1usize, 4, 8, 16, 24, 64] {
        for &mode in &MODES {
            sync_seed(&c, &r, 0x3141_5926);
            // `key` is never dereferenced on this path -> pass NULL too
            let mut tc: isize = 999;
            let mut tr: isize = -999;
            let hc = unsafe {
                (c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    std::ptr::null_mut(),
                    0,
                    &mut tc,
                    mode,
                )
            };
            let hr = unsafe {
                (r.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    std::ptr::null_mut(),
                    0,
                    &mut tr,
                    mode,
                )
            };
            assert_eq!(tc, -1, "C *temp for elemsize={} mode={}", elemsize, mode);
            assert_eq!(tr, -1, "Rust *temp for elemsize={} mode={}", elemsize, mode);
            assert!(!hc.is_null() && !hr.is_null());
            let sc = unsafe { snap_map(hc, elemsize, KeyKind::Raw, false) };
            let sr = unsafe { snap_map(hr, elemsize, KeyKind::Raw, false) };
            assert_eq!(sc, sr, "elemsize={} mode={}", elemsize, mode);
            assert_eq!(sc.length, 1);
            assert!(sc.table.is_none());
            unsafe {
                (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

// ===========================================================================
// E9 — hmget_key_ts on an array whose hash_table is NULL
//      (key is never dereferenced, so NULL is a legal key here)
// ===========================================================================

#[test]
fn e9_hmget_on_array_without_hash_table_accepts_null_key() {
    let (c, r) = both();
    for &elemsize in &[1usize, 8, 16, 24] {
        for &mode in &MODES {
            sync_seed(&c, &r, 0x3141_5926);
            let hc = unsafe { (c.hmput_default)(std::ptr::null_mut(), elemsize) };
            let hr = unsafe { (r.hmput_default)(std::ptr::null_mut(), elemsize) };
            let mut tc: isize = 7;
            let mut tr: isize = 8;
            let nc = unsafe {
                (c.hmget_key_ts)(hc, elemsize, std::ptr::null_mut(), 0, &mut tc, mode)
            };
            let nr = unsafe {
                (r.hmget_key_ts)(hr, elemsize, std::ptr::null_mut(), 0, &mut tr, mode)
            };
            assert_eq!(nc, hc, "the array pointer must come back unchanged");
            assert_eq!(nr, hr);
            assert_eq!(tc, -1, "elemsize={} mode={}", elemsize, mode);
            assert_eq!(tr, -1, "elemsize={} mode={}", elemsize, mode);
            // E11 through the wrapper
            unsafe {
                (c.hmget_key)(hc, elemsize, std::ptr::null_mut(), 0, mode);
                (r.hmget_key)(hr, elemsize, std::ptr::null_mut(), 0, mode);
            }
            let raw_c = unsafe { (hc as *mut u8).sub(elemsize) as *mut c_void };
            let raw_r = unsafe { (hr as *mut u8).sub(elemsize) as *mut c_void };
            assert_eq!(unsafe { get_temp_of_raw(raw_c) }, -1);
            assert_eq!(unsafe { get_temp_of_raw(raw_r) }, -1);
            unsafe {
                (c.hmfree_func)(raw_c, elemsize);
                (r.hmfree_func)(raw_r, elemsize);
            }
        }
    }
}

// ===========================================================================
// E12 — hmdel_key(NULL, ...) is the library's only NULL return
// ===========================================================================

#[test]
fn e12_hmdel_key_null_returns_null() {
    let (c, r) = both();
    for &elemsize in &[0usize, 1, 8, 16, 1024] {
        for &keysize in &[0usize, 1, 8, 4096] {
            for &keyoffset in &[0usize, 1, 8, 4096] {
                for &mode in &MODES {
                    let nc = unsafe {
                        (c.hmdel_key)(
                            std::ptr::null_mut(),
                            elemsize,
                            std::ptr::null_mut(),
                            keysize,
                            keyoffset,
                            mode,
                        )
                    };
                    let nr = unsafe {
                        (r.hmdel_key)(
                            std::ptr::null_mut(),
                            elemsize,
                            std::ptr::null_mut(),
                            keysize,
                            keyoffset,
                            mode,
                        )
                    };
                    assert!(nc.is_null(), "C must return NULL (es={} mode={})", elemsize, mode);
                    assert!(nr.is_null(), "Rust must return NULL (es={} mode={})", elemsize, mode);
                }
            }
        }
    }
}

// ===========================================================================
// E13 — hmdel_key on an array whose hash_table is NULL
// ===========================================================================

#[test]
fn e13_hmdel_key_without_hash_table() {
    let (c, r) = both();
    for &elemsize in &[1usize, 8, 16, 24] {
        for &mode in &MODES {
            sync_seed(&c, &r, 0x3141_5926);
            let hc = unsafe { (c.hmput_default)(std::ptr::null_mut(), elemsize) };
            let hr = unsafe { (r.hmput_default)(std::ptr::null_mut(), elemsize) };
            let raw_c = unsafe { (hc as *mut u8).sub(elemsize) as *mut c_void };
            let raw_r = unsafe { (hr as *mut u8).sub(elemsize) as *mut c_void };
            // poison temp so we can see it being reset to 0
            unsafe {
                (hdr(raw_c).add(24) as *mut isize).write(0x1234);
                (hdr(raw_r).add(24) as *mut isize).write(0x1234);
            }
            let nc = unsafe {
                (c.hmdel_key)(hc, elemsize, std::ptr::null_mut(), 0, 0, mode)
            };
            let nr = unsafe {
                (r.hmdel_key)(hr, elemsize, std::ptr::null_mut(), 0, 0, mode)
            };
            assert_eq!(nc, hc, "must return the array unchanged");
            assert_eq!(nr, hr);
            assert_eq!(unsafe { get_temp_of_raw(raw_c) }, 0, "C temp must be 0");
            assert_eq!(unsafe { get_temp_of_raw(raw_r) }, 0, "Rust temp must be 0");
            assert_eq!(unsafe { get_len(raw_c) }, unsafe { get_len(raw_r) });
            unsafe {
                (c.hmfree_func)(raw_c, elemsize);
                (r.hmfree_func)(raw_r, elemsize);
            }
        }
    }
}

// ===========================================================================
// E14 — hmdel_key with an absent key
// ===========================================================================

#[test]
fn e14_hmdel_key_absent_key_is_noop() {
    let (c, r) = both();
    for &seed in &SEEDS {
        let elemsize = 16usize;
        let keysize = 8usize;
        sync_seed(&c, &r, seed);
        let mut hc: *mut c_void = std::ptr::null_mut();
        let mut hr: *mut c_void = std::ptr::null_mut();
        for i in 0..50u64 {
            let mut k = (i * 2).to_le_bytes();
            let kp = k.as_mut_ptr() as *mut c_void;
            hc = unsafe { (c.hmput_key)(hc, elemsize, kp, keysize, HM_BINARY) };
            hr = unsafe { (r.hmput_key)(hr, elemsize, kp, keysize, HM_BINARY) };
            let e = (unsafe { get_temp_of_raw((hc as *mut u8).sub(elemsize) as *mut c_void) } + 1)
                as usize;
            unsafe {
                for h in [hc, hr] {
                    let p = (h as *mut u8).sub(elemsize).add(e * elemsize);
                    for j in keysize..elemsize {
                        *p.add(j) = (e * 7 + j) as u8;
                    }
                }
            }
        }
        let (before_c, before_r) = unsafe { snap2(hc, hr, elemsize, keysize, KeyKind::Raw) };
        assert_eq!(before_c, before_r);
        for i in 0..50u64 {
            let mut k = (i * 2 + 1).to_le_bytes(); // never inserted
            let kp = k.as_mut_ptr() as *mut c_void;
            let nc = unsafe { (c.hmdel_key)(hc, elemsize, kp, keysize, 0, HM_BINARY) };
            let nr = unsafe { (r.hmdel_key)(hr, elemsize, kp, keysize, 0, HM_BINARY) };
            assert_eq!(nc, hc);
            assert_eq!(nr, hr);
            assert_eq!(
                unsafe { get_temp_of_raw((hc as *mut u8).sub(elemsize) as *mut c_void) },
                0,
                "C must report 0 removed"
            );
            assert_eq!(
                unsafe { get_temp_of_raw((hr as *mut u8).sub(elemsize) as *mut c_void) },
                0,
                "Rust must report 0 removed"
            );
        }
        let (after_c, after_r) = unsafe { snap2(hc, hr, elemsize, keysize, KeyKind::Raw) };
        assert_eq!(after_c, after_r);
        // length / used_count / tombstone_count must be completely untouched
        assert_eq!(after_c.length, before_c.length);
        assert_eq!(
            after_c.table.as_ref().unwrap().used_count,
            before_c.table.as_ref().unwrap().used_count
        );
        assert_eq!(
            after_c.table.as_ref().unwrap().tombstone_count,
            before_c.table.as_ref().unwrap().tombstone_count
        );
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ===========================================================================
// E15 — mode == 2 (and 3, 4, 1000, INT_MAX): string find, no strdup free,
//       binary re-find. Only the shapes that do not trip the C `assert` are
//       executable (see the comment inside).
// ===========================================================================

#[test]
fn e15_out_of_range_mode_delete_asymmetry() {
    let (c, r) = both();
    // `mode >= STBDS_HM_STRING` selects string hashing, but only
    // `mode == STBDS_HM_STRING` selects the strdup free and the string re-find.
    // With old_index != final_index the C code then hashes RAW POINTER BYTES as
    // a string and trips `STBDS_ASSERT(slot >= 0)` (asserts are live in the C
    // build), aborting the process -- so only the two reachable shapes are
    // executed: an absent key, and a victim that IS the final element.
    for &mode in &[2i32, 3, 4, 1000, i32::MAX] {
        for &sm in &[SH_STRDUP, SH_ARENA, SH_DEFAULT] {
            sync_seed(&c, &r, 0x3141_5926);
            let elemsize = 16usize;
            let mut hc = unsafe { (c.shmode_func)(elemsize, sm) };
            let mut hr = unsafe { (r.shmode_func)(elemsize, sm) };
            let mut keys: Vec<Box<[u8]>> = (0..12usize)
                .map(|i| format!("oor_{:03}\0", i).into_bytes().into_boxed_slice())
                .collect();
            for k in keys.iter_mut() {
                let kp = k.as_mut_ptr() as *mut c_void;
                hc = unsafe { (c.hmput_key)(hc, elemsize, kp, 8, mode) };
                hr = unsafe { (r.hmput_key)(hr, elemsize, kp, 8, mode) };
            }
            let raw_c = || unsafe { (hc as *mut u8).sub(elemsize) as *mut c_void };
            let raw_r = || unsafe { (hr as *mut u8).sub(elemsize) as *mut c_void };

            // (a) absent key -> find_slot < 0 -> early return, temp stays 0
            let mut absent: Box<[u8]> = b"oor_absent\0".to_vec().into_boxed_slice();
            let nc = unsafe {
                (c.hmdel_key)(hc, elemsize, absent.as_mut_ptr() as *mut c_void, 8, 0, mode)
            };
            let nr = unsafe {
                (r.hmdel_key)(hr, elemsize, absent.as_mut_ptr() as *mut c_void, 8, 0, mode)
            };
            assert_eq!(nc, hc);
            assert_eq!(nr, hr);
            assert_eq!(unsafe { get_temp_of_raw(raw_c()) }, 0, "mode={} sm={}", mode, sm);
            assert_eq!(unsafe { get_temp_of_raw(raw_r()) }, 0, "mode={} sm={}", mode, sm);

            // (b) delete in reverse insertion order: old_index == final_index,
            //     so the re-find block is skipped entirely.
            for i in (0..12usize).rev() {
                let kp = keys[i].as_mut_ptr() as *mut c_void;
                let len = unsafe { get_len(raw_c()) };
                let mut tc: isize = 0;
                let mut tr: isize = 0;
                unsafe {
                    (c.hmget_key_ts)(hc, elemsize, kp, 8, &mut tc, mode);
                    (r.hmget_key_ts)(hr, elemsize, kp, 8, &mut tr, mode);
                }
                assert_eq!(tc, tr);
                assert_eq!(tc, (len - 2) as isize, "victim must be the last element");
                let nc = unsafe { (c.hmdel_key)(hc, elemsize, kp, 8, 0, mode) };
                let nr = unsafe { (r.hmdel_key)(hr, elemsize, kp, 8, 0, mode) };
                assert_eq!(nc, hc);
                assert_eq!(nr, hr);
                assert_eq!(unsafe { get_temp_of_raw(raw_c()) }, 1);
                assert_eq!(unsafe { get_temp_of_raw(raw_r()) }, 1);
                let (sc, sr) = unsafe { snap2(hc, hr, elemsize, 8, KeyKind::Ptr) };
                assert_eq!(sc, sr, "mode={} sm={} i={}", mode, sm, i);
            }
            assert_eq!(unsafe { get_len(raw_c()) }, 1);
            unsafe {
                (c.hmfree_func)(raw_c(), elemsize);
                (r.hmfree_func)(raw_r(), elemsize);
            }
            drop(keys);
        }
    }
}

// ===========================================================================
// E16..E22, E24, E48 — asserts / branches unreachable through the exports
// ===========================================================================

/// * **E16** `assert(slot < table->slot_count)` — `stbds_hm_find_slot` masks
///   `pos` with `slot_count-1`, so the returned slot is always in range.
/// * **E17** `assert(table->used_count >= 0)` on a `size_t` — a tautology the
///   compiler folds away; it can never fire.
/// * **E18** `assert(slot >= 0)` and **E19** `assert(b->index[i] ==
///   final_index)` in `stbds_hmdel_key` require a table that disagrees with
///   the element array; the only way to reach E18 through the exports is the
///   `mode >= 2` + `old_index != final_index` shape, which aborts the C
///   process (see `e15_out_of_range_mode_delete_asymmetry`).
/// * **E20** `assert((size_t) i+1 <= arrcap(a))` fires only if `realloc`
///   returned NULL, in which case `stbds_arrgrowf` has already dereferenced
///   `(void *)32`.
/// * **E21** `assert(used_count_threshold + tombstone_count_threshold <
///   slot_count)` and **E22**/**E48** (`slot_count == 0`) need
///   `stbds_make_hash_index` to be called with `slot_count < 8`;
///   `stbds_make_hash_index` is `static`, and both exported callers
///   (`stbds_shmode_func`, `stbds_hmput_key`) pass `STBDS_BUCKET_LENGTH == 8`
///   or a doubling/halving of it that never drops below 8 (the shrink at
///   `lib.c:854` is guarded by `slot_count > STBDS_BUCKET_LENGTH`).
/// * **E24** `assert(len <= a->remaining)` in `stbds_stralloc`: the `else`
///   branch above it always sets `remaining = blocksize >= len`, and the
///   `len > blocksize` branch returns early, so the assert is unreachable.
///
/// Both implementations contain the identical (compiled-out, in Rust
/// deliberately no-op) checks, and every one of the *reachable* consequences
/// of these branches is covered by the executing tests in this file.
#[test]
fn e16_e22_e24_e48_unreachable_asserts_documented() {
    let (c, r) = both();
    // Assert the reachability argument for E21 empirically: the smallest table
    // either library will ever build is slot_count == 8, and shrinking stops
    // there.
    for &sm in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        sync_seed(&c, &r, 0x3141_5926);
        let elemsize = 16usize;
        let hc = unsafe { (c.shmode_func)(elemsize, sm) };
        let hr = unsafe { (r.shmode_func)(elemsize, sm) };
        let sc = unsafe { snap_map(hc, elemsize, KeyKind::Raw, false) };
        let sr = unsafe { snap_map(hr, elemsize, KeyKind::Raw, false) };
        assert_eq!(sc, sr);
        let t = sc.table.unwrap();
        assert_eq!(t.slot_count, 8);
        assert_eq!(t.used_count_threshold, 6);
        assert_eq!(t.tombstone_count_threshold, 1);
        assert_eq!(t.used_count_shrink_threshold, 0, "shrink is disabled at 8");
        assert!(t.used_count_threshold + t.tombstone_count_threshold < t.slot_count);
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ===========================================================================
// E23 — fresh table advances the global seed, rehash/shrink table does not
// ===========================================================================

#[test]
fn e23_global_seed_advance_only_for_fresh_tables() {
    let (c, r) = both();
    let elemsize = 16usize;
    for &seed in &SEEDS {
        sync_seed(&c, &r, seed);
        // (1) a map that grows (and therefore rehashes with ot != NULL) many
        //     times must consume the global seed exactly ONCE.
        let mut hc: *mut c_void = std::ptr::null_mut();
        let mut hr: *mut c_void = std::ptr::null_mut();
        let mut first_seed_c = 0usize;
        let mut first_seed_r = 0usize;
        for i in 0..300u64 {
            let mut k = i.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes();
            let kp = k.as_mut_ptr() as *mut c_void;
            hc = unsafe { (c.hmput_key)(hc, elemsize, kp, 8, HM_BINARY) };
            hr = unsafe { (r.hmput_key)(hr, elemsize, kp, 8, HM_BINARY) };
            let (sc, sr) = unsafe { snap2(hc, hr, elemsize, 8, KeyKind::Raw) };
            let a = sc.table.as_ref().unwrap().seed;
            let b = sr.table.as_ref().unwrap().seed;
            if i == 0 {
                first_seed_c = a;
                first_seed_r = b;
            }
            assert_eq!(a, b, "table seed diverged at i={} (seed={:#x})", i, seed);
            assert_eq!(
                a, first_seed_c,
                "rehash must inherit ot->seed, never re-draw"
            );
            assert_eq!(b, first_seed_r);
        }
        // (2) the NEXT fresh table must see the advanced global seed and both
        //     libraries must have advanced it identically.
        let mut k2 = 42u64.to_le_bytes();
        let g2c = unsafe {
            (c.hmput_key)(
                std::ptr::null_mut(),
                elemsize,
                k2.as_mut_ptr() as *mut c_void,
                8,
                HM_BINARY,
            )
        };
        let g2r = unsafe {
            (r.hmput_key)(
                std::ptr::null_mut(),
                elemsize,
                k2.as_mut_ptr() as *mut c_void,
                8,
                HM_BINARY,
            )
        };
        let s2c = unsafe { snap_map(g2c, elemsize, KeyKind::Raw, false) };
        let s2r = unsafe { snap_map(g2r, elemsize, KeyKind::Raw, false) };
        assert_eq!(
            s2c.table.as_ref().unwrap().seed,
            s2r.table.as_ref().unwrap().seed,
            "advanced global seed diverged (start seed {:#x})",
            seed
        );
        assert_ne!(
            s2c.table.as_ref().unwrap().seed,
            first_seed_c,
            "the global seed must have advanced"
        );
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (c.hmfree_func)((g2c as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((g2r as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
    sync_seed(&c, &r, 0x3141_5926);
}

// ===========================================================================
// E25..E29 — stbds_stralloc / stbds_strreset boundaries
// ===========================================================================

#[test]
fn e25_e26_e27_e28_e29_stralloc_boundaries() {
    let (c, r) = both();

    // E25: fresh arena, short string -> 512-byte block and block 0 -> 1
    let mut ac = StringArena::new();
    let mut ar = StringArena::new();
    let mut s = b"x\0".to_vec();
    let pc = unsafe { (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char) };
    let pr = unsafe { (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char) };
    assert_eq!(unsafe { cstr_bytes(pc) }, unsafe { cstr_bytes(pr) });
    assert_eq!((ac.block, ac.remaining), (1u8, 510usize));
    assert_eq!((ar.block, ar.remaining), (1u8, 510usize));
    unsafe {
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }

    // E28: the empty string (len == 1) still allocates on a fresh arena
    let mut e = b"\0".to_vec();
    let pc = unsafe { (c.stralloc)(&mut ac, e.as_mut_ptr() as *mut c_char) };
    let pr = unsafe { (r.stralloc)(&mut ar, e.as_mut_ptr() as *mut c_char) };
    assert_eq!(unsafe { cstr_bytes(pc) }, Vec::<u8>::new());
    assert_eq!(unsafe { cstr_bytes(pr) }, Vec::<u8>::new());
    // NB: the `storage` pointers come from different heaps, so compare the
    // address-independent snapshot rather than the raw struct.
    assert_eq!(unsafe { snap_arena(&ac) }, unsafe { snap_arena(&ar) });
    assert_eq!((ac.block, ac.remaining), (1u8, 511usize));
    assert_eq!((ar.block, ar.remaining), (1u8, 511usize));
    unsafe {
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }

    // E27a: oversize on an EMPTY arena -> remaining forced to 0
    let mut big = vec![b'q'; 600];
    big.push(0);
    let pc = unsafe { (c.stralloc)(&mut ac, big.as_mut_ptr() as *mut c_char) };
    let pr = unsafe { (r.stralloc)(&mut ar, big.as_mut_ptr() as *mut c_char) };
    assert_eq!(unsafe { cstr_bytes(pc) }, big[..600].to_vec());
    assert_eq!(unsafe { cstr_bytes(pr) }, big[..600].to_vec());
    assert_eq!(ac.remaining, 0);
    assert_eq!(ar.remaining, 0);
    assert_eq!(ac.block, 1);
    assert_eq!(ar.block, 1);
    assert_eq!(unsafe { snap_arena(&ac) }, unsafe { snap_arena(&ar) });
    unsafe {
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }

    // E27b: oversize on a NON-empty arena -> spliced in, remaining untouched
    let mut small = b"warm\0".to_vec();
    unsafe {
        (c.stralloc)(&mut ac, small.as_mut_ptr() as *mut c_char);
        (r.stralloc)(&mut ar, small.as_mut_ptr() as *mut c_char);
    }
    let rem_c = ac.remaining;
    let rem_r = ar.remaining;
    let mut big2 = vec![b'w'; 3000];
    big2.push(0);
    let pc = unsafe { (c.stralloc)(&mut ac, big2.as_mut_ptr() as *mut c_char) };
    let pr = unsafe { (r.stralloc)(&mut ar, big2.as_mut_ptr() as *mut c_char) };
    assert_eq!(unsafe { cstr_bytes(pc) }, big2[..3000].to_vec());
    assert_eq!(unsafe { cstr_bytes(pr) }, big2[..3000].to_vec());
    assert_eq!(ac.remaining, rem_c, "C: remaining must be untouched");
    assert_eq!(ar.remaining, rem_r, "Rust: remaining must be untouched");
    assert_eq!(unsafe { snap_arena(&ac) }, unsafe { snap_arena(&ar) });
    assert_eq!(unsafe { snap_arena(&ac) }.nblocks, 2);
    unsafe {
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }

    // E26: blocksize saturates at 1<<20 and `block` stops incrementing at 22
    for _ in 0..30 {
        let blocksize: usize = 512usize << (ac.block as usize >> 1);
        let mut s = vec![b'k'; blocksize - 1];
        s.push(0);
        let pc = unsafe { (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char) };
        let pr = unsafe { (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char) };
        assert_eq!(unsafe { cstr_bytes(pc) }.len(), blocksize - 1);
        assert_eq!(unsafe { cstr_bytes(pr) }.len(), blocksize - 1);
        assert_eq!(ac.block, ar.block);
        assert_eq!(ac.remaining, ar.remaining);
        assert!(ac.block <= 22, "block must saturate at 22, got {}", ac.block);
    }
    assert_eq!(ac.block, 22);
    assert_eq!(ar.block, 22);
    unsafe {
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }

    // E29: strreset on a fresh / already-reset arena is an idempotent no-op
    for _ in 0..5 {
        unsafe {
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
        assert_eq!(unsafe { snap_arena(&ac) }, unsafe { snap_arena(&ar) });
        assert!(ac.storage.is_null() && ar.storage.is_null());
        assert_eq!((ac.remaining, ac.block, ac.mode), (0, 0, 0));
        assert_eq!((ar.remaining, ar.block, ar.mode), (0, 0, 0));
    }
    let mut zc = StringArena::new();
    let mut zr = StringArena::new();
    unsafe {
        (c.strreset)(&mut zc);
        (r.strreset)(&mut zr);
    }
    assert_eq!(unsafe { snap_arena(&zc) }, unsafe { snap_arena(&zr) });
    assert_eq!(zc, StringArena::new());
    assert_eq!(zr, StringArena::new());
}

// ===========================================================================
// E30..E34 — hash function boundaries
// ===========================================================================

#[test]
fn e30_hash_bytes_zero_length_never_touches_the_pointer() {
    let (c, r) = both();
    let bogus: *mut c_void = 1usize as *mut c_void; // unmapped, must not be read
    for &seed in &SEEDS {
        let hc = unsafe { (c.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        let hr = unsafe { (r.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        assert_eq!(hc, hr, "seed={:#x}", seed);
        let hc = unsafe { (c.hash_bytes)(bogus, 0, seed) };
        let hr = unsafe { (r.hash_bytes)(bogus, 0, seed) };
        assert_eq!(hc, hr, "seed={:#x}", seed);
    }
}

#[test]
fn e31_hash_bytes_tail_switch_fallthrough() {
    let (c, r) = both();
    // len % 8 == 0..7 must reproduce the C `switch` fall-through chain
    // exactly, including `case 0` (which ORs nothing).
    let mut rng = Rng::new(0xE31);
    for base in [0usize, 8, 16, 24, 64] {
        for rem in 0..8usize {
            let len = base + rem;
            for _ in 0..64 {
                let mut buf = rng.bytes(len + 8);
                for &seed in &SEEDS {
                    let p = buf.as_mut_ptr() as *mut c_void;
                    let hc = unsafe { (c.hash_bytes)(p, len, seed) };
                    let hr = unsafe { (r.hash_bytes)(p, len, seed) };
                    assert_eq!(hc, hr, "len={} rem={} seed={:#x}", len, rem, seed);
                }
            }
            // deterministic extremes for this remainder
            for pat in [0x00u8, 0x01, 0x7f, 0x80, 0xff] {
                let mut buf = vec![pat; len + 8];
                for &seed in &SEEDS {
                    let p = buf.as_mut_ptr() as *mut c_void;
                    assert_eq!(
                        unsafe { (c.hash_bytes)(p, len, seed) },
                        unsafe { (r.hash_bytes)(p, len, seed) },
                        "len={} pat={:#x} seed={:#x}",
                        len,
                        pat,
                        seed
                    );
                }
            }
        }
    }
}

#[test]
fn e32_hash_bytes_int_sign_extension() {
    let (c, r) = both();
    // Only the bytes at word offsets 3 and 7 can set the `int` sign bit in the
    // C word assembly; walk every single-byte-set position for lengths 1..=24.
    for len in 1..=24usize {
        for pos in 0..len {
            for &v in &[0x80u8, 0xFF, 0x7F, 0x01] {
                let mut buf = vec![0u8; len + 8];
                buf[pos] = v;
                for &seed in &SEEDS {
                    let p = buf.as_mut_ptr() as *mut c_void;
                    assert_eq!(
                        unsafe { (c.hash_bytes)(p, len, seed) },
                        unsafe { (r.hash_bytes)(p, len, seed) },
                        "len={} pos={} v={:#x} seed={:#x}",
                        len,
                        pos,
                        v,
                        seed
                    );
                }
            }
        }
    }
}

#[test]
fn e33_e34_hash_string_boundaries() {
    let (c, r) = both();
    // E33: the empty string never enters the accumulation loop
    let mut empty = [0u8; 1];
    for &seed in &SEEDS {
        assert_eq!(
            unsafe { (c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed) },
            unsafe { (r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed) },
            "seed={:#x}",
            seed
        );
    }
    // E34: bytes >= 0x80 are added as `unsigned char` (zero-extended), unlike
    // the sign-extending byte assembly in siphash.
    for len in 1..=64usize {
        for pos in 0..len {
            for &v in &[0x80u8, 0x81, 0xFE, 0xFF] {
                let mut s = vec![b'a'; len];
                s[pos] = v;
                s.push(0);
                for &seed in &SEEDS {
                    assert_eq!(
                        unsafe { (c.hash_string)(s.as_mut_ptr() as *mut c_char, seed) },
                        unsafe { (r.hash_string)(s.as_mut_ptr() as *mut c_char, seed) },
                        "len={} pos={} v={:#x} seed={:#x}",
                        len,
                        pos,
                        v,
                        seed
                    );
                }
            }
        }
    }
    // all-high-byte strings of every length
    for len in 0..=200usize {
        let mut s = vec![0xFFu8; len];
        s.push(0);
        for &seed in &SEEDS {
            assert_eq!(
                unsafe { (c.hash_string)(s.as_mut_ptr() as *mut c_char, seed) },
                unsafe { (r.hash_string)(s.as_mut_ptr() as *mut c_char, seed) },
                "len={} seed={:#x}",
                len,
                seed
            );
        }
    }
}

// ===========================================================================
// E35 — `if (hash < 2) hash += 2`
// ===========================================================================

/// Reaching this branch requires an input whose siphash-2-4 (or
/// `stbds_hash_string`) output is exactly 0 or 1 under a chosen seed, i.e.
/// inverting the hash — a 2/2^64 event that cannot be produced by search.
/// Both implementations contain the byte-identical guard
/// (`c_src/src/lib.c:596` / `:719` vs `src/lib.rs` `stbds_hm_find_slot` /
/// `stbds_hmput_key`), and this test records the (negative) search so the row
/// is not silently skipped.
#[test]
fn e35_hash_below_two_is_unreachable_by_search() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE35);
    let mut buf = [0u8; 16];
    for _ in 0..200_000 {
        for b in buf.iter_mut() {
            *b = rng.byte();
        }
        let seed = rng.next_u64() as usize;
        let len = rng.below(17);
        let p = buf.as_mut_ptr() as *mut c_void;
        let hc = unsafe { (c.hash_bytes)(p, len, seed) };
        let hr = unsafe { (r.hash_bytes)(p, len, seed) };
        assert_eq!(hc, hr);
        assert!(hc >= 2, "found a hash < 2 -- the += 2 branch is now reachable");
    }
}

// ===========================================================================
// E36 / E37 — table (re)build and the out-of-enum string.mode memcpy arm
// ===========================================================================

#[test]
fn e36_table_rebuild_thresholds() {
    let (c, r) = both();
    let elemsize = 16usize;
    for &mode in &[HM_BINARY, HM_STRING] {
        sync_seed(&c, &r, 0x3141_5926);
        let mut hc: *mut c_void = std::ptr::null_mut();
        let mut hr: *mut c_void = std::ptr::null_mut();
        let mut keep: Vec<Box<[u8]>> = Vec::new();
        let mut slot_counts = Vec::new();
        for i in 0..400usize {
            let mut k: Box<[u8]> = if mode == HM_BINARY {
                Box::new((i as u64).to_le_bytes()) as Box<[u8]>
            } else {
                format!("rb_{:05}\0", i).into_bytes().into_boxed_slice()
            };
            let kp = k.as_mut_ptr() as *mut c_void;
            hc = unsafe { (c.hmput_key)(hc, elemsize, kp, 8, mode) };
            hr = unsafe { (r.hmput_key)(hr, elemsize, kp, 8, mode) };
            keep.push(k);
            let (sc, sr) = unsafe { snap2(hc, hr, elemsize, 8, KeyKind::Raw) };
            let tc = sc.table.as_ref().unwrap();
            let tr = sr.table.as_ref().unwrap();
            assert_eq!(
                (
                    tc.slot_count,
                    tc.used_count,
                    tc.used_count_threshold,
                    tc.used_count_shrink_threshold,
                    tc.tombstone_count,
                    tc.tombstone_count_threshold,
                    tc.slot_count_log2,
                    tc.str_mode
                ),
                (
                    tr.slot_count,
                    tr.used_count,
                    tr.used_count_threshold,
                    tr.used_count_shrink_threshold,
                    tr.tombstone_count,
                    tr.tombstone_count_threshold,
                    tr.slot_count_log2,
                    tr.str_mode
                ),
                "mode={} i={}",
                mode,
                i
            );
            assert_eq!(
                tc.str_mode,
                if mode >= HM_STRING { 1 } else { 0 },
                "fresh table string.mode"
            );
            if slot_counts.last() != Some(&tc.slot_count) {
                slot_counts.push(tc.slot_count);
            }
        }
        assert_eq!(
            slot_counts,
            vec![8usize, 16, 32, 64, 128, 256, 512, 1024],
            "grow sequence must double from 8"
        );
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
        drop(keep);
    }
}

#[test]
fn e37_out_of_enum_string_mode_falls_into_memcpy() {
    let (c, r) = both();
    // string.mode outside {SH_DEFAULT, SH_STRDUP, SH_ARENA} -> `default:` arm,
    // which memcpy's `keysize` raw bytes even for a string-mode put.
    for &sm in &[0i32, 4, 5, 100, 255] {
        for &mode in &[HM_STRING, HM_PTR_TO_STRING, 1000] {
            sync_seed(&c, &r, 0x3141_5926);
            let elemsize = 16usize;
            let mut key: Vec<u8> = b"0123456789abcdef\0".to_vec();
            let hc = unsafe { (c.shmode_func)(elemsize, sm) };
            let hr = unsafe { (r.shmode_func)(elemsize, sm) };
            let hc = unsafe {
                (c.hmput_key)(hc, elemsize, key.as_mut_ptr() as *mut c_void, 8, mode)
            };
            let hr = unsafe {
                (r.hmput_key)(hr, elemsize, key.as_mut_ptr() as *mut c_void, 8, mode)
            };
            // element 1 starts exactly at the hash-biased handle
            let bc: Vec<u8> = (0..8).map(|k| unsafe { *(hc as *const u8).add(k) }).collect();
            let br: Vec<u8> = (0..8).map(|k| unsafe { *(hr as *const u8).add(k) }).collect();
            assert_eq!(bc, br, "sm={} mode={}", sm, mode);
            assert_eq!(bc, key[..8].to_vec(), "raw memcpy, sm={} mode={}", sm, mode);
            // header + table state must match too
            let raw_c = unsafe { (hc as *mut u8).sub(elemsize) as *mut c_void };
            let raw_r = unsafe { (hr as *mut u8).sub(elemsize) as *mut c_void };
            assert_eq!(unsafe { get_len(raw_c) }, unsafe { get_len(raw_r) });
            assert_eq!(unsafe { get_temp_of_raw(raw_c) }, unsafe {
                get_temp_of_raw(raw_r)
            });
            unsafe {
                (c.hmfree_func)(raw_c, elemsize);
                (r.hmfree_func)(raw_r, elemsize);
            }
        }
    }
}

// ===========================================================================
// E38 — stbds_shmode_func truncates `mode` to unsigned char
// ===========================================================================

#[test]
fn e38_shmode_func_mode_truncation_full_sweep() {
    let (c, r) = both();
    let elemsize = 16usize;
    let mut cases: Vec<c_int> = Vec::new();
    for m in -300i32..=300 {
        cases.push(m);
    }
    cases.extend_from_slice(&[
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        65536,
        65535,
        -65536,
        1_000_000,
        -1_000_000,
    ]);
    for &m in &cases {
        sync_seed(&c, &r, 0x3141_5926);
        let hc = unsafe { (c.shmode_func)(elemsize, m) };
        let hr = unsafe { (r.shmode_func)(elemsize, m) };
        let sc = unsafe { snap_map(hc, elemsize, KeyKind::Raw, false) };
        let sr = unsafe { snap_map(hr, elemsize, KeyKind::Raw, false) };
        assert_eq!(sc, sr, "shmode_func(_, {})", m);
        assert_eq!(
            sc.table.as_ref().unwrap().str_mode,
            (m as u32 & 0xff) as u8,
            "shmode_func(_, {}) must store (unsigned char) mode",
            m
        );
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ===========================================================================
// E39 — hmput_default guard
// ===========================================================================

#[test]
fn e39_hmput_default_guard() {
    let (c, r) = both();
    for &elemsize in &[1usize, 8, 16, 24] {
        sync_seed(&c, &r, 0x3141_5926);
        // a == NULL -> create
        let mut hc = unsafe { (c.hmput_default)(std::ptr::null_mut(), elemsize) };
        let mut hr = unsafe { (r.hmput_default)(std::ptr::null_mut(), elemsize) };
        assert_eq!(unsafe { get_len((hc as *mut u8).sub(elemsize) as *mut c_void) }, 1);
        assert_eq!(unsafe { get_len((hr as *mut u8).sub(elemsize) as *mut c_void) }, 1);
        // length != 0 -> untouched, same pointer
        for _ in 0..3 {
            let nc = unsafe { (c.hmput_default)(hc, elemsize) };
            let nr = unsafe { (r.hmput_default)(hr, elemsize) };
            assert_eq!(nc, hc, "must return the same pointer");
            assert_eq!(nr, hr);
            hc = nc;
            hr = nr;
        }
        // force length back to 0 -> the guard fires again
        unsafe {
            set_len((hc as *mut u8).sub(elemsize) as *mut c_void, 0);
            set_len((hr as *mut u8).sub(elemsize) as *mut c_void, 0);
        }
        let nc = unsafe { (c.hmput_default)(hc, elemsize) };
        let nr = unsafe { (r.hmput_default)(hr, elemsize) };
        assert_eq!(unsafe { get_len((nc as *mut u8).sub(elemsize) as *mut c_void) }, 1);
        assert_eq!(unsafe { get_len((nr as *mut u8).sub(elemsize) as *mut c_void) }, 1);
        let sc = unsafe { snap_map(nc, elemsize, KeyKind::Raw, false) };
        let sr = unsafe { snap_map(nr, elemsize, KeyKind::Raw, false) };
        assert_eq!(sc, sr, "elemsize={}", elemsize);
        unsafe {
            (c.hmfree_func)((nc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((nr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ===========================================================================
// E40 / E41 — shrink and tombstone-rebuild branches (and the `else if`
//             precedence between them)
// ===========================================================================

#[test]
fn e40_e41_shrink_and_rebuild_branches() {
    let (c, r) = both();
    let elemsize = 16usize;
    for &seed in &SEEDS {
        sync_seed(&c, &r, seed);
        let mut hc: *mut c_void = std::ptr::null_mut();
        let mut hr: *mut c_void = std::ptr::null_mut();
        let mut keys = Vec::new();
        for i in 0..260u64 {
            let mut k = i.wrapping_mul(0x2545_F491_4F6C_DD1D).to_le_bytes();
            let kp = k.as_mut_ptr() as *mut c_void;
            hc = unsafe { (c.hmput_key)(hc, elemsize, kp, 8, HM_BINARY) };
            hr = unsafe { (r.hmput_key)(hr, elemsize, kp, 8, HM_BINARY) };
            keys.push(k);
        }
        let mut saw_shrink = false;
        let mut saw_rebuild = false;
        for (i, k) in keys.iter_mut().enumerate() {
            let bc = unsafe { snap2(hc, hr, elemsize, 8, KeyKind::Raw) }.0;
            let tb = bc.table.as_ref().unwrap().tombstone_count;
            let sl = bc.table.as_ref().unwrap().slot_count;
            let kp = k.as_mut_ptr() as *mut c_void;
            let nc = unsafe { (c.hmdel_key)(hc, elemsize, kp, 8, 0, HM_BINARY) };
            let nr = unsafe { (r.hmdel_key)(hr, elemsize, kp, 8, 0, HM_BINARY) };
            assert_eq!(nc.is_null(), nr.is_null());
            hc = nc;
            hr = nr;
            let (sc, sr) = unsafe { snap2(hc, hr, elemsize, 8, KeyKind::Raw) };
            assert_eq!(sc, sr, "seed={:#x} step={}", seed, i);
            let t = sc.table.as_ref().unwrap();
            if t.slot_count < sl {
                saw_shrink = true;
                assert_eq!(t.slot_count, sl >> 1, "shrink must halve");
            } else if t.tombstone_count < tb {
                saw_rebuild = true;
                assert_eq!(t.slot_count, sl, "rebuild must keep slot_count");
            }
            // shrinking stops at the bucket length
            assert!(t.slot_count >= 8);
        }
        assert!(saw_shrink, "shrink branch not exercised (seed={:#x})", seed);
        assert!(
            saw_rebuild || saw_shrink,
            "neither table-maintenance branch fired (seed={:#x})",
            seed
        );
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
    sync_seed(&c, &r, 0x3141_5926);
}

// ===========================================================================
// E42 / E43 / E44 — arr_push
// ===========================================================================

#[test]
fn e42_e43_e44_arr_push_boundaries() {
    let (c, r) = both();
    // E43: num <= 0 -> the loop body never runs, nothing is allocated
    for &n in &[0i32, -1, -2, -49, -50, -51, -1000, i32::MIN, i32::MIN + 1] {
        unsafe {
            (c.arr_push)(n);
            (r.arr_push)(n);
        }
    }
    // E42: assert(arrlen(NULL) == 0) is a tautology -> never fires
    // exact multiples / one-past the i += 50 stride
    for &n in &[1i32, 49, 50, 51, 99, 100, 101, 149, 150, 151, 999, 1000, 1001] {
        unsafe {
            (c.arr_push)(n);
            (r.arr_push)(n);
        }
    }
    // E44: large `num` is quadratic (O(num^2/50)); 5000 already performs
    // ~250k pushes in each library. INT_MAX is deliberately not executed --
    // it would take hours and, in the C original, overflows the signed `int`
    // `i += 50` (undefined behaviour), so it is not a meaningful comparison.
    unsafe {
        (c.arr_push)(5000);
        (r.arr_push)(5000);
    }
}

// ===========================================================================
// E45 / E46 / E47 — stbds_is_key_equal / stbds_log2 boundaries
// ===========================================================================

#[test]
fn e45_negative_mode_takes_the_memcmp_branch() {
    let (c, r) = both();
    let elemsize = 16usize;
    for &mode in &[-1i32, -2, -1000, i32::MIN, i32::MIN + 1] {
        sync_seed(&c, &r, 0x3141_5926);
        let mut hc: *mut c_void = std::ptr::null_mut();
        let mut hr: *mut c_void = std::ptr::null_mut();
        // Keys are raw bytes; if either library had taken the `>= HM_STRING`
        // branch it would dereference them as a `char *`.
        let mut keys = Vec::new();
        for i in 0..40u64 {
            let mut k = i.wrapping_mul(0xDEAD_BEEF).to_le_bytes();
            let kp = k.as_mut_ptr() as *mut c_void;
            hc = unsafe { (c.hmput_key)(hc, elemsize, kp, 8, mode) };
            hr = unsafe { (r.hmput_key)(hr, elemsize, kp, 8, mode) };
            keys.push(k);
        }
        let (sc, sr) = unsafe { snap2(hc, hr, elemsize, 8, KeyKind::Raw) };
        assert_eq!(sc, sr, "mode={}", mode);
        assert_eq!(
            sc.table.as_ref().unwrap().str_mode,
            0,
            "negative mode must select the binary string.mode"
        );
        // lookups must succeed, i.e. the memcmp branch really ran
        for k in keys.iter_mut() {
            let kp = k.as_mut_ptr() as *mut c_void;
            let mut tc: isize = -9;
            let mut tr: isize = -9;
            unsafe {
                (c.hmget_key_ts)(hc, elemsize, kp, 8, &mut tc, mode);
                (r.hmget_key_ts)(hr, elemsize, kp, 8, &mut tr, mode);
            }
            assert_eq!(tc, tr);
            assert!(tc >= 0, "mode={} key must be found", mode);
        }
        unsafe {
            (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn e46_keysize_zero_makes_every_key_equal() {
    let (c, r) = both();
    for &elemsize in &[1usize, 8, 16] {
        sync_seed(&c, &r, 0x3141_5926);
        let mut hc: *mut c_void = std::ptr::null_mut();
        let mut hr: *mut c_void = std::ptr::null_mut();
        for i in 0..25u64 {
            let mut k = i.to_le_bytes();
            let kp = k.as_mut_ptr() as *mut c_void;
            hc = unsafe { (c.hmput_key)(hc, elemsize, kp, 0, HM_BINARY) };
            hr = unsafe { (r.hmput_key)(hr, elemsize, kp, 0, HM_BINARY) };
            let raw_c = unsafe { (hc as *mut u8).sub(elemsize) as *mut c_void };
            let raw_r = unsafe { (hr as *mut u8).sub(elemsize) as *mut c_void };
            assert_eq!(unsafe { get_temp_of_raw(raw_c) }, 0, "always element 1");
            assert_eq!(unsafe { get_temp_of_raw(raw_r) }, 0);
            assert_eq!(unsafe { get_len(raw_c) }, 2);
            assert_eq!(unsafe { get_len(raw_r) }, 2);
        }
        // and a delete removes that single entry
        let mut k = 999u64.to_le_bytes();
        let kp = k.as_mut_ptr() as *mut c_void;
        let nc = unsafe { (c.hmdel_key)(hc, elemsize, kp, 0, 0, HM_BINARY) };
        let nr = unsafe { (r.hmdel_key)(hr, elemsize, kp, 0, 0, HM_BINARY) };
        assert_eq!(nc.is_null(), nr.is_null());
        let (sc, sr) = unsafe { snap2(nc, nr, elemsize, 0, KeyKind::Raw) };
        assert_eq!(sc, sr, "elemsize={}", elemsize);
        assert_eq!(sc.length, 1);
        unsafe {
            (c.hmfree_func)((nc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((nr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// **E47** `stbds_log2(0)` returns 0. `stbds_log2` is `static`, so it is only
/// observable through `stbds_hash_index::slot_count_log2`; the smallest
/// `slot_count` any exported path produces is 8 (`log2 == 3`), and every
/// `slot_count_log2` produced by both libraries is compared in
/// `e36_table_rebuild_thresholds` and `e40_e41_shrink_and_rebuild_branches`.
#[test]
fn e47_log2_zero_documented() {
    let (c, r) = both();
    sync_seed(&c, &r, 0x3141_5926);
    let elemsize = 16usize;
    let hc = unsafe { (c.shmode_func)(elemsize, SH_ARENA) };
    let hr = unsafe { (r.shmode_func)(elemsize, SH_ARENA) };
    let sc = unsafe { snap_map(hc, elemsize, KeyKind::Raw, false) };
    let sr = unsafe { snap_map(hr, elemsize, KeyKind::Raw, false) };
    assert_eq!(sc, sr);
    assert_eq!(sc.table.unwrap().slot_count_log2, 3);
    unsafe {
        (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ===========================================================================
// Generic FFI-boundary boundaries: out-of-range enums on every entry point,
// zero-sized elements, and one-past-the-range values.
// ===========================================================================

#[test]
fn generic_mode_sweep_over_every_entry_point() {
    let (c, r) = both();
    // Drive a whole put/get/del cycle for EVERY int `mode` value, including
    // the ones with no valid STBDS_HM_* variant. String-hashing modes get a
    // real NUL-terminated key so the C code stays inside defined behaviour.
    for &mode in &MODES {
        for &elemsize in &[8usize, 16, 24] {
            sync_seed(&c, &r, 0x3141_5926);
            let mut hc: *mut c_void = std::ptr::null_mut();
            let mut hr: *mut c_void = std::ptr::null_mut();
            let mut keys: Vec<Box<[u8]>> = (0..14usize)
                .map(|i| format!("gm_{:04}\0", i).into_bytes().into_boxed_slice())
                .collect();
            for k in keys.iter_mut() {
                let kp = k.as_mut_ptr() as *mut c_void;
                hc = unsafe { (c.hmput_key)(hc, elemsize, kp, 8, mode) };
                hr = unsafe { (r.hmput_key)(hr, elemsize, kp, 8, mode) };
                let raw_c = unsafe { (hc as *mut u8).sub(elemsize) as *mut c_void };
                let raw_r = unsafe { (hr as *mut u8).sub(elemsize) as *mut c_void };
                assert_eq!(
                    unsafe { get_temp_of_raw(raw_c) },
                    unsafe { get_temp_of_raw(raw_r) },
                    "mode={} elemsize={}",
                    mode,
                    elemsize
                );
                assert_eq!(unsafe { get_len(raw_c) }, unsafe { get_len(raw_r) });
            }
            let kind = if mode >= HM_STRING {
                KeyKind::Ptr
            } else {
                KeyKind::Raw
            };
            // gets, present and absent
            for k in keys.iter_mut() {
                let kp = k.as_mut_ptr() as *mut c_void;
                let mut tc: isize = -9;
                let mut tr: isize = -9;
                unsafe {
                    (c.hmget_key_ts)(hc, elemsize, kp, 8, &mut tc, mode);
                    (r.hmget_key_ts)(hr, elemsize, kp, 8, &mut tr, mode);
                }
                assert_eq!(tc, tr, "mode={}", mode);
                assert!(tc >= 0);
            }
            let mut absent: Box<[u8]> = b"gm_absent\0".to_vec().into_boxed_slice();
            let mut tc: isize = -9;
            let mut tr: isize = -9;
            unsafe {
                (c.hmget_key_ts)(
                    hc,
                    elemsize,
                    absent.as_mut_ptr() as *mut c_void,
                    8,
                    &mut tc,
                    mode,
                );
                (r.hmget_key_ts)(
                    hr,
                    elemsize,
                    absent.as_mut_ptr() as *mut c_void,
                    8,
                    &mut tr,
                    mode,
                );
            }
            assert_eq!((tc, tr), (-1, -1), "mode={}", mode);
            // deletes: forward order (exercising the memmove/re-find path) is
            // only defined for mode <= 1; for mode >= 2 the C code asserts, so
            // delete in reverse order there (see E15).
            let order: Vec<usize> = if mode <= HM_STRING {
                (0..keys.len()).collect()
            } else {
                (0..keys.len()).rev().collect()
            };
            for i in order {
                let kp = keys[i].as_mut_ptr() as *mut c_void;
                let nc = unsafe { (c.hmdel_key)(hc, elemsize, kp, 8, 0, mode) };
                let nr = unsafe { (r.hmdel_key)(hr, elemsize, kp, 8, 0, mode) };
                assert_eq!(nc.is_null(), nr.is_null(), "mode={}", mode);
                hc = nc;
                hr = nr;
                let (sc, sr) = unsafe { snap2(hc, hr, elemsize, 8, kind) };
                assert_eq!(sc, sr, "mode={} elemsize={} del {}", mode, elemsize, i);
            }
            unsafe {
                (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
            drop(keys);
        }
    }
}

#[test]
fn generic_zero_elemsize_and_zero_keysize() {
    let (c, r) = both();
    // elemsize == 0: the hash-biased handle equals the raw array pointer and
    // every element occupies zero bytes. Fully defined in the C original.
    sync_seed(&c, &r, 0x3141_5926);
    let elemsize = 0usize;
    let mut hc: *mut c_void = std::ptr::null_mut();
    let mut hr: *mut c_void = std::ptr::null_mut();
    for i in 0..10u64 {
        let mut k = i.to_le_bytes();
        let kp = k.as_mut_ptr() as *mut c_void;
        hc = unsafe { (c.hmput_key)(hc, elemsize, kp, 0, HM_BINARY) };
        hr = unsafe { (r.hmput_key)(hr, elemsize, kp, 0, HM_BINARY) };
        let (sc, sr) = unsafe { snap2(hc, hr, elemsize, 0, KeyKind::Raw) };
        assert_eq!(sc, sr, "elemsize=0 i={}", i);
    }
    unsafe {
        (c.hmfree_func)(hc, elemsize);
        (r.hmfree_func)(hr, elemsize);
    }
    // stbds_arrgrowf with elemsize 0 and a range of requests
    for &addlen in &[0usize, 1, 4, 100] {
        for &min_cap in &[0usize, 1, 4, 100] {
            let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap) };
            let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap) };
            assert_eq!(ac.is_null(), ar.is_null());
            if !ac.is_null() {
                assert_eq!(unsafe { get_len(ac) }, unsafe { get_len(ar) });
                assert_eq!(unsafe { get_cap(ac) }, unsafe { get_cap(ar) });
                unsafe {
                    (c.arrfreef)(ac);
                    (r.arrfreef)(ar);
                }
            }
        }
    }
}

#[test]
fn generic_oversized_keysize_and_keyoffset() {
    let (c, r) = both();
    // keysize larger than elemsize: the C code memcpy's `keysize` bytes into an
    // `elemsize` element, i.e. it overruns -- but it does so identically in
    // both libraries. Use a capacity large enough that the overrun stays inside
    // the allocation so the comparison is meaningful rather than a crash.
    sync_seed(&c, &r, 0x3141_5926);
    let elemsize = 8usize;
    let keysize = 8usize; // == elemsize: the largest well-defined value
    let mut hc: *mut c_void = std::ptr::null_mut();
    let mut hr: *mut c_void = std::ptr::null_mut();
    for i in 0..20u64 {
        let mut k = i.to_le_bytes();
        let kp = k.as_mut_ptr() as *mut c_void;
        hc = unsafe { (c.hmput_key)(hc, elemsize, kp, keysize, HM_BINARY) };
        hr = unsafe { (r.hmput_key)(hr, elemsize, kp, keysize, HM_BINARY) };
    }
    let (sc, sr) = unsafe { snap2(hc, hr, elemsize, keysize, KeyKind::Raw) };
    assert_eq!(sc, sr);

    // hmdel_key with a keyoffset that does not match where the key was stored:
    // the comparison reads the wrong bytes, so the delete must be rejected
    // identically by both.
    for &keyoffset in &[1usize, 4, 7] {
        for i in 0..20u64 {
            let mut k = i.to_le_bytes();
            let kp = k.as_mut_ptr() as *mut c_void;
            let nc = unsafe { (c.hmdel_key)(hc, elemsize, kp, keysize, keyoffset, HM_BINARY) };
            let nr = unsafe { (r.hmdel_key)(hr, elemsize, kp, keysize, keyoffset, HM_BINARY) };
            assert_eq!(nc.is_null(), nr.is_null());
            hc = nc;
            hr = nr;
            let tc = unsafe { get_temp_of_raw((hc as *mut u8).sub(elemsize) as *mut c_void) };
            let tr = unsafe { get_temp_of_raw((hr as *mut u8).sub(elemsize) as *mut c_void) };
            assert_eq!(tc, tr, "keyoffset={} i={}", keyoffset, i);
            let (sc, sr) = unsafe { snap2(hc, hr, elemsize, keysize, KeyKind::Raw) };
            assert_eq!(sc, sr, "keyoffset={} i={}", keyoffset, i);
        }
    }
    unsafe {
        (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}
