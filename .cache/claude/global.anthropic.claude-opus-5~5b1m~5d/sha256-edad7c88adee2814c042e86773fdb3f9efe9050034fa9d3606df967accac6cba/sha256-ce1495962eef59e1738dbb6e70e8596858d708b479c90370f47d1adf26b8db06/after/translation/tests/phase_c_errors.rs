//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid input or
//! rejecting condition, calls BOTH libraries through their `.so` exports, and
//! asserts they produce the SAME sentinel / error observable:
//!   * the returned pointer (`NULL`, or the unchanged input pointer),
//!   * the `*temp` out-parameter of `stbds_hmget_key_ts`,
//!   * the `temp` field of the array header (`-1` = not found for the `get`
//!     family, `0`/`1` = not-deleted/deleted for `stbds_hmdel_key`),
//!   * the full header + hash-index + bucket + element state afterwards,
//!   * the absence of an `assert` abort.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEED: u64 = 0xE2_0000_0000_1111;

// ---------------------------------------------------------------------------
// row 1 - arrgrowf(NULL, es, 0, 0): min_cap(0) <= arrcap(NULL)(0) -> return NULL
// ---------------------------------------------------------------------------
#[test]
fn err_01_arrgrowf_null_zero_returns_null() {
    diff("err01", |lib, log| unsafe {
        for &es in &[0usize, 1, 8, 16, 1024, usize::MAX] {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            log.usz("es", es);
            log.flag("is_null", a.is_null());
            snap_array(log, a, 0);
        }
    });
}

// ---------------------------------------------------------------------------
// row 2 - arrgrowf identity when the capacity already suffices
// ---------------------------------------------------------------------------
#[test]
fn err_02_arrgrowf_noop_when_cap_sufficient() {
    diff("err02", |lib, log| unsafe {
        for &es in &[1usize, 8, 24] {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), es, 0, 16);
            (*header(a)).length = 7;
            (*header(a)).temp = -12345;
            let before = *header(a);
            for &(addlen, mc) in &[
                (0usize, 0usize),
                (0, 1),
                (0, 16),
                (9, 0),
                (9, 16),
                (1, 8),
            ] {
                let b = (lib.arrgrowf)(a, es, addlen, mc);
                log.usz("es", es);
                log.usz("addlen", addlen);
                log.usz("mc", mc);
                log.flag("same_ptr", b == a);
                log.flag("hdr_unchanged", *header(b) == before);
                snap_array(log, b, 0);
            }
            (lib.arrfreef)(a);
        }
    });
}

// ---------------------------------------------------------------------------
// row 3 - min_cap clamped up to 4
// ---------------------------------------------------------------------------
#[test]
fn err_03_arrgrowf_min_cap_clamped_to_4() {
    diff("err03", |lib, log| unsafe {
        for &es in &[1usize, 8, 40] {
            for &(addlen, mc) in &[(0usize, 1usize), (0, 2), (0, 3), (1, 0), (2, 0), (3, 0), (1, 3)]
            {
                let a = (lib.arrgrowf)(std::ptr::null_mut(), es, addlen, mc);
                log.usz("es", es);
                log.usz("addlen", addlen);
                log.usz("mc", mc);
                snap_array(log, a, 0);
                (lib.arrfreef)(a);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 4 - elemsize == 0
// ---------------------------------------------------------------------------
#[test]
fn err_04_arrgrowf_zero_elemsize() {
    diff("err04", |lib, log| unsafe {
        for &(addlen, mc) in &[(0usize, 1usize), (0, 4), (7, 0), (0, 100), (1000, 0)] {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), 0, addlen, mc);
            log.usz("addlen", addlen);
            log.usz("mc", mc);
            snap_array(log, a, 0);
            // growing it again keeps elemsize 0 -> still a 32-byte allocation
            let b = (lib.arrgrowf)(a, 0, 1, 0);
            snap_array(log, b, 0);
            (lib.arrfreef)(b);
        }
    });
}

// ---------------------------------------------------------------------------
// row 5 - oversized addlen / min_cap: the size arithmetic wraps.
//
// `elemsize` is kept at 0 (or 1) so `elemsize*min_cap + sizeof(header)` stays a
// small, allocatable number - with e.g. elemsize 8 the wrapped request would be
// 24 bytes while the header write needs 32, i.e. real heap corruption in BOTH
// libraries, which says nothing about the translation.
// ---------------------------------------------------------------------------
#[test]
fn err_05_arrgrowf_oversized_addlen_wraps() {
    diff("err05", |lib, log| unsafe {
        for &(es, addlen, mc) in &[
            (0usize, usize::MAX, 0usize),
            (0, 0, usize::MAX),
            (0, usize::MAX, usize::MAX),
            (0, usize::MAX / 2, 0),
            (0, 1 << 62, 0),
            (1, usize::MAX, 0),
        ] {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), es, addlen, mc);
            log.usz("es", es);
            log.usz("addlen", addlen);
            log.usz("mc", mc);
            snap_array(log, a, 0);
            (lib.arrfreef)(a);
        }

        // Now the `min_cap < 2*arrcap(a)` wrap, reached by hand-crafting a huge
        // capacity in the header (elemsize 0 keeps every allocation 32 bytes).
        for &cap in &[1usize << 62, 1 << 63, usize::MAX, usize::MAX - 1] {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), 0, 0, 4);
            (*header(a)).capacity = cap;
            (*header(a)).length = 0;
            for &mc in &[0usize, 1, 4, cap, cap.wrapping_add(1), usize::MAX] {
                let b = (lib.arrgrowf)(a, 0, 0, mc);
                log.usz("cap", cap);
                log.usz("mc", mc);
                log.flag("same", b == a);
                snap_array(log, b, 0);
                (*header(a)).capacity = cap;
            }
            (lib.arrfreef)(a);
        }
    });
}

// ---------------------------------------------------------------------------
// row 6 - hmfree_func(NULL, es) is a pure no-op
// ---------------------------------------------------------------------------
#[test]
fn err_06_hmfree_null_is_noop() {
    diff("err06", |lib, log| unsafe {
        for &es in &[0usize, 1, 8, 16, usize::MAX] {
            (lib.hmfree_func)(std::ptr::null_mut(), es);
            log.usz("es", es);
            log.tag("returned");
        }
    });
}

// ---------------------------------------------------------------------------
// row 7 - hmfree_func on an array whose hash_table is NULL
// ---------------------------------------------------------------------------
#[test]
fn err_07_hmfree_no_hash_table() {
    diff("err07", |lib, log| unsafe {
        for &es in &[1usize, 8, 16, 24] {
            // via arrgrowf (hash_table zeroed by the fresh-allocation branch)
            let a = (lib.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            (*header(a)).length = 1;
            log.usz("es", es);
            snap_array(log, a, 0);
            (lib.hmfree_func)(a, es);
            log.tag("freed_arr");

            // via hmput_default (array exists, still no table)
            let t = (lib.hmput_default)(std::ptr::null_mut(), es);
            snap_map(log, t, es, KeyKind::Binary);
            (lib.hmfree_func)((t as *mut u8).sub(es) as *mut c_void, es);
            log.tag("freed_map");
        }
    });
}

// ---------------------------------------------------------------------------
// rows 8+9 - lookup of an absent key returns the -1 (STBDS_INDEX_EMPTY)
//            sentinel, whether the empty slot is found in the forward loop or
//            in the wrap-around `i < limit` loop
// ---------------------------------------------------------------------------
#[test]
fn err_08_09_get_missing_key_returns_minus1() {
    diff("err08_09", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 8);
        let es = 16usize;
        // Sweep table sizes so the probe hits both inner loops; 2000 random
        // misses per size makes the wrap-around loop statistically certain.
        for &n in &[0usize, 1, 5, 6, 12, 24, 50, 120] {
            (lib.rand_seed)(0x3141_5926);
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..n {
                let k = (i as u64).to_le_bytes();
                t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
            }
            log.usz("n", n);
            let mut misses = 0usize;
            for _ in 0..2000 {
                let k = (1_000_000u64 + rng.next_u64() % 1_000_000).to_le_bytes();
                let (nt, idx) = hmgeti(lib, t, es, &k, HM_BINARY);
                t = nt;
                if idx == -1 {
                    misses += 1;
                }
                log.isz("idx", idx);
            }
            log.usz("misses", misses);
            hmfree(lib, t, es);
        }
    });
}

