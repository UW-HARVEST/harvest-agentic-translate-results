//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input/condition, calls BOTH the C
//! `.so` and the Rust `.so`, and asserts they return the same sentinel /
//! error code / rejection state — not merely "both failed".

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

const ELEMSIZES: [usize; 6] = [1, 4, 8, 12, 16, 32];

fn cmp_raw(mc: *mut c_void, mr: *mut c_void, es: usize, ctx: &str) {
    unsafe {
        assert_snap_eq(
            &snap_map(mc, es, ElemFmt::Raw, false),
            &snap_map(mr, es, ElemFmt::Raw, false),
            ctx,
        );
    }
}

// ===========================================================================
// Rows 1–4 — stbds_arrgrowf
// ===========================================================================

/// ERRORS row 1 — `a == NULL`: fresh alloc with a fully initialised header.
#[test]
fn err_01_arrgrowf_null() {
    run(1, |c, r| unsafe {
        for &es in &ELEMSIZES {
            for (addlen, min_cap) in [(0usize, 1usize), (1, 0), (1, 1), (5, 0), (0, 7), (9, 3)] {
                let ac = (c.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                let ar = (r.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                assert!(!ac.is_null() && !ar.is_null());
                let hc = (ac as *mut ArrayHeader).wrapping_sub(1);
                let hr = (ar as *mut ArrayHeader).wrapping_sub(1);
                assert_eq!((*hc).length, 0, "C length must be 0");
                assert_eq!((*hr).length, (*hc).length);
                assert!((*hc).hash_table.is_null() && (*hr).hash_table.is_null());
                assert_eq!((*hc).temp, 0);
                assert_eq!((*hr).temp, (*hc).temp);
                assert_eq!((*hr).capacity, (*hc).capacity, "capacity es={}", es);
                (c.arrfreef)(ac);
                (r.arrfreef)(ar);
            }
        }
    });
}

/// ERRORS row 2 — `min_cap <= arrcap(a)`: returns the same pointer, no realloc.
#[test]
fn err_02_arrgrowf_noop() {
    run(1, |c, r| unsafe {
        for &es in &ELEMSIZES {
            let ac = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 6);
            let ar = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 6);
            let cap = (*(ac as *mut ArrayHeader).wrapping_sub(1)).capacity;
            for min_cap in 0..=cap {
                let bc = (c.arrgrowf)(ac, es, 0, min_cap);
                let br = (r.arrgrowf)(ar, es, 0, min_cap);
                assert_eq!(bc, ac, "C must return the same pointer");
                assert_eq!(br, ar, "Rust must return the same pointer");
                assert_eq!(bc == ac, br == ar);
            }
            (c.arrfreef)(ac);
            (r.arrfreef)(ar);
        }
    });
}

/// ERRORS row 3 — `addlen == 0 && min_cap == 0` on NULL: returns NULL, no alloc.
#[test]
fn err_03_arrgrowf_zero_zero() {
    run(1, |c, r| unsafe {
        for &es in &ELEMSIZES {
            let ac = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let ar = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(ac.is_null(), "C: arrgrowf(NULL,{},0,0) must return NULL", es);
            assert!(ar.is_null(), "Rust: arrgrowf(NULL,{},0,0) must return NULL", es);
        }
    });
}

