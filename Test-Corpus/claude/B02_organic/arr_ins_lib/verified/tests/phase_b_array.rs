//! Phase B — CONFIGS.md rows 1-8 (`stbds_arrgrowf` / `stbds_arrfreef`) and
//! rows 20-23 (`stbds_hmput_default`).
mod common;

use common::*;
use std::ffi::c_void;

const ELEMSIZES: [usize; 9] = [1, 2, 3, 4, 8, 12, 16, 40, 128];

unsafe fn hdr(a: *mut c_void) -> Header {
    *(((a as *mut u8).sub(HEADER_SIZE)) as *mut Header)
}

unsafe fn set_len(a: *mut c_void, n: usize) {
    (*(((a as *mut u8).sub(HEADER_SIZE)) as *mut Header)).length = n;
}

/// Fill `len` elements with a deterministic pattern so the payload is never
/// uninitialised when it is compared.
unsafe fn fill(a: *mut c_void, elemsize: usize, len: usize, tag: u8) {
    for i in 0..(elemsize * len) {
        *(a as *mut u8).add(i) = tag.wrapping_add(i as u8).wrapping_mul(31);
    }
}

// --- row 1 ---------------------------------------------------------------
#[test]
fn cfg_01_arrgrowf_null_nogrow_returns_null() {
    let s = session();
    for &es in ELEMSIZES.iter() {
        unsafe {
            let c = (s.c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let r = (s.rust.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(c.is_null(), "C arrgrowf(NULL,{},0,0) should stay NULL", es);
            assert!(r.is_null(), "RUST arrgrowf(NULL,{},0,0) should stay NULL", es);
        }
    }
}

// --- row 2 ---------------------------------------------------------------
#[test]
fn cfg_02_arrgrowf_min_cap_bump_to_four() {
    let s = session();
    for &es in ELEMSIZES.iter() {
        for min_cap in [1usize, 2, 3, 4, 5, 6, 7, 8, 9, 100, 1000] {
            unsafe {
                let c = (s.c.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap);
                let r = (s.rust.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap);
                assert!(!c.is_null() && !r.is_null());
                fill(c, es, 0, 0);
                fill(r, es, 0, 0);
                assert_same(
                    &format!("arrgrowf(NULL, es={}, 0, min_cap={})", es, min_cap),
                    &dump_array(c, es, 0),
                    &dump_array(r, es, 0),
                );
                let expect = if min_cap < 4 { 4 } else { min_cap };
                assert_eq!(hdr(c).capacity, expect, "C capacity");
                (s.c.arrfreef)(c);
                (s.rust.arrfreef)(r);
            }
        }
    }
}

// --- row 3 ---------------------------------------------------------------
#[test]
fn cfg_03_arrgrowf_addlen_drives_min_cap() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 3);
    for &es in ELEMSIZES.iter() {
        for addlen in [1usize, 2, 3, 4, 5, 17, 63, 64, 1000] {
            unsafe {
                let c = (s.c.arrgrowf)(std::ptr::null_mut(), es, addlen, 0);
                let r = (s.rust.arrgrowf)(std::ptr::null_mut(), es, addlen, 0);
                assert_same(
                    &format!("arrgrowf(NULL, es={}, addlen={}, 0)", es, addlen),
                    &dump_array(c, es, 0),
                    &dump_array(r, es, 0),
                );
                (s.c.arrfreef)(c);
                (s.rust.arrfreef)(r);
            }
        }
    }
    for _ in 0..300 {
        let es = ELEMSIZES[rng.below(ELEMSIZES.len())];
        let addlen = rng.range(0, 4096);
        unsafe {
            let c = (s.c.arrgrowf)(std::ptr::null_mut(), es, addlen, 0);
            let r = (s.rust.arrgrowf)(std::ptr::null_mut(), es, addlen, 0);
            if c.is_null() {
                assert!(r.is_null());
                continue;
            }
            assert_same(
                &format!("arrgrowf(NULL, es={}, addlen={}, 0)", es, addlen),
                &dump_array(c, es, 0),
                &dump_array(r, es, 0),
            );
            (s.c.arrfreef)(c);
            (s.rust.arrfreef)(r);
        }
    }
}