// ---------------------------------------------------------------------------
// row 10 - hmget_key_ts with a == NULL
// ---------------------------------------------------------------------------
#[test]
fn err_10_hmget_key_ts_null_a() {
    diff("err10", |lib, log| unsafe {
        for &es in &[1usize, 8, 16, 32] {
            for &mode in &[HM_BINARY, HM_STRING, 2, -1] {
                let mut key = *b"abcdefg\0";
                let mut temp: isize = 0x7BAD;
                let t = (lib.hmget_key_ts)(
                    std::ptr::null_mut(),
                    es,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    &mut temp,
                    mode,
                );
                log.usz("es", es);
                log.i32v("mode", mode);
                log.isz("temp", temp);
                log.flag("t_null", t.is_null());
                snap_map(log, t, es, KeyKind::Binary);
                hmfree(lib, t, es);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 11 - hmget_key_ts with a != NULL but hash_table == NULL
// ---------------------------------------------------------------------------
#[test]
fn err_11_hmget_key_ts_no_table() {
    diff("err11", |lib, log| unsafe {
        for &es in &[1usize, 8, 16, 32] {
            for &mode in &[HM_BINARY, HM_STRING, 2, -1] {
                let t0 = (lib.hmput_default)(std::ptr::null_mut(), es);
                let mut key = *b"abcdefg\0";
                let mut temp: isize = 0x7BAD;
                let t = (lib.hmget_key_ts)(
                    t0,
                    es,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    &mut temp,
                    mode,
                );
                log.usz("es", es);
                log.i32v("mode", mode);
                log.isz("temp", temp);
                log.flag("same", t == t0);
                snap_map(log, t, es, KeyKind::Binary);
                hmfree(lib, t, es);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 12 - hmget_key with a == NULL sets the header temp field to -1
// ---------------------------------------------------------------------------
#[test]
fn err_12_hmget_key_null_a() {
    diff("err12", |lib, log| unsafe {
        for &es in &[1usize, 8, 16, 32] {
            for &mode in &[HM_BINARY, HM_STRING, 2, -1] {
                let mut key = *b"zzz\0";
                let t = (lib.hmget_key)(
                    std::ptr::null_mut(),
                    es,
                    key.as_mut_ptr() as *mut c_void,
                    4,
                    mode,
                );
                let raw = (t as *mut u8).sub(es) as *mut c_void;
                log.usz("es", es);
                log.i32v("mode", mode);
                log.isz("temp", (*header(raw)).temp);
                snap_map(log, t, es, KeyKind::Binary);
                hmfree(lib, t, es);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 13 - hmget_key with an absent key sets the header temp field to -1
// ---------------------------------------------------------------------------
#[test]
fn err_13_hmget_key_missing() {
    diff("err13", |lib, log| unsafe {
        let es = 16usize;
        for &n in &[1usize, 6, 12, 40] {
            (lib.rand_seed)(0x3141_5926);
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..n {
                let k = (i as u64).to_le_bytes();
                t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
            }
            for j in 0..200usize {
                let k = (5_000_000u64 + j as u64).to_le_bytes();
                let (nt, idx) = hmgeti(lib, t, es, &k, HM_BINARY);
                t = nt;
                log.usz("n", n);
                log.isz("idx", idx);
                assert_eq!(idx, -1, "{}: absent key must yield -1", lib.name);
            }
            hmfree(lib, t, es);
        }
    });
}

// ---------------------------------------------------------------------------
// rows 14/15/16 - hmput_default: NULL, length==0, and the no-op path
// ---------------------------------------------------------------------------
#[test]
fn err_14_15_16_hmput_default_paths() {
    diff("err14_16", |lib, log| unsafe {
        for &es in &[1usize, 8, 16, 40] {
            // row 14: a == NULL
            let t = (lib.hmput_default)(std::ptr::null_mut(), es);
            log.usz("es", es);
            log.tag("null");
            snap_map(log, t, es, KeyKind::Binary);

            // row 16: no-op (length != 0)
            let t2 = (lib.hmput_default)(t, es);
            log.tag("noop");
            log.flag("same", t2 == t);
            snap_map(log, t2, es, KeyKind::Binary);

            // row 15: length == 0 forces another grow
            (*header((t2 as *mut u8).sub(es) as *mut c_void)).length = 0;
            let t3 = (lib.hmput_default)(t2, es);
            log.tag("zero_len");
            snap_map(log, t3, es, KeyKind::Binary);

            // ...and once more, from length 0 again
            (*header((t3 as *mut u8).sub(es) as *mut c_void)).length = 0;
            let t4 = (lib.hmput_default)(t3, es);
            log.tag("zero_len_2");
            snap_map(log, t4, es, KeyKind::Binary);

            hmfree(lib, t4, es);
        }
    });
}

// ---------------------------------------------------------------------------
// row 17 - hmput_key with a == NULL bootstraps the array first
// ---------------------------------------------------------------------------
#[test]
fn err_17_hmput_key_null_a() {
    diff("err17", |lib, log| unsafe {
        for &es in &[8usize, 16, 32] {
            for &mode in &[HM_BINARY, -1] {
                (lib.rand_seed)(0x3141_5926);
                let mut k = *b"\x01\x02\x03\x04\x05\x06\x07\x08";
                let t = (lib.hmput_key)(
                    std::ptr::null_mut(),
                    es,
                    k.as_mut_ptr() as *mut c_void,
                    8,
                    mode,
                );
                log.usz("es", es);
                log.i32v("mode", mode);
                snap_map(log, t, es, KeyKind::Binary);
                hmfree(lib, t, es);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// rows 18+19 - table == NULL: string.mode becomes 0 for mode<1 and
//              STBDS_SH_DEFAULT(1) for mode>=1
// ---------------------------------------------------------------------------
#[test]
fn err_18_19_hmput_key_initial_string_mode() {
    diff("err18_19", |lib, log| unsafe {
        let es = 16usize;
        for &mode in &[c_int::MIN, -2, -1, 0, 1, 2, 3, 255, 256, c_int::MAX] {
            (lib.rand_seed)(0x3141_5926);
            let mut key = *b"hello\0\0\0";
            let t = (lib.hmput_key)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                8,
                mode,
            );
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let table = (*header(raw)).hash_table as *mut HashIndex;
            log.i32v("mode", mode);
            log.u8v("string_mode", (*table).string.mode);
            // header/table state only: for mode>=1 the element holds the raw
            // caller pointer, which differs per library by construction.
            log.usz("length", (*header(raw)).length);
            log.isz("temp", (*header(raw)).temp);
            log.usz("slot_count", (*table).slot_count);
            log.usz("used_count", (*table).used_count);
            hmfree(lib, t, es);
        }
    });
}

// ---------------------------------------------------------------------------
// rows 20+21 - duplicate-key hits: length unchanged, temp == existing index
// ---------------------------------------------------------------------------
#[test]
fn err_20_21_hmput_duplicate_key() {
    diff("err20_21", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 20);
        let es = 24usize;
        for &n in &[1usize, 5, 6, 7, 20, 60] {
            (lib.rand_seed)(0x3141_5926);
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..n {
                let k = (i as u64).to_le_bytes();
                t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
            }
            let len_before = (*header((t as *mut u8).sub(es) as *mut c_void)).length;
            // re-put every key 3x in random order: never grows the array
            for _ in 0..3 {
                for _ in 0..n {
                    let i = rng.below(n);
                    let k = (i as u64).to_le_bytes();
                    t = hmput(lib, t, es, &k, HM_BINARY, 0xF00 + i as u64);
                    let raw = (t as *mut u8).sub(es) as *mut c_void;
                    log.usz("n", n);
                    log.usz("i", i);
                    log.isz("temp", (*header(raw)).temp);
                    log.usz("length", (*header(raw)).length);
                    assert_eq!(
                        (*header(raw)).length,
                        len_before,
                        "{}: duplicate put must not grow the array",
                        lib.name
                    );
                }
            }
            snap_map(log, t, es, KeyKind::Binary);
            hmfree(lib, t, es);
        }
    });
}

// ---------------------------------------------------------------------------
// row 22 - insertion reuses a tombstone slot (--tombstone_count, ++used_count)
// ---------------------------------------------------------------------------
#[test]
fn err_22_hmput_reuses_tombstone() {
    diff("err22", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 22);
        let es = 16usize;
        for rep in 0..6usize {
            (lib.rand_seed)(rng.next_u64() as usize);
            let mut t: *mut c_void = std::ptr::null_mut();
            // fill, delete a few (leaving tombstones), then insert again
            for i in 0..5usize {
                let k = (i as u64).to_le_bytes();
                t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
            }
            for i in 0..3usize {
                let k = (i as u64).to_le_bytes();
                let (nt, d) = hmdel(lib, t, es, &k, 8, 0, HM_BINARY);
                t = nt;
                log.isz("d", d);
            }
            log.usz("rep", rep);
            snap_map(log, t, es, KeyKind::Binary);
            for i in 100..112usize {
                let k = (i as u64).to_le_bytes();
                t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
                snap_map(log, t, es, KeyKind::Binary);
            }
            hmfree(lib, t, es);
        }
    });
}

// ---------------------------------------------------------------------------
// row 23 - used_count >= used_count_threshold grows the table
// ---------------------------------------------------------------------------
#[test]
fn err_23_hmput_grows_table() {
    diff("err23", |lib, log| unsafe {
        let es = 16usize;
        (lib.rand_seed)(0x3141_5926);
        let mut t: *mut c_void = std::ptr::null_mut();
        let mut prev_slots = 0usize;
        for i in 0..300usize {
            let k = (i as u64).to_le_bytes();
            t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let table = (*header(raw)).hash_table as *mut HashIndex;
            let sc = (*table).slot_count;
            log.usz("i", i);
            log.usz("slot_count", sc);
            log.usz("used", (*table).used_count);
            log.usz("used_thr", (*table).used_count_threshold);
            log.flag("grew", sc != prev_slots);
            if sc != prev_slots && prev_slots != 0 {
                assert_eq!(sc, prev_slots * 2, "{}: table must double", lib.name);
            }
            prev_slots = sc;
        }
        snap_map(log, t, es, KeyKind::Binary);
        hmfree(lib, t, es);
    });
}

// ---------------------------------------------------------------------------
// row 24 - the capacity assert at c_src/src/lib.c:778 is unreachable: hammering
//          every array-growth boundary must never abort either library
// ---------------------------------------------------------------------------
#[test]
fn err_24_hmput_capacity_assert_unreachable() {
    diff("err24", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 24);
        for &es in &[1usize, 7, 8, 9, 16, 33] {
            (lib.rand_seed)(rng.next_u64() as usize);
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..260usize {
                let k = (i as u64).to_le_bytes();
                let ks = 8usize.min(es);
                t = hmput(lib, t, es, &k[..ks], HM_BINARY, i as u64);
                let raw = (t as *mut u8).sub(es) as *mut c_void;
                let h = header(raw);
                log.usz("es", es);
                log.usz("i", i);
                log.usz("len", (*h).length);
                log.usz("cap", (*h).capacity);
                assert!(
                    (*h).length <= (*h).capacity,
                    "{}: length {} exceeded capacity {}",
                    lib.name,
                    (*h).length,
                    (*h).capacity
                );
            }
            hmfree(lib, t, es);
        }
        log.tag("no_abort");
    });
}

// ---------------------------------------------------------------------------
// row 25 - shmode_func: out-of-range enum values are truncated, not validated
// ---------------------------------------------------------------------------
#[test]
fn err_25_shmode_out_of_range_enum() {
    diff("err25", |lib, log| unsafe {
        let modes: [c_int; 24] = [
            0,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            63,
            64,
            127,
            128,
            200,
            253,
            254,
            255,
            256,
            257,
            511,
            512,
            -1,
            c_int::MIN,
            c_int::MAX,
        ];
        for &m in &modes {
            (lib.rand_seed)(0x3141_5926);
            let es = 16usize;
            let t = (lib.shmode_func)(es, m);
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let table = (*header(raw)).hash_table as *mut HashIndex;
            log.i32v("mode", m);
            log.u8v("string_mode", (*table).string.mode);
            assert_eq!(
                (*table).string.mode,
                (m as u32 & 0xff) as u8,
                "{}: (unsigned char) truncation",
                lib.name
            );
            snap_map(log, t, es, KeyKind::Binary);
            hmfree(lib, t, es);
        }
    });
}

// ---------------------------------------------------------------------------
// row 26 - shmode_func with elemsize == 0
// ---------------------------------------------------------------------------
#[test]
fn err_26_shmode_zero_elemsize() {
    diff("err26", |lib, log| unsafe {
        for &m in &[0 as c_int, 1, 2, 3, 255, -1] {
            (lib.rand_seed)(0x3141_5926);
            let t = (lib.shmode_func)(0, m);
            log.i32v("mode", m);
            log.flag("hash_eq_raw", true);
            snap_map(log, t, 0, KeyKind::Binary);
            hmfree(lib, t, 0);
        }
    });
}

// ---------------------------------------------------------------------------
// row 27 - hmdel_key(NULL, ...) returns 0 (NULL)
// ---------------------------------------------------------------------------
#[test]
fn err_27_hmdel_null_a_returns_null() {
    diff("err27", |lib, log| unsafe {
        for &es in &[0usize, 1, 8, 16, usize::MAX] {
            for &mode in &[c_int::MIN, -1, 0, 1, 2, 255, c_int::MAX] {
                for &keyoffset in &[0usize, 4, 8, usize::MAX] {
                    let mut key = *b"whatever";
                    let r = (lib.hmdel_key)(
                        std::ptr::null_mut(),
                        es,
                        key.as_mut_ptr() as *mut c_void,
                        8,
                        keyoffset,
                        mode,
                    );
                    log.usz("es", es);
                    log.i32v("mode", mode);
                    log.usz("ko", keyoffset);
                    log.flag("is_null", r.is_null());
                    assert!(r.is_null(), "{}: hmdel_key(NULL) must return NULL", lib.name);
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 28 - hmdel_key with hash_table == NULL: temp = 0, pointer unchanged
// ---------------------------------------------------------------------------
#[test]
fn err_28_hmdel_no_table() {
    diff("err28", |lib, log| unsafe {
        for &es in &[1usize, 8, 16, 32] {
            for &mode in &[c_int::MIN, -1, 0, 1, 2, c_int::MAX] {
                let t0 = (lib.hmput_default)(std::ptr::null_mut(), es);
                // poison temp so we can see hmdel_key set it to 0
                (*header((t0 as *mut u8).sub(es) as *mut c_void)).temp = 0x7EAD;
                let mut key = *b"abcdefgh";
                let t = (lib.hmdel_key)(
                    t0,
                    es,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    0,
                    mode,
                );
                let raw = (t as *mut u8).sub(es) as *mut c_void;
                log.usz("es", es);
                log.i32v("mode", mode);
                log.flag("same", t == t0);
                log.isz("temp", (*header(raw)).temp);
                assert_eq!((*header(raw)).temp, 0, "{}: temp must be 0", lib.name);
                snap_map(log, t, es, KeyKind::Binary);
                hmfree(lib, t, es);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 29 - hmdel_key with an absent key: temp = 0, length unchanged
// ---------------------------------------------------------------------------
#[test]
fn err_29_hmdel_missing_key() {
    diff("err29", |lib, log| unsafe {
        let es = 16usize;
        for &n in &[1usize, 6, 12, 40] {
            (lib.rand_seed)(0x3141_5926);
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..n {
                let k = (i as u64).to_le_bytes();
                t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
            }
            let len = (*header((t as *mut u8).sub(es) as *mut c_void)).length;
            for j in 0..300usize {
                let k = (9_000_000u64 + j as u64).to_le_bytes();
                let (nt, d) = hmdel(lib, t, es, &k, 8, 0, HM_BINARY);
                t = nt;
                log.usz("n", n);
                log.isz("d", d);
                assert_eq!(d, 0, "{}: absent delete must report 0", lib.name);
                assert_eq!(
                    (*header((t as *mut u8).sub(es) as *mut c_void)).length,
                    len,
                    "{}: absent delete must not change length",
                    lib.name
                );
            }
            snap_map(log, t, es, KeyKind::Binary);
            hmfree(lib, t, es);
        }
    });
}

// ---------------------------------------------------------------------------
// rows 30+31 - a present key: temp = 1, tombstone written, length--; and the
//              `old_index == final_index` shortcut
// ---------------------------------------------------------------------------
#[test]
fn err_30_31_hmdel_present_key() {
    diff("err30_31", |lib, log| unsafe {
        let es = 16usize;
        for &n in &[1usize, 2, 3, 10, 40] {
            for &reverse in &[true, false] {
                (lib.rand_seed)(0x3141_5926);
                let mut t: *mut c_void = std::ptr::null_mut();
                for i in 0..n {
                    let k = (i as u64).to_le_bytes();
                    t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
                }
                log.usz("n", n);
                log.flag("reverse", reverse);
                let idxs: Vec<usize> = if reverse {
                    (0..n).rev().collect()
                } else {
                    (0..n).collect()
                };
                for &i in &idxs {
                    let raw = (t as *mut u8).sub(es) as *mut c_void;
                    let len_before = (*header(raw)).length;
                    let k = (i as u64).to_le_bytes();
                    let (nt, d) = hmdel(lib, t, es, &k, 8, 0, HM_BINARY);
                    t = nt;
                    let raw = (t as *mut u8).sub(es) as *mut c_void;
                    log.usz("i", i);
                    log.isz("d", d);
                    assert_eq!(d, 1, "{}: present delete must report 1", lib.name);
                    assert_eq!(
                        (*header(raw)).length,
                        len_before - 1,
                        "{}: length must drop by one",
                        lib.name
                    );
                    snap_map(log, t, es, KeyKind::Binary);
                }
                hmfree(lib, t, es);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// rows 32+34+35 - the `slot < slot_count`, `slot >= 0` and
//                 `b->index[i] == final_index` asserts are unreachable
// ---------------------------------------------------------------------------
#[test]
fn err_32_34_35_hmdel_asserts_unreachable() {
    diff("err32_34_35", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 32);
        let es = 16usize;
        // 6 x 400 random insert/delete ops across every table size; any of the
        // three asserts firing would abort the process.
        for rep in 0..6usize {
            (lib.rand_seed)(rng.next_u64() as usize);
            let mut t: *mut c_void = std::ptr::null_mut();
            let mut live: Vec<usize> = Vec::new();
            for step in 0..400usize {
                if live.is_empty() || rng.below(2) == 0 {
                    let i = step;
                    let k = (i as u64).to_le_bytes();
                    t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
                    live.push(i);
                } else {
                    let j = rng.below(live.len());
                    let i = live.swap_remove(j);
                    let k = (i as u64).to_le_bytes();
                    let (nt, d) = hmdel(lib, t, es, &k, 8, 0, HM_BINARY);
                    t = nt;
                    log.isz("d", d);
                }
                log.usz("rep", rep);
                log.usz("step", step);
                snap_map(log, t, es, KeyKind::Binary);
            }
            hmfree(lib, t, es);
        }
        log.tag("no_abort");
    });
}

// ---------------------------------------------------------------------------
// row 33 - `STBDS_ASSERT(table->used_count >= 0)` is vacuous because
//          used_count is a size_t. Forcing used_count to 0 and then deleting a
//          live key makes `--used_count` wrap to SIZE_MAX; the C does NOT
//          abort, and the Rust must not either.
// ---------------------------------------------------------------------------
#[test]
fn err_33_hmdel_used_count_assert_vacuous() {
    diff("err33", |lib, log| unsafe {
        let es = 16usize;
        for &n in &[1usize, 2, 5] {
            (lib.rand_seed)(0x3141_5926);
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..n {
                let k = (i as u64).to_le_bytes();
                t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
            }
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let table = (*header(raw)).hash_table as *mut HashIndex;
            (*table).used_count = 0; // <- force the wrap
            let k = ((n - 1) as u64).to_le_bytes();
            let (nt, d) = hmdel(lib, t, es, &k, 8, 0, HM_BINARY);
            t = nt;
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let table = (*header(raw)).hash_table as *mut HashIndex;
            log.usz("n", n);
            log.isz("d", d);
            log.usz("used_count", (*table).used_count);
            assert_eq!(d, 1, "{}: the delete still succeeds", lib.name);
            assert_eq!(
                (*table).used_count,
                usize::MAX,
                "{}: used_count must wrap to SIZE_MAX without aborting",
                lib.name
            );
            snap_map(log, t, es, KeyKind::Binary);
            hmfree(lib, t, es);
        }
        log.tag("no_abort");
    });
}

// ---------------------------------------------------------------------------
// row 36 - the shrink path (used_count < used_count_shrink_threshold &&
//          slot_count > 8)
// ---------------------------------------------------------------------------
#[test]
fn err_36_hmdel_shrinks_table() {
    diff("err36", |lib, log| unsafe {
        let es = 16usize;
        (lib.rand_seed)(0x3141_5926);
        let mut t: *mut c_void = std::ptr::null_mut();
        let n = 300usize;
        for i in 0..n {
            let k = (i as u64).to_le_bytes();
            t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
        }
        let mut shrinks = 0usize;
        let mut prev = {
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            (*((*header(raw)).hash_table as *mut HashIndex)).slot_count
        };
        for i in 0..n {
            let k = (i as u64).to_le_bytes();
            let (nt, d) = hmdel(lib, t, es, &k, 8, 0, HM_BINARY);
            t = nt;
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let sc = (*((*header(raw)).hash_table as *mut HashIndex)).slot_count;
            if sc < prev {
                shrinks += 1;
                assert_eq!(sc, prev / 2, "{}: table must halve", lib.name);
            }
            prev = sc;
            log.usz("i", i);
            log.isz("d", d);
            log.usz("slot_count", sc);
            snap_map(log, t, es, KeyKind::Binary);
        }
        log.usz("shrinks", shrinks);
        assert!(shrinks >= 4, "{}: expected several shrinks", lib.name);
        assert_eq!(prev, 8, "{}: must bottom out at STBDS_BUCKET_LENGTH", lib.name);
        hmfree(lib, t, es);
    });
}

// ---------------------------------------------------------------------------
// row 37 - the rebuild path (tombstone_count > tombstone_count_threshold)
// ---------------------------------------------------------------------------
#[test]
fn err_37_hmdel_rebuilds_table() {
    diff("err37", |lib, log| unsafe {
        let es = 16usize;
        (lib.rand_seed)(0x3141_5926);
        let mut t: *mut c_void = std::ptr::null_mut();
        // Keep used_count roughly constant so the shrink path never fires and
        // tombstones accumulate until the rebuild threshold is crossed.
        let mut rebuilds = 0usize;
        for i in 0..6usize {
            let k = (i as u64).to_le_bytes();
            t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
        }
        for step in 0..500usize {
            let del = (step % 6) as u64;
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let tb_before = (*((*header(raw)).hash_table as *mut HashIndex)).tombstone_count;
            let k = (del + (step as u64 / 6) * 6).to_le_bytes();
            let (nt, d) = hmdel(lib, t, es, &k, 8, 0, HM_BINARY);
            t = nt;
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let tb_after = (*((*header(raw)).hash_table as *mut HashIndex)).tombstone_count;
            if d == 1 && tb_after < tb_before {
                rebuilds += 1;
            }
            // re-insert a brand-new key to keep used_count up
            let k2 = (1_000u64 + step as u64).to_le_bytes();
            t = hmput(lib, t, es, &k2, HM_BINARY, step as u64);
            log.usz("step", step);
            log.isz("d", d);
            log.usz("tomb", tb_after);
            snap_map(log, t, es, KeyKind::Binary);
        }
        log.usz("rebuilds", rebuilds);
        hmfree(lib, t, es);
    });
}

// ---------------------------------------------------------------------------
// row 38 - the strdup'd key is freed only when `mode == STBDS_HM_STRING`
//          exactly (c_src/src/lib.c:836 uses `==`, not `>=`)
// ---------------------------------------------------------------------------
#[test]
fn err_38_hmdel_strdup_free_only_mode_eq_1() {
    diff("err38", |lib, log| unsafe {
        let es = 16usize;
        for &del_mode in &[1 as c_int, 2, 3, 255, c_int::MAX] {
            (lib.rand_seed)(0x3141_5926);
            let t0 = (lib.shmode_func)(es, SH_STRDUP);
            let mut t = t0;
            let n = 20usize;
            let keys: Vec<Vec<u8>> = (0..n)
                .map(|i| {
                    let mut v = format!("key-{:04}", i).into_bytes();
                    v.push(0);
                    v
                })
                .collect();
            for (i, k) in keys.iter().enumerate() {
                let mut kk = k.clone();
                t = shput(
                    lib,
                    t,
                    es,
                    kk.as_mut_ptr() as *mut c_char,
                    HM_STRING,
                    8,
                    i as u64,
                    false,
                );
            }
            log.i32v("del_mode", del_mode);
            snap_map(log, t, es, KeyKind::StringAt(0));
            // reverse order -> `old_index == final_index`, so the C skips the
            // address-hashing fix-up branch that `mode != 1` would otherwise
            // take (see cfg45 in phase_b_string.rs).
            for i in (0..n).rev() {
                let mut kk = keys[i].clone();
                let (nt, d) = shdel(lib, t, es, kk.as_mut_ptr() as *mut c_char, 0, del_mode);
                t = nt;
                log.usz("i", i);
                log.isz("d", d);
                assert_eq!(d, 1, "{}: mode {} must still find the key", lib.name, del_mode);
                snap_map(log, t, es, KeyKind::StringAt(0));
            }
            hmfree(lib, t, es);
        }
    });
}

// ---------------------------------------------------------------------------
// row 39 - a non-zero keyoffset.
//
// Run inside forked children: with `keyoffset != 0` the C's LIVE
// `STBDS_ASSERT(slot >= 0)` at c_src/src/lib.c:846 really does fire. The
// element's key lives at offset 0 (that is where `hmput_key`'s `memcpy` arm
// puts it) while `hmdel_key` compares at `elem + keyoffset`, so a delete can
// match by coincidence, take the swap-delete branch, and then fail to re-find
// the moved element - which trips the assert and aborts. The Rust translation
// carries the same assert, so both must die with the same signal at the same
// point in the sequence.
// ---------------------------------------------------------------------------
#[test]
fn err_39_hmdel_nonzero_keyoffset() {
    for &keysize in &[1usize, 4, 8] {
        for &keyoffset in &[0usize, 1, 2, 4, 8, 16, 33] {
            diff_child(
                &format!("err39 ks={} ko={}", keysize, keyoffset),
                move |lib, log| unsafe {
                    let es = 64usize;
                    (lib.rand_seed)(0x3141_5926);
                    let n = 15usize;
                    let mut t: *mut c_void = std::ptr::null_mut();
                    let keys: Vec<Vec<u8>> = (0..n)
                        .map(|i| (i as u64).to_le_bytes()[..keysize.min(8)].to_vec())
                        .collect();
                    for (i, k) in keys.iter().enumerate() {
                        t = hmput(lib, t, es, k, HM_BINARY, i as u64);
                    }
                    log.usz("ks", keysize);
                    log.usz("ko", keyoffset);
                    snap_map(log, t, es, KeyKind::Binary);
                    for (i, k) in keys.iter().enumerate() {
                        let (nt, d) = hmdel(lib, t, es, k, keysize, keyoffset, HM_BINARY);
                        t = nt;
                        log.usz("i", i);
                        log.isz("d", d);
                        snap_map(log, t, es, KeyKind::Binary);
                    }
                    hmfree(lib, t, es);
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 34 (reachable half) - `mode >= 2` on a string map takes the `else` branch
// at c_src/src/lib.c:845, which hashes the ADDRESS of the moved element instead
// of its key string. `stbds_hm_find_slot` then cannot find it and returns -1,
// tripping `STBDS_ASSERT(slot >= 0)`. Both implementations must abort.
// ---------------------------------------------------------------------------
#[test]
fn err_34_hmdel_mode_ge_2_mid_delete_aborts() {
    for &del_mode in &[2 as c_int, 3, 255, c_int::MAX] {
        diff_child(
            &format!("err34 mode={}", del_mode),
            move |lib, log| unsafe {
                let es = 16usize;
                (lib.rand_seed)(0x3141_5926);
                let t0 = (lib.shmode_func)(es, SH_DEFAULT);
                let mut t = t0;
                let n = 10usize;
                let mut keys: Vec<Vec<u8>> = (0..n)
                    .map(|i| {
                        let mut v = format!("mk-{:04}", i).into_bytes();
                        v.push(0);
                        v
                    })
                    .collect();
                for i in 0..n {
                    let p = keys[i].as_mut_ptr() as *mut c_char;
                    t = shput(lib, t, es, p, HM_STRING, 8, i as u64, false);
                }
                log.i32v("mode", del_mode);
                snap_map(log, t, es, KeyKind::StringAt(0));
                // deleting entry 0 (NOT the last one) forces old_index !=
                // final_index -> the address-hashing fix-up -> assert
                let p = keys[0].as_mut_ptr() as *mut c_char;
                let (nt, d) = shdel(lib, t, es, p, 0, del_mode);
                t = nt;
                log.isz("d", d);
                snap_map(log, t, es, KeyKind::StringAt(0));
                hmfree(lib, t, es);
            },
        );
    }
}

// ---------------------------------------------------------------------------
// row 40 - the make_hash_index threshold assert is unreachable for every
//          reachable slot_count (8, 16, 32, 64, 128, 256, 512, ...)
// ---------------------------------------------------------------------------
#[test]
fn err_40_make_hash_index_assert_unreachable() {
    diff("err40", |lib, log| unsafe {
        let es = 16usize;
        (lib.rand_seed)(0x3141_5926);
        let mut t: *mut c_void = std::ptr::null_mut();
        // grow all the way to slot_count 1024, then shrink all the way back
        for i in 0..800usize {
            let k = (i as u64).to_le_bytes();
            t = hmput(lib, t, es, &k, HM_BINARY, i as u64);
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let tb = (*header(raw)).hash_table as *mut HashIndex;
            log.usz("sc", (*tb).slot_count);
            log.usz("ut", (*tb).used_count_threshold);
            log.usz("tt", (*tb).tombstone_count_threshold);
            assert!(
                (*tb).used_count_threshold + (*tb).tombstone_count_threshold < (*tb).slot_count,
                "{}: the C assert would have fired for slot_count {}",
                lib.name,
                (*tb).slot_count
            );
        }
        for i in 0..800usize {
            let k = (i as u64).to_le_bytes();
            let (nt, _d) = hmdel(lib, t, es, &k, 8, 0, HM_BINARY);
            t = nt;
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let tb = (*header(raw)).hash_table as *mut HashIndex;
            log.usz("sc2", (*tb).slot_count);
            assert!(
                (*tb).used_count_threshold + (*tb).tombstone_count_threshold < (*tb).slot_count,
                "{}: the C assert would have fired for slot_count {}",
                lib.name,
                (*tb).slot_count
            );
        }
        snap_map(log, t, es, KeyKind::Binary);
        hmfree(lib, t, es);
        log.tag("no_abort");
    });
}

// ---------------------------------------------------------------------------
// row 41 - stralloc: len > remaining && len > blocksize && storage == NULL
// ---------------------------------------------------------------------------
#[test]
fn err_41_stralloc_oversized_first() {
    diff("err41", |lib, log| unsafe {
        for &len in &[512usize, 513, 1000, 65536] {
            let mut a = StringArena::zeroed();
            let mut s = vec![b'x'; len];
            s.push(0);
            let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            log.usz("len", len);
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p);
            // the C sets remaining = 0 and storage = the new dedicated block
            assert_eq!((*(&a as *const StringArena)).remaining, 0, "{}", lib.name);
            assert!(!a.storage.is_null(), "{}", lib.name);
            (lib.strreset)(&mut a);
            snap_arena(log, &a);
        }
    });
}

// ---------------------------------------------------------------------------
// row 42 - stralloc: the oversized block is spliced in as head->next and
//          `remaining` is deliberately NOT reset
// ---------------------------------------------------------------------------
#[test]
fn err_42_stralloc_oversized_splice() {
    diff("err42", |lib, log| unsafe {
        let mut a = StringArena::zeroed();
        // small string first -> a 512-byte head block with remaining != 0
        let mut small = vec![b'a'; 10];
        small.push(0);
        (lib.stralloc)(&mut a, small.as_mut_ptr() as *mut c_char);
        let rem_before = a.remaining;
        let block_before = a.block;
        snap_arena(log, &a);
        for k in 0..6usize {
            let mut big = vec![b'B'; 3000 + k * 111];
            big.push(0);
            let p = (lib.stralloc)(&mut a, big.as_mut_ptr() as *mut c_char);
            log.usz("k", k);
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p);
            log.flag("remaining_preserved", a.remaining == rem_before);
            log.flag("block_bumped", a.block != block_before);
        }
        (lib.strreset)(&mut a);
        snap_arena(log, &a);
    });
}

// ---------------------------------------------------------------------------
// row 43 - `++a->block` saturates once 512 << (block>>1) reaches 1<<20
// ---------------------------------------------------------------------------
#[test]
fn err_43_stralloc_block_saturates() {
    diff("err43", |lib, log| unsafe {
        for blk in 0u8..=24 {
            let mut a = StringArena::zeroed();
            a.block = blk;
            let mut s = vec![b'q'; 20];
            s.push(0);
            (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            log.u8v("blk_in", blk);
            log.u8v("blk_out", a.block);
            log.usz("remaining", a.remaining);
            // 512 << (blk>>1) >= 1<<20  <=>  blk >= 22
            let expect_bump = (512usize << (blk >> 1) as u32) < (1usize << 20);
            log.flag("expect_bump", expect_bump);
            assert_eq!(
                a.block != blk,
                expect_bump,
                "{}: block bump for blk {}",
                lib.name,
                blk
            );
            (lib.strreset)(&mut a);
        }
    });
}

// ---------------------------------------------------------------------------
// row 44 - block>>1 >= 64: the C shift is UB; x86-64 masks the count to 6 bits.
//          Only the values whose masked shift yields blocksize 0 (or a small
//          block) are used - otherwise the C would try to malloc terabytes and
//          then dereference the NULL result.
// ---------------------------------------------------------------------------
#[test]
fn err_44_stralloc_shift_overflow_ub() {
    diff("err44", |lib, log| unsafe {
        for &blk in &[110u8, 111, 118, 126, 127, 128, 129, 130, 131, 238, 250, 254, 255] {
            for &len in &[1usize, 20, 700] {
                let mut a = StringArena::zeroed();
                a.block = blk;
                let mut s = vec![b'w'; len];
                s.push(0);
                let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
                log.u8v("blk", blk);
                log.usz("len", len);
                log.u8v("blk_out", a.block);
                snap_arena(log, &a);
                snap_stralloc_result(log, &a, p);
                (lib.strreset)(&mut a);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 45 - `STBDS_ASSERT(len <= a->remaining)` holds for every well-formed
//          arena, i.e. neither library ever aborts here
// ---------------------------------------------------------------------------
#[test]
fn err_45_stralloc_assert_holds() {
    diff("err45", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 45);
        let mut a = StringArena::zeroed();
        for i in 0..3000usize {
            let n = if rng.below(20) == 0 {
                rng.range(500, 4000)
            } else {
                rng.range(0, 200)
            };
            let mut s = rng.ascii(n);
            let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            assert!(!p.is_null(), "{}", lib.name);
            if i % 400 == 0 {
                log.usz("i", i);
                snap_arena(log, &a);
            }
            if i % 700 == 699 {
                (lib.strreset)(&mut a);
            }
        }
        (lib.strreset)(&mut a);
        snap_arena(log, &a);
        log.tag("no_abort");
    });
}

// ---------------------------------------------------------------------------
// row 46 - the empty string ("" -> len == 1)
// ---------------------------------------------------------------------------
#[test]
fn err_46_stralloc_empty_string() {
    diff("err46", |lib, log| unsafe {
        let mut a = StringArena::zeroed();
        let mut empty = vec![0u8];
        for i in 0..600usize {
            let p = (lib.stralloc)(&mut a, empty.as_mut_ptr() as *mut c_char);
            log.usz("i", i);
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p);
        }
        (lib.strreset)(&mut a);
        snap_arena(log, &a);

        // ...and on a fresh arena on its own
        let mut b = StringArena::zeroed();
        let p = (lib.stralloc)(&mut b, empty.as_mut_ptr() as *mut c_char);
        snap_arena(log, &b);
        snap_stralloc_result(log, &b, p);
        assert_eq!(b.remaining, 511, "{}: 512-byte block minus 1", lib.name);
        (lib.strreset)(&mut b);
    });
}

// ---------------------------------------------------------------------------
// row 47 - strreset on an arena with storage == NULL
// ---------------------------------------------------------------------------
#[test]
fn err_47_strreset_empty_arena() {
    diff("err47", |lib, log| unsafe {
        // fully zeroed
        let mut a = StringArena::zeroed();
        (lib.strreset)(&mut a);
        snap_arena(log, &a);
        // storage NULL but the other fields dirty: memset must clear them all
        let mut b = StringArena::zeroed();
        b.remaining = 0xDEAD;
        b.block = 0xAB;
        b.mode = 0xCD;
        (lib.strreset)(&mut b);
        snap_arena(log, &b);
        assert_eq!(b.remaining, 0, "{}", lib.name);
        assert_eq!(b.block, 0, "{}", lib.name);
        assert_eq!(b.mode, 0, "{}", lib.name);
        // repeated resets stay a no-op
        for _ in 0..5 {
            (lib.strreset)(&mut b);
            snap_arena(log, &b);
        }
    });
}

// ---------------------------------------------------------------------------
// row 48 - hash_bytes with len == 0 (never reads a byte, so even NULL is fine)
// ---------------------------------------------------------------------------
#[test]
fn err_48_hash_bytes_zero_len() {
    diff("err48", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 48);
        let mut seeds: Vec<usize> = vec![0, 1, 2, 0x3141_5926, usize::MAX, usize::MAX - 1];
        for _ in 0..64 {
            seeds.push(rng.next_u64() as usize);
        }
        for &seed in &seeds {
            let h_null = (lib.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let mut buf = [0xAAu8; 64];
            let h_buf = (lib.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            log.usz("seed", seed);
            log.usz("h_null", h_null);
            log.usz("h_buf", h_buf);
            assert_eq!(
                h_null, h_buf,
                "{}: len 0 must ignore the pointer entirely",
                lib.name
            );
        }
    });
}

// ---------------------------------------------------------------------------
// row 49 - hash_bytes tail sign-extension (`d[3] << 24` is a negative int in C,
//          then sign-extended into size_t)
// ---------------------------------------------------------------------------
#[test]
fn err_49_hash_bytes_sign_extension() {
    diff("err49", |lib, log| unsafe {
        // every tail length 1..=7, every "which byte has the high bit" choice
        for tail in 1usize..=7 {
            for hi in 0..tail {
                for &nblocks in &[0usize, 1, 2, 5] {
                    let len = nblocks * 8 + tail;
                    let mut b: Vec<u8> = (0..len).map(|i| (i as u8) | 0x01).collect();
                    b[nblocks * 8 + hi] |= 0x80;
                    for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
                        let h = (lib.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                        log.usz("tail", tail);
                        log.usz("hi", hi);
                        log.usz("len", len);
                        log.usz("seed", seed);
                        log.usz("h", h);
                    }
                }
            }
        }
        // and full 8-byte blocks with every byte >= 0x80
        for nblocks in 1usize..=6 {
            let len = nblocks * 8;
            let mut b: Vec<u8> = vec![0xFF; len];
            let h = (lib.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, 0);
            log.usz("all_ff_len", len);
            log.usz("h", h);
            for k in 0..len {
                let mut c: Vec<u8> = vec![0x01; len];
                c[k] = 0x80;
                let h = (lib.hash_bytes)(c.as_mut_ptr() as *mut c_void, len, 0);
                log.usz("k", k);
                log.usz("h", h);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 50 - hash_string("")
// ---------------------------------------------------------------------------
#[test]
fn err_50_hash_string_empty() {
    diff("err50", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 50);
        let mut empty = vec![0u8];
        let k0 = (lib.hash_string)(empty.as_mut_ptr() as *mut c_char, 0);
        log.usz("k0", k0);
        // hash_string("") is `K + seed` (everything after `hash ^= seed` is
        // seed-independent when the loop never runs), so this must hold exactly:
        let mut seeds: Vec<usize> = vec![0, 1, 2, usize::MAX, 0x3141_5926];
        for _ in 0..64 {
            seeds.push(rng.next_u64() as usize);
        }
        for &s in &seeds {
            let h = (lib.hash_string)(empty.as_mut_ptr() as *mut c_char, s);
            log.usz("seed", s);
            log.usz("h", h);
            assert_eq!(h, k0.wrapping_add(s), "{}: hash_string(\"\") = K+seed", lib.name);
        }
    });
}

// ---------------------------------------------------------------------------
// row 51 - hash_string with bytes >= 0x80: the C casts to `unsigned char`
//          before adding, so there is no sign extension of the character
// ---------------------------------------------------------------------------
#[test]
fn err_51_hash_string_high_bit() {
    diff("err51", |lib, log| unsafe {
        for len in 1usize..=24 {
            for hi in 0..len {
                let mut s: Vec<u8> = vec![b'a'; len];
                s[hi] = 0x80;
                s.push(0);
                let h1 = (lib.hash_string)(s.as_mut_ptr() as *mut c_char, 0x3141_5926);
                let mut s2: Vec<u8> = vec![b'a'; len];
                s2[hi] = 0xFF;
                s2.push(0);
                let h2 = (lib.hash_string)(s2.as_mut_ptr() as *mut c_char, 0x3141_5926);
                log.usz("len", len);
                log.usz("hi", hi);
                log.usz("h_80", h1);
                log.usz("h_ff", h2);
            }
        }
        // all 255 possible non-NUL single characters
        for c in 1u8..=255 {
            let mut s = vec![c, 0];
            let h = (lib.hash_string)(s.as_mut_ptr() as *mut c_char, 0);
            log.u8v("c", c);
            log.usz("h", h);
        }
    });
}

// ---------------------------------------------------------------------------
// row 52 - a key whose raw hash is 0 or 1 must be bumped by `hash += 2` so the
//          STBDS_HASH_EMPTY / STBDS_HASH_DELETED sentinels are never used as a
//          real hash.
//
// `stbds_hash_string("", seed) == K + seed` exactly (the character loop never
// runs, and everything after `hash ^= seed` is seed-independent), so the seeds
// that force a raw hash of 0 and 1 can be computed rather than searched for.
// ---------------------------------------------------------------------------
#[test]
fn err_52_hash_lt_2_bumped() {
    diff("err52", |lib, log| unsafe {
        let mut empty = vec![0u8];
        let k = (lib.hash_string)(empty.as_mut_ptr() as *mut c_char, 0);
        for want in 0usize..=3 {
            let seed = want.wrapping_sub(k);
            let raw = (lib.hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            log.usz("want", want);
            log.usz("seed", seed);
            log.usz("raw_hash", raw);
            assert_eq!(raw, want, "{}: seed solve failed", lib.name);

            for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
                let es = 16usize;
                (lib.rand_seed)(seed);
                let t0 = (lib.shmode_func)(es, sh);
                let mut t = t0;
                // the empty string is the key whose raw hash is `want`
                t = shput(
                    lib,
                    t,
                    es,
                    empty.as_mut_ptr() as *mut c_char,
                    HM_STRING,
                    8,
                    0xABCD,
                    false,
                );
                log.i32v("sh", sh);
                snap_map_tk(log, t, es, KeyKind::StringAt(0));
                // the stored bucket hash must be `max(want, want+2)` i.e. never
                // 0 or 1
                let raw_arr = (t as *mut u8).sub(es) as *mut c_void;
                let table = (*header(raw_arr)).hash_table as *mut HashIndex;
                let mut found = 0usize;
                for bi in 0..((*table).slot_count >> BUCKET_SHIFT) {
                    let b = (*table).storage.add(bi);
                    for j in 0..BUCKET_LENGTH {
                        if (*b).index[j] >= 0 {
                            found = (*b).hash[j];
                        }
                    }
                }
                log.usz("stored_hash", found);
                assert_eq!(
                    found,
                    if want < 2 { want + 2 } else { want },
                    "{}: hash < 2 must be bumped",
                    lib.name
                );
                // and it must still be findable and deletable
                let (nt, idx) = shgeti(lib, t, es, empty.as_mut_ptr() as *mut c_char, HM_STRING);
                t = nt;
                log.isz("idx", idx);
                assert_eq!(idx, 0, "{}: must be findable", lib.name);
                let (nt, d) = shdel(lib, t, es, empty.as_mut_ptr() as *mut c_char, 0, HM_STRING);
                t = nt;
                log.isz("d", d);
                assert_eq!(d, 1, "{}: must be deletable", lib.name);
                snap_map(log, t, es, KeyKind::StringAt(0));
                hmfree(lib, t, es);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// row 53 - out-of-range `mode` enum values across the FFI boundary. C enums
//          accept any int; `mode >= STBDS_HM_STRING(1)` splits the space.
// ---------------------------------------------------------------------------
#[test]
fn err_53_out_of_range_mode_enum() {
    diff("err53", |lib, log| unsafe {
        let modes: [c_int; 14] = [
            c_int::MIN,
            -1000,
            -3,
            -2,
            -1,
            0,
            1,
            2,
            3,
            4,
            255,
            256,
            65536,
            c_int::MAX,
        ];
        let es = 16usize;
        for &m in &modes {
            // (a) which branch does the *insert* take? Observable through
            //     `string.mode` of a freshly created table (line 707).
            (lib.rand_seed)(0x3141_5926);
            let mut key = *b"probe!\0\0";
            let t = (lib.hmput_key)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                8,
                m,
            );
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let table = (*header(raw)).hash_table as *mut HashIndex;
            log.i32v("mode", m);
            log.u8v("string_mode", (*table).string.mode);
            assert_eq!(
                (*table).string.mode,
                if m >= 1 { 1u8 } else { 0u8 },
                "{}: mode {} branch",
                lib.name,
                m
            );
            log.usz("length", (*header(raw)).length);
            log.isz("temp", (*header(raw)).temp);
            hmfree(lib, t, es);

            // (b) the *hash function* chosen (hash_string vs hash_bytes) is
            //     observable through the stored bucket hash.
            (lib.rand_seed)(0x3141_5926);
            let mut k2 = *b"probe!\0\0";
            let t = (lib.hmput_key)(
                std::ptr::null_mut(),
                es,
                k2.as_mut_ptr() as *mut c_void,
                8,
                m,
            );
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            let table = (*header(raw)).hash_table as *mut HashIndex;
            let seed = (*table).seed;
            let expect = if m >= 1 {
                (lib.hash_string)(k2.as_mut_ptr() as *mut c_char, seed)
            } else {
                (lib.hash_bytes)(k2.as_mut_ptr() as *mut c_void, 8, seed)
            };
            let expect = if expect < 2 { expect + 2 } else { expect };
            let mut stored = 0usize;
            for bi in 0..((*table).slot_count >> BUCKET_SHIFT) {
                let b = (*table).storage.add(bi);
                for j in 0..BUCKET_LENGTH {
                    if (*b).index[j] >= 0 {
                        stored = (*b).hash[j];
                    }
                }
            }
            log.usz("stored", stored);
            log.usz("expect", expect);
            assert_eq!(stored, expect, "{}: mode {} hash choice", lib.name, m);
            hmfree(lib, t, es);

            // (c) hmdel_key's `mode == STBDS_HM_STRING` (exactly 1) test
            (lib.rand_seed)(0x3141_5926);
            let t0 = (lib.hmdel_key)(
                std::ptr::null_mut(),
                es,
                k2.as_mut_ptr() as *mut c_void,
                8,
                0,
                m,
            );
            log.flag("del_null", t0.is_null());
        }
    });
}

// ---------------------------------------------------------------------------
// row 54 - keysize == 0 in binary mode: `memcmp(..., 0) == 0` is always true,
//          so every key with a colliding hash compares equal, and
//          `memcpy(..., 0)` copies nothing.
// ---------------------------------------------------------------------------
#[test]
fn err_54_zero_keysize_binary() {
    diff("err54", |lib, log| unsafe {
        for &es in &[8usize, 16, 32] {
            (lib.rand_seed)(0x3141_5926);
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..25usize {
                // different key *pointers*, but keysize 0 -> the same hash
                let mut k = (i as u64).to_le_bytes();
                t = (lib.hmput_key)(t, es, k.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                let raw = (t as *mut u8).sub(es) as *mut c_void;
                log.usz("es", es);
                log.usz("i", i);
                log.isz("temp", (*header(raw)).temp);
                log.usz("length", (*header(raw)).length);
                assert_eq!(
                    (*header(raw)).length,
                    2,
                    "{}: keysize 0 collapses every key into one entry",
                    lib.name
                );
            }
            snap_map(log, t, es, KeyKind::Binary);
            // get and delete with keysize 0
            let mut k = [9u8; 8];
            let mut temp: isize = 0x55;
            let t = (lib.hmget_key_ts)(t, es, k.as_mut_ptr() as *mut c_void, 0, &mut temp, HM_BINARY);
            log.isz("get_ts", temp);
            let t = (lib.hmdel_key)(t, es, k.as_mut_ptr() as *mut c_void, 0, 0, HM_BINARY);
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            log.isz("del", (*header(raw)).temp);
            snap_map(log, t, es, KeyKind::Binary);
            let t = (lib.hmdel_key)(t, es, k.as_mut_ptr() as *mut c_void, 0, 0, HM_BINARY);
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            log.isz("del2", (*header(raw)).temp);
            snap_map(log, t, es, KeyKind::Binary);
            hmfree(lib, t, es);
        }
    });
}

// ---------------------------------------------------------------------------
// row 55 - keysize > elemsize: `memcpy(elem, key, keysize)` overruns the
//          element. `elemsize` and the element count are chosen so the overrun
//          stays inside the array's own allocation (capacity 4 x 64 bytes), so
//          the two libraries really are comparable rather than both corrupting
//          the heap.
// ---------------------------------------------------------------------------
#[test]
fn err_55_oversized_keysize() {
    diff("err55", |lib, log| unsafe {
        let es = 64usize;
        for &keysize in &[65usize, 72, 96, 128] {
            (lib.rand_seed)(0x3141_5926);
            let mut t: *mut c_void = std::ptr::null_mut();
            // 2 inserts only: elements live at raw offsets 0/64/128, capacity 4
            // means the array data spans [0, 256).
            for i in 0..2usize {
                let mut k: Vec<u8> = (0..keysize).map(|j| (i as u8) ^ (j as u8)).collect();
                t = (lib.hmput_key)(t, es, k.as_mut_ptr() as *mut c_void, keysize, HM_BINARY);
                let raw = (t as *mut u8).sub(es) as *mut c_void;
                log.usz("keysize", keysize);
                log.usz("i", i);
                log.isz("temp", (*header(raw)).temp);
                log.usz("length", (*header(raw)).length);
                log.usz("capacity", (*header(raw)).capacity);
            }
            let raw = (t as *mut u8).sub(es) as *mut c_void;
            // Dump ONLY the bytes the C definitely wrote:
            //   [0, es)                  memset by the bootstrap,
            //   [es, es + keysize)       memcpy'd by insert #1,
            //   [2*es, 2*es + keysize)   memcpy'd by insert #2.
            // Since keysize > es these three ranges are contiguous, so the
            // written region is exactly [0, 2*es + keysize). Anything past that
            // is uninitialised `realloc` memory and differs between the two
            // libraries for reasons unrelated to the translation.
            let dump = 2 * es + keysize;
            assert!(
                dump <= es * (*header(raw)).capacity,
                "{}: the overrun must stay inside the array allocation",
                lib.name
            );
            snap_array(log, raw, dump);
            // lookups
            for i in 0..2usize {
                let mut k: Vec<u8> = (0..keysize).map(|j| (i as u8) ^ (j as u8)).collect();
                let t2 = (lib.hmget_key)(t, es, k.as_mut_ptr() as *mut c_void, keysize, HM_BINARY);
                let raw = (t2 as *mut u8).sub(es) as *mut c_void;
                log.isz("idx", (*header(raw)).temp);
            }
            hmfree(lib, t, es);
        }
    });
}

// ---------------------------------------------------------------------------
// rows 56+57 - str_dups with non-positive `num` (the arena loop body never
//              runs) and the three str_dups asserts, which always hold
// ---------------------------------------------------------------------------
#[test]
fn err_56_57_str_dups_non_positive() {
    let p = pair();
    let _g = lock();
    for &n in &[0 as c_int, -1, -2, -1000, c_int::MIN, c_int::MIN + 1] {
        let oc = capture_stdout("c_np", || unsafe {
            (p.c.rand_seed)(0x3141_5926);
            (p.c.str_dups)(n);
        });
        let orr = capture_stdout("rs_np", || unsafe {
            (p.rs.rand_seed)(0x3141_5926);
            (p.rs.str_dups)(n);
        });
        assert_eq!(
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&orr),
            "str_dups({}) mismatch",
            n
        );
        // the strdup-map block still runs and prints exactly one line
        assert_eq!(
            String::from_utf8_lossy(&oc),
            format!("a {}\n", n),
            "str_dups({}) must print `a <num>` even for non-positive num",
            n
        );
    }
}

// ---------------------------------------------------------------------------
// row 58 - strkey at the int extremes
// ---------------------------------------------------------------------------
#[test]
fn err_58_strkey_extremes() {
    diff("err58", |lib, log| unsafe {
        for &n in &[
            c_int::MIN,
            c_int::MIN + 1,
            -1000000000,
            -1,
            0,
            1,
            1000000000,
            c_int::MAX - 1,
            c_int::MAX,
        ] {
            let p = (lib.strkey)(n);
            let b = cstr_bytes(p);
            log.i32v("n", n);
            log.blob("s", &b);
            assert_eq!(
                b,
                format!("test_{}", n).into_bytes(),
                "{}: strkey({})",
                lib.name,
                n
            );
            assert!(b.len() < 256, "{}: fits the 256-byte static buffer", lib.name);
        }
    });
}

// ---------------------------------------------------------------------------
// row 59 - arrfreef(NULL) frees `(char*)NULL - 32`. glibc rejects it with
//          "free(): invalid pointer" and aborts. Verified in forked children so
//          both implementations must die the same way.
// ---------------------------------------------------------------------------
#[test]
fn err_59_arrfreef_null_aborts_identically() {
    diff_child("err59", |lib, log| unsafe {
        log.tag("about_to_free_null");
        (lib.arrfreef)(std::ptr::null_mut());
        log.tag("survived");
    });
}

// ---------------------------------------------------------------------------
// row 60 - hash_string(NULL, seed) dereferences NULL. Verified in forked
//          children: both must die with the same signal.
// ---------------------------------------------------------------------------
#[test]
fn err_60_hash_string_null_aborts_identically() {
    diff_child("err60", |lib, log| unsafe {
        log.tag("about_to_hash_null");
        let h = (lib.hash_string)(std::ptr::null_mut(), 0x3141_5926);
        log.usz("h", h);
        log.tag("survived");
    });
}