/// ERRORS row 4 — `elemsize == 0`.
#[test]
fn err_04_arrgrowf_elemsize0() {
    run(1, |c, r| unsafe {
        for (addlen, min_cap) in [(0usize, 1usize), (1, 0), (3, 9), (0, 100)] {
            let ac = (c.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            let ar = (r.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            assert_eq!(ac.is_null(), ar.is_null());
            if !ac.is_null() {
                assert_eq!(
                    snap_array(ac, 0, 0),
                    snap_array(ar, 0, 0),
                    "elemsize=0 addlen={} min_cap={}",
                    addlen,
                    min_cap
                );
                (c.arrfreef)(ac);
                (r.arrfreef)(ar);
            }
        }
        // and growing an existing zero-elemsize array
        let mut ac = (c.arrgrowf)(std::ptr::null_mut(), 0, 0, 1);
        let mut ar = (r.arrgrowf)(std::ptr::null_mut(), 0, 0, 1);
        for i in 0..20 {
            ac = (c.arrgrowf)(ac, 0, 1, 0);
            ar = (r.arrgrowf)(ar, 0, 1, 0);
            assert_eq!(snap_array(ac, 0, 0), snap_array(ar, 0, 0), "step {}", i);
            (*(ac as *mut ArrayHeader).wrapping_sub(1)).length += 1;
            (*(ar as *mut ArrayHeader).wrapping_sub(1)).length += 1;
        }
        (c.arrfreef)(ac);
        (r.arrfreef)(ar);
    });
}

// ===========================================================================
// Rows 5–6 — stbds_hmfree_func
// ===========================================================================

/// ERRORS row 5 — `hmfree_func(NULL, es)` is a no-op.
#[test]
fn err_05_hmfree_null() {
    run(1, |c, r| unsafe {
        for &es in &ELEMSIZES {
            (c.hmfree_func)(std::ptr::null_mut(), es);
            (r.hmfree_func)(std::ptr::null_mut(), es);
        }
        // If either had dereferenced NULL we would not get here.
    });
}

/// ERRORS row 6 — map with `hash_table == NULL` (created by `hmput_default`).
#[test]
fn err_06_hmfree_no_table() {
    run(1, |c, r| unsafe {
        for &es in &ELEMSIZES {
            let mc = (c.hmput_default)(std::ptr::null_mut(), es);
            let mr = (r.hmput_default)(std::ptr::null_mut(), es);
            assert!((*header_of(mc, es)).hash_table.is_null());
            assert!((*header_of(mr, es)).hash_table.is_null());
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        }
    });
}

// ===========================================================================
// Rows 7–12 — lookup "not found" sentinels
// ===========================================================================

/// ERRORS rows 7 + 8 — `hm_find_slot` returns `-1` from both probe loops.
///
/// The wrap-around (`0..limit`) loop is reached whenever the initial
/// `pos & 7` is non-zero, so a large corpus of random misses covers both.
#[test]
fn err_07_08_hmget_absent() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mut rng = Rng::new(0xC0DE_0001);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        let mut present: Vec<[u8; 4]> = Vec::new();
        for i in 0..50u32 {
            let k = i.wrapping_mul(0x9E37_79B9).to_le_bytes();
            present.push(k);
            mc = hm_put(c, mc, es, &k, ks, &i.to_le_bytes(), HM_BINARY);
            mr = hm_put(r, mr, es, &k, ks, &i.to_le_bytes(), HM_BINARY);
        }
        let mut misses = 0usize;
        for _ in 0..3000 {
            let k = rng.next_u32().to_le_bytes();
            if present.contains(&k) {
                continue;
            }
            misses += 1;
            let (m1, tc) = hm_get(c, mc, es, &k, ks, HM_BINARY);
            let (m2, tr) = hm_get(r, mr, es, &k, ks, HM_BINARY);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr, "miss sentinel for {:02x?}", k);
            assert_eq!(tc, -1, "expected STBDS_INDEX_EMPTY (-1)");
        }
        assert!(misses > 2500, "expected plenty of misses, got {}", misses);
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// ERRORS row 9 — `hmget_key_ts(NULL, …)`: allocates, `*temp = -1`.
#[test]
fn err_09_hmget_ts_null() {
    run(1, |c, r| unsafe {
        for &es in &ELEMSIZES.iter().copied().filter(|&e| e >= 4).collect::<Vec<_>>() {
            let key = [1u8, 2, 3, 4];
            let mut tc: isize = 0x1234;
            let mut tr: isize = 0x1234;
            let mc = (c.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                key.as_ptr() as *mut c_void,
                4,
                &mut tc,
                HM_BINARY,
            );
            let mr = (r.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                key.as_ptr() as *mut c_void,
                4,
                &mut tr,
                HM_BINARY,
            );
            assert_eq!(tc, -1, "C: *temp must be STBDS_INDEX_EMPTY, es={}", es);
            assert_eq!(tr, tc, "Rust: *temp must match, es={}", es);
            cmp_raw(mc, mr, es, &format!("row9 es={}", es));
            assert_eq!((*header_of(mc, es)).length, 1);
            assert_eq!((*header_of(mr, es)).length, 1);
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        }
    });
}

/// ERRORS row 10 — `hash_table == NULL`: `*temp = -1`, pointer unchanged.
#[test]
fn err_10_hmget_ts_no_table() {
    run(1, |c, r| unsafe {
        let es = 8usize;
        let mc = (c.hmput_default)(std::ptr::null_mut(), es);
        let mr = (r.hmput_default)(std::ptr::null_mut(), es);
        let key = [7u8, 7, 7, 7];
        let mut tc: isize = 0x1234;
        let mut tr: isize = 0x1234;
        let nc = (c.hmget_key_ts)(mc, es, key.as_ptr() as *mut c_void, 4, &mut tc, HM_BINARY);
        let nr = (r.hmget_key_ts)(mr, es, key.as_ptr() as *mut c_void, 4, &mut tr, HM_BINARY);
        assert_eq!(tc, -1);
        assert_eq!(tr, tc);
        assert_eq!(nc, mc, "C must return `a` unchanged");
        assert_eq!(nr, mr, "Rust must return `a` unchanged");
        cmp_raw(nc, nr, es, "row10");
        hm_free(c, nc, es);
        hm_free(r, nr, es);
    });
}

/// ERRORS row 11 — key absent through `_ts`.
#[test]
fn err_11_hmget_ts_absent() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mut rng = Rng::new(0xC0DE_0002);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        let mut present: Vec<[u8; 4]> = Vec::new();
        for i in 0..25u32 {
            let k = i.wrapping_mul(2_654_435_761).to_le_bytes();
            present.push(k);
            mc = hm_put(c, mc, es, &k, ks, &i.to_le_bytes(), HM_BINARY);
            mr = hm_put(r, mr, es, &k, ks, &i.to_le_bytes(), HM_BINARY);
        }
        for _ in 0..800 {
            let k = rng.next_u32().to_le_bytes();
            if present.contains(&k) {
                continue;
            }
            let (m1, tc) = hm_get_ts(c, mc, es, &k, ks, HM_BINARY);
            let (m2, tr) = hm_get_ts(r, mr, es, &k, ks, HM_BINARY);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr);
            assert_eq!(tc, -1);
        }
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// ERRORS row 12 — `hmget_key(NULL, …)`: also writes `header->temp = -1`.
#[test]
fn err_12_hmget_null() {
    run(1, |c, r| unsafe {
        for es in [4usize, 8, 12, 16, 32] {
            let key = [9u8, 9, 9, 9];
            let mc = (c.hmget_key)(std::ptr::null_mut(), es, key.as_ptr() as *mut c_void, 4, HM_BINARY);
            let mr = (r.hmget_key)(std::ptr::null_mut(), es, key.as_ptr() as *mut c_void, 4, HM_BINARY);
            assert_eq!((*header_of(mc, es)).temp, -1, "C temp, es={}", es);
            assert_eq!((*header_of(mr, es)).temp, (*header_of(mc, es)).temp);
            cmp_raw(mc, mr, es, &format!("row12 es={}", es));
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        }
    });
}

// ===========================================================================
// Rows 13–17 — insert-side early-outs
// ===========================================================================

/// ERRORS row 13 — `hmput_default(NULL, es)`.
#[test]
fn err_13_hmput_default_null() {
    run(1, |c, r| unsafe {
        for &es in &ELEMSIZES {
            let mc = (c.hmput_default)(std::ptr::null_mut(), es);
            let mr = (r.hmput_default)(std::ptr::null_mut(), es);
            assert!(!mc.is_null() && !mr.is_null());
            assert_eq!((*header_of(mc, es)).length, 1);
            cmp_raw(mc, mr, es, &format!("row13 es={}", es));
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        }
    });
}