// --- row 4 ---------------------------------------------------------------
#[test]
fn cfg_04_arrgrowf_addlen_vs_min_cap_orderings() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 4);
    for _ in 0..600 {
        let es = ELEMSIZES[rng.below(ELEMSIZES.len())];
        let addlen = rng.range(0, 64);
        let min_cap = rng.range(0, 64);
        unsafe {
            let c = (s.c.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
            let r = (s.rust.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
            if c.is_null() {
                assert!(r.is_null(), "es={} addlen={} min_cap={}", es, addlen, min_cap);
                continue;
            }
            assert!(!r.is_null());
            assert_same(
                &format!("arrgrowf(NULL, {}, {}, {})", es, addlen, min_cap),
                &dump_array(c, es, 0),
                &dump_array(r, es, 0),
            );
            (s.c.arrfreef)(c);
            (s.rust.arrfreef)(r);
        }
    }
}

// --- row 5 ---------------------------------------------------------------
#[test]
fn cfg_05_arrgrowf_doubling_ladder() {
    let s = session();
    for &es in ELEMSIZES.iter() {
        unsafe {
            let mut c: *mut c_void = std::ptr::null_mut();
            let mut r: *mut c_void = std::ptr::null_mut();
            for n in 0..200usize {
                // emulate stbds_arrmaybegrow(a, 1) + length++
                if c.is_null() || hdr(c).length + 1 > hdr(c).capacity {
                    c = (s.c.arrgrowf)(c, es, 1, 0);
                }
                if r.is_null() || hdr(r).length + 1 > hdr(r).capacity {
                    r = (s.rust.arrgrowf)(r, es, 1, 0);
                }
                set_len(c, n + 1);
                set_len(r, n + 1);
                fill(c, es, n + 1, 7);
                fill(r, es, n + 1, 7);
                assert_same(
                    &format!("arrgrowf ladder es={} n={}", es, n),
                    &dump_array(c, es, n + 1),
                    &dump_array(r, es, n + 1),
                );
            }
            (s.c.arrfreef)(c);
            (s.rust.arrfreef)(r);
        }
    }
}

// --- row 6 ---------------------------------------------------------------
#[test]
fn cfg_06_arrgrowf_nogrow_returns_identical_pointer() {
    let s = session();
    for &es in ELEMSIZES.iter() {
        unsafe {
            let c0 = (s.c.arrgrowf)(std::ptr::null_mut(), es, 0, 10);
            let r0 = (s.rust.arrgrowf)(std::ptr::null_mut(), es, 0, 10);
            set_len(c0, 3);
            set_len(r0, 3);
            fill(c0, es, 3, 11);
            fill(r0, es, 3, 11);
            let before_c = dump_array(c0, es, 3);
            let before_r = dump_array(r0, es, 3);
            for min_cap in [0usize, 1, 3, 5, 9, 10] {
                let c1 = (s.c.arrgrowf)(c0, es, 0, min_cap);
                let r1 = (s.rust.arrgrowf)(r0, es, 0, min_cap);
                assert_eq!(c1, c0, "C should not realloc (es={} min_cap={})", es, min_cap);
                assert_eq!(r1, r0, "RUST should not realloc (es={} min_cap={})", es, min_cap);
                assert_same(
                    &format!("arrgrowf no-grow es={} min_cap={}", es, min_cap),
                    &dump_array(c1, es, 3),
                    &dump_array(r1, es, 3),
                );
            }
            assert_eq!(before_c, dump_array(c0, es, 3));
            assert_eq!(before_r, dump_array(r0, es, 3));
            // addlen that keeps min_len <= cap must not grow either
            let c1 = (s.c.arrgrowf)(c0, es, 7, 0);
            let r1 = (s.rust.arrgrowf)(r0, es, 7, 0);
            assert_eq!(c1, c0);
            assert_eq!(r1, r0);
            (s.c.arrfreef)(c0);
            (s.rust.arrfreef)(r0);
        }
    }
}

// --- row 7 ---------------------------------------------------------------
#[test]
fn cfg_07_arrgrowf_jump_past_double() {
    let s = session();
    for &es in ELEMSIZES.iter() {
        for target in [9usize, 100, 1000, 4096] {
            unsafe {
                let mut c = (s.c.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
                let mut r = (s.rust.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
                set_len(c, 4);
                set_len(r, 4);
                fill(c, es, 4, 3);
                fill(r, es, 4, 3);
                c = (s.c.arrgrowf)(c, es, 0, target);
                r = (s.rust.arrgrowf)(r, es, 0, target);
                assert_same(
                    &format!("arrgrowf jump es={} target={}", es, target),
                    &dump_array(c, es, 4),
                    &dump_array(r, es, 4),
                );
                assert_eq!(hdr(c).capacity, target);
                (s.c.arrfreef)(c);
                (s.rust.arrfreef)(r);
            }
        }
    }
}

// --- row 8 ---------------------------------------------------------------
#[test]
fn cfg_08_arrgrowf_full_round_trip() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 8);
    for _ in 0..200 {
        let es = rng.range(1, 64);
        let n = rng.range(1, 40);
        unsafe {
            let mut c: *mut c_void = std::ptr::null_mut();
            let mut r: *mut c_void = std::ptr::null_mut();
            let payload = rng.bytes(es * n);
            for i in 0..n {
                if c.is_null() || hdr(c).length + 1 > hdr(c).capacity {
                    c = (s.c.arrgrowf)(c, es, 1, 0);
                }
                if r.is_null() || hdr(r).length + 1 > hdr(r).capacity {
                    r = (s.rust.arrgrowf)(r, es, 1, 0);
                }
                set_len(c, i + 1);
                set_len(r, i + 1);
                std::ptr::copy_nonoverlapping(
                    payload[es * i..].as_ptr(),
                    (c as *mut u8).add(es * i),
                    es,
                );
                std::ptr::copy_nonoverlapping(
                    payload[es * i..].as_ptr(),
                    (r as *mut u8).add(es * i),
                    es,
                );
            }
            assert_same(
                &format!("round trip es={} n={}", es, n),
                &dump_array(c, es, n),
                &dump_array(r, es, n),
            );
            // payload preserved through all the reallocs
            assert_eq!(
                std::slice::from_raw_parts(c as *const u8, es * n),
                &payload[..]
            );
            assert_eq!(
                std::slice::from_raw_parts(r as *const u8, es * n),
                &payload[..]
            );
            (s.c.arrfreef)(c);
            (s.rust.arrfreef)(r);
        }
    }
}

// --- row 20 --------------------------------------------------------------
#[test]
fn cfg_20_hmput_default_null_array() {
    let s = session();
    for &es in ELEMSIZES.iter() {
        unsafe {
            let c = (s.c.hmput_default)(std::ptr::null_mut(), es);
            let r = (s.rust.hmput_default)(std::ptr::null_mut(), es);
            assert!(!c.is_null() && !r.is_null());
            assert_same(
                &format!("hmput_default(NULL, {})", es),
                &dump_map(c, DumpOpts::raw(es)),
                &dump_map(r, DumpOpts::raw(es)),
            );
            (s.c.hmfree_func)((c as *mut u8).sub(es) as *mut c_void, es);
            (s.rust.hmfree_func)((r as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// --- row 21 --------------------------------------------------------------
#[test]
fn cfg_21_hmput_default_zero_length_array() {
    let s = session();
    for &es in ELEMSIZES.iter() {
        for pre_cap in [1usize, 4, 8, 64] {
            unsafe {
                let ca = (s.c.arrgrowf)(std::ptr::null_mut(), es, 0, pre_cap);
                let ra = (s.rust.arrgrowf)(std::ptr::null_mut(), es, 0, pre_cap);
                assert_eq!(hdr(ca).length, 0);
                let c = (s.c.hmput_default)((ca as *mut u8).add(es) as *mut c_void, es);
                let r = (s.rust.hmput_default)((ra as *mut u8).add(es) as *mut c_void, es);
                assert_same(
                    &format!("hmput_default(len=0 cap={}, es={})", pre_cap, es),
                    &dump_map(c, DumpOpts::raw(es)),
                    &dump_map(r, DumpOpts::raw(es)),
                );
                (s.c.hmfree_func)((c as *mut u8).sub(es) as *mut c_void, es);
                (s.rust.hmfree_func)((r as *mut u8).sub(es) as *mut c_void, es);
            }
        }
    }
}

// --- row 22 --------------------------------------------------------------
#[test]
fn cfg_22_hmput_default_nonempty_is_noop() {
    let s = session();
    for &es in ELEMSIZES.iter() {
        unsafe {
            let mut c = (s.c.hmput_default)(std::ptr::null_mut(), es);
            let mut r = (s.rust.hmput_default)(std::ptr::null_mut(), es);
            let before_c = dump_map(c, DumpOpts::raw(es));
            let before_r = dump_map(r, DumpOpts::raw(es));
            for _ in 0..5 {
                let c2 = (s.c.hmput_default)(c, es);
                let r2 = (s.rust.hmput_default)(r, es);
                assert_eq!(c2, c, "C hmput_default must be a no-op (es={})", es);
                assert_eq!(r2, r, "RUST hmput_default must be a no-op (es={})", es);
                c = c2;
                r = r2;
            }
            assert_same(
                &format!("hmput_default no-op es={}", es),
                &dump_map(c, DumpOpts::raw(es)),
                &dump_map(r, DumpOpts::raw(es)),
            );
            assert_eq!(before_c, dump_map(c, DumpOpts::raw(es)));
            assert_eq!(before_r, dump_map(r, DumpOpts::raw(es)));
            (s.c.hmfree_func)((c as *mut u8).sub(es) as *mut c_void, es);
            (s.rust.hmfree_func)((r as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// --- row 23 --------------------------------------------------------------
#[test]
fn cfg_23_hmput_default_after_hmput_key() {
    let s = session();
    let lay = L_I2I;
    unsafe {
        let mut c: *mut c_void = std::ptr::null_mut();
        let mut r: *mut c_void = std::ptr::null_mut();
        for i in 0..10i32 {
            let key = i.to_ne_bytes();
            let val = (i * 3).to_ne_bytes();
            c = map_put_binary(s.c, c, lay, &key, &val, HM_BINARY);
            r = map_put_binary(s.rust, r, lay, &key, &val, HM_BINARY);
        }
        let before_c = dump_map(c, DumpOpts::raw(lay.elemsize));
        let before_r = dump_map(r, DumpOpts::raw(lay.elemsize));
        let c2 = (s.c.hmput_default)(c, lay.elemsize);
        let r2 = (s.rust.hmput_default)(r, lay.elemsize);
        assert_eq!(c2, c);
        assert_eq!(r2, r);
        assert_same(
            "hmput_default after hmput_key",
            &dump_map(c2, DumpOpts::raw(lay.elemsize)),
            &dump_map(r2, DumpOpts::raw(lay.elemsize)),
        );
        assert_eq!(before_c, dump_map(c2, DumpOpts::raw(lay.elemsize)));
        assert_eq!(before_r, dump_map(r2, DumpOpts::raw(lay.elemsize)));
        map_free(s.c, c2, lay);
        map_free(s.rust, r2, lay);
    }
}
