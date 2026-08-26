//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of ERRORS.md that is reachable through the public ABI.
//! Each constructs the exact invalid input/condition, calls BOTH shared objects
//! and asserts they produce the SAME sentinel / error / abort — not merely that
//! "both failed somehow".

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::ptr::null_mut;

// ===========================================================================
// ERRORS row 1 / 2 — stbds_arrgrowf early-out and size_t wrap-around
// ===========================================================================

#[test]
fn err01_arrgrowf_no_grow_returns_input() {
    let (_g, c, r) = scenario(0x3141_5926);
    unsafe {
        // NULL in, nothing requested -> NULL out, no allocation at all
        for es in [0usize, 1, 4, 16, 4096] {
            let cp = (c.arrgrowf)(null_mut(), es, 0, 0);
            let rp = (r.arrgrowf)(null_mut(), es, 0, 0);
            assert!(cp.is_null(), "C arrgrowf(NULL,{es},0,0) must return NULL");
            assert!(rp.is_null(), "RUST arrgrowf(NULL,{es},0,0) must return NULL");
        }
        // existing array, request already satisfied -> identical pointer back
        for es in [1usize, 8, 17] {
            let ca = (c.arrgrowf)(null_mut(), es, 0, 4);
            let ra = (r.arrgrowf)(null_mut(), es, 0, 4);
            for mc in [0usize, 1, 2, 3, 4] {
                assert_eq!((c.arrgrowf)(ca, es, 0, mc), ca, "C must return input");
                assert_eq!((r.arrgrowf)(ra, es, 0, mc), ra, "RUST must return input");
            }
            same(
                &format!("arrgrowf no-grow es={es}"),
                &dump_array(ca as *mut u8, es, KeyKind::Raw),
                &dump_array(ra as *mut u8, es, KeyKind::Raw),
            );
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}

#[test]
fn err02_arrgrowf_addlen_overflow() {
    let (_g, c, r) = scenario(0x3141_5926);
    unsafe {
        // arrlen(NULL) + SIZE_MAX wraps to SIZE_MAX-1... with min_cap 0 the
        // comparison `min_len > min_cap` holds, so min_cap becomes huge and the
        // realloc fails -> both would crash. The *well-defined* wrap is the one
        // that lands back on the early-out: addlen == SIZE_MAX and an array of
        // length 1 -> min_len == 0 -> min_cap stays 0 -> `0 <= cap` -> return a.
        for es in [1usize, 8, 16] {
            let ca = (c.arrgrowf)(null_mut(), es, 0, 4);
            let ra = (r.arrgrowf)(null_mut(), es, 0, 4);
            (*(ca as *mut u8).sub(HEADER_SIZE).cast::<ArrayHeader>()).length = 1;
            (*(ra as *mut u8).sub(HEADER_SIZE).cast::<ArrayHeader>()).length = 1;
            let cp = (c.arrgrowf)(ca, es, usize::MAX, 0);
            let rp = (r.arrgrowf)(ra, es, usize::MAX, 0);
            assert_eq!(cp, ca, "C: wrapped min_len must hit the early-out");
            assert_eq!(rp, ra, "RUST: wrapped min_len must hit the early-out");
            canon_elements(ca as *mut u8, es, 0, 0xC3);
            canon_elements(ra as *mut u8, es, 0, 0xC3);
            same(
                &format!("arrgrowf addlen wrap es={es}"),
                &dump_array(ca as *mut u8, es, KeyKind::Raw),
                &dump_array(ra as *mut u8, es, KeyKind::Raw),
            );
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}

// ===========================================================================
// ERRORS row 6 / 8 — degenerate hash inputs
// ===========================================================================

#[test]
fn err06_hash_string_empty() {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut empty = [0u8; 1];
    for &seed in &[0usize, 1, 2, usize::MAX, 0x3141_5926] {
        let cv = unsafe { (c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed) };
        let rv = unsafe { (r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed) };
        assert_eq!(cv, rv, "hash_string(\"\", {seed:#x})");
    }
}

#[test]
fn err08_hash_bytes_zero_len_null_ptr() {
    let (_g, c, r) = scenario(0x3141_5926);
    for &seed in &[0usize, 1, 2, usize::MAX, 0x3141_5926] {
        let cv = unsafe { (c.hash_bytes)(null_mut(), 0, seed) };
        let rv = unsafe { (r.hash_bytes)(null_mut(), 0, seed) };
        assert_eq!(cv, rv, "hash_bytes(NULL, 0, {seed:#x})");
    }
}

// ===========================================================================
// ERRORS row 10 / 11 — stbds_hmfree_func rejections
// ===========================================================================

#[test]
fn err10_hmfree_null_is_a_noop() {
    let (_g, c, r) = scenario(0x3141_5926);
    unsafe {
        for es in [0usize, 1, 16, 4096] {
            (c.hmfree_func)(null_mut(), es);
            (r.hmfree_func)(null_mut(), es);
        }
    }
}

#[test]
fn err11_hmfree_without_hash_table() {
    let (_g, c, r) = scenario(0x3141_5926);
    unsafe {
        for es in [1usize, 16, 20] {
            let ca = (c.arrgrowf)(null_mut(), es, 0, 4);
            let ra = (r.arrgrowf)(null_mut(), es, 0, 4);
            assert!((*(ca as *mut u8).sub(HEADER_SIZE).cast::<ArrayHeader>())
                .hash_table
                .is_null());
            assert!((*(ra as *mut u8).sub(HEADER_SIZE).cast::<ArrayHeader>())
                .hash_table
                .is_null());
            (c.hmfree_func)(ca, es);
            (r.hmfree_func)(ra, es);
        }
    }
}

// ===========================================================================
// ERRORS rows 12–17 — lookup misses and the NULL/index-less map
// ===========================================================================

#[test]
fn err12_get_missing_key_binary_and_string() {
    let (_g, c, r) = scenario(0x3141_5926);
    let es = 16usize;
    // binary
    unsafe {
        let mut present: Vec<[u8; 8]> = (0..10u64).map(|i| (i * 37 + 11).to_le_bytes()).collect();
        let mut ct = null_mut::<c_void>();
        let mut rt = null_mut::<c_void>();
        for k in present.iter_mut() {
            ct = (c.hmput_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
            rt = (r.hmput_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
        }
        for miss in [0u64, u64::MAX, 0xDEAD_BEEF, 12345] {
            let mut k = miss.to_le_bytes();
            ct = (c.hmget_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
            rt = (r.hmget_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
            let ci = (*(ct as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
            let ri = (*(rt as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
            assert_eq!(ci, -1, "C: missing binary key must yield STBDS_INDEX_EMPTY");
            assert_eq!(ri, -1, "RUST: missing binary key must yield STBDS_INDEX_EMPTY");
        }
        (c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
        (r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
    }
    // string
    unsafe {
        let mut keys: Vec<Vec<u8>> = (0..10)
            .map(|i| {
                let mut s = format!("present-{i}").into_bytes();
                s.push(0);
                s
            })
            .collect();
        let mut ct = null_mut::<c_void>();
        let mut rt = null_mut::<c_void>();
        for k in keys.iter_mut() {
            ct = (c.hmput_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
            rt = (r.hmput_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
        }
        for miss in ["", "absent", "present-99", "PRESENT-0"] {
            let mut k = miss.as_bytes().to_vec();
            k.push(0);
            ct = (c.hmget_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
            rt = (r.hmget_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
            let ci = (*(ct as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
            let ri = (*(rt as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
            assert_eq!(ci, -1, "C: missing string key {miss:?}");
            assert_eq!(ri, -1, "RUST: missing string key {miss:?}");
        }
        (c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
        (r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
    }
}

#[test]
fn err13_17_get_on_null_map_bootstraps_and_returns_minus_one() {
    for &mode in &[STBDS_HM_BINARY, STBDS_HM_STRING, STBDS_HM_PTR_TO_STRING, -1, 99] {
        let (_g, c, r) = scenario(0x3141_5926);
        let es = 16usize;
        let mut key = *b"whatever\0\0\0\0\0\0\0\0";
        unsafe {
            // hmget_key_ts: *temp must be set to STBDS_INDEX_EMPTY, and the
            // returned pointer must be a freshly bootstrapped 1-element array
            let mut ctmp: isize = 0x1234;
            let mut rtmp: isize = 0x1234;
            let ct = (c.hmget_key_ts)(
                null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                8,
                &mut ctmp,
                mode,
            ) as *mut u8;
            let rt = (r.hmget_key_ts)(
                null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                8,
                &mut rtmp,
                mode,
            ) as *mut u8;
            assert_eq!(ctmp, -1, "C hmget_key_ts(NULL) temp, mode={mode}");
            assert_eq!(rtmp, -1, "RUST hmget_key_ts(NULL) temp, mode={mode}");
            same(
                &format!("hmget_key_ts(NULL) mode={mode}"),
                &dump_map(ct, es, KeyKind::Raw),
                &dump_map(rt, es, KeyKind::Raw),
            );
            (c.hmfree_func)(ct.sub(es) as *mut c_void, es);
            (r.hmfree_func)(rt.sub(es) as *mut c_void, es);

            // hmget_key: additionally writes the sentinel into header->temp
            let ct = (c.hmget_key)(null_mut(), es, key.as_mut_ptr() as *mut c_void, 8, mode)
                as *mut u8;
            let rt = (r.hmget_key)(null_mut(), es, key.as_mut_ptr() as *mut c_void, 8, mode)
                as *mut u8;
            let ci = (*ct.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
            let ri = (*rt.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
            assert_eq!(ci, -1, "C hmget_key(NULL) header temp, mode={mode}");
            assert_eq!(ri, -1, "RUST hmget_key(NULL) header temp, mode={mode}");
            same(
                &format!("hmget_key(NULL) mode={mode}"),
                &dump_map(ct, es, KeyKind::Raw),
                &dump_map(rt, es, KeyKind::Raw),
            );
            (c.hmfree_func)(ct.sub(es) as *mut c_void, es);
            (r.hmfree_func)(rt.sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn err14_get_on_map_without_hash_table() {
    let (_g, c, r) = scenario(0x3141_5926);
    let es = 16usize;
    let mut key = *b"whatever\0\0\0\0\0\0\0\0";
    unsafe {
        let ct = (c.hmput_default)(null_mut(), es) as *mut u8;
        let rt = (r.hmput_default)(null_mut(), es) as *mut u8;
        let mut ctmp: isize = 0x1234;
        let mut rtmp: isize = 0x1234;
        let ct2 = (c.hmget_key_ts)(
            ct as *mut c_void,
            es,
            key.as_mut_ptr() as *mut c_void,
            8,
            &mut ctmp,
            STBDS_HM_BINARY,
        ) as *mut u8;
        let rt2 = (r.hmget_key_ts)(
            rt as *mut c_void,
            es,
            key.as_mut_ptr() as *mut c_void,
            8,
            &mut rtmp,
            STBDS_HM_BINARY,
        ) as *mut u8;
        assert_eq!(ct2, ct, "C must return the map unchanged");
        assert_eq!(rt2, rt, "RUST must return the map unchanged");
        assert_eq!(ctmp, -1);
        assert_eq!(rtmp, -1);
        same(
            "hmget_key_ts on index-less map",
            &dump_map(ct2, es, KeyKind::Raw),
            &dump_map(rt2, es, KeyKind::Raw),
        );
        (c.hmfree_func)(ct.sub(es) as *mut c_void, es);
        (r.hmfree_func)(rt.sub(es) as *mut c_void, es);
    }
}

// ===========================================================================
// ERRORS rows 18–20 — stbds_hmput_default
// ===========================================================================

#[test]
fn err18_19_20_hmput_default_paths() {
    let (_g, c, r) = scenario(0x3141_5926);
    unsafe {
        for es in [1usize, 8, 16, 20, 32] {
            // (18) NULL in
            let ct = (c.hmput_default)(null_mut(), es) as *mut u8;
            let rt = (r.hmput_default)(null_mut(), es) as *mut u8;
            same(
                &format!("hmput_default(NULL,{es})"),
                &dump_map(ct, es, KeyKind::Raw),
                &dump_map(rt, es, KeyKind::Raw),
            );
            assert_eq!((*ct.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).length, 1);
            // (20) already non-empty -> unchanged
            let ct2 = (c.hmput_default)(ct as *mut c_void, es) as *mut u8;
            let rt2 = (r.hmput_default)(rt as *mut c_void, es) as *mut u8;
            assert_eq!(ct2, ct, "C: hmput_default must be idempotent");
            assert_eq!(rt2, rt, "RUST: hmput_default must be idempotent");
            // (19) length forced back to 0 -> re-init
            (*ct.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).length = 0;
            (*rt.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).length = 0;
            let ct3 = (c.hmput_default)(ct as *mut c_void, es) as *mut u8;
            let rt3 = (r.hmput_default)(rt as *mut c_void, es) as *mut u8;
            same(
                &format!("hmput_default(length==0,{es})"),
                &dump_map(ct3, es, KeyKind::Raw),
                &dump_map(rt3, es, KeyKind::Raw),
            );
            assert_eq!((*ct3.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).length, 1);
            (c.hmfree_func)(ct3.sub(es) as *mut c_void, es);
            (r.hmfree_func)(rt3.sub(es) as *mut c_void, es);
        }
    }
}

// ===========================================================================
// ERRORS rows 21 / 22 / 25 — stbds_hmput_key bootstrap, string.mode init,
// duplicate keys
// ===========================================================================

#[test]
fn err21_22_put_on_null_map_sets_string_mode() {
    for &(mode, want_mode) in &[
        (STBDS_HM_BINARY, SH_NONE as u8),
        (-1i32, SH_NONE as u8),
        (i32::MIN, SH_NONE as u8),
        (STBDS_HM_STRING, SH_DEFAULT as u8),
        (STBDS_HM_PTR_TO_STRING, SH_DEFAULT as u8),
        (99, SH_DEFAULT as u8),
        (i32::MAX, SH_DEFAULT as u8),
    ] {
        let (_g, c, r) = scenario(0x3141_5926);
        let es = 16usize;
        let mut key = *b"a-key-here\0\0\0\0\0\0";
        unsafe {
            let ct = (c.hmput_key)(null_mut(), es, key.as_mut_ptr() as *mut c_void, 8, mode)
                as *mut u8;
            let rt = (r.hmput_key)(null_mut(), es, key.as_mut_ptr() as *mut c_void, 8, mode)
                as *mut u8;
            let kk = if mode >= STBDS_HM_STRING { KeyKind::StrPtr } else { KeyKind::Raw };
            canon_pair(ct as *mut c_void, rt as *mut c_void, es, 8);
            same(
                &format!("put on NULL map mode={mode}"),
                &dump_map(ct, es, kk),
                &dump_map(rt, es, kk),
            );
            for (nm, t) in [("C", ct), ("RUST", rt)] {
                let ht = (*t.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).hash_table
                    as *const HashIndex;
                assert_eq!(
                    (*ht).string.mode, want_mode,
                    "{nm}: mode={mode} must yield string.mode={want_mode}"
                );
                assert_eq!((*ht).slot_count, 8, "{nm}: first index has 8 slots");
            }
            (c.hmfree_func)(ct.sub(es) as *mut c_void, es);
            (r.hmfree_func)(rt.sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn err25_put_duplicate_key_does_not_insert() {
    let (_g, c, r) = scenario(0x3141_5926);
    let es = 16usize;
    unsafe {
        let mut ct = null_mut::<c_void>();
        let mut rt = null_mut::<c_void>();
        let mut key = 0x0102_0304_0506_0708u64.to_le_bytes();
        for round in 0..20 {
            ct = (c.hmput_key)(ct, es, key.as_mut_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
            rt = (r.hmput_key)(rt, es, key.as_mut_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
            let ch = (ct as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>();
            let rh = (rt as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>();
            assert_eq!((*ch).temp, 0, "round {round}: C index of the single key");
            assert_eq!((*rh).temp, 0, "round {round}: RUST index of the single key");
            assert_eq!((*ch).length, 2, "round {round}: C length must stay 2");
            assert_eq!((*rh).length, 2, "round {round}: RUST length must stay 2");
            let cht = (*ch).hash_table as *const HashIndex;
            let rht = (*rh).hash_table as *const HashIndex;
            assert_eq!((*cht).used_count, 1, "round {round}: C used_count");
            assert_eq!((*rht).used_count, 1, "round {round}: RUST used_count");
        }
        (c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
        (r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
    }
}

/// The duplicate-key hit in the *wrapped-around* probe loop (lib.c:746-759)
/// does NOT refresh `temp_key`, unlike the first loop (lib.c:728-743). Forcing
/// a probe start of `pos & 7 == 5` puts keys #3 and #4 into slots 0 and 1, i.e.
/// into the wrapped part of the bucket, so re-putting them must leave
/// `temp_key` stale.
///
/// Only 5 keys are used so that `used_count` stays below
/// `used_count_threshold == 6`: a rehash would replace the hash index and leave
/// `temp_key` genuinely indeterminate (uninitialised malloc bytes), which is
/// not comparable in either direction.
#[test]
fn err25b_duplicate_key_in_wrapped_probe_loop() {
    const SEED: usize = 0x3141_5926;
    let (_g, c, r) = scenario(SEED);
    let es = 16usize;
    let mut keys: Vec<Vec<u8>> = Vec::new();
    let mut cand = 0u64;
    while keys.len() < 5 {
        let mut s = format!("w{cand}").into_bytes();
        s.push(0);
        cand += 1;
        let mut h = unsafe { (c.hash_string)(s.as_ptr() as *mut c_char, SEED) };
        if h < 2 {
            h += 2;
        }
        if h & 7 == 5 {
            keys.push(s);
        }
        assert!(cand < 10_000_000);
    }
    unsafe {
        let mut ct = null_mut::<c_void>();
        let mut rt = null_mut::<c_void>();
        for k in keys.iter_mut() {
            ct = (c.hmput_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
            rt = (r.hmput_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
        }
        // no rehash may have happened
        let cht = (*(ct as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).hash_table
            as *const HashIndex;
        assert_eq!((*cht).slot_count, 8);
        assert_eq!((*cht).used_count, 5);

        let mut stale_seen = 0usize;
        let mut fresh_seen = 0usize;
        for (i, k) in keys.iter_mut().enumerate() {
            ct = (c.hmput_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
            rt = (r.hmput_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
            let ck = cstr_to_vec(temp_key(ct as *mut u8, es));
            let rk = cstr_to_vec(temp_key(rt as *mut u8, es));
            assert_eq!(
                ck, rk,
                "temp_key after re-put of key #{i} ({:?})",
                String::from_utf8_lossy(&k[..k.len() - 1])
            );
            if ck == k[..k.len() - 1].to_vec() {
                fresh_seen += 1;
            } else {
                stale_seen += 1;
            }
            canon_pair(ct, rt, es, 8);
            same(
                &format!("wrapped-loop duplicate put #{i}"),
                &dump_map(ct as *mut u8, es, KeyKind::StrPtr),
                &dump_map(rt as *mut u8, es, KeyKind::StrPtr),
            );
        }
        assert!(fresh_seen > 0, "the first probe loop (which sets temp_key) never ran");
        assert!(
            stale_seen > 0,
            "the wrapped-around probe loop (which leaves temp_key stale) never ran"
        );
        (c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
        (r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
    }
}

// ===========================================================================
// ERRORS row 26 — out-of-enum-range mode for stbds_shmode_func
// ===========================================================================

#[test]
fn err26_shmode_out_of_range_enum() {
    // C enums accept any int. Every one of these has NO valid variant.
    for mode in [
        4i32, 5, 6, 7, 255, 256, 257, 258, 259, 511, 512, 0x1_0000, 0x1_0001, 0x1_0002, 0x1_0003,
        -1, -2, -3, -4, -255, -256, i32::MIN, i32::MAX,
    ] {
        let (_g, c, r) = scenario(0x3141_5926);
        let es = 16usize;
        unsafe {
            let ct = (c.shmode_func)(es, mode) as *mut u8;
            let rt = (r.shmode_func)(es, mode) as *mut u8;
            same(
                &format!("shmode_func(16,{mode})"),
                &dump_map(ct, es, KeyKind::Raw),
                &dump_map(rt, es, KeyKind::Raw),
            );
            for (nm, t) in [("C", ct), ("RUST", rt)] {
                let ht = (*t.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).hash_table
                    as *const HashIndex;
                assert_eq!(
                    (*ht).string.mode,
                    mode as u8,
                    "{nm}: shmode_func must store (unsigned char){mode}"
                );
            }
            // A put on such a map must take the `default:` (memcpy) arm unless
            // the truncated byte happens to be 1/2/3.
            let trunc = mode as u8;
            if trunc != 1 && trunc != 2 && trunc != 3 {
                let mut key = 0x1122_3344_5566_7788u64.to_le_bytes();
                let ct2 =
                    (c.hmput_key)(ct as *mut c_void, es, key.as_mut_ptr() as *mut c_void, 8, 0)
                        as *mut u8;
                let rt2 =
                    (r.hmput_key)(rt as *mut c_void, es, key.as_mut_ptr() as *mut c_void, 8, 0)
                        as *mut u8;
                canon_pair(ct2 as *mut c_void, rt2 as *mut c_void, es, 8);
                same(
                    &format!("put on shmode({mode}) map"),
                    &dump_map(ct2, es, KeyKind::Raw),
                    &dump_map(rt2, es, KeyKind::Raw),
                );
                (c.hmfree_func)(ct2.sub(es) as *mut c_void, es);
                (r.hmfree_func)(rt2.sub(es) as *mut c_void, es);
            } else {
                (c.hmfree_func)(ct.sub(es) as *mut c_void, es);
                (r.hmfree_func)(rt.sub(es) as *mut c_void, es);
            }
        }
    }
}

// ===========================================================================
// ERRORS rows 27–29 / 34 — stbds_hmdel_key rejections
// ===========================================================================

#[test]
fn err27_del_on_null_map_returns_null() {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut key = 0x1234_5678_9ABC_DEF0u64.to_le_bytes();
    unsafe {
        for es in [1usize, 8, 16] {
            for keyoffset in [0usize, 4, 8] {
                for &mode in &[STBDS_HM_BINARY, STBDS_HM_STRING, -1, 99] {
                    let cp = (c.hmdel_key)(
                        null_mut(),
                        es,
                        key.as_mut_ptr() as *mut c_void,
                        8,
                        keyoffset,
                        mode,
                    );
                    let rp = (r.hmdel_key)(
                        null_mut(),
                        es,
                        key.as_mut_ptr() as *mut c_void,
                        8,
                        keyoffset,
                        mode,
                    );
                    assert!(cp.is_null(), "C hmdel_key(NULL) must return NULL");
                    assert!(rp.is_null(), "RUST hmdel_key(NULL) must return NULL");
                }
            }
        }
    }
}

#[test]
fn err28_del_on_map_without_hash_table() {
    let (_g, c, r) = scenario(0x3141_5926);
    let es = 16usize;
    let mut key = 0x1234_5678_9ABC_DEF0u64.to_le_bytes();
    unsafe {
        let ct = (c.hmput_default)(null_mut(), es) as *mut u8;
        let rt = (r.hmput_default)(null_mut(), es) as *mut u8;
        // poison temp so we can see it being cleared
        (*ct.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp = 0x7777;
        (*rt.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp = 0x7777;
        let cp = (c.hmdel_key)(ct as *mut c_void, es, key.as_mut_ptr() as *mut c_void, 8, 0, 0)
            as *mut u8;
        let rp = (r.hmdel_key)(rt as *mut c_void, es, key.as_mut_ptr() as *mut c_void, 8, 0, 0)
            as *mut u8;
        assert_eq!(cp, ct, "C must return the map unchanged");
        assert_eq!(rp, rt, "RUST must return the map unchanged");
        assert_eq!((*cp.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp, 0, "C temp cleared");
        assert_eq!((*rp.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp, 0, "RUST temp cleared");
        same(
            "hmdel_key on index-less map",
            &dump_map(cp, es, KeyKind::Raw),
            &dump_map(rp, es, KeyKind::Raw),
        );
        (c.hmfree_func)(cp.sub(es) as *mut c_void, es);
        (r.hmfree_func)(rp.sub(es) as *mut c_void, es);
    }
}

#[test]
fn err29_del_missing_key_leaves_map_untouched() {
    let (_g, c, r) = scenario(0x3141_5926);
    let es = 16usize;
    unsafe {
        let mut ct = null_mut::<c_void>();
        let mut rt = null_mut::<c_void>();
        for i in 0..12u64 {
            let mut k = (i * 91 + 5).to_le_bytes();
            ct = (c.hmput_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
            rt = (r.hmput_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
        }
        canon_pair(ct, rt, es, 8);
        // hmdel_key legitimately writes `stbds_temp(arr) = 0` (lib.c:815), so
        // normalise that one field before taking the reference snapshot.
        (*(ct as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp = 0;
        (*(rt as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp = 0;
        let before_c = dump_map(ct as *mut u8, es, KeyKind::Raw);
        for miss in [0u64, u64::MAX, 7, 999_999] {
            let mut k = miss.to_le_bytes();
            let cp = (c.hmdel_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, 0, STBDS_HM_BINARY);
            let rp = (r.hmdel_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, 0, STBDS_HM_BINARY);
            assert_eq!(cp, ct, "C: missing key must return the map unchanged");
            assert_eq!(rp, rt, "RUST: missing key must return the map unchanged");
            let ch = (ct as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>();
            let rh = (rt as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>();
            assert_eq!((*ch).temp, 0, "C: hmdel of a missing key yields 0");
            assert_eq!((*rh).temp, 0, "RUST: hmdel of a missing key yields 0");
            let now_c = dump_map(ct as *mut u8, es, KeyKind::Raw);
            let now_r = dump_map(rt as *mut u8, es, KeyKind::Raw);
            same("hmdel missing key", &now_c, &now_r);
            assert_eq!(before_c, now_c, "the map must not change");
        }
        (c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
        (r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
    }
}

// ===========================================================================
// ERRORS row 39 — arena block-size saturation
// ===========================================================================

#[test]
fn err39_stralloc_block_counter_saturation() {
    // blocksize == 512 << (block>>1); `++a->block` only happens while
    // blocksize < 1<<20, i.e. while block <= 21.
    for (block, expect_next) in [
        (0u8, 1u8),
        (1, 2),
        (19, 20),
        (20, 21),
        (21, 22),
        (22, 22), // 512<<11 == 1<<20 -> saturated
        (23, 23),
        (24, 24),
    ] {
        let (_g, c, r) = scenario(0x3141_5926);
        let mut s = b"short\0".to_vec();
        unsafe {
            let mut ca = Arena::zeroed();
            let mut ra = Arena::zeroed();
            ca.block = block;
            ra.block = block;
            let cp = (c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
            let rp = (r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr_to_vec(cp), b"short".to_vec());
            assert_eq!(cstr_to_vec(rp), b"short".to_vec());
            assert_eq!(ca.block, expect_next, "C block counter for start={block}");
            assert_eq!(ra.block, expect_next, "RUST block counter for start={block}");
            same(
                &format!("stralloc saturation block={block}"),
                &dump_arena(&ca),
                &dump_arena(&ra),
            );
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
        }
    }
}

// ===========================================================================
// ERRORS row 43 — stbds_strreset on an empty arena
// ===========================================================================

#[test]
fn err43_strreset_empty_arena() {
    let (_g, c, r) = scenario(0x3141_5926);
    unsafe {
        let mut ca = Arena::zeroed();
        let mut ra = Arena::zeroed();
        // also poison the scalar fields: strreset must zero all of them
        ca.remaining = 12345;
        ca.block = 7;
        ca.mode = 3;
        ra.remaining = 12345;
        ra.block = 7;
        ra.mode = 3;
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
        same("strreset on empty arena", &dump_arena(&ca), &dump_arena(&ra));
        assert_eq!(dump_arena(&ca), dump_arena(&Arena::zeroed()));
        // and again, twice in a row
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
        same("strreset twice", &dump_arena(&ca), &dump_arena(&ra));
    }
}

// ===========================================================================
// ERRORS row 45 / 46 — out-of-enum-range `mode` across the FFI boundary
// ===========================================================================

#[test]
fn err45_mode_out_of_range_enum_routing() {
    // `stbds_is_key_equal` selects strcmp with `mode >= STBDS_HM_STRING`, so
    // every int is accepted and routed. Verify the *routing* is identical by
    // running a whole put/get/del cycle for each out-of-range value.
    let string_modes: &[c_int] = &[1, 2, 3, 4, 5, 99, 0x7FFF, i32::MAX];
    let binary_modes: &[c_int] = &[0, -1, -2, -3, -99, -0x7FFF, i32::MIN];
    let es = 16usize;

    for &mode in string_modes {
        let (_g, c, r) = scenario(0x3141_5926);
        let mut keys: Vec<Vec<u8>> = (0..8)
            .map(|i| {
                let mut s = format!("oor-key-{i}").into_bytes();
                s.push(0);
                s
            })
            .collect();
        unsafe {
            let mut ct = null_mut::<c_void>();
            let mut rt = null_mut::<c_void>();
            for k in keys.iter_mut() {
                ct = (c.hmput_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, mode);
                rt = (r.hmput_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, mode);
                canon_pair(ct, rt, es, 8);
                same(
                    &format!("put mode={mode}"),
                    &dump_map(ct as *mut u8, es, KeyKind::StrPtr),
                    &dump_map(rt as *mut u8, es, KeyKind::StrPtr),
                );
            }
            for k in keys.iter_mut() {
                ct = (c.hmget_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, mode);
                rt = (r.hmget_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, mode);
                let ci = (*(ct as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
                let ri = (*(rt as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
                assert_eq!(ci, ri, "get index mode={mode}");
                assert!(ci >= 0, "string-path mode={mode} must find the key");
            }
            // delete in reverse order (old_index == final_index) so the
            // mode-specific re-find at lib.c:842 is not needed
            for k in keys.iter_mut().rev() {
                ct = (c.hmdel_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, 0, mode);
                rt = (r.hmdel_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, 0, mode);
                same(
                    &format!("del mode={mode}"),
                    &dump_map(ct as *mut u8, es, KeyKind::StrPtr),
                    &dump_map(rt as *mut u8, es, KeyKind::StrPtr),
                );
            }
            (c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
        }
    }

    for &mode in binary_modes {
        let (_g, c, r) = scenario(0x3141_5926);
        unsafe {
            let mut ct = null_mut::<c_void>();
            let mut rt = null_mut::<c_void>();
            let mut ks: Vec<[u8; 8]> = (0..8u64).map(|i| (i * 131 + 17).to_le_bytes()).collect();
            for k in ks.iter_mut() {
                ct = (c.hmput_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, mode);
                rt = (r.hmput_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, mode);
                canon_pair(ct, rt, es, 8);
                same(
                    &format!("put mode={mode}"),
                    &dump_map(ct as *mut u8, es, KeyKind::Raw),
                    &dump_map(rt as *mut u8, es, KeyKind::Raw),
                );
            }
            for k in ks.iter_mut() {
                ct = (c.hmget_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, mode);
                rt = (r.hmget_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, mode);
                let ci = (*(ct as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
                let ri = (*(rt as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
                assert_eq!(ci, ri, "get index mode={mode}");
                assert!(ci >= 0, "binary-path mode={mode} must find the key");
            }
            for k in ks.iter_mut() {
                ct = (c.hmdel_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, 0, mode);
                rt = (r.hmdel_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, 0, mode);
                same(
                    &format!("del mode={mode}"),
                    &dump_map(ct as *mut u8, es, KeyKind::Raw),
                    &dump_map(rt as *mut u8, es, KeyKind::Raw),
                );
            }
            (c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// ===========================================================================
// Generic boundaries: zero-size keys, zero-size elements
// ===========================================================================

#[test]
fn err_b1_zero_keysize() {
    // keysize == 0: siphash of nothing is a constant, and memcmp of 0 bytes is
    // always "equal", so the map can only ever hold one entry.
    let (_g, c, r) = scenario(0x3141_5926);
    let es = 16usize;
    unsafe {
        let mut ct = null_mut::<c_void>();
        let mut rt = null_mut::<c_void>();
        for i in 0..6u64 {
            let mut k = (i + 1).to_le_bytes();
            ct = (c.hmput_key)(ct, es, k.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
            rt = (r.hmput_key)(rt, es, k.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
            canon_pair(ct, rt, es, 0);
            same(
                "put keysize=0",
                &dump_map(ct as *mut u8, es, KeyKind::Raw),
                &dump_map(rt as *mut u8, es, KeyKind::Raw),
            );
        }
        let ch = (ct as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>();
        assert_eq!((*ch).length, 2, "keysize=0 collapses every key into one");
        let mut k = 0u64.to_le_bytes();
        ct = (c.hmdel_key)(ct, es, k.as_mut_ptr() as *mut c_void, 0, 0, STBDS_HM_BINARY);
        rt = (r.hmdel_key)(rt, es, k.as_mut_ptr() as *mut c_void, 0, 0, STBDS_HM_BINARY);
        same(
            "del keysize=0",
            &dump_map(ct as *mut u8, es, KeyKind::Raw),
            &dump_map(rt as *mut u8, es, KeyKind::Raw),
        );
        (c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
        (r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
    }
}

#[test]
fn err_b2_zero_elemsize_array() {
    // elemsize == 0: the payload is empty, only the header is allocated.
    let (_g, c, r) = scenario(0x3141_5926);
    unsafe {
        for (addlen, min_cap) in [(0usize, 4usize), (1, 0), (7, 0), (0, 1000)] {
            let ca = (c.arrgrowf)(null_mut(), 0, addlen, min_cap) as *mut u8;
            let ra = (r.arrgrowf)(null_mut(), 0, addlen, min_cap) as *mut u8;
            same(
                &format!("arrgrowf elemsize=0 addlen={addlen} min_cap={min_cap}"),
                &dump_array(ca, 0, KeyKind::Raw),
                &dump_array(ra, 0, KeyKind::Raw),
            );
            (c.arrfreef)(ca as *mut c_void);
            (r.arrfreef)(ra as *mut c_void);
        }
    }
}

#[test]
fn err_b3_keyoffset_one_past_the_key() {
    // keyoffset values that are valid ints but do not match the offset the key
    // was written at: the first find_slot compares the wrong bytes.
    let es = 16usize;
    for keyoffset in [0usize, 1, 4, 7, 8] {
        let (_g, c, r) = scenario(0x3141_5926);
        unsafe {
            let mut ct = null_mut::<c_void>();
            let mut rt = null_mut::<c_void>();
            let mut ks: Vec<[u8; 8]> = (0..10u64).map(|i| (i * 77 + 3).to_le_bytes()).collect();
            for k in ks.iter_mut() {
                ct = (c.hmput_key)(ct, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
                rt = (r.hmput_key)(rt, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
            }
            // make the tail of every element deterministic
            for i in 0..10usize {
                for j in 8..es {
                    *(ct as *mut u8).add(i * es + j) = (i as u8) ^ (j as u8);
                    *(rt as *mut u8).add(i * es + j) = (i as u8) ^ (j as u8);
                }
            }
            for k in ks.iter_mut() {
                let cp = (c.hmdel_key)(
                    ct,
                    es,
                    k.as_mut_ptr() as *mut c_void,
                    8,
                    keyoffset,
                    STBDS_HM_BINARY,
                );
                let rp = (r.hmdel_key)(
                    rt,
                    es,
                    k.as_mut_ptr() as *mut c_void,
                    8,
                    keyoffset,
                    STBDS_HM_BINARY,
                );
                assert_eq!(cp.is_null(), rp.is_null());
                ct = cp;
                rt = rp;
                let ci = (*(ct as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
                let ri = (*(rt as *mut u8).sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).temp;
                assert_eq!(ci, ri, "del keyoffset={keyoffset} result");
                same(
                    &format!("del keyoffset={keyoffset}"),
                    &dump_map(ct as *mut u8, es, KeyKind::Raw),
                    &dump_map(rt as *mut u8, es, KeyKind::Raw),
                );
            }
            (c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// ===========================================================================
// ERRORS rows 32 / 33 — the reachable assert() in stbds_hmdel_key
// (abort parity, checked in a child process)
// ===========================================================================

/// SH_STRDUP map, keys inserted with `mode = STBDS_HM_STRING`, then deleted
/// with `mode = STBDS_HM_PTR_TO_STRING` in *insertion* order so that
/// `old_index != final_index`. The `mode == STBDS_HM_STRING` test at lib.c:842
/// then fails, the re-find hashes the raw element bytes as a string, does not
/// find them, and `STBDS_ASSERT(slot >= 0)` (lib.c:846) fires.
unsafe fn abort_scenario_del_mode2_refind(api: &Api) {
    let es = 16usize;
    let mut t = (api.shmode_func)(es, SH_STRDUP);
    let mut keys: Vec<Vec<u8>> = (0..4)
        .map(|i| {
            let mut s = format!("abort-key-{i}").into_bytes();
            s.push(0);
            s
        })
        .collect();
    for k in keys.iter_mut() {
        t = (api.hmput_key)(t, es, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
    }
    // delete the FIRST key -> old_index 0, final_index 2 -> index fix-up runs
    t = (api.hmdel_key)(
        t,
        es,
        keys[0].as_mut_ptr() as *mut c_void,
        8,
        0,
        STBDS_HM_PTR_TO_STRING,
    );
    // must not be reached
    println!("NO ABORT, t={t:p}");
}

fn run_abort_child(scenario_name: &str, which: &str) -> std::process::ExitStatus {
    let exe = std::env::current_exe().expect("current_exe");
    Command::new(exe)
        .arg("--exact")
        .arg("zz_abort_child")
        .arg("--test-threads=1")
        .arg("--nocapture")
        .env("HARVEST_ABORT_SCENARIO", scenario_name)
        .env("HARVEST_ABORT_LIB", which)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn abort child")
        .status
}

/// Helper "test" used only as the child-process entry point. Without the
/// environment variable it is a no-op.
#[test]
fn zz_abort_child() {
    let Ok(name) = std::env::var("HARVEST_ABORT_SCENARIO") else {
        return;
    };
    let which = std::env::var("HARVEST_ABORT_LIB").unwrap();
    let (_g, c, r) = scenario(0x3141_5926);
    let api = if which == "c" { c } else { r };
    unsafe {
        match name.as_str() {
            "del_mode2_refind" => abort_scenario_del_mode2_refind(api),
            other => panic!("unknown abort scenario {other}"),
        }
    }
}

#[test]
fn err32_del_mode2_refind_aborts() {
    let cs = run_abort_child("del_mode2_refind", "c");
    let rs = run_abort_child("del_mode2_refind", "rust");
    assert_eq!(
        cs.signal(),
        Some(libc_sigabrt()),
        "the C library must die from assert()/abort(), got {cs:?}"
    );
    assert_eq!(
        rs.signal(),
        cs.signal(),
        "abort parity: C {cs:?} vs RUST {rs:?}"
    );
    assert_eq!(rs.code(), cs.code(), "exit-code parity: C {cs:?} vs RUST {rs:?}");
}

fn libc_sigabrt() -> i32 {
    6
}
