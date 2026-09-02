//! Phase C: one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact rejecting condition, calls BOTH libraries
//! through their `.so` exports, and asserts the SAME sentinel / error result
//! (not merely "both failed").

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void, CStr, CString};

unsafe fn arr_hdr(a: *mut c_void) -> String {
    if a.is_null() {
        return "NULL".to_string();
    }
    let h = &*((a as *mut u8).sub(HDRSIZE) as *mut ArrayHeader);
    format!(
        "length={} capacity={} temp={} table_null={}",
        h.length, h.capacity, h.temp, h.hash_table.is_null()
    )
}

// ===========================================================================
// row 1 -- arrgrowf request already satisfied -> returns `a` unchanged
// ===========================================================================
#[test]
fn e01_arrgrowf_noop() {
    let _g = serial();
    let p = pair();
    unsafe {
        // from NULL: returns NULL, allocates nothing
        for elemsize in [1usize, 8, 64] {
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(ca.is_null(), "C arrgrowf(NULL,{elemsize},0,0) must return NULL");
            assert!(ra.is_null(), "RUST arrgrowf(NULL,{elemsize},0,0) must return NULL");
        }
        // from an existing array: returns the SAME pointer, unmodified
        for elemsize in [1usize, 8, 64] {
            let mut ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let mut ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let (c0, r0) = (ca, ra);
            for (addlen, min_cap) in [(0usize, 0usize), (0, 1), (0, 4), (1, 0), (2, 3)] {
                ca = (p.c.arrgrowf)(ca, elemsize, addlen, min_cap);
                ra = (p.r.arrgrowf)(ra, elemsize, addlen, min_cap);
                assert_eq!(ca, c0, "C arrgrowf must be a no-op for ({addlen},{min_cap})");
                assert_eq!(ra, r0, "RUST arrgrowf must be a no-op for ({addlen},{min_cap})");
                assert_eq_dump("row1 header", &arr_hdr(ca), &arr_hdr(ra));
            }
            (p.c.arrfreef)(ca);
            (p.r.arrfreef)(ra);
        }
    }
}

// ===========================================================================
// row 2 -- arrgrowf(a == NULL) initialises the header
// ===========================================================================
#[test]
fn e02_arrgrowf_null_input() {
    let _g = serial();
    let p = pair();
    unsafe {
        for elemsize in [1usize, 4, 8, 16] {
            for (addlen, min_cap, expect_cap) in [
                (0usize, 1usize, 4usize),
                (1, 0, 4),
                (3, 0, 4),
                (4, 0, 4),
                (5, 0, 5),
                (0, 100, 100),
                (100, 0, 100),
                (7, 3, 7),
            ] {
                let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let what = format!("row2 arrgrowf(NULL,{elemsize},{addlen},{min_cap})");
                assert_eq_dump(&what, &arr_hdr(ca), &arr_hdr(ra));
                let ch = &*((ca as *mut u8).sub(HDRSIZE) as *mut ArrayHeader);
                assert_eq!(ch.length, 0, "{what}: length");
                assert_eq!(ch.temp, 0, "{what}: temp");
                assert!(ch.hash_table.is_null(), "{what}: hash_table");
                assert_eq!(ch.capacity, expect_cap, "{what}: capacity");
                (p.c.arrfreef)(ca);
                (p.r.arrfreef)(ra);
            }
        }
    }
}

// rows 3 and 4 are documented in ERRORS.md as crashes (realloc failure /
// missing NULL check in stbds_arrfreef) and are deliberately not executed.

