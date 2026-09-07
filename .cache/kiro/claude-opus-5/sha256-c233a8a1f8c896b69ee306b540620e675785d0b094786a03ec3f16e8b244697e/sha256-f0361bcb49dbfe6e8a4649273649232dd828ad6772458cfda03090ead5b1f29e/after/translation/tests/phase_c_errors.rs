//! Phase C — one differential test per ERRORS.md row.
//!
//! Every test asserts the two libraries return the *same* sentinel / error
//! value, not merely that both "failed somehow".  Sentinels in this library
//! are `-1` (`STBDS_INDEX_EMPTY`), `-2` (`STBDS_INDEX_DELETED`), `NULL`,
//! `header->temp == 0`, and `SIGABRT` from `assert` (see phase_c_abort.rs).

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const HM_BINARY: c_int = 0;
const HM_STRING: c_int = 1;
const SH_NONE: c_int = 0;
const SH_DEFAULT: c_int = 1;
const SH_STRDUP: c_int = 2;
const SH_ARENA: c_int = 3;

/// Every `mode` value worth pushing across the FFI boundary, including values
/// with no corresponding C enum variant.
const MODES: [c_int; 12] = [
    0,
    1,
    2,
    3,
    4,
    7,
    255,
    256,
    -1,
    -2,
    c_int::MAX,
    c_int::MIN,
];

unsafe fn temp_of(t: *mut c_void, elemsize: usize) -> isize {
    if t.is_null() {
        0
    } else {
        (*hdr(hash_to_arr(t, elemsize))).temp
    }
}

// ===========================================================================
// ERRORS.md #1, #2, #3 — stbds_arrgrowf rejection / bootstrap paths
// ===========================================================================

