//! Phase B rows 12–20: the raw array primitive `stbds_arrgrowf` / `stbds_arrfreef`
//! and `stbds_hmput_default`.

mod common;
use common::*;
use std::ffi::c_void;

const ELEMSIZES: [usize; 6] = [1, 4, 8, 12, 16, 24];

// --- row 12 ---------------------------------------------------------------

#[test]
fn row12_arrgrowf_from_null() {
    let (c, r) = libs();
    for &es in &ELEMSIZES {
        for &addlen in &[0usize, 1, 3, 17] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 7, 8, 100, 1000] {
                unsafe {
                    let a = (c.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    let b = (r.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    let ctx = format!("row12 es={es} addlen={addlen} min_cap={min_cap}");
                    assert_eq!(a.is_null(), b.is_null(), "{ctx}: null-ness differs");
                    assert_eq!(snapshot_raw(a), snapshot_raw(b), "{ctx}");
                    if !a.is_null() {
                        // expected capacity per the C: min_len=addlen; min_cap=max;
                        // then `min_cap < 2*0` is false so the `< 4` bump applies
                        let want = {
                            let ml = addlen;
                            let mut mc = if ml > min_cap { ml } else { min_cap };
                            if mc < 4 {
                                mc = 4;
                            }
                            mc
                        };
                        assert_eq!(snapshot_raw(a).capacity, want, "{ctx}: capacity");
                        (c.arrfreef)(a);
                        (r.arrfreef)(b);
                    } else {
                        // row 2 of ERRORS.md: addlen == 0 && min_cap == 0 -> NULL
                        assert!(addlen == 0 && min_cap == 0, "{ctx}: unexpected NULL");
                    }
                }
            }
        }
    }
}

// --- rows 13/14/15 --------------------------------------------------------

#[test]
fn row13_arrgrowf_early_return_when_capacity_suffices() {
    let (c, r) = libs();
    for &es in &ELEMSIZES {
        unsafe {
            let mut a = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 10);
            let mut b = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 10);
            assert_eq!(snapshot_raw(a).capacity, 10);
            for &min_cap in &[0usize, 1, 5, 9, 10] {
                let a2 = (c.arrgrowf)(a, es, 0, min_cap);
                let b2 = (r.arrgrowf)(b, es, 0, min_cap);
                let ctx = format!("row13 es={es} min_cap={min_cap}");
                assert_eq!(a2, a, "{ctx}: C must return the same pointer");
                assert_eq!(b2, b, "{ctx}: Rust must return the same pointer");
                assert_eq!(snapshot_raw(a2), snapshot_raw(b2), "{ctx}");
                a = a2;
                b = b2;
            }
            (c.arrfreef)(a);
            (r.arrfreef)(b);
        }
    }
}

#[test]
fn row14_arrgrowf_doubling_window() {
    let (c, r) = libs();
    for &es in &ELEMSIZES {
        for &(cap0, min_cap) in &[(10usize, 11usize), (10, 15), (10, 19), (4, 5), (4, 7), (100, 101), (100, 199)] {
            unsafe {
                let a = (c.arrgrowf)(std::ptr::null_mut(), es, 0, cap0);
                let b = (r.arrgrowf)(std::ptr::null_mut(), es, 0, cap0);
                let a2 = (c.arrgrowf)(a, es, 0, min_cap);
                let b2 = (r.arrgrowf)(b, es, 0, min_cap);
                let ctx = format!("row14 es={es} cap0={cap0} min_cap={min_cap}");
                assert_eq!(snapshot_raw(a2), snapshot_raw(b2), "{ctx}");
                assert_eq!(snapshot_raw(a2).capacity, 2 * cap0, "{ctx}: expected doubling");
                (c.arrfreef)(a2);
                (r.arrfreef)(b2);
            }
        }
    }
}

#[test]
fn row15_arrgrowf_exact_when_beyond_double() {
    let (c, r) = libs();
    for &es in &ELEMSIZES {
        for &(cap0, min_cap) in &[(10usize, 20usize), (10, 21), (10, 1000), (4, 8), (4, 9)] {
            unsafe {
                let a = (c.arrgrowf)(std::ptr::null_mut(), es, 0, cap0);
                let b = (r.arrgrowf)(std::ptr::null_mut(), es, 0, cap0);
                let a2 = (c.arrgrowf)(a, es, 0, min_cap);
                let b2 = (r.arrgrowf)(b, es, 0, min_cap);
                let ctx = format!("row15 es={es} cap0={cap0} min_cap={min_cap}");
                assert_eq!(snapshot_raw(a2), snapshot_raw(b2), "{ctx}");
                assert_eq!(snapshot_raw(a2).capacity, min_cap, "{ctx}: expected exact min_cap");
                (c.arrfreef)(a2);
                (r.arrfreef)(b2);
            }
        }
    }
}