/// ERRORS row 14 — `a != NULL` but `length == 0` (built with `arrgrowf` directly).
#[test]
fn err_14_hmput_default_len0() {
    run(1, |c, r| unsafe {
        for es in [4usize, 8, 16] {
            // arrgrowf gives length == 0; STBDS_ARR_TO_HASH shifts by one element
            let ac = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let ar = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            assert_eq!((*(ac as *mut ArrayHeader).wrapping_sub(1)).length, 0);
            let hc = (ac as *mut u8).add(es) as *mut c_void;
            let hr = (ar as *mut u8).add(es) as *mut c_void;
            let mc = (c.hmput_default)(hc, es);
            let mr = (r.hmput_default)(hr, es);
            assert_eq!((*header_of(mc, es)).length, 1, "C must bump length to 1");
            assert_eq!((*header_of(mr, es)).length, 1);
            cmp_raw(mc, mr, es, &format!("row14 es={}", es));
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        }
    });
}

/// ERRORS row 15 — `hmput_default` on a non-empty map is a pure no-op.
#[test]
fn err_15_hmput_default_noop() {
    run(1, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mc = hm_put(c, std::ptr::null_mut(), es, &[1, 2, 3, 4], ks, &[5, 6, 7, 8], HM_BINARY);
        let mr = hm_put(r, std::ptr::null_mut(), es, &[1, 2, 3, 4], ks, &[5, 6, 7, 8], HM_BINARY);
        let before_c = snap_map(mc, es, ElemFmt::Raw, false);
        let nc = (c.hmput_default)(mc, es);
        let nr = (r.hmput_default)(mr, es);
        assert_eq!(nc, mc);
        assert_eq!(nr, mr);
        assert_eq!(before_c, snap_map(nc, es, ElemFmt::Raw, false));
        cmp_raw(nc, nr, es, "row15");
        hm_free(c, nc, es);
        hm_free(r, nr, es);
    });
}

/// ERRORS row 16 — `hmput_key(NULL, …)` bootstraps.
#[test]
fn err_16_hmput_null() {
    run(0x3141_5926, |c, r| unsafe {
        for &es in &ELEMSIZES.iter().copied().filter(|&e| e >= 4).collect::<Vec<_>>() {
            let mc = hm_put(c, std::ptr::null_mut(), es, &[3, 1, 4, 1], 4, &[0xAA; 32], HM_BINARY);
            let mr = hm_put(r, std::ptr::null_mut(), es, &[3, 1, 4, 1], 4, &[0xAA; 32], HM_BINARY);
            assert_eq!((*header_of(mc, es)).length, 2, "default slot + 1 element");
            cmp_raw(mc, mr, es, &format!("row16 es={}", es));
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        }
    });
}

/// ERRORS row 17 — `hmput_key` on a table-less map builds an 8-slot index and
/// sets `string.mode` from `mode`.
#[test]
fn err_17_hmput_no_table() {
    for mode in [HM_BINARY, HM_STRING] {
        run(0x3141_5926, |c, r| unsafe {
            let es = 16usize;
            let mc0 = (c.hmput_default)(std::ptr::null_mut(), es);
            let mr0 = (r.hmput_default)(std::ptr::null_mut(), es);
            let key: Vec<u8> = b"abcdefgh\0".to_vec();
            let (mc, mr) = if mode == HM_STRING {
                (
                    sh_put_v(c, mc0, es, key.as_ptr() as *mut c_char, &[1; 8], mode),
                    sh_put_v(r, mr0, es, key.as_ptr() as *mut c_char, &[1; 8], mode),
                )
            } else {
                (
                    hm_put(c, mc0, es, &key[..8], 8, &[1; 8], mode),
                    hm_put(r, mr0, es, &key[..8], 8, &[1; 8], mode),
                )
            };
            let tc = (*header_of(mc, es)).hash_table as *mut HashIndex;
            let tr = (*header_of(mr, es)).hash_table as *mut HashIndex;
            assert_eq!((*tc).slot_count, 8, "fresh index must have 8 slots");
            assert_eq!((*tr).slot_count, (*tc).slot_count);
            let expect = if mode >= HM_STRING { SH_DEFAULT as u8 } else { 0 };
            assert_eq!((*tc).string.mode, expect, "C string.mode for mode={}", mode);
            assert_eq!((*tr).string.mode, (*tc).string.mode);
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        });
    }
}

// ===========================================================================
// Rows 18–21 — delete-side rejections
// ===========================================================================

/// ERRORS row 18 — `hmdel_key(NULL, …)` returns NULL (the only NULL return).
#[test]
fn err_18_hmdel_null() {
    run(1, |c, r| unsafe {
        for &es in &ELEMSIZES {
            for mode in [HM_BINARY, HM_STRING, 2, -1] {
                let key = [1u8, 2, 3, 4, 0, 0, 0, 0];
                let nc = (c.hmdel_key)(
                    std::ptr::null_mut(),
                    es,
                    key.as_ptr() as *mut c_void,
                    4,
                    0,
                    mode,
                );
                let nr = (r.hmdel_key)(
                    std::ptr::null_mut(),
                    es,
                    key.as_ptr() as *mut c_void,
                    4,
                    0,
                    mode,
                );
                assert!(nc.is_null(), "C: hmdel_key(NULL) must return NULL");
                assert!(nr.is_null(), "Rust: hmdel_key(NULL) must return NULL");
            }
        }
    });
}

/// ERRORS row 19 — `hash_table == NULL`: `temp = 0`, map returned unchanged.
#[test]
fn err_19_hmdel_no_table() {
    run(1, |c, r| unsafe {
        let es = 8usize;
        let mc = (c.hmput_default)(std::ptr::null_mut(), es);
        let mr = (r.hmput_default)(std::ptr::null_mut(), es);
        // pre-poison temp so we can see it being reset to 0
        (*header_of(mc, es)).temp = 77;
        (*header_of(mr, es)).temp = 77;
        let (nc, tc) = hm_del(c, mc, es, &[1, 2, 3, 4], 4, 0, HM_BINARY);
        let (nr, tr) = hm_del(r, mr, es, &[1, 2, 3, 4], 4, 0, HM_BINARY);
        assert_eq!(nc, mc, "C must return `a`");
        assert_eq!(nr, mr, "Rust must return `a`");
        assert_eq!(tc, 0, "C temp must be reset to 0");
        assert_eq!(tr, tc);
        cmp_raw(nc, nr, es, "row19");
        hm_free(c, nc, es);
        hm_free(r, nr, es);
    });
}