// ===========================================================================
// row 5 -- hmfree_func(NULL) is an early return
// ===========================================================================
#[test]
fn e05_hmfree_null() {
    let _g = serial();
    let p = pair();
    unsafe {
        for elemsize in [0usize, 1, 8, 16, 4096] {
            (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (p.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
    // reaching here without a crash in either library is the assertion
}

// ===========================================================================
// row 6 -- hmfree_func on an array whose hash_table is NULL
// ===========================================================================
#[test]
fn e06_hmfree_no_table() {
    let _g = serial();
    let p = pair();
    unsafe {
        for elemsize in [1usize, 8, 16] {
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            assert_eq_dump("row6 pre-free header", &arr_hdr(ca), &arr_hdr(ra));
            (p.c.hmfree_func)(ca, elemsize);
            (p.r.hmfree_func)(ra, elemsize);
        }
        // also: the array produced by hmget_key_ts(NULL, ...) has no table
        let elemsize = 16usize;
        let key = 0u64;
        let mut ct: isize = 0;
        let mut rt: isize = 0;
        let ch = (p.c.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            &key as *const u64 as *mut c_void,
            8,
            &mut ct,
            STBDS_HM_BINARY,
        );
        let rh = (p.r.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            &key as *const u64 as *mut c_void,
            8,
            &mut rt,
            STBDS_HM_BINARY,
        );
        assert_eq!(ct, rt);
        (p.c.hmfree_func)((ch as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((rh as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ===========================================================================
// rows 7, 8 -- stbds_hm_find_slot returns -1 (empty-hash slot hit in the
// first inner loop and in the wrap-around loop).  Reached through get/del.
// ===========================================================================
unsafe fn build_binary_table(lib: &Lib, keys: &mut [u64], elemsize: usize) -> *mut c_void {
    let mut h: *mut c_void = std::ptr::null_mut();
    for (i, k) in keys.iter_mut().enumerate() {
        let (nh, _) = put_and_fill(
            lib,
            h,
            elemsize,
            8,
            k as *mut u64 as *mut c_void,
            STBDS_HM_BINARY,
            i as u8,
        );
        h = nh;
    }
    h
}

#[test]
fn e07_find_slot_miss() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 7);
    unsafe {
        for n in [1usize, 3, 8, 9, 17, 40, 130] {
            (p.c.rand_seed)(0x3141_5926);
            (p.r.rand_seed)(0x3141_5926);
            let mut keys: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
            let mut ck = keys.clone();
            let mut rk = keys.clone();
            let mut ch = build_binary_table(&p.c, &mut ck, 16);
            let mut rh = build_binary_table(&p.r, &mut rk, 16);
            // 300 absent keys -> hits both inner loops at many probe offsets
            for _ in 0..300 {
                let mut miss = rng.next_u64() | 1 << 63;
                if keys.contains(&miss) {
                    continue;
                }
                let kp = &mut miss as *mut u64 as *mut c_void;
                ch = (p.c.hmget_key)(ch, 16, kp, 8, STBDS_HM_BINARY);
                rh = (p.r.hmget_key)(rh, 16, kp, 8, STBDS_HM_BINARY);
                let ct = (*(((ch as *mut u8).sub(16)).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                let rt = (*(((rh as *mut u8).sub(16)).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                assert_eq!(ct, -1, "C: absent key must yield -1");
                assert_eq!(rt, -1, "RUST: absent key must yield -1");
            }
            keys.clear();
            (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

#[test]
fn e08_find_slot_miss_wrap() {
    // Deliberately overfill single buckets so probing wraps: string keys with
    // a shared prefix collide into long probe chains, and every miss must
    // still return -1 identically.
    let _g = serial();
    let p = pair();
    unsafe {
        for n in [8usize, 40, 200] {
            (p.c.rand_seed)(0xabc_def);
            (p.r.rand_seed)(0xabc_def);
            let keys: Vec<CString> =
                (0..n).map(|i| CString::new(format!("k{i:06}")).unwrap()).collect();
            let mut ch: *mut c_void = std::ptr::null_mut();
            let mut rh: *mut c_void = std::ptr::null_mut();
            for (i, k) in keys.iter().enumerate() {
                let kp = k.as_ptr() as *mut c_void;
                ch = put_and_fill(&p.c, ch, 16, 8, kp, STBDS_HM_STRING, i as u8).0;
                rh = put_and_fill(&p.r, rh, 16, 8, kp, STBDS_HM_STRING, i as u8).0;
            }
            for i in 0..500usize {
                let m = CString::new(format!("m{i:06}")).unwrap();
                let kp = m.as_ptr() as *mut c_void;
                ch = (p.c.hmget_key)(ch, 16, kp, 8, STBDS_HM_STRING);
                rh = (p.r.hmget_key)(rh, 16, kp, 8, STBDS_HM_STRING);
                let ct = (*(((ch as *mut u8).sub(16)).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                let rt = (*(((rh as *mut u8).sub(16)).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                assert_eq!(ct, -1);
                assert_eq!(rt, -1);
            }
            assert_eq_dump("row8 table", &dump_table(ch, 16, 8), &dump_table(rh, 16, 8));
            (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

// ===========================================================================
// rows 9, 10, 11, 12 -- hmget_key_ts / hmget_key sentinels
// ===========================================================================
#[test]
fn e09_hmget_ts_null() {
    let _g = serial();
    let p = pair();
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 64] {
            let mut key = [0xABu8; 64];
            let mut ct: isize = 0x1234;
            let mut rt: isize = 0x1234;
            let ch = (p.c.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                8,
                &mut ct,
                STBDS_HM_BINARY,
            );
            let rh = (p.r.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                8,
                &mut rt,
                STBDS_HM_BINARY,
            );
            assert_eq!(ct, -1, "C: *temp must be STBDS_INDEX_EMPTY");
            assert_eq!(rt, -1, "RUST: *temp must be STBDS_INDEX_EMPTY");
            assert!(!ch.is_null() && !rh.is_null(), "must return a fresh array");
            assert_eq_dump(
                &format!("row9 elemsize={elemsize}"),
                &dump_table(ch, elemsize, 8),
                &dump_table(rh, elemsize, 8),
            );
            (p.c.hmfree_func)((ch as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((rh as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn e10_hmget_ts_no_table() {
    let _g = serial();
    let p = pair();
    unsafe {
        let elemsize = 16usize;
        let mut key = [7u8; 8];
        let mut ct: isize = 0;
        let mut rt: isize = 0;
        let mut ch = (p.c.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            8,
            &mut ct,
            STBDS_HM_BINARY,
        );
        let mut rh = (p.r.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            8,
            &mut rt,
            STBDS_HM_BINARY,
        );
        // second call: hash_table is still NULL -> *temp = -1, `a` returned
        // unchanged, and the key is never dereferenced (NULL key is fine).
        for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2, -1, i32::MAX, i32::MIN] {
            ct = 0x55;
            rt = 0x55;
            let c_prev = ch;
            let r_prev = rh;
            ch = (p.c.hmget_key_ts)(
                ch,
                elemsize,
                std::ptr::null_mut(),
                8,
                &mut ct,
                mode as c_int,
            );
            rh = (p.r.hmget_key_ts)(
                rh,
                elemsize,
                std::ptr::null_mut(),
                8,
                &mut rt,
                mode as c_int,
            );
            assert_eq!(ch, c_prev, "C must return `a` unchanged (mode={mode})");
            assert_eq!(rh, r_prev, "RUST must return `a` unchanged (mode={mode})");
            assert_eq!(ct, -1, "C *temp (mode={mode})");
            assert_eq!(rt, -1, "RUST *temp (mode={mode})");
        }
        (p.c.hmfree_func)((ch as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((rh as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

#[test]
fn e11_hmget_ts_absent() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 11);
    unsafe {
        (p.c.rand_seed)(1);
        (p.r.rand_seed)(1);
        let mut ck: Vec<u64> = (0..40).map(|_| rng.next_u64() & 0x0fff_ffff).collect();
        let mut rk = ck.clone();
        let mut ch = build_binary_table(&p.c, &mut ck, 16);
        let mut rh = build_binary_table(&p.r, &mut rk, 16);
        for i in 0..200u64 {
            let mut miss = 0xffff_0000_0000_0000u64 | i;
            let kp = &mut miss as *mut u64 as *mut c_void;
            let mut ct: isize = 9;
            let mut rt: isize = 9;
            ch = (p.c.hmget_key_ts)(ch, 16, kp, 8, &mut ct, STBDS_HM_BINARY);
            rh = (p.r.hmget_key_ts)(rh, 16, kp, 8, &mut rt, STBDS_HM_BINARY);
            assert_eq!(ct, -1);
            assert_eq!(rt, -1);
        }
        (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
        (p.r.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
    }
}

#[test]
fn e12_hmget_key_sentinel() {
    let _g = serial();
    let p = pair();
    unsafe {
        let elemsize = 16usize;
        let mut key = [1u8; 8];
        // (a) NULL table
        let ch = (p.c.hmget_key)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            8,
            STBDS_HM_BINARY,
        );
        let rh = (p.r.hmget_key)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            8,
            STBDS_HM_BINARY,
        );
        let ct = (*((ch as *mut u8).sub(elemsize).sub(HDRSIZE) as *mut ArrayHeader)).temp;
        let rt = (*((rh as *mut u8).sub(elemsize).sub(HDRSIZE) as *mut ArrayHeader)).temp;
        assert_eq!(ct, -1, "C: hmget_key must write -1 into hdr->temp");
        assert_eq!(rt, -1, "RUST: hmget_key must write -1 into hdr->temp");
        assert_eq_dump("row12", &dump_table(ch, elemsize, 8), &dump_table(rh, elemsize, 8));
        (p.c.hmfree_func)((ch as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((rh as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ===========================================================================
// rows 13, 14, 15, 16 -- hmdel_key
// ===========================================================================
#[test]
fn e13_hmdel_null() {
    let _g = serial();
    let p = pair();
    unsafe {
        for elemsize in [0usize, 1, 8, 16] {
            for keysize in [0usize, 1, 8] {
                for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2, -1] {
                    let cv = (p.c.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        std::ptr::null_mut(),
                        keysize,
                        0,
                        mode as c_int,
                    );
                    let rv = (p.r.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        std::ptr::null_mut(),
                        keysize,
                        0,
                        mode as c_int,
                    );
                    assert!(cv.is_null(), "C hmdel_key(NULL) must return NULL");
                    assert!(rv.is_null(), "RUST hmdel_key(NULL) must return NULL");
                }
            }
        }
    }
}

#[test]
fn e14_hmdel_no_table() {
    let _g = serial();
    let p = pair();
    unsafe {
        let elemsize = 16usize;
        let mut key = [3u8; 8];
        let mut ct: isize = 0;
        let mut rt: isize = 0;
        let ch = (p.c.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            8,
            &mut ct,
            STBDS_HM_BINARY,
        );
        let rh = (p.r.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            8,
            &mut rt,
            STBDS_HM_BINARY,
        );
        // seed temp with a non-zero value so we can see hmdel_key zero it
        (*((ch as *mut u8).sub(elemsize).sub(HDRSIZE) as *mut ArrayHeader)).temp = 42;
        (*((rh as *mut u8).sub(elemsize).sub(HDRSIZE) as *mut ArrayHeader)).temp = 42;
        let cv = (p.c.hmdel_key)(ch, elemsize, std::ptr::null_mut(), 8, 0, STBDS_HM_BINARY);
        let rv = (p.r.hmdel_key)(rh, elemsize, std::ptr::null_mut(), 8, 0, STBDS_HM_BINARY);
        assert_eq!(cv, ch, "C must return `a`");
        assert_eq!(rv, rh, "RUST must return `a`");
        let ct = (*((cv as *mut u8).sub(elemsize).sub(HDRSIZE) as *mut ArrayHeader)).temp;
        let rt = (*((rv as *mut u8).sub(elemsize).sub(HDRSIZE) as *mut ArrayHeader)).temp;
        assert_eq!(ct, 0, "C: temp must be reset to 0");
        assert_eq!(rt, 0, "RUST: temp must be reset to 0");
        (p.c.hmfree_func)((cv as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((rv as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

#[test]
fn e15_hmdel_absent() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 15);
    unsafe {
        (p.c.rand_seed)(2);
        (p.r.rand_seed)(2);
        let mut ck: Vec<u64> = (0..30).map(|_| rng.next_u64() & 0xffff).collect();
        let mut rk = ck.clone();
        let mut ch = build_binary_table(&p.c, &mut ck, 16);
        let mut rh = build_binary_table(&p.r, &mut rk, 16);
        let before_c = dump_table(ch, 16, 8);
        let before_r = dump_table(rh, 16, 8);
        assert_eq_dump("row15 before", &before_c, &before_r);
        for i in 0..100u64 {
            let mut miss = 0xdead_0000_0000_0000u64 | i;
            let kp = &mut miss as *mut u64 as *mut c_void;
            ch = (p.c.hmdel_key)(ch, 16, kp, 8, 0, STBDS_HM_BINARY);
            rh = (p.r.hmdel_key)(rh, 16, kp, 8, 0, STBDS_HM_BINARY);
            let ct = (*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
            let rt = (*((rh as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
            assert_eq!(ct, 0, "C: absent delete leaves temp == 0");
            assert_eq!(rt, 0, "RUST: absent delete leaves temp == 0");
        }
        // table content untouched apart from `temp`
        let after_c = dump_table(ch, 16, 8);
        let after_r = dump_table(rh, 16, 8);
        assert_eq_dump("row15 after", &after_c, &after_r);
        assert_eq!(
            before_c.lines().skip(1).collect::<Vec<_>>(),
            after_c.lines().skip(1).collect::<Vec<_>>(),
            "absent deletes must not modify the table"
        );
        (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
        (p.r.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
    }
}

#[test]
fn e16_hmdel_twice() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 16);
    unsafe {
        for n in [1usize, 5, 20, 90] {
            (p.c.rand_seed)(3);
            (p.r.rand_seed)(3);
            let keys: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
            let mut ck = keys.clone();
            let mut rk = keys.clone();
            let mut ch = build_binary_table(&p.c, &mut ck, 16);
            let mut rh = build_binary_table(&p.r, &mut rk, 16);
            for i in 0..n {
                let mut k = keys[i];
                let kp = &mut k as *mut u64 as *mut c_void;
                ch = (p.c.hmdel_key)(ch, 16, kp, 8, 0, STBDS_HM_BINARY);
                rh = (p.r.hmdel_key)(rh, 16, kp, 8, 0, STBDS_HM_BINARY);
                let c1 = (*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                let r1 = (*((rh as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                assert_eq!(c1, 1, "C: first delete of a present key sets temp = 1");
                assert_eq!(r1, 1, "RUST: first delete of a present key sets temp = 1");
                ch = (p.c.hmdel_key)(ch, 16, kp, 8, 0, STBDS_HM_BINARY);
                rh = (p.r.hmdel_key)(rh, 16, kp, 8, 0, STBDS_HM_BINARY);
                let c2 = (*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                let r2 = (*((rh as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                assert_eq!(c2, 0, "C: second delete leaves temp = 0");
                assert_eq!(r2, 0, "RUST: second delete leaves temp = 0");
                assert_eq_dump(
                    &format!("row16 n={n} i={i}"),
                    &dump_table(ch, 16, 8),
                    &dump_table(rh, 16, 8),
                );
            }
            (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

// rows 17-23 are asserts that ERRORS.md documents as unreachable from the
// public ABI (or vacuous); they are not executed because a failing assert
// aborts the process in both libraries.

// ===========================================================================
// rows 24, 25, 26 -- stbds_stralloc boundaries
// ===========================================================================
#[test]
fn e24_stralloc_oversize() {
    let _g = serial();
    let p = pair();
    unsafe {
        // (a) storage == NULL branch
        for len in [512usize, 513, 5000] {
            let mut ca = StringArena::new();
            let mut ra = StringArena::new();
            let s = CString::new(vec![b'A'; len]).unwrap();
            let cp = (p.c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
            let rp = (p.r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
            assert_eq!(CStr::from_ptr(cp).to_bytes(), s.to_bytes());
            assert_eq!(CStr::from_ptr(rp).to_bytes(), s.to_bytes());
            assert_eq_dump(
                &format!("row24a len={len}"),
                &dump_arena(&ca),
                &dump_arena(&ra),
            );
            assert_eq!(ca.remaining, 0, "oversize into fresh arena leaves remaining = 0");
            assert_eq!(ca.block, 1, "block was incremented once");
            assert_eq!(ra.remaining, 0);
            assert_eq!(ra.block, 1);
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
        }
        // (b) storage != NULL branch: `remaining` is NOT touched
        for len in [600usize, 4000] {
            let mut ca = StringArena::new();
            let mut ra = StringArena::new();
            let small = CString::new("abc").unwrap();
            (p.c.stralloc)(&mut ca, small.as_ptr() as *mut c_char);
            (p.r.stralloc)(&mut ra, small.as_ptr() as *mut c_char);
            let rem_before = ca.remaining;
            assert_eq!(rem_before, ra.remaining);
            let s = CString::new(vec![b'B'; len]).unwrap();
            let cp = (p.c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
            let rp = (p.r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
            assert_eq!(CStr::from_ptr(cp).to_bytes(), s.to_bytes());
            assert_eq!(CStr::from_ptr(rp).to_bytes(), s.to_bytes());
            assert_eq!(ca.remaining, rem_before, "C: remaining must be unchanged");
            assert_eq!(ra.remaining, rem_before, "RUST: remaining must be unchanged");
            assert_eq_dump(
                &format!("row24b len={len}"),
                &dump_arena(&ca),
                &dump_arena(&ra),
            );
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
        }
    }
}

#[test]
fn e25_stralloc_empty_string() {
    let _g = serial();
    let p = pair();
    unsafe {
        let mut ca = StringArena::new();
        let mut ra = StringArena::new();
        let e = CString::new("").unwrap();
        let cp = (p.c.stralloc)(&mut ca, e.as_ptr() as *mut c_char);
        let rp = (p.r.stralloc)(&mut ra, e.as_ptr() as *mut c_char);
        assert_eq!(*cp, 0);
        assert_eq!(*rp, 0);
        assert_eq!(ca.block, 1);
        assert_eq!(ca.remaining, 511);
        assert_eq!(ra.block, 1);
        assert_eq!(ra.remaining, 511);
        assert_eq_dump("row25", &dump_arena(&ca), &dump_arena(&ra));
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
}

#[test]
fn e26_stralloc_block_saturation() {
    let _g = serial();
    let p = pair();
    unsafe {
        let mut ca = StringArena::new();
        let mut ra = StringArena::new();
        let mut iters = 0;
        while ca.block < 24 && iters < 400 {
            iters += 1;
            let rem = ca.remaining;
            assert_eq!(rem, ra.remaining, "arenas desynchronised");
            let len = if rem > 1 { (rem - 1).min(120_000) } else { 1 };
            let s = CString::new(vec![b'c'; len]).unwrap();
            (p.c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
            (p.r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
            assert_eq!(ca.block, ra.block, "block field desynchronised");
        }
        // blocksize = 512 << (block>>1); saturates at 1<<20 -> block stops at 22
        assert_eq!(ca.block, 22, "C block saturation value");
        assert_eq!(ra.block, 22, "RUST block saturation value");
        for _ in 0..6 {
            let rem = ca.remaining;
            let len = if rem > 1 { rem - 1 } else { 1 };
            let s = CString::new(vec![b'd'; len]).unwrap();
            (p.c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
            (p.r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
            assert_eq!(ca.block, 22, "C: block must not grow past saturation");
            assert_eq!(ra.block, 22, "RUST: block must not grow past saturation");
        }
        assert_eq_dump("row26", &dump_arena(&ca), &dump_arena(&ra));
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
}

// ===========================================================================
// rows 27, 28 -- stbds_strreset
// ===========================================================================
#[test]
fn e27_strreset_empty() {
    let _g = serial();
    let p = pair();
    unsafe {
        let mut ca = StringArena::new();
        let mut ra = StringArena::new();
        ca.block = 9;
        ca.mode = 3;
        ca.remaining = 1234;
        ra.block = 9;
        ra.mode = 3;
        ra.remaining = 1234;
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
        for (n, a) in [("C", &ca), ("RUST", &ra)] {
            assert!(a.storage.is_null(), "{n}: storage");
            assert_eq!(a.remaining, 0, "{n}: remaining");
            assert_eq!(a.block, 0, "{n}: block");
            assert_eq!(a.mode, 0, "{n}: mode");
        }
        assert_eq_dump("row27", &dump_arena(&ca), &dump_arena(&ra));
    }
}

#[test]
fn e28_strreset_twice() {
    let _g = serial();
    let p = pair();
    unsafe {
        let mut ca = StringArena::new();
        let mut ra = StringArena::new();
        let s = CString::new("hello").unwrap();
        for _ in 0..40 {
            (p.c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
            (p.r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
        }
        for _ in 0..3 {
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
            assert_eq_dump("row28", &dump_arena(&ca), &dump_arena(&ra));
            assert!(ca.storage.is_null() && ra.storage.is_null());
        }
    }
}

// ===========================================================================
// rows 29, 30, 31 -- hashing degenerate inputs
// ===========================================================================
#[test]
fn e29_hash_bytes_zero_len() {
    let _g = serial();
    let p = pair();
    unsafe {
        let mut buf = [0xEEu8; 16];
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            let cv = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            let rv = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(cv, rv, "hash_bytes(_,0,{seed:#x})");
            assert_ne!(cv, 0, "len==0 must still run the finalisation rounds");
        }
    }
}

#[test]
fn e30_hash_bytes_null_zero_len() {
    let _g = serial();
    let p = pair();
    unsafe {
        for seed in [0usize, 1, usize::MAX, 0xfeed_face] {
            let cv = (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let rv = (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(cv, rv, "hash_bytes(NULL,0,{seed:#x})");
        }
    }
}

#[test]
fn e31_hash_string_empty() {
    let _g = serial();
    let p = pair();
    unsafe {
        let e = CString::new("").unwrap();
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            let cv = (p.c.hash_string)(e.as_ptr() as *mut c_char, seed);
            let rv = (p.r.hash_string)(e.as_ptr() as *mut c_char, seed);
            assert_eq!(cv, rv, "hash_string(\"\",{seed:#x})");
        }
    }
}

// ===========================================================================
// row 32 -- out-of-range `mode` enum ints across the FFI boundary
// ===========================================================================
#[test]
fn e32_mode_out_of_range_string_side() {
    // every mode >= STBDS_HM_STRING behaves as STRING
    let _g = serial();
    let p = pair();
    unsafe {
        let keys: Vec<CString> =
            (0..30).map(|i| CString::new(format!("key_{i}")).unwrap()).collect();
        let mut reference: Option<String> = None;
        for mode in [1i32, 2, 7, 1000, i32::MAX] {
            (p.c.rand_seed)(0x4444);
            (p.r.rand_seed)(0x4444);
            let mut ch: *mut c_void = std::ptr::null_mut();
            let mut rh: *mut c_void = std::ptr::null_mut();
            for (i, k) in keys.iter().enumerate() {
                let kp = k.as_ptr() as *mut c_void;
                ch = put_and_fill(&p.c, ch, 16, 8, kp, mode, i as u8).0;
                rh = put_and_fill(&p.r, rh, 16, 8, kp, mode, i as u8).0;
            }
            for k in &keys {
                let kp = k.as_ptr() as *mut c_void;
                ch = (p.c.hmget_key)(ch, 16, kp, 8, mode);
                rh = (p.r.hmget_key)(rh, 16, kp, 8, mode);
                let ct = (*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                let rt = (*((rh as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                assert!(ct >= 0, "mode={mode}: present key must be found (C)");
                assert_eq!(ct, rt, "mode={mode}: index mismatch");
            }
            let cd = dump_table(ch, 16, 8);
            let rd = dump_table(rh, 16, 8);
            assert_eq_dump(&format!("row32 string-side mode={mode}"), &cd, &rd);
            // all of these modes must produce the *same* table as mode == 1
            match &reference {
                None => reference = Some(cd),
                Some(r0) => assert_eq!(
                    r0, &cd,
                    "mode={mode} must behave exactly like STBDS_HM_STRING"
                ),
            }
            (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

#[test]
fn e32_mode_out_of_range_binary_side() {
    // every mode < STBDS_HM_STRING behaves as BINARY
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 0x32);
    unsafe {
        let keys: Vec<u64> = (0..30).map(|_| rng.next_u64()).collect();
        let mut reference: Option<String> = None;
        for mode in [0i32, -1, -7, i32::MIN] {
            (p.c.rand_seed)(0x5555);
            (p.r.rand_seed)(0x5555);
            let mut ck = keys.clone();
            let mut rk = keys.clone();
            let mut ch: *mut c_void = std::ptr::null_mut();
            let mut rh: *mut c_void = std::ptr::null_mut();
            for i in 0..keys.len() {
                ch = (p.c.hmput_key)(ch, 16, &mut ck[i] as *mut u64 as *mut c_void, 8, mode);
                rh = (p.r.hmput_key)(rh, 16, &mut rk[i] as *mut u64 as *mut c_void, 8, mode);
            }
            for i in 0..keys.len() {
                ch = (p.c.hmget_key)(ch, 16, &mut ck[i] as *mut u64 as *mut c_void, 8, mode);
                rh = (p.r.hmget_key)(rh, 16, &mut rk[i] as *mut u64 as *mut c_void, 8, mode);
                let ct = (*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                let rt = (*((rh as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                assert!(ct >= 0, "mode={mode}: present key must be found (C)");
                assert_eq!(ct, rt);
            }
            for i in 0..keys.len() {
                ch = (p.c.hmdel_key)(ch, 16, &mut ck[i] as *mut u64 as *mut c_void, 8, 0, mode);
                rh = (p.r.hmdel_key)(rh, 16, &mut rk[i] as *mut u64 as *mut c_void, 8, 0, mode);
                let ct = (*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                let rt = (*((rh as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                assert_eq!(ct, 1, "mode={mode}: delete of present key");
                assert_eq!(ct, rt);
            }
            let cd = dump_table(ch, 16, 8);
            let rd = dump_table(rh, 16, 8);
            assert_eq_dump(&format!("row32 binary-side mode={mode}"), &cd, &rd);
            match &reference {
                None => reference = Some(cd),
                Some(r0) => assert_eq!(
                    r0, &cd,
                    "mode={mode} must behave exactly like STBDS_HM_BINARY"
                ),
            }
            (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

#[test]
fn e32_mode_out_of_range_del_exact_string_check() {
    // stbds_hmdel_key uses `mode == STBDS_HM_STRING` (exact 1), not
    // `mode >= STBDS_HM_STRING`, for the strdup-free and the re-find.  With
    // mode == 2 the delete therefore skips the free and takes the *binary*
    // re-find path.  We delete the most recently inserted key so that
    // old_index == final_index and no re-find happens (otherwise the C's
    // `STBDS_ASSERT(slot >= 0)` would abort -- which is exactly what ERRORS.md
    // row 19 documents).
    let _g = serial();
    let p = pair();
    unsafe {
        for sh in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for mode in [2i32, 9, i32::MAX] {
                (p.c.rand_seed)(0x6666);
                (p.r.rand_seed)(0x6666);
                let keys: Vec<CString> =
                    (0..6).map(|i| CString::new(format!("zz_{i}")).unwrap()).collect();
                let mut ch = (p.c.shmode_func)(16, sh);
                let mut rh = (p.r.shmode_func)(16, sh);
                for (i, k) in keys.iter().enumerate() {
                    let kp = k.as_ptr() as *mut c_void;
                    ch = put_and_fill(&p.c, ch, 16, 8, kp, mode, i as u8).0;
                    rh = put_and_fill(&p.r, rh, 16, 8, kp, mode, i as u8).0;
                }
                // delete newest -> oldest: old_index == final_index every time
                for k in keys.iter().rev() {
                    let kp = k.as_ptr() as *mut c_void;
                    // Capture the STORED key pointer before the delete.  With
                    // mode != 1 the C skips `free()` on the strdup'd key
                    // (`mode == STBDS_HM_STRING` is an exact comparison), so the
                    // bytes behind this pointer must still be intact afterwards.
                    let ci_before = {
                        let mut t: isize = 0;
                        (p.c.hmget_key_ts)(ch, 16, kp, 8, &mut t, mode);
                        t
                    };
                    let ri_before = {
                        let mut t: isize = 0;
                        (p.r.hmget_key_ts)(rh, 16, kp, 8, &mut t, mode);
                        t
                    };
                    assert_eq!(ci_before, ri_before);
                    let c_keyp = *((ch as *mut u8).offset(ci_before * 16) as *mut *mut c_char);
                    let r_keyp = *((rh as *mut u8).offset(ri_before * 16) as *mut *mut c_char);

                    ch = (p.c.hmdel_key)(ch, 16, kp, 8, 0, mode);
                    rh = (p.r.hmdel_key)(rh, 16, kp, 8, 0, mode);
                    let ct = temp_of(ch, 16);
                    let rt = temp_of(rh, 16);
                    assert_eq!(ct, 1, "sh={sh} mode={mode}: delete must report temp=1");
                    assert_eq!(ct, rt, "sh={sh} mode={mode}");
                    // If either library had freed the key, glibc would have
                    // written its tcache bookkeeping over these bytes.
                    let c_after = CStr::from_ptr(c_keyp).to_bytes().to_vec();
                    let r_after = CStr::from_ptr(r_keyp).to_bytes().to_vec();
                    assert_eq!(
                        c_after,
                        k.to_bytes(),
                        "sh={sh} mode={mode}: C must NOT free the key when mode != STBDS_HM_STRING"
                    );
                    assert_eq!(
                        r_after, c_after,
                        "sh={sh} mode={mode}: key-buffer state diverged (a free() was added or removed)"
                    );
                    assert_eq_dump(
                        &format!("row32-del sh={sh} mode={mode}"),
                        &dump_table(ch, 16, 8),
                        &dump_table(rh, 16, 8),
                    );
                }
                (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
                (p.r.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
}

// ===========================================================================
// row 33 -- stbds_shmode_func out-of-range mode is truncated to u8
// ===========================================================================
#[test]
fn e33_shmode_out_of_range() {
    let _g = serial();
    let p = pair();
    unsafe {
        for mode in [4i32, 5, 255, 256, 257, 258, 259, -1, -2, i32::MAX, i32::MIN] {
            (p.c.rand_seed)(0x7777);
            (p.r.rand_seed)(0x7777);
            let ch = (p.c.shmode_func)(16, mode);
            let rh = (p.r.shmode_func)(16, mode);
            let cm = (*((*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).hash_table
                as *mut HashIndex))
                .string
                .mode;
            let rm = (*((*((rh as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).hash_table
                as *mut HashIndex))
                .string
                .mode;
            assert_eq!(cm, (mode as u32 & 0xff) as u8, "C: mode {mode} must truncate");
            assert_eq!(rm, cm, "RUST: mode {mode} truncation mismatch");
            assert_eq_dump(
                &format!("row33 mode={mode}"),
                &dump_table(ch, 16, 8),
                &dump_table(rh, 16, 8),
            );
            (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
        }

        // and: truncated modes that land on the `default:` memcpy branch
        // (0, 4, 256, ...) really do store raw key bytes
        for mode in [4i32, 256, 260] {
            (p.c.rand_seed)(0x8888);
            (p.r.rand_seed)(0x8888);
            let keys: Vec<CString> = (0..5)
                .map(|i| CString::new(format!("long_enough_key_{i}")).unwrap())
                .collect();
            let mut ch = (p.c.shmode_func)(24, mode);
            let mut rh = (p.r.shmode_func)(24, mode);
            for k in &keys {
                let kp = k.as_ptr() as *mut c_void;
                ch = (p.c.hmput_key)(ch, 24, kp, 8, STBDS_HM_STRING);
                rh = (p.r.hmput_key)(rh, 24, kp, 8, STBDS_HM_STRING);
                // initialise the value bytes so the dumps hold no realloc garbage
                let ci = (*((ch as *mut u8).sub(24).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                let ri = (*((rh as *mut u8).sub(24).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                for b in 8..24usize {
                    *(ch as *mut u8).offset(ci * 24).add(b) = b as u8;
                    *(rh as *mut u8).offset(ri * 24).add(b) = b as u8;
                }
            }
            assert_eq_dump(
                &format!("row33 memcpy-branch mode={mode}"),
                &dump_table(ch, 24, 8),
                &dump_table(rh, 24, 8),
            );
            (p.c.hmfree_func)((ch as *mut u8).sub(24) as *mut c_void, 24);
            (p.r.hmfree_func)((rh as *mut u8).sub(24) as *mut c_void, 24);
        }
    }
}

// ===========================================================================
// row 34 -- stbds_shmode_func(elemsize == 0)
// ===========================================================================
#[test]
fn e34_shmode_zero_elemsize() {
    let _g = serial();
    let p = pair();
    unsafe {
        for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            (p.c.rand_seed)(0x9999);
            (p.r.rand_seed)(0x9999);
            let ch = (p.c.shmode_func)(0, mode);
            let rh = (p.r.shmode_func)(0, mode);
            assert!(!ch.is_null() && !rh.is_null());
            let chh = &*((ch as *mut u8).sub(HDRSIZE) as *mut ArrayHeader);
            let rhh = &*((rh as *mut u8).sub(HDRSIZE) as *mut ArrayHeader);
            assert_eq!(chh.length, 1, "C length");
            assert_eq!(rhh.length, 1, "RUST length");
            assert_eq!(chh.capacity, rhh.capacity, "capacity");
            assert_eq_dump(
                &format!("row34 mode={mode}"),
                &dump_table(ch, 0, 0),
                &dump_table(rh, 0, 0),
            );
            (p.c.hmfree_func)(ch, 0);
            (p.r.hmfree_func)(rh, 0);
        }
    }
}

// ===========================================================================
// rows 35, 36, 37 -- stbds_hmput_default
// ===========================================================================
#[test]
fn e35_hmput_default_null() {
    let _g = serial();
    let p = pair();
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 64] {
            let ch = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let rh = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
            assert!(!ch.is_null() && !rh.is_null());
            assert_eq_dump(
                &format!("row35 elemsize={elemsize}"),
                &dump_table(ch, elemsize, elemsize.min(8)),
                &dump_table(rh, elemsize, elemsize.min(8)),
            );
            let chh = &*((ch as *mut u8).sub(elemsize).sub(HDRSIZE) as *mut ArrayHeader);
            assert_eq!(chh.length, 1);
            // element 0 must be zeroed
            let z = std::slice::from_raw_parts((ch as *mut u8).sub(elemsize), elemsize);
            assert!(z.iter().all(|b| *b == 0), "default element must be zeroed");
            (p.c.hmfree_func)((ch as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((rh as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn e36_hmput_default_zero_len() {
    let _g = serial();
    let p = pair();
    unsafe {
        for elemsize in [1usize, 8, 16] {
            // arrgrowf leaves length == 0
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            assert_eq!((*((ca as *mut u8).sub(HDRSIZE) as *mut ArrayHeader)).length, 0);
            let ch = (p.c.hmput_default)((ca as *mut u8).add(elemsize) as *mut c_void, elemsize);
            let rh = (p.r.hmput_default)((ra as *mut u8).add(elemsize) as *mut c_void, elemsize);
            assert_eq_dump(
                &format!("row36 elemsize={elemsize}"),
                &dump_table(ch, elemsize, elemsize.min(8)),
                &dump_table(rh, elemsize, elemsize.min(8)),
            );
            assert_eq!(
                (*((ch as *mut u8).sub(elemsize).sub(HDRSIZE) as *mut ArrayHeader)).length,
                1
            );
            (p.c.hmfree_func)((ch as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((rh as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn e37_hmput_default_idempotent() {
    let _g = serial();
    let p = pair();
    unsafe {
        let elemsize = 16usize;
        let mut ch = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
        let mut rh = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
        // write a marker into the default element
        for b in 0..elemsize {
            *(ch as *mut u8).sub(elemsize).add(b) = 0xA5;
            *(rh as *mut u8).sub(elemsize).add(b) = 0xA5;
        }
        let c_prev = ch;
        let r_prev = rh;
        for _ in 0..3 {
            ch = (p.c.hmput_default)(ch, elemsize);
            rh = (p.r.hmput_default)(rh, elemsize);
            assert_eq!(ch, c_prev, "C: must return `a` unchanged");
            assert_eq!(rh, r_prev, "RUST: must return `a` unchanged");
            let z = std::slice::from_raw_parts((ch as *mut u8).sub(elemsize), elemsize);
            assert!(z.iter().all(|b| *b == 0xA5), "default element must NOT be re-zeroed");
            assert_eq_dump("row37", &dump_table(ch, elemsize, 8), &dump_table(rh, elemsize, 8));
        }
        (p.c.hmfree_func)((ch as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((rh as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ===========================================================================
// rows 38, 39, 40 -- stbds_hmput_key
// ===========================================================================
#[test]
fn e38_hmput_null_bootstrap() {
    let _g = serial();
    let p = pair();
    unsafe {
        for mode in [STBDS_HM_BINARY, STBDS_HM_STRING] {
            (p.c.rand_seed)(0xaaaa);
            (p.r.rand_seed)(0xaaaa);
            let k = CString::new("bootstrap").unwrap();
            let mut bk = 0x1122_3344_5566_7788u64;
            let kp = if mode == STBDS_HM_STRING {
                k.as_ptr() as *mut c_void
            } else {
                &mut bk as *mut u64 as *mut c_void
            };
            let ch = (p.c.hmput_key)(std::ptr::null_mut(), 16, kp, 8, mode);
            let rh = (p.r.hmput_key)(std::ptr::null_mut(), 16, kp, 8, mode);
            let ct = (*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
            let rt = (*((rh as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
            assert_eq!(ct, 0, "C: first key goes to index 0");
            assert_eq!(rt, 0, "RUST: first key goes to index 0");
            for b in 8..16usize {
                *(ch as *mut u8).add(b) = 0x11;
                *(rh as *mut u8).add(b) = 0x11;
            }
            assert_eq_dump(
                &format!("row38 mode={mode}"),
                &dump_table(ch, 16, 8),
                &dump_table(rh, 16, 8),
            );
            (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

#[test]
fn e39_hmput_existing_key() {
    let _g = serial();
    let p = pair();
    unsafe {
        for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
            (p.c.rand_seed)(0xbbbb);
            (p.r.rand_seed)(0xbbbb);
            let keys: Vec<CString> =
                (0..12).map(|i| CString::new(format!("dup_{i}")).unwrap()).collect();
            let (mut ch, mut rh) = match sh {
                None => (std::ptr::null_mut(), std::ptr::null_mut()),
                Some(m) => ((p.c.shmode_func)(16, m), (p.r.shmode_func)(16, m)),
            };
            for k in &keys {
                let kp = k.as_ptr() as *mut c_void;
                ch = (p.c.hmput_key)(ch, 16, kp, 8, STBDS_HM_STRING);
                rh = (p.r.hmput_key)(rh, 16, kp, 8, STBDS_HM_STRING);
                let ci = (*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                let ri = (*((rh as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                for b in 8..16usize {
                    *(ch as *mut u8).offset(ci * 16).add(b) = 0x22;
                    *(rh as *mut u8).offset(ri * 16).add(b) = 0x22;
                }
            }
            let c_len = (*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).length;
            let c_used = (*((*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader))
                .hash_table as *mut HashIndex))
                .used_count;
            // re-put every key with a *fresh* pointer to identical content
            for k in &keys {
                let dup = CString::new(k.to_bytes()).unwrap();
                let kp = dup.as_ptr() as *mut c_void;
                ch = (p.c.hmput_key)(ch, 16, kp, 8, STBDS_HM_STRING);
                rh = (p.r.hmput_key)(rh, 16, kp, 8, STBDS_HM_STRING);
                let ci = (*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                let ri = (*((rh as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).temp;
                assert!(ci >= 0);
                assert_eq!(ci, ri, "sh={sh:?}: existing-key index mismatch");
                // temp_key must be the STORED key pointer, not `dup`
                let ct = (*((*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader))
                    .hash_table as *mut HashIndex))
                    .temp_key;
                let rtk = (*((*((rh as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader))
                    .hash_table as *mut HashIndex))
                    .temp_key;
                assert_ne!(ct as *const c_char, dup.as_ptr(), "sh={sh:?}: temp_key must be the stored pointer");
                assert_eq!(
                    CStr::from_ptr(ct).to_bytes(),
                    CStr::from_ptr(rtk).to_bytes(),
                    "sh={sh:?}: temp_key content mismatch"
                );
            }
            assert_eq!(
                (*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).length,
                c_len,
                "length must not change on re-put"
            );
            assert_eq!(
                (*((*((ch as *mut u8).sub(16).sub(HDRSIZE) as *mut ArrayHeader)).hash_table
                    as *mut HashIndex))
                    .used_count,
                c_used,
                "used_count must not change on re-put"
            );
            assert_eq_dump(
                &format!("row39 sh={sh:?}"),
                &dump_table(ch, 16, 8),
                &dump_table(rh, 16, 8),
            );
            (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

#[test]
fn e40_hmput_zero_keysize() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 0x40);
    unsafe {
        (p.c.rand_seed)(0xcccc);
        (p.r.rand_seed)(0xcccc);
        let mut keys: Vec<[u8; 8]> = (0..20)
            .map(|_| {
                let v = rng.bytes(8);
                let mut a = [0u8; 8];
                a.copy_from_slice(&v);
                a
            })
            .collect();
        let mut ch: *mut c_void = std::ptr::null_mut();
        let mut rh: *mut c_void = std::ptr::null_mut();
        for i in 0..keys.len() {
            let kp = keys[i].as_mut_ptr() as *mut c_void;
            ch = (p.c.hmput_key)(ch, 8, kp, 0, STBDS_HM_BINARY);
            rh = (p.r.hmput_key)(rh, 8, kp, 0, STBDS_HM_BINARY);
            let ci = (*((ch as *mut u8).sub(8).sub(HDRSIZE) as *mut ArrayHeader)).temp;
            let ri = (*((rh as *mut u8).sub(8).sub(HDRSIZE) as *mut ArrayHeader)).temp;
            assert_eq!(ci, 0, "keysize == 0: memcmp always matches, so index stays 0");
            assert_eq!(ci, ri);
            for b in 0..8usize {
                *(ch as *mut u8).offset(ci * 8).add(b) = i as u8;
                *(rh as *mut u8).offset(ri * 8).add(b) = i as u8;
            }
        }
        let clen = (*((ch as *mut u8).sub(8).sub(HDRSIZE) as *mut ArrayHeader)).length;
        assert_eq!(clen, 2, "the table must hold exactly one entry");
        assert_eq_dump("row40", &dump_table(ch, 8, 0), &dump_table(rh, 8, 0));
        // NULL key with keysize 0 is also fine (nothing is dereferenced)
        ch = (p.c.hmput_key)(ch, 8, std::ptr::null_mut(), 0, STBDS_HM_BINARY);
        rh = (p.r.hmput_key)(rh, 8, std::ptr::null_mut(), 0, STBDS_HM_BINARY);
        assert_eq_dump("row40 NULL key", &dump_table(ch, 8, 0), &dump_table(rh, 8, 0));
        (p.c.hmfree_func)((ch as *mut u8).sub(8) as *mut c_void, 8);
        (p.r.hmfree_func)((rh as *mut u8).sub(8) as *mut c_void, 8);
    }
}

// ===========================================================================
// row 41 -- sh_geti with non-positive num
// ===========================================================================
#[test]
fn e41_sh_geti_nonpositive() {
    let _g = serial();
    let p = pair();
    unsafe {
        for n in [0i32, -1, -2, -1000, i32::MIN, i32::MIN + 1] {
            (p.c.rand_seed)(0x3141_5926);
            let cout = capture_stdout(&format!("e41c{n}"), || (p.c.sh_geti)(n));
            (p.r.rand_seed)(0x3141_5926);
            let rout = capture_stdout(&format!("e41r{n}"), || (p.r.sh_geti)(n));
            assert!(cout.is_empty(), "C sh_geti({n}) must print nothing");
            assert_eq!(cout, rout, "sh_geti({n}) stdout mismatch");
        }
    }
}

// ===========================================================================
// row 42 -- strkey with negative / extreme n
// ===========================================================================
#[test]
fn e42_strkey_negative() {
    let _g = serial();
    let p = pair();
    unsafe {
        for n in [-1i32, -2, -99, i32::MIN, i32::MIN + 1, i32::MAX] {
            let cp = (p.c.strkey)(n);
            let cs = CStr::from_ptr(cp).to_bytes().to_vec();
            let rp = (p.r.strkey)(n);
            let rs = CStr::from_ptr(rp).to_bytes().to_vec();
            assert_eq!(cs, rs, "strkey({n})");
            assert_eq!(
                String::from_utf8_lossy(&cs),
                format!("test_{n}"),
                "strkey({n}) formatting"
            );
        }
        assert_eq!(
            String::from_utf8_lossy(CStr::from_ptr((p.c.strkey)(i32::MIN)).to_bytes()),
            "test_-2147483648"
        );
        assert_eq!(
            String::from_utf8_lossy(CStr::from_ptr((p.r.strkey)(i32::MIN)).to_bytes()),
            "test_-2147483648"
        );
    }
}