#[test]
fn err_1_arrgrowf_early_return_is_identity() {
    let (_g, b) = both();
    unsafe {
        // #1 on NULL: min_cap(0) <= arrcap(NULL)(0) -> returns the input, NULL
        for es in [1usize, 8, 16, 32] {
            let c = (b.c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let r = (b.r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(c.is_null(), "C arrgrowf(NULL,{es},0,0) must return NULL");
            assert!(r.is_null(), "Rust arrgrowf(NULL,{es},0,0) must return NULL");
        }
        // #1 on an existing array: identical pointer back
        for es in [1usize, 8, 16, 32] {
            let mut c = (b.c.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            let mut r = (b.r.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            for min_cap in 0..=4usize {
                let c2 = (b.c.arrgrowf)(c, es, 0, min_cap);
                let r2 = (b.r.arrgrowf)(r, es, 0, min_cap);
                assert_eq!(c2, c, "C must not reallocate (es={es} min_cap={min_cap})");
                assert_eq!(r2, r, "Rust must not reallocate (es={es} min_cap={min_cap})");
                c = c2;
                r = r2;
            }
            (b.c.arrfreef)(c);
            (b.r.arrfreef)(r);
        }
    }
}

#[test]
fn err_2_3_arrgrowf_bootstrap_header() {
    let (_g, b) = both();
    unsafe {
        for es in [1usize, 4, 8, 16, 24, 32, 64] {
            // #3: the degenerate zero request still clamps capacity to 4
            let c = (b.c.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let r = (b.r.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let ch = &*hdr(c);
            let rh = &*hdr(r);
            assert_eq!((ch.length, ch.capacity, ch.temp), (0, 4, 0), "C header es={es}");
            assert_eq!((rh.length, rh.capacity, rh.temp), (0, 4, 0), "Rust header es={es}");
            assert!(ch.hash_table.is_null() && rh.hash_table.is_null());
            (b.c.arrfreef)(c);
            (b.r.arrfreef)(r);
        }
    }
}

// ERRORS.md #4 lives in tests/phase_b_low.rs::err_4_arrgrowf_overflow_decision
//
// ERRORS.md #5 (`stbds_arrfreef(NULL)` -> `free((char*)NULL - 32)`) is an
// invalid free in the C.  The Rust reproduces the identical unchecked
// `free(hdr(a))`, but exercising it would corrupt the test process, so it is
// deliberately NOT executed.  Verified textually instead:
#[test]
fn err_5_arrfreef_has_no_null_check_in_either_impl() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let c = std::fs::read_to_string(root.join("c_src/src/lib.c")).unwrap();
    let cbody = c
        .split("void stbds_arrfreef(void *a)")
        .nth(1)
        .expect("stbds_arrfreef not found in C")
        .split('}')
        .next()
        .unwrap()
        .to_string();
    // The C body is a single unconditional `STBDS_FREE(NULL, stbds_header(a))`
    // — note the `NULL` there is the allocator-context argument, not a guard.
    assert!(
        !cbody.contains("if") && !cbody.contains("a == NULL") && !cbody.contains("!a"),
        "the C gained a null check; update this test: {cbody:?}"
    );
    let rs = std::fs::read_to_string(root.join("translation/src/lib.rs")).unwrap();
    let rbody = rs
        .split("pub unsafe extern \"C\" fn stbds_arrfreef")
        .nth(1)
        .expect("stbds_arrfreef not found in Rust")
        .split("\n}")
        .next()
        .unwrap()
        .to_string();
    assert!(
        !rbody.contains("is_null") && !rbody.contains("if "),
        "the Rust added a null check the C does not have: {rbody:?}"
    );
}

// ERRORS.md #6, #21, #27, #28, #35 — asserts that cannot be reached through any
// exported symbol.  Documented as unreachable; the reasoning is checked here so
// the claim cannot silently rot.
#[test]
fn err_6_make_hash_index_assert_is_unreachable() {
    let (_g, b) = both();
    // Every table the public API creates has slot_count == 8 * 2^k, for which
    // used_count_threshold + tombstone_count_threshold < slot_count holds.
    // (The assert only fires for slot_count <= 2.)
    for k in 0..12u32 {
        let sc = 8usize << k;
        let used = sc - (sc >> 2);
        let tomb = (sc >> 3) + (sc >> 4);
        assert!(used + tomb < sc, "slot_count={sc} would trip the assert");
    }
    // And the smallest table the API can produce really is 8 slots.
    seed_both(b, 1);
    unsafe {
        let c = (b.c.shmode_func)(16, SH_DEFAULT);
        let r = (b.r.shmode_func)(16, SH_DEFAULT);
        assert_eq!(snap(c, 16, 0, KeyKind::Raw, 0, 0).slot_count, 8);
        assert_eq!(snap(r, 16, 0, KeyKind::Raw, 0, 0).slot_count, 8);
        (b.c.hmfree_func)(hash_to_arr(c, 16), 16);
        (b.r.hmfree_func)(hash_to_arr(r, 16), 16);
    }
}

// ===========================================================================
// #7, #24, #25 — NULL / no-table inputs
// ===========================================================================

#[test]
fn err_7_hmfree_func_null_is_noop() {
    let (_g, b) = both();
    unsafe {
        for es in [0usize, 1, 8, 16, 4096] {
            (b.c.hmfree_func)(std::ptr::null_mut(), es);
            (b.r.hmfree_func)(std::ptr::null_mut(), es);
        }
    }
}

#[test]
fn err_24_hmdel_key_null_returns_null() {
    let (_g, b) = both();
    let k = CBuf::new(&[7u8; 8]);
    unsafe {
        for &mode in &MODES {
            for es in [8usize, 16, 32] {
                for keyoffset in [0usize, 8] {
                    let c = (b.c.hmdel_key)(std::ptr::null_mut(), es, k.ptr(), 8, keyoffset, mode);
                    let r = (b.r.hmdel_key)(std::ptr::null_mut(), es, k.ptr(), 8, keyoffset, mode);
                    assert!(c.is_null(), "C hmdel_key(NULL) mode={mode} es={es}");
                    assert!(r.is_null(), "Rust hmdel_key(NULL) mode={mode} es={es}");
                }
            }
        }
    }
}

#[test]
fn err_25_10_no_table_paths() {
    let (_g, b) = both();
    let k = CBuf::new(&[3u8; 8]);
    for es in [8usize, 16, 32] {
        seed_both(b, 11);
        unsafe {
            // an array with header->hash_table == NULL
            let mut c = (b.c.hmput_default)(std::ptr::null_mut(), es);
            let mut r = (b.r.hmput_default)(std::ptr::null_mut(), es);
            assert!(!(*hdr(hash_to_arr(c, es))).hash_table.is_null() == false);
            for &mode in &MODES {
                // #10: get -> temp == -1
                c = (b.c.hmget_key)(c, es, k.ptr(), 8, mode);
                r = (b.r.hmget_key)(r, es, k.ptr(), 8, mode);
                assert_eq!(temp_of(c, es), -1, "C get no-table mode={mode}");
                assert_eq!(temp_of(r, es), -1, "Rust get no-table mode={mode}");

                let mut ct: isize = 0x1234;
                let mut rt: isize = 0x1234;
                c = (b.c.hmget_key_ts)(c, es, k.ptr(), 8, &mut ct, mode);
                r = (b.r.hmget_key_ts)(r, es, k.ptr(), 8, &mut rt, mode);
                assert_eq!((ct, rt), (-1, -1), "get_ts no-table mode={mode}");

                // #25: del -> temp == 0, pointer unchanged
                let c2 = (b.c.hmdel_key)(c, es, k.ptr(), 8, 0, mode);
                let r2 = (b.r.hmdel_key)(r, es, k.ptr(), 8, 0, mode);
                assert_eq!(c2, c, "C hmdel_key no-table must return `a`");
                assert_eq!(r2, r, "Rust hmdel_key no-table must return `a`");
                assert_eq!(temp_of(c2, es), 0, "C del no-table temp");
                assert_eq!(temp_of(r2, es), 0, "Rust del no-table temp");
                c = c2;
                r = r2;
            }
            (b.c.hmfree_func)(hash_to_arr(c, es), es);
            (b.r.hmfree_func)(hash_to_arr(r, es), es);
        }
    }
}

// ===========================================================================
// #8, #9, #11, #12 — the "key absent" sentinel, from every entry point
// ===========================================================================

#[test]
fn err_9_hmget_key_ts_null_bootstrap() {
    let (_g, b) = both();
    let k = CBuf::new(&[1u8; 8]);
    for es in [1usize, 8, 16, 32] {
        seed_both(b, 5);
        unsafe {
            for &mode in &MODES {
                let mut ct: isize = 0x7777;
                let mut rt: isize = 0x7777;
                let c = (b.c.hmget_key_ts)(std::ptr::null_mut(), es, k.ptr(), 8, &mut ct, mode);
                let r = (b.r.hmget_key_ts)(std::ptr::null_mut(), es, k.ptr(), 8, &mut rt, mode);
                assert!(!c.is_null() && !r.is_null(), "bootstrap must allocate");
                assert_eq!((ct, rt), (-1, -1), "es={es} mode={mode}");
                let cs = snap(c, es, 0, KeyKind::Raw, 0, 0);
                let rs = snap(r, es, 0, KeyKind::Raw, 0, 0);
                assert_eq!(cs, rs, "bootstrap snapshot es={es} mode={mode}");
                assert_eq!((cs.length, cs.has_table), (1, false));
                (b.c.hmfree_func)(hash_to_arr(c, es), es);
                (b.r.hmfree_func)(hash_to_arr(r, es), es);
            }
            // #12: stbds_hmget_key writes the sentinel into header->temp
            for &mode in &MODES {
                let c = (b.c.hmget_key)(std::ptr::null_mut(), es, k.ptr(), 8, mode);
                let r = (b.r.hmget_key)(std::ptr::null_mut(), es, k.ptr(), 8, mode);
                assert_eq!(temp_of(c, es), -1, "C hmget_key(NULL) mode={mode}");
                assert_eq!(temp_of(r, es), -1, "Rust hmget_key(NULL) mode={mode}");
                (b.c.hmfree_func)(hash_to_arr(c, es), es);
                (b.r.hmfree_func)(hash_to_arr(r, es), es);
            }
        }
    }
}

#[test]
fn err_8_11_absent_key_returns_minus_one() {
    let (_g, b) = both();
    let mut rng = Rng::new(0x811);
    // binary
    for n in [1usize, 6, 12, 48] {
        seed_both(b, 0x3141_5926);
        unsafe {
            let mut c: *mut c_void = std::ptr::null_mut();
            let mut r: *mut c_void = std::ptr::null_mut();
            let keys: Vec<CBuf> = (0..n).map(|_| CBuf::new(&rng.bytes(8))).collect();
            for k in &keys {
                c = (b.c.hmput_key)(c, 16, k.ptr(), 8, HM_BINARY);
                r = (b.r.hmput_key)(r, 16, k.ptr(), 8, HM_BINARY);
            }
            for _ in 0..50 {
                let miss = CBuf::new(&rng.bytes(8));
                c = (b.c.hmget_key)(c, 16, miss.ptr(), 8, HM_BINARY);
                r = (b.r.hmget_key)(r, 16, miss.ptr(), 8, HM_BINARY);
                let ctv = temp_of(c, 16);
                let rtv = temp_of(r, 16);
                assert_eq!(ctv, rtv, "absent-key temp n={n}");
                assert_eq!(ctv, -1, "absent key must yield -1 (n={n})");
            }
            (b.c.hmfree_func)(hash_to_arr(c, 16), 16);
            (b.r.hmfree_func)(hash_to_arr(r, 16), 16);
        }
    }
    // string, all arena modes
    for shmode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        seed_both(b, 0x3141_5926);
        unsafe {
            let mut c = (b.c.shmode_func)(16, shmode);
            let mut r = (b.r.shmode_func)(16, shmode);
            let keys: Vec<CBuf> = (0..20)
                .map(|i| CBuf::new(format!("present-{i}\0").as_bytes()))
                .collect();
            for k in &keys {
                c = (b.c.hmput_key)(c, 16, k.cptr() as *mut c_void, 8, HM_STRING);
                r = (b.r.hmput_key)(r, 16, k.cptr() as *mut c_void, 8, HM_STRING);
            }
            for probe in ["", "present", "present-", "present-99", "PRESENT-1", "\u{7f}"] {
                let m = CBuf::new(format!("{probe}\0").as_bytes());
                c = (b.c.hmget_key)(c, 16, m.cptr() as *mut c_void, 8, HM_STRING);
                r = (b.r.hmget_key)(r, 16, m.cptr() as *mut c_void, 8, HM_STRING);
                assert_eq!(temp_of(c, 16), temp_of(r, 16), "probe={probe:?}");
                assert_eq!(temp_of(c, 16), -1, "probe={probe:?} must be absent");
            }
            (b.c.hmfree_func)(hash_to_arr(c, 16), 16);
            (b.r.hmfree_func)(hash_to_arr(r, 16), 16);
        }
    }
}

// ===========================================================================
// #13, #14 — out-of-range `mode` across the FFI boundary
// ===========================================================================

#[test]
fn err_13_14_out_of_range_mode_classification() {
    let (_g, b) = both();
    // A binary map: modes < 1 must find the key, modes >= 1 must take the
    // string path (which hashes the key *pointer's target*, so it must not
    // find it) — and both libraries must agree on which.
    let mut rng = Rng::new(0xE13);
    seed_both(b, 0x3141_5926);
    unsafe {
        let mut c: *mut c_void = std::ptr::null_mut();
        let mut r: *mut c_void = std::ptr::null_mut();
        // keys are valid NUL-terminated strings *and* 8 raw bytes, so both
        // interpretations are memory-safe.
        let keys: Vec<CBuf> = (0..8)
            .map(|i| CBuf::new(format!("k{i}\0\0\0\0\0").as_bytes()))
            .collect();
        for k in &keys {
            c = (b.c.hmput_key)(c, 16, k.ptr(), 8, HM_BINARY);
            r = (b.r.hmput_key)(r, 16, k.ptr(), 8, HM_BINARY);
        }
        for &mode in &[0i32, -1, -2, i32::MIN, -1000] {
            for k in &keys {
                c = (b.c.hmget_key)(c, 16, k.ptr(), 8, mode);
                r = (b.r.hmget_key)(r, 16, k.ptr(), 8, mode);
                let (ct, rt) = (temp_of(c, 16), temp_of(r, 16));
                assert_eq!(ct, rt, "mode={mode} binary-class lookup");
                assert!(ct >= 0, "mode={mode} must behave as BINARY and find the key");
            }
        }
        (b.c.hmfree_func)(hash_to_arr(c, 16), 16);
        (b.r.hmfree_func)(hash_to_arr(r, 16), 16);
    }

    // A string map: modes >= 1 must all behave identically to mode == 1 for
    // put/get (only `hmdel_key` distinguishes `== 1`).
    for &mode in &[1i32, 2, 3, 7, 255, 256, i32::MAX] {
        seed_both(b, 0x3141_5926);
        unsafe {
            let mut c = (b.c.shmode_func)(16, SH_DEFAULT);
            let mut r = (b.r.shmode_func)(16, SH_DEFAULT);
            let keys: Vec<CBuf> = (0..12)
                .map(|_| {
                    let l = 1 + rng.below(12);
                    let mut s = rng.cstring(l);
                    s.pop();
                    s.extend_from_slice(format!("-{mode}").as_bytes());
                    s.push(0);
                    CBuf::new(&s)
                })
                .collect();
            for k in &keys {
                c = (b.c.hmput_key)(c, 16, k.cptr() as *mut c_void, 8, mode);
                r = (b.r.hmput_key)(r, 16, k.cptr() as *mut c_void, 8, mode);
                assert_eq!(temp_of(c, 16), temp_of(r, 16), "put mode={mode}");
            }
            for k in &keys {
                c = (b.c.hmget_key)(c, 16, k.cptr() as *mut c_void, 8, mode);
                r = (b.r.hmget_key)(r, 16, k.cptr() as *mut c_void, 8, mode);
                let (ct, rt) = (temp_of(c, 16), temp_of(r, 16));
                assert_eq!(ct, rt, "get mode={mode}");
                assert!(ct >= 0, "mode={mode} must behave as STRING and find the key");
            }
            assert_eq!(
                snap(c, 16, 8, KeyKind::Str, 8, 0),
                snap(r, 16, 8, KeyKind::Str, 8, 0),
                "string map state mode={mode}"
            );
            (b.c.hmfree_func)(hash_to_arr(c, 16), 16);
            (b.r.hmfree_func)(hash_to_arr(r, 16), 16);
        }
    }
}

// ===========================================================================
// #15, #16, #17 — stbds_hmput_default
// ===========================================================================

#[test]
fn err_15_16_17_hmput_default() {
    let (_g, b) = both();
    for es in [1usize, 8, 16, 32] {
        seed_both(b, 13);
        unsafe {
            // #15: NULL -> allocate, length == 1, element zeroed
            let c = (b.c.hmput_default)(std::ptr::null_mut(), es);
            let r = (b.r.hmput_default)(std::ptr::null_mut(), es);
            let cs = snap(c, es, es, KeyKind::Raw, 0, 0);
            let rs = snap(r, es, es, KeyKind::Raw, 0, 0);
            assert_eq!(cs, rs, "hmput_default(NULL) es={es}");
            assert_eq!(cs.length, 1);
            assert_eq!(cs.elems[0][..es], vec![0u8; es][..], "default element must be zeroed");

            // #17: second call is a no-op and returns the very same pointer
            let c2 = (b.c.hmput_default)(c, es);
            let r2 = (b.r.hmput_default)(r, es);
            assert_eq!(c2, c, "C hmput_default must be idempotent");
            assert_eq!(r2, r, "Rust hmput_default must be idempotent");

            // #16: length forced to 0 -> grows and bumps length back to 1
            (*(hdr(hash_to_arr(c2, es)) as *mut ArrHeader)).length = 0;
            (*(hdr(hash_to_arr(r2, es)) as *mut ArrHeader)).length = 0;
            let c3 = (b.c.hmput_default)(c2, es);
            let r3 = (b.r.hmput_default)(r2, es);
            let cs3 = snap(c3, es, es, KeyKind::Raw, 0, 0);
            let rs3 = snap(r3, es, es, KeyKind::Raw, 0, 0);
            assert_eq!(cs3, rs3, "hmput_default length==0 es={es}");
            assert_eq!(cs3.length, 1);
            (b.c.hmfree_func)(hash_to_arr(c3, es), es);
            (b.r.hmfree_func)(hash_to_arr(r3, es), es);
        }
    }
}

// ===========================================================================
// #19, #20, #22, #23 — table creation, growth, and the `default:` switch arm
// ===========================================================================

#[test]
fn err_19_20_table_creation_and_growth_thresholds() {
    let (_g, b) = both();
    let mut rng = Rng::new(0x1920);
    for &mode in &[HM_BINARY, HM_STRING, -1, 2] {
        seed_both(b, 0x3141_5926);
        unsafe {
            let mut c: *mut c_void = std::ptr::null_mut();
            let mut r: *mut c_void = std::ptr::null_mut();
            let mut prev_slots = 0usize;
            for i in 0..200 {
                // keys are valid strings AND distinct byte patterns
                let key = CBuf::new(format!("key{i}\0\0\0").as_bytes());
                c = (b.c.hmput_key)(c, 16, key.ptr(), 8, mode);
                r = (b.r.hmput_key)(r, 16, key.ptr(), 8, mode);
                let cs = snap(c, 16, 0, KeyKind::Raw, 0, 0);
                let rs = snap(r, 16, 0, KeyKind::Raw, 0, 0);
                assert_eq!(cs, rs, "growth step {i} mode={mode}");
                // #19: the first put creates an 8-slot table with the
                // mode-derived string.mode
                if i == 0 {
                    assert_eq!(cs.slot_count, 8);
                    assert_eq!(
                        cs.arena_mode,
                        if mode >= 1 { 1u8 } else { 0u8 },
                        "string.mode must be SH_DEFAULT iff mode >= 1"
                    );
                }
                // #20: doubling happens exactly at used_count_threshold
                if cs.slot_count != prev_slots && prev_slots != 0 {
                    assert_eq!(cs.slot_count, prev_slots * 2, "table must double");
                }
                prev_slots = cs.slot_count;
                assert_eq!(
                    cs.used_count_threshold,
                    cs.slot_count - (cs.slot_count >> 2)
                );
                // growth is checked on *entry* to the next put, so equality
                // with the threshold is a valid post-put state
                assert!(
                    cs.used_count <= cs.used_count_threshold,
                    "used_count {} exceeded threshold {} at step {i}",
                    cs.used_count,
                    cs.used_count_threshold
                );
                let _ = &mut rng;
            }
            (b.c.hmfree_func)(hash_to_arr(c, 16), 16);
            (b.r.hmfree_func)(hash_to_arr(r, 16), 16);
        }
    }
}

/// #22, #23 — `string.mode` values with no matching `switch` case fall to
/// `default:`, which `memcpy`s `keysize` bytes (0 bytes when `keysize == 0`).
#[test]
fn err_22_23_default_switch_arm() {
    let (_g, b) = both();
    for shmode in [SH_NONE, 4, 5, 200, 255] {
        for keysize in [0usize, 1, 4, 8] {
            let elemsize = keysize.max(1) + 8;
            seed_both(b, 0x3141_5926);
            unsafe {
                let mut c = (b.c.shmode_func)(elemsize, shmode);
                let mut r = (b.r.shmode_func)(elemsize, shmode);
                let mut rng = Rng::new(0x2223 ^ keysize as u64 ^ shmode as u64);
                for i in 0..12 {
                    let key = CBuf::new(&rng.bytes(keysize.max(1)));
                    c = (b.c.hmput_key)(c, elemsize, key.ptr(), keysize, HM_BINARY);
                    r = (b.r.hmput_key)(r, elemsize, key.ptr(), keysize, HM_BINARY);
                    assert_eq!(temp_of(c, elemsize), temp_of(r, elemsize), "put {i}");
                    let cs = snap(c, elemsize, keysize, KeyKind::Raw, 0, 0);
                    let rs = snap(r, elemsize, keysize, KeyKind::Raw, 0, 0);
                    assert_eq!(cs, rs, "shmode={shmode} keysize={keysize} step {i}");
                    // #23: keysize == 0 makes every key "equal", so the map
                    // never grows past a single entry.
                    if keysize == 0 {
                        assert_eq!(cs.length, 2, "keysize==0 must collapse to one entry");
                    }
                }
                (b.c.hmfree_func)(hash_to_arr(c, elemsize), elemsize);
                (b.r.hmfree_func)(hash_to_arr(r, elemsize), elemsize);
            }
        }
    }
}

// ===========================================================================
// #26, #31, #32, #33, #34 — deletion sentinels and table resize triggers
// ===========================================================================

#[test]
fn err_26_delete_absent_reports_zero() {
    let (_g, b) = both();
    let mut rng = Rng::new(0x260);
    for n in [0usize, 1, 6, 12, 48] {
        seed_both(b, 0x3141_5926);
        unsafe {
            let mut c: *mut c_void = std::ptr::null_mut();
            let mut r: *mut c_void = std::ptr::null_mut();
            let keys: Vec<CBuf> = (0..n).map(|_| CBuf::new(&rng.bytes(8))).collect();
            for k in &keys {
                c = (b.c.hmput_key)(c, 16, k.ptr(), 8, HM_BINARY);
                r = (b.r.hmput_key)(r, 16, k.ptr(), 8, HM_BINARY);
            }
            if c.is_null() {
                continue;
            }
            for _ in 0..30 {
                let miss = CBuf::new(&rng.bytes(8));
                let c2 = (b.c.hmdel_key)(c, 16, miss.ptr(), 8, 0, HM_BINARY);
                let r2 = (b.r.hmdel_key)(r, 16, miss.ptr(), 8, 0, HM_BINARY);
                assert_eq!(c2, c, "C: absent delete must not reallocate");
                assert_eq!(r2, r, "Rust: absent delete must not reallocate");
                assert_eq!(temp_of(c2, 16), 0, "C absent delete temp");
                assert_eq!(temp_of(r2, 16), 0, "Rust absent delete temp");
            }
            (b.c.hmfree_func)(hash_to_arr(c, 16), 16);
            (b.r.hmfree_func)(hash_to_arr(r, 16), 16);
        }
    }
}

/// #31, #32 — `mode == 1` exactly vs `mode >= 2` inside `hmdel_key`, and
/// #33, #34 — the shrink and tombstone-rebuild triggers.
#[test]
fn err_31_to_34_delete_modes_and_resizes() {
    let (_g, b) = both();
    for shmode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        seed_both(b, 0x3141_5926);
        unsafe {
            let mut c = (b.c.shmode_func)(16, shmode);
            let mut r = (b.r.shmode_func)(16, shmode);
            let keys: Vec<CBuf> = (0..120)
                .map(|i| CBuf::new(format!("delkey-{i}\0").as_bytes()))
                .collect();
            for k in &keys {
                c = (b.c.hmput_key)(c, 16, k.cptr() as *mut c_void, 8, HM_STRING);
                r = (b.r.hmput_key)(r, 16, k.cptr() as *mut c_void, 8, HM_STRING);
            }
            let mut saw_shrink = false;
            let mut saw_rebuild = false;
            let mut prev = snap(c, 16, 8, KeyKind::Str, 8, 0);
            for k in &keys {
                let cd = (b.c.hmdel_key)(c, 16, k.cptr() as *mut c_void, 8, 0, HM_STRING);
                let rd = (b.r.hmdel_key)(r, 16, k.cptr() as *mut c_void, 8, 0, HM_STRING);
                c = cd;
                r = rd;
                assert_eq!(temp_of(c, 16), 1, "C: successful delete sets temp = 1");
                assert_eq!(temp_of(r, 16), 1, "Rust: successful delete sets temp = 1");
                let cs = snap(c, 16, 8, KeyKind::Str, 8, 0);
                let rs = snap(r, 16, 8, KeyKind::Str, 8, 0);
                assert_eq!(cs, rs, "delete state shmode={shmode}");
                if cs.slot_count < prev.slot_count {
                    saw_shrink = true;
                }
                if cs.slot_count == prev.slot_count && cs.tombstone_count < prev.tombstone_count {
                    saw_rebuild = true;
                }
                prev = cs;
            }
            assert!(saw_shrink, "#33: the shrink path was never taken (shmode={shmode})");
            assert!(
                saw_rebuild,
                "#34: the tombstone-rebuild path was never taken (shmode={shmode})"
            );
            (b.c.hmfree_func)(hash_to_arr(c, 16), 16);
            (b.r.hmfree_func)(hash_to_arr(r, 16), 16);
        }
    }
}

/// #31 — `stbds_hmdel_key` frees the strdup'd key only when `mode` is
/// **exactly** `STBDS_HM_STRING`.  With an out-of-range `mode >= 2` the key
/// must be left allocated.
///
/// "Was it freed?" is observed two ways per library: whether the key bytes were
/// clobbered by the allocator's free-list metadata, and whether the next
/// `strdup` of the same size reuses that exact address.  Only the C-vs-Rust
/// *agreement* on those booleans is asserted, never the raw addresses.
#[test]
fn err_31_strdup_key_freed_only_for_mode_exactly_1() {
    let (_g, b) = both();
    const ES: usize = 16;

    // `mode == 1` must free; `mode >= 2` must not.
    for (mode, should_free) in [(1i32, true), (2, false), (3, false), (i32::MAX, false)] {
        let mut observed = Vec::new();
        for api in [&b.c, &b.r] {
            unsafe {
                (api.rand_seed)(0x3141_5926);
                let mut t = (api.shmode_func)(ES, SH_STRDUP);
                // one entry only, so the delete never relocates (ERRORS.md #29)
                let key = CBuf::new(b"a-reasonably-long-strdup-key-value\0");
                t = (api.hmput_key)(t, ES, key.cptr() as *mut c_void, 8, mode);
                let stored = *((t as *const u8).add(0) as *const *mut u8);
                assert_ne!(stored as usize, 0);
                assert_ne!(stored, key.0, "SH_STRDUP must copy the key");
                let before: Vec<u8> = std::slice::from_raw_parts(stored, 24).to_vec();

                t = (api.hmdel_key)(t, ES, key.cptr() as *mut c_void, 8, 0, mode);
                assert!(!t.is_null());

                let after: Vec<u8> = std::slice::from_raw_parts(stored, 24).to_vec();
                let clobbered = before != after;

                // put the same key again; a freed same-size chunk is reused
                t = (api.hmput_key)(t, ES, key.cptr() as *mut c_void, 8, mode);
                let stored2 = *((t as *const u8).add(0) as *const *mut u8);
                let reused = stored2 == stored;

                observed.push((clobbered, reused));

                // clean up without tripping the strdup double-free
                (api.hmfree_func)(hash_to_arr(t, ES), ES);
            }
        }
        assert_eq!(
            observed[0], observed[1],
            "mode={mode}: C observed (clobbered, reused)={:?} but Rust observed {:?}",
            observed[0], observed[1]
        );
        // and the observation must actually match the documented C behaviour,
        // otherwise this test would pass vacuously
        assert_eq!(
            observed[0].1, should_free,
            "mode={mode}: expected freed={should_free}, but chunk-reuse said {}",
            observed[0].1
        );
    }
}

/// The `stbds_hmfree_func` strdup loop starts at element **1**, so the default
/// element's key (which `stbds_shdefaults(t, s)` can legitimately set, exactly
/// as `sh_geti` sets `t[-1].value`) must NOT be freed.
#[test]
fn err_47_hmfree_func_skips_the_default_element_key() {
    let (_g, b) = both();
    const ES: usize = 16;
    let mut observed = Vec::new();
    for api in [&b.c, &b.r] {
        unsafe {
            (api.rand_seed)(0x3141_5926);
            let mut t = (api.shmode_func)(ES, SH_STRDUP);
            for i in 0..6 {
                let k = CBuf::new(format!("live-key-{i}\0").as_bytes());
                t = (api.hmput_key)(t, ES, k.cptr() as *mut c_void, 8, HM_STRING);
            }
            // emulate `stbds_shdefaults(t, s)`: element -1 gets a real key
            let owned = CBuf::new(b"default-element-key-not-owned-by-the-map\0");
            let arr = hash_to_arr(t, ES) as *mut *mut u8;
            *arr = owned.0;
            let before: Vec<u8> = std::slice::from_raw_parts(owned.0, 24).to_vec();

            (api.hmfree_func)(hash_to_arr(t, ES), ES);

            let after: Vec<u8> = std::slice::from_raw_parts(owned.0, 24).to_vec();
            observed.push(before == after);
            // `owned` is dropped (freed) by the test, as it should be
        }
    }
    assert_eq!(
        observed[0], observed[1],
        "C left the default key intact={} but Rust left it intact={}",
        observed[0], observed[1]
    );
    assert!(
        observed[0],
        "the C must not free the default element's key; test would be vacuous"
    );
}

// ===========================================================================
// #36 - #40 — string-arena rejection paths (see also phase_b_arena.rs)
// ===========================================================================

#[test]
fn err_36_to_40_arena_paths() {
    let (_g, b) = both();
    const ARENA: usize = 24;
    unsafe {
        // #39: empty string into a fresh arena -> 512-byte block, p = base+511
        let ca = CBuf::new(&[0u8; ARENA]);
        let ra = CBuf::new(&[0u8; ARENA]);
        let e = CBuf::new(b"\0");
        let cp = (b.c.stralloc)(ca.ptr(), e.cptr());
        let rp = (b.r.stralloc)(ra.ptr(), e.cptr());
        let cv = &*(ca.0 as *const StringArena);
        let rv = &*(ra.0 as *const StringArena);
        assert_eq!(cv.remaining, 511, "C remaining after empty string");
        assert_eq!(rv.remaining, 511, "Rust remaining after empty string");
        assert_eq!(cv.block, 1);
        assert_eq!(rv.block, 1);
        assert_eq!(cp.offset_from((cv.storage as *const i8).add(8)), 511);
        assert_eq!(rp.offset_from((rv.storage as *const i8).add(8)), 511);
        (b.c.strreset)(ca.ptr());
        (b.r.strreset)(ra.ptr());

        // #36: first string longer than the block -> remaining forced to 0
        let big: Vec<u8> = std::iter::repeat(b'Z').take(600).chain([0]).collect();
        let bigb = CBuf::new(&big);
        let cp = (b.c.stralloc)(ca.ptr(), bigb.cptr());
        let rp = (b.r.stralloc)(ra.ptr(), bigb.cptr());
        assert_eq!(cv.remaining, 0, "#36 C remaining must be forced to 0");
        assert_eq!(rv.remaining, 0, "#36 Rust remaining must be forced to 0");
        assert_eq!(cstr(cp), cstr(rp));
        // the dedicated block became the head, and its `next` is NULL
        assert_eq!(*(cv.storage as *const usize), 0, "#36 C next must be NULL");
        assert_eq!(*(rv.storage as *const usize), 0, "#36 Rust next must be NULL");

        // #37: another oversized string, now that storage != NULL -> spliced
        // *after* the head, and `remaining` is left untouched
        let cp2 = (b.c.stralloc)(ca.ptr(), bigb.cptr());
        let rp2 = (b.r.stralloc)(ra.ptr(), bigb.cptr());
        assert_eq!(cv.remaining, 0, "#37 C remaining unchanged");
        assert_eq!(rv.remaining, 0, "#37 Rust remaining unchanged");
        assert_eq!(cstr(cp2), cstr(rp2));
        // head->next now points at the new block
        let cnext = *(cv.storage as *const usize);
        let rnext = *(rv.storage as *const usize);
        assert_ne!(cnext, 0, "#37 C must splice after the head");
        assert_ne!(rnext, 0, "#37 Rust must splice after the head");
        assert_eq!(cp2 as usize, cnext + 8, "#37 C returned block start");
        assert_eq!(rp2 as usize, rnext + 8, "#37 Rust returned block start");
        (b.c.strreset)(ca.ptr());
        (b.r.strreset)(ra.ptr());

        // #40: strreset on an already-empty arena
        for _ in 0..3 {
            (b.c.strreset)(ca.ptr());
            (b.r.strreset)(ra.ptr());
            assert_eq!(cv.remaining, 0);
            assert_eq!(rv.remaining, 0);
            assert_eq!(cv.block, 0);
            assert_eq!(rv.block, 0);
            assert_eq!(cv.mode, 0);
            assert_eq!(rv.mode, 0);
            assert!(cv.storage.is_null() && rv.storage.is_null());
        }
    }
}

// ===========================================================================
// #41 - #45 — hash-function boundaries (see also phase_b_low.rs)
// ===========================================================================

#[test]
fn err_41_to_45_hash_boundaries() {
    let (_g, b) = both();
    let seeds = [0usize, 1, usize::MAX, 0x3141_5926];
    unsafe {
        // #44: NULL pointer with len == 0 must not be dereferenced
        for &s in &seeds {
            assert_eq!(
                (b.c.hash_bytes)(std::ptr::null_mut(), 0, s),
                (b.r.hash_bytes)(std::ptr::null_mut(), 0, s),
                "#44 seed={s:#x}"
            );
        }
        // #42: len == 0 for a real buffer agrees with the NULL case
        let buf = CBuf::new(&[0xAAu8; 64]);
        for &s in &seeds {
            let z = (b.c.hash_bytes)(std::ptr::null_mut(), 0, s);
            assert_eq!((b.c.hash_bytes)(buf.ptr(), 0, s), z);
            assert_eq!((b.r.hash_bytes)(buf.ptr(), 0, s), z, "#42 seed={s:#x}");
        }
        // #43: every tail length
        for len in 1..8usize {
            for &s in &seeds {
                assert_eq!(
                    (b.c.hash_bytes)(buf.ptr(), len, s),
                    (b.r.hash_bytes)(buf.ptr(), len, s),
                    "#43 len={len}"
                );
            }
        }
        // #45: high bit set at offsets 3 and 7 of each word
        for len in 1..=32usize {
            let data: Vec<u8> = (0..len)
                .map(|i| if i % 4 == 3 { 0xFF } else { i as u8 })
                .collect();
            let d = CBuf::new(&data);
            for &s in &seeds {
                assert_eq!(
                    (b.c.hash_bytes)(d.ptr(), len, s),
                    (b.r.hash_bytes)(d.ptr(), len, s),
                    "#45 len={len} seed={s:#x}"
                );
            }
        }
        // #41: the empty string
        let empty = CBuf::new(b"\0");
        for &s in &seeds {
            assert_eq!(
                (b.c.hash_string)(empty.cptr(), s),
                (b.r.hash_string)(empty.cptr(), s),
                "#41 seed={s:#x}"
            );
        }
    }
}

// ===========================================================================
// #46, #47 — sh_geti boundaries (stdout compared in phase_b_shgeti.rs)
// ===========================================================================

#[test]
fn err_46_47_sh_geti_boundaries() {
    let (_g, b) = both();
    for num in [i32::MIN, -1000, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9] {
        seed_both(b, 0x3141_5926);
        let cout = capture_stdout(|| unsafe { (b.c.sh_geti)(num) });
        seed_both(b, 0x3141_5926);
        let rout = capture_stdout(|| unsafe { (b.r.sh_geti)(num) });
        assert_eq!(cout, rout, "sh_geti({num})");
        if num <= 0 {
            assert!(cout.is_empty(), "sh_geti({num}) must print nothing");
        }
    }
}