/// ERRORS row 20 — key absent: `temp = 0`, length unchanged.
#[test]
fn err_20_hmdel_absent() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mut rng = Rng::new(0xC0DE_0003);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        let mut present: Vec<[u8; 4]> = Vec::new();
        for i in 0..30u32 {
            let k = i.wrapping_mul(0x85EB_CA6B).to_le_bytes();
            present.push(k);
            mc = hm_put(c, mc, es, &k, ks, &i.to_le_bytes(), HM_BINARY);
            mr = hm_put(r, mr, es, &k, ks, &i.to_le_bytes(), HM_BINARY);
        }
        let len_c = (*header_of(mc, es)).length;
        for _ in 0..600 {
            let k = rng.next_u32().to_le_bytes();
            if present.contains(&k) {
                continue;
            }
            let (m1, tc) = hm_del(c, mc, es, &k, ks, 0, HM_BINARY);
            let (m2, tr) = hm_del(r, mr, es, &k, ks, 0, HM_BINARY);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr, "absent-delete temp for {:02x?}", k);
            assert_eq!(tc, 0, "expected reject (temp == 0)");
            assert_eq!((*header_of(mc, es)).length, len_c, "length must not change");
            assert_eq!((*header_of(mr, es)).length, len_c);
        }
        cmp_raw(mc, mr, es, "row20 final");
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// ERRORS row 21 — deleting the same key twice: second call rejects.
#[test]
fn err_21_hmdel_twice() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        let keys: Vec<[u8; 4]> = (0..20u32).map(|i| i.wrapping_mul(0x27D4_EB2F).to_le_bytes()).collect();
        for (i, k) in keys.iter().enumerate() {
            mc = hm_put(c, mc, es, k, ks, &(i as u32).to_le_bytes(), HM_BINARY);
            mr = hm_put(r, mr, es, k, ks, &(i as u32).to_le_bytes(), HM_BINARY);
        }
        for k in &keys {
            let (m1, t1c) = hm_del(c, mc, es, k, ks, 0, HM_BINARY);
            let (m2, t1r) = hm_del(r, mr, es, k, ks, 0, HM_BINARY);
            mc = m1;
            mr = m2;
            assert_eq!(t1c, t1r);
            assert_eq!(t1c, 1, "first delete must succeed");
            let len_after = (*header_of(mc, es)).length;
            let (m3, t2c) = hm_del(c, mc, es, k, ks, 0, HM_BINARY);
            let (m4, t2r) = hm_del(r, mr, es, k, ks, 0, HM_BINARY);
            mc = m3;
            mr = m4;
            assert_eq!(t2c, t2r);
            assert_eq!(t2c, 0, "second delete must reject");
            assert_eq!((*header_of(mc, es)).length, len_after);
            cmp_raw(mc, mr, es, "row21");
        }
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// ERRORS row 22 — the reserved hash values 0 and 1 (`HASH_EMPTY` / `HASH_DELETED`)
/// are never stored: `if (hash < 2) hash += 2`.
///
/// A key whose 64-bit hash is 0 or 1 cannot be found by search (probability
/// ~2⁻⁶³), so the check is verified through its observable invariant: across a
/// large corpus no *occupied* slot ever carries hash 0 or 1, in either library,
/// and every occupied slot's hash agrees between the two.
#[test]
fn err_22_reserved_hash() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mut rng = Rng::new(0xC0DE_0004);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        for i in 0..400u32 {
            let k = rng.next_u32().to_le_bytes();
            mc = hm_put(c, mc, es, &k, ks, &i.to_le_bytes(), HM_BINARY);
            mr = hm_put(r, mr, es, &k, ks, &i.to_le_bytes(), HM_BINARY);
        }
        for (name, m) in [("C", mc), ("Rust", mr)] {
            let t = (*header_of(m, es)).hash_table as *mut HashIndex;
            for b in 0..((*t).slot_count / BUCKET_LENGTH) {
                let bk = (*t).storage.wrapping_add(b);
                for s in 0..BUCKET_LENGTH {
                    if (*bk).index[s] >= 0 {
                        assert!(
                            (*bk).hash[s] >= 2,
                            "{}: occupied slot carries reserved hash {}",
                            name,
                            (*bk).hash[s]
                        );
                    }
                }
            }
        }
        cmp_raw(mc, mr, es, "row22 (full bucket hash/index parity)");
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

// ===========================================================================
// Rows 23–27 — out-of-domain `mode` / `string.mode` ints across the FFI
// ===========================================================================

/// ERRORS row 23 — `mode > STBDS_HM_STRING` is treated as string mode.
#[test]
fn err_23_mode_out_of_range_hi() {
    for mode in [2i32, 3, 7, 255, 65536, c_int::MAX] {
        run(0x3141_5926, |c, r| unsafe {
            let es = 16usize;
            let mut rng = Rng::new(0xC0DE_0005 + mode as u64);
            let keys: Vec<Vec<u8>> = (0..25)
                .map(|i| {
                    let n = 1 + rng.below(16);
                    let mut v = rng.ascii(n);
                    v.pop();
                    v.extend_from_slice(format!("#{}", i).as_bytes());
                    v.push(0);
                    v
                })
                .collect();
            let mut mc: *mut c_void = std::ptr::null_mut();
            let mut mr: *mut c_void = std::ptr::null_mut();
            for (i, k) in keys.iter().enumerate() {
                mc = sh_put_v(c, mc, es, k.as_ptr() as *mut c_char, &(i as u64).to_le_bytes(), mode);
                mr = sh_put_v(r, mr, es, k.as_ptr() as *mut c_char, &(i as u64).to_le_bytes(), mode);
                // string path => table->string.mode becomes SH_DEFAULT
                let tc = (*header_of(mc, es)).hash_table as *mut HashIndex;
                let tr = (*header_of(mr, es)).hash_table as *mut HashIndex;
                assert_eq!((*tc).string.mode, SH_DEFAULT as u8, "mode={}", mode);
                assert_eq!((*tr).string.mode, (*tc).string.mode);
                assert_snap_eq(
                    &snap_map(mc, es, ElemFmt::KeyPtr, true),
                    &snap_map(mr, es, ElemFmt::KeyPtr, true),
                    &format!("row23 mode={} insert #{}", mode, i),
                );
            }
            for k in &keys {
                let (m1, tc) = sh_get(c, mc, es, k.as_ptr() as *mut c_char, mode);
                let (m2, tr) = sh_get(r, mr, es, k.as_ptr() as *mut c_char, mode);
                mc = m1;
                mr = m2;
                assert_eq!(tc, tr, "row23 mode={} get", mode);
                assert!(tc >= 0, "string-mode lookup must find the key");
            }
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        });
    }
}