// --- row 16 ---------------------------------------------------------------

#[test]
fn row16_arrgrowf_growth_chain() {
    let (c, r) = libs();
    let mut rng = Rng::new(0x1616_1616);
    for &es in &ELEMSIZES {
        unsafe {
            let mut a: *mut c_void = std::ptr::null_mut();
            let mut b: *mut c_void = std::ptr::null_mut();
            let mut caps_c = Vec::new();
            let mut caps_r = Vec::new();
            for step in 0..200 {
                // emulate `stbds_arrmaybegrow`: length is advanced by hand
                let addlen = 1 + rng.below(3);
                a = (c.arrgrowf)(a, es, addlen, 0);
                b = (r.arrgrowf)(b, es, addlen, 0);
                let ctx = format!("row16 es={es} step={step} addlen={addlen}");
                assert_eq!(snapshot_raw(a), snapshot_raw(b), "{ctx}");
                caps_c.push(snapshot_raw(a).capacity);
                caps_r.push(snapshot_raw(b).capacity);
                // advance length like the arrput macro does
                let ha = (a as *mut ArrayHeader).offset(-1);
                let hb = (b as *mut ArrayHeader).offset(-1);
                (*ha).length += addlen;
                (*hb).length += addlen;
            }
            assert_eq!(caps_c, caps_r, "row16 es={es}: capacity sequence");
            (c.arrfreef)(a);
            (r.arrfreef)(b);
        }
    }
}

// --- row 17 ---------------------------------------------------------------

#[test]
fn row17_arrgrowf_then_free() {
    let (c, r) = libs();
    for &es in &ELEMSIZES {
        unsafe {
            for &n in &[1usize, 4, 64, 4096] {
                let a = (c.arrgrowf)(std::ptr::null_mut(), es, 0, n);
                let b = (r.arrgrowf)(std::ptr::null_mut(), es, 0, n);
                assert_eq!(snapshot_raw(a), snapshot_raw(b));
                (c.arrfreef)(a);
                (r.arrfreef)(b);
            }
        }
    }
}

// --- rows 18/19/20: stbds_hmput_default ----------------------------------

#[test]
fn row18_hmput_default_from_null() {
    let (c, r) = libs();
    for &es in &[1usize, 8, 16, 24, 12] {
        unsafe {
            let a = (c.hmput_default)(std::ptr::null_mut(), es);
            let b = (r.hmput_default)(std::ptr::null_mut(), es);
            let ctx = format!("row18 es={es}");
            assert_eq!(snapshot(a, es), snapshot(b, es), "{ctx}");
            let s = snapshot(a, es);
            assert_eq!(s.length, 1, "{ctx}");
            assert!(!s.has_table, "{ctx}: hmput_default must not create a table");
            // element 0 must be zeroed in both
            let ea = std::slice::from_raw_parts((a as *mut u8).sub(es), es).to_vec();
            let eb = std::slice::from_raw_parts((b as *mut u8).sub(es), es).to_vec();
            assert_eq!(ea, eb, "{ctx}: default element bytes");
            assert!(ea.iter().all(|&x| x == 0), "{ctx}: default element must be zeroed");
            (c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn row19_hmput_default_idempotent() {
    let (c, r) = libs();
    for &es in &[1usize, 8, 16, 24] {
        unsafe {
            let a = (c.hmput_default)(std::ptr::null_mut(), es);
            let b = (r.hmput_default)(std::ptr::null_mut(), es);
            for i in 0..5 {
                let a2 = (c.hmput_default)(a, es);
                let b2 = (r.hmput_default)(b, es);
                let ctx = format!("row19 es={es} i={i}");
                assert_eq!(a2, a, "{ctx}: C must return the same pointer");
                assert_eq!(b2, b, "{ctx}: Rust must return the same pointer");
                assert_eq!(snapshot(a2, es), snapshot(b2, es), "{ctx}");
                assert_eq!(snapshot(a2, es).length, 1, "{ctx}");
            }
            (c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn row20_hmput_default_with_zero_length() {
    let (c, r) = libs();
    for &es in &[1usize, 8, 16, 24] {
        unsafe {
            // build an array via arrgrowf (length == 0), bias it to a "hash" ptr
            let ra = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            let rb = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            let a = (ra as *mut u8).add(es) as *mut c_void;
            let b = (rb as *mut u8).add(es) as *mut c_void;
            let a2 = (c.hmput_default)(a, es);
            let b2 = (r.hmput_default)(b, es);
            let ctx = format!("row20 es={es}");
            assert_eq!(snapshot(a2, es), snapshot(b2, es), "{ctx}");
            assert_eq!(snapshot(a2, es).length, 1, "{ctx}: length must become 1");
            assert_eq!(snapshot(a2, es).capacity, 4, "{ctx}: capacity kept");
            (c.hmfree_func)((a2 as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((b2 as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}