/// ERRORS row 24 — negative `mode` is treated as binary mode.
#[test]
fn err_24_mode_negative() {
    for mode in [-1i32, -2, -128, c_int::MIN, c_int::MIN + 1] {
        run(0x3141_5926, |c, r| unsafe {
            let (es, ks) = (8usize, 4usize);
            let mut rng = Rng::new(0xC0DE_0006 + mode.unsigned_abs() as u64);
            let mut mc: *mut c_void = std::ptr::null_mut();
            let mut mr: *mut c_void = std::ptr::null_mut();
            let mut keys: Vec<[u8; 4]> = Vec::new();
            for i in 0..25u32 {
                let k = rng.next_u32().to_le_bytes();
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k);
                mc = hm_put(c, mc, es, &k, ks, &i.to_le_bytes(), mode);
                mr = hm_put(r, mr, es, &k, ks, &i.to_le_bytes(), mode);
                cmp_raw(mc, mr, es, &format!("row24 mode={} insert #{}", mode, i));
            }
            // binary path => string.mode stays 0
            let tc = (*header_of(mc, es)).hash_table as *mut HashIndex;
            let tr = (*header_of(mr, es)).hash_table as *mut HashIndex;
            assert_eq!((*tc).string.mode, 0, "mode={} must take the binary path", mode);
            assert_eq!((*tr).string.mode, 0);
            for k in &keys {
                let (m1, gc) = hm_get(c, mc, es, k, ks, mode);
                let (m2, gr) = hm_get(r, mr, es, k, ks, mode);
                mc = m1;
                mr = m2;
                assert_eq!(gc, gr);
                assert!(gc >= 0);
            }
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        });
    }
}

/// ERRORS row 25 — `hmdel_key` with `mode == 2` on a `SH_STRDUP` map:
/// `mode == STBDS_HM_STRING` is false, so the duplicated key is NOT freed,
/// while `mode >= STBDS_HM_STRING` still selects the string comparison.
///
/// Restricted to deleting the LAST element: with `old_index != final_index`
/// the C takes the `else` branch of the re-find and passes the *address* of the
/// key field to a string hash, which makes `assert(slot >= 0)` at lib.c:846
/// fire. That abort case is covered separately by `err_25b`.
#[test]
fn err_25_hmdel_mode2_last_element() {
    run(0x3141_5926, |c, r| unsafe {
        let es = 16usize;
        let mut rng = Rng::new(0xC0DE_0007);
        let keys: Vec<Vec<u8>> = (0..12)
            .map(|i| {
                let n = 3 + rng.below(12);
                let mut v = rng.ascii(n);
                v.pop();
                v.extend_from_slice(format!("#{}", i).as_bytes());
                v.push(0);
                v
            })
            .collect();
        let mut mc = (c.shmode_func)(es, SH_STRDUP);
        let mut mr = (r.shmode_func)(es, SH_STRDUP);
        for (i, k) in keys.iter().enumerate() {
            mc = sh_put(c, mc, es, k.as_ptr() as *mut c_char, &(i as u64).to_le_bytes(), HM_STRING);
            mr = sh_put(r, mr, es, k.as_ptr() as *mut c_char, &(i as u64).to_le_bytes(), HM_STRING);
        }
        // pop from the back with mode = 2
        for i in (0..12).rev() {
            let (m1, tc) = sh_del(c, mc, es, keys[i].as_ptr() as *mut c_char, 0, 2);
            let (m2, tr) = sh_del(r, mr, es, keys[i].as_ptr() as *mut c_char, 0, 2);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr, "row25 del#{} temp", i);
            assert_eq!(tc, 1, "row25 delete must succeed with mode=2");
            assert_snap_eq(
                &snap_map(mc, es, ElemFmt::KeyPtr, false),
                &snap_map(mr, es, ElemFmt::KeyPtr, false),
                &format!("row25 after del #{}", i),
            );
        }
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// ERRORS row 25 (abort variant) — `mode == 2` deleting a NON-final element on a
/// string map trips the live `assert(slot >= 0)` at lib.c:846. Both libraries
/// must terminate the same way (SIGABRT), so each is run in a forked child.
#[test]
fn err_25b_hmdel_mode2_middle_aborts() {
    run_locked(0x3141_5926, |c, r| unsafe {
        let es = 16usize;
        let keys: Vec<Vec<u8>> = (0..8)
            .map(|i| format!("key_{}\0", i).into_bytes())
            .collect();

        let build_and_del = |imp: &'static Impl| {
            let keys = &keys;
            move || {
                let mut m = (imp.shmode_func)(es, SH_STRDUP);
                for (i, k) in keys.iter().enumerate() {
                    m = sh_put(
                        imp,
                        m,
                        es,
                        k.as_ptr() as *mut c_char,
                        &(i as u64).to_le_bytes(),
                        HM_STRING,
                    );
                }
                // delete the FIRST element => old_index != final_index
                let _ = sh_del(imp, m, es, keys[0].as_ptr() as *mut c_char, 0, 2);
            }
        };

        let oc = fork_outcome(build_and_del(c));
        let or_ = fork_outcome(build_and_del(r));
        assert_eq!(
            oc, or_,
            "C and Rust must terminate identically on the assert path"
        );
        assert_eq!(oc, Outcome::Signalled(6), "expected SIGABRT from assert()");
    });
}

/// ERRORS row 26 — `shmode_func` truncates `mode` with `(unsigned char)`.
#[test]
fn err_26_shmode_out_of_range() {
    for mode in [
        4i32, 7, 255, 256, 257, 258, 259, 260, 511, 512, 65539, -1, -2, -253, c_int::MIN, c_int::MAX,
    ] {
        run(0x3141_5926, |c, r| unsafe {
            let es = 16usize;
            let mc = (c.shmode_func)(es, mode);
            let mr = (r.shmode_func)(es, mode);
            let tc = (*header_of(mc, es)).hash_table as *mut HashIndex;
            let tr = (*header_of(mr, es)).hash_table as *mut HashIndex;
            let expect = (mode as u32 & 0xFF) as u8;
            assert_eq!(
                (*tc).string.mode, expect,
                "C: shmode_func(_, {}) must store (unsigned char) mode",
                mode
            );
            assert_eq!((*tr).string.mode, (*tc).string.mode, "mode={}", mode);
            assert_snap_eq(
                &snap_map(mc, es, ElemFmt::Raw, false),
                &snap_map(mr, es, ElemFmt::Raw, false),
                &format!("row26 mode={}", mode),
            );
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        });
    }
}

/// ERRORS row 27 — `string.mode` outside `{1,2,3}` takes the `switch` `default:`
/// arm and `memcpy`s `keysize` bytes of the *string* into the element.
#[test]
fn err_27_string_mode_default_branch() {
    // 0 (SH_NONE), 4, 7, 255 (from mode = -1) all hit `default:`
    for shmode in [0i32, 4, 7, 255, 256 /* -> 0 */, 260 /* -> 4 */] {
        run(0x3141_5926, |c, r| unsafe {
            let es = 16usize;
            let ks = 8usize;
            let keys: Vec<Vec<u8>> = (0..6)
                .map(|i| format!("abcdefghij_{}\0", i).into_bytes())
                .collect();
            let mc0 = (c.shmode_func)(es, shmode);
            let mr0 = (r.shmode_func)(es, shmode);
            let mut mc = mc0;
            let mut mr = mr0;
            for (i, k) in keys.iter().enumerate() {
                mc = sh_put_v(c, mc, es, k.as_ptr() as *mut c_char, &(i as u64).to_le_bytes(), HM_STRING);
                mr = sh_put_v(r, mr, es, k.as_ptr() as *mut c_char, &(i as u64).to_le_bytes(), HM_STRING);
                let t = (*header_of(mc, es)).temp;
                let ec = std::slice::from_raw_parts((mc as *const u8).offset(t * es as isize), ks);
                assert_eq!(
                    ec,
                    &k[..ks],
                    "shmode={}: default: arm must memcpy the key bytes",
                    shmode
                );
                assert_snap_eq(
                    &snap_map(mc, es, ElemFmt::Raw, false),
                    &snap_map(mr, es, ElemFmt::Raw, false),
                    &format!("row27 shmode={} insert #{}", shmode, i),
                );
            }
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        });
    }
}

// ===========================================================================
// Rows 28–33 — string arena edge cases
// ===========================================================================

unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    let mut out = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        out.push(*q);
        q = q.add(1);
    }
    out
}

/// ERRORS row 28 — oversized string on an EMPTY arena: `next = NULL`,
/// `storage = sb`, `remaining = 0`.
#[test]
fn err_28_stralloc_big_first() {
    run(1, |c, r| unsafe {
        for len in [512usize, 513, 1000, 5000] {
            let mut rng = Rng::new(0xC0DE_0008 + len as u64);
            let s = rng.ascii(len);
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            let pc = (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(pc), cstr_bytes(pr), "len={}", len);
            assert_eq!(ac.remaining, 0, "C: remaining must be 0, len={}", len);
            assert_eq!(snap_arena(&ac), snap_arena(&ar), "len={}", len);
            assert_eq!(ac.block, 1, "block must have been incremented once");
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
    });
}

/// ERRORS row 29 — oversized string on a NON-EMPTY arena: spliced in after the
/// head block, and `a->remaining` deliberately left untouched.
#[test]
fn err_29_stralloc_big_after() {
    run(1, |c, r| unsafe {
        let mut rng = Rng::new(0xC0DE_0009);
        let small = rng.ascii(10);
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        (c.stralloc)(&mut ac, small.as_ptr() as *mut c_char);
        (r.stralloc)(&mut ar, small.as_ptr() as *mut c_char);
        let rem_before = ac.remaining;
        assert_eq!(ar.remaining, rem_before);
        assert!(rem_before > 0);

        for len in [1024usize, 2048, 9000] {
            let big = rng.ascii(len);
            let pc = (c.stralloc)(&mut ac, big.as_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, big.as_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(pc), cstr_bytes(pr), "len={}", len);
            assert_eq!(
                ac.remaining, rem_before,
                "C: the oversized-after-head branch must not touch `remaining`"
            );
            assert_eq!(ar.remaining, ac.remaining);
            assert_eq!(snap_arena(&ac), snap_arena(&ar), "len={}", len);
        }
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
        assert_eq!(snap_arena(&ac), snap_arena(&ar));
    });
}

/// ERRORS row 30 — `a->block` saturates once `512 << (block>>1) >= 1 MiB`.
#[test]
fn err_30_stralloc_block_saturate() {
    run(1, |c, r| unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        let mut rng = Rng::new(0xC0DE_000A);
        let mut saturated_at: Option<u8> = None;
        for _ in 0..80usize {
            let blocksize = 512usize << (ac.block as usize >> 1);
            let n = (blocksize / 2).max(1);
            let s = rng.ascii(n);
            (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            (r.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
            assert_eq!(snap_arena(&ac), snap_arena(&ar));
            if 512usize << (ac.block as usize >> 1) >= (1 << 20) && saturated_at.is_none() {
                saturated_at = Some(ac.block);
            }
        }
        assert!(saturated_at.is_some(), "block never reached saturation");
        assert_eq!(ac.block, ar.block, "saturated block value must match");
        // further allocations must not move `block` any more
        let frozen = ac.block;
        for _ in 0..10 {
            let s = rng.ascii(600_000);
            (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            (r.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
            assert_eq!(ac.block, frozen, "C: block must stay saturated");
            assert_eq!(ar.block, ac.block);
            assert_eq!(snap_arena(&ac), snap_arena(&ar));
        }
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    });
}

/// ERRORS row 31 — the empty string still consumes one arena byte.
#[test]
fn err_31_stralloc_empty() {
    run(1, |c, r| unsafe {
        let empty: Vec<u8> = vec![0];
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        for i in 0..600usize {
            let pc = (c.stralloc)(&mut ac, empty.as_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, empty.as_ptr() as *mut c_char);
            assert_eq!(*pc, 0, "C must return a pointer to '\\0'");
            assert_eq!(*pr, 0);
            assert_eq!(snap_arena(&ac), snap_arena(&ar), "empty alloc #{}", i);
        }
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
        assert_eq!(snap_arena(&ac), snap_arena(&ar));
    });
}

/// ERRORS row 32 — `strreset` on a fresh / never-used arena.
#[test]
fn err_32_strreset_fresh() {
    run(1, |c, r| unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
        assert_eq!(snap_arena(&ac), snap_arena(&ar));
        assert_eq!(snap_arena(&ac), snap_arena(&StringArena::new()));
    });
}

/// ERRORS row 33 — `strreset` twice in a row is a no-op the second time.
#[test]
fn err_33_strreset_twice() {
    run(1, |c, r| unsafe {
        let mut rng = Rng::new(0xC0DE_000B);
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        for _ in 0..50 {
            let n = 1 + rng.below(2000);
            let s = rng.ascii(n);
            (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            (r.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
        }
        for pass in 0..4 {
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
            assert_eq!(snap_arena(&ac), snap_arena(&ar), "reset pass {}", pass);
            assert_eq!(snap_arena(&ac), snap_arena(&StringArena::new()));
        }
    });
}

// ===========================================================================
// Rows 34–38 — hashing boundary inputs
// ===========================================================================

/// ERRORS row 34 — `hash_string("")`.
#[test]
fn err_34_hash_string_empty() {
    run(1, |c, r| unsafe {
        let mut e = [0i8; 1];
        for s in [0usize, 1, 2, 0x3141_5926, usize::MAX, usize::MAX - 1] {
            let hc = (c.hash_string)(e.as_mut_ptr(), s);
            let hr = (r.hash_string)(e.as_mut_ptr(), s);
            assert_eq!(hc, hr, "hash_string(\"\", {:#x})", s);
        }
    });
}

/// ERRORS row 35 — `hash_bytes(p, 0, seed)`.
#[test]
fn err_35_hash_bytes_len0() {
    run(1, |c, r| unsafe {
        let mut d = [0u8; 16];
        for s in [0usize, 1, 0x3141_5926, usize::MAX] {
            let hc = (c.hash_bytes)(d.as_mut_ptr() as *mut c_void, 0, s);
            let hr = (r.hash_bytes)(d.as_mut_ptr() as *mut c_void, 0, s);
            assert_eq!(hc, hr, "hash_bytes(len=0, {:#x})", s);
        }
        // a zero length must ignore the buffer entirely: same result for any bytes
        let mut d2 = [0xFFu8; 16];
        let h1 = (c.hash_bytes)(d.as_mut_ptr() as *mut c_void, 0, 7);
        let h2 = (c.hash_bytes)(d2.as_mut_ptr() as *mut c_void, 0, 7);
        assert_eq!(h1, h2);
        assert_eq!(h1, (r.hash_bytes)(d2.as_mut_ptr() as *mut c_void, 0, 7));
    });
}

/// ERRORS row 36 — short tails 1..7 (every `switch` fall-through arm).
#[test]
fn err_36_hash_bytes_tail() {
    run(1, |c, r| unsafe {
        let mut rng = Rng::new(0xC0DE_000C);
        for len in 1..8usize {
            for _ in 0..200 {
                let mut d = rng.bytes(8);
                for s in [0usize, 1, 0x3141_5926, usize::MAX] {
                    let hc = (c.hash_bytes)(d.as_mut_ptr() as *mut c_void, len, s);
                    let hr = (r.hash_bytes)(d.as_mut_ptr() as *mut c_void, len, s);
                    assert_eq!(hc, hr, "len={} bytes={:02x?} seed={:#x}", len, &d[..len], s);
                }
            }
        }
    });
}

/// ERRORS row 37 — `d[3]` / `d[7]` >= 0x80: the C loader builds an `int` that
/// goes negative and sign-extends into `size_t`.
#[test]
fn err_37_hash_bytes_signext() {
    run(1, |c, r| unsafe {
        let mut rng = Rng::new(0xC0DE_000D);
        for len in 4..40usize {
            for _ in 0..40 {
                let mut d = rng.bytes(len.max(8));
                for pos in [3usize, 7, 11, 15, 19, 23, 27, 31, 35] {
                    if pos < d.len() {
                        d[pos] = 0x80 | (rng.next_u64() & 0x7F) as u8;
                    }
                }
                for s in [0usize, 0x3141_5926, usize::MAX] {
                    let hc = (c.hash_bytes)(d.as_mut_ptr() as *mut c_void, len, s);
                    let hr = (r.hash_bytes)(d.as_mut_ptr() as *mut c_void, len, s);
                    assert_eq!(hc, hr, "signext len={} bytes={:02x?}", len, &d[..len]);
                }
            }
        }
    });
}

/// ERRORS row 38 — boundary seeds.
#[test]
fn err_38_hash_seed_bounds() {
    run(1, |c, r| unsafe {
        let mut rng = Rng::new(0xC0DE_000E);
        for s in [0usize, 1, 2, usize::MAX, usize::MAX - 1, 1 << 63, (1 << 63) + 1] {
            for len in 0..40usize {
                let mut d = rng.bytes(len.max(1));
                assert_eq!(
                    (c.hash_bytes)(d.as_mut_ptr() as *mut c_void, len, s),
                    (r.hash_bytes)(d.as_mut_ptr() as *mut c_void, len, s),
                    "hash_bytes seed={:#x} len={}",
                    s,
                    len
                );
                let mut v = rng.ascii(len);
                assert_eq!(
                    (c.hash_string)(v.as_mut_ptr() as *mut c_char, s),
                    (r.hash_string)(v.as_mut_ptr() as *mut c_char, s),
                    "hash_string seed={:#x} len={}",
                    s,
                    len
                );
            }
        }
    });
}

// ===========================================================================
// Rows 39–41 — driver boundary inputs
// ===========================================================================

/// ERRORS row 39 — `strkey` with negative `n`.
#[test]
fn err_39_strkey_negative() {
    run(1, |c, r| unsafe {
        for n in [-1i32, -2, -9, -10, -11, -99, -100, -12345, -1_000_000_000] {
            let sc = cstr_bytes((c.strkey)(n));
            let sr = cstr_bytes((r.strkey)(n));
            assert_eq!(sc, sr, "strkey({})", n);
            assert_eq!(sc, format!("test_{}", n).into_bytes());
        }
    });
}

/// ERRORS row 40 — `strkey(INT_MIN)`: the magnitude is not representable in `int`.
#[test]
fn err_40_strkey_int_min() {
    run(1, |c, r| unsafe {
        for n in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
            let sc = cstr_bytes((c.strkey)(n));
            let sr = cstr_bytes((r.strkey)(n));
            assert_eq!(
                String::from_utf8_lossy(&sc),
                String::from_utf8_lossy(&sr),
                "strkey({})",
                n
            );
            assert_eq!(sc, format!("test_{}", n).into_bytes());
        }
    });
}

/// ERRORS row 41 — `sh_puts` with non-positive `num`.
#[test]
fn err_41_sh_puts_nonpositive() {
    run_locked(0x3141_5926, |c, r| unsafe {
        for num in [0i32, -1, -2, -7, -100, -32768, -1_000_000, i32::MIN, i32::MIN + 1] {
            let oc = capture_stdout("c", || (c.sh_puts)(num));
            let or_ = capture_stdout("r", || (r.sh_puts)(num));
            assert_eq!(
                String::from_utf8_lossy(&oc),
                String::from_utf8_lossy(&or_),
                "sh_puts({})",
                num
            );
            assert_eq!(
                String::from_utf8_lossy(&oc),
                format!("a {}\n", num),
                "sh_puts({}) must still print the single map entry",
                num
            );
        }
    });
}

// ===========================================================================
// Rows 42–46 — live asserts
// ===========================================================================

/// ERRORS rows 42–46 — the remaining `assert()`s must NOT fire on well-formed
/// input. Both libraries are exercised hard through every entry point inside a
/// forked child; if any assert were reachable the child would die with SIGABRT.
#[test]
fn err_42_46_asserts_do_not_fire_on_valid_input() {
    run_locked(0x3141_5926, |c, r| unsafe {
        let exercise = |imp: &'static Impl| {
            move || {
                let (es, ks) = (16usize, 8usize);
                // arrays
                let mut a: *mut c_void = std::ptr::null_mut();
                for _ in 0..500 {
                    a = (imp.arrgrowf)(a, es, 1, 0);
                    (*(a as *mut ArrayHeader).wrapping_sub(1)).length += 1;
                }
                (imp.arrfreef)(a);

                // binary map: grow, delete, shrink, rebuild
                let mut m: *mut c_void = std::ptr::null_mut();
                let keys: Vec<[u8; 8]> = (0..400u64)
                    .map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes())
                    .collect();
                for (i, k) in keys.iter().enumerate() {
                    m = hm_put(imp, m, es, k, ks, &(i as u64).to_le_bytes(), HM_BINARY);
                }
                for k in keys.iter() {
                    let (mm, _) = hm_del(imp, m, es, k, ks, 0, HM_BINARY);
                    m = mm;
                }
                hm_free(imp, m, es);

                // string maps in every mode
                for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
                    let strs: Vec<Vec<u8>> = (0..300)
                        .map(|i| format!("sk_{}_{}\0", mode, i).into_bytes())
                        .collect();
                    let mut sm = (imp.shmode_func)(es, mode);
                    for (i, s) in strs.iter().enumerate() {
                        sm = sh_put(
                            imp,
                            sm,
                            es,
                            s.as_ptr() as *mut c_char,
                            &(i as u64).to_le_bytes(),
                            HM_STRING,
                        );
                    }
                    for s in strs.iter() {
                        let (mm, _) = sh_del(imp, sm, es, s.as_ptr() as *mut c_char, 0, HM_STRING);
                        sm = mm;
                    }
                    hm_free(imp, sm, es);
                }

                // arena
                let mut ar = StringArena::new();
                for i in 0..400usize {
                    let n = if i % 13 == 0 { 3000 } else { 1 + i % 50 };
                    let s: Vec<u8> = std::iter::repeat(b'q').take(n).chain([0]).collect();
                    (imp.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
                }
                (imp.strreset)(&mut ar);

                // sh_puts (its three asserts at lib.c:959-961)
                for n in [0i32, 1, 5, 100, 3000, -5] {
                    (imp.sh_puts)(n);
                }
            }
        };

        // silence sh_puts output from the children
        let oc = capture_stdout("assertc", || {
            let o = fork_outcome(exercise(c));
            assert_eq!(o, Outcome::Exited(0), "C died unexpectedly: {:?}", o);
        });
        let or_ = capture_stdout("assertr", || {
            let o = fork_outcome(exercise(r));
            assert_eq!(o, Outcome::Exited(0), "Rust died unexpectedly: {:?}", o);
        });
        assert_eq!(
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or_),
            "sh_puts output from the exerciser must match too"
        );
    });
}
