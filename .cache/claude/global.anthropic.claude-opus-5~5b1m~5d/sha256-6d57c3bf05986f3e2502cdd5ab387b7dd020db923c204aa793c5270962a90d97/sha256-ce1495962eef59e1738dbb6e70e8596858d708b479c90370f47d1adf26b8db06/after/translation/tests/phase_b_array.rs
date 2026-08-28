//! Phase B — CONFIGS.md rows 10..14: the raw dynamic array
//! (`stbds_arrgrowf` / `stbds_arrfreef`), i.e. the `arrput` / `arrsetcap` /
//! `arrsetlen` / `arrfree` macro family driven at its lowest level.

mod common;

use common::*;
use std::ffi::c_void;

/// Canonical dump of an array-header + `n` payload bytes.
unsafe fn arr_snapshot(p: *mut u8, payload: usize) -> String {
    if p.is_null() {
        return "ARR=NULL".into();
    }
    let h = read_header(p);
    let bytes = std::slice::from_raw_parts(p, payload);
    format!(
        "len={} cap={} temp={} table={} payload={:?}",
        h.length,
        h.capacity,
        h.temp,
        if h.hash_table.is_null() { "null" } else { "some" },
        bytes
    )
}

// row 10 ----------------------------------------------------------------------
#[test]
fn cfg_10_arrgrowf_fresh_min_cap() {
    let (c, r, _g) = libs();
    unsafe {
        for elemsize in [1usize, 2, 3, 8, 16, 64] {
            for min_cap in [0usize, 1, 2, 3, 4, 5, 7, 8, 9, 1000] {
                let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                let b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                assert_eq!(a.is_null(), b.is_null(), "NULL-ness (elemsize={}, min_cap={})", elemsize, min_cap);
                if a.is_null() {
                    continue;
                }
                let sa = arr_snapshot(a as *mut u8, 0);
                let sb = arr_snapshot(b as *mut u8, 0);
                assert_eq!(sa, sb, "arrgrowf(NULL,{},0,{})", elemsize, min_cap);
                (c.arrfreef)(a);
                (r.arrfreef)(b);
            }
        }
    }
}

// row 11 ----------------------------------------------------------------------
#[test]
fn cfg_11_arrgrowf_fresh_addlen() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xA11_11);
    unsafe {
        let mut addlens: Vec<usize> = vec![0, 1, 2, 3, 4, 5, 1000];
        for _ in 0..200 {
            addlens.push(rng.below(4096));
        }
        for addlen in addlens {
            for elemsize in [1usize, 8, 13] {
                for min_cap in [0usize, 1, 4, 17] {
                    let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    assert_eq!(a.is_null(), b.is_null());
                    if a.is_null() {
                        continue;
                    }
                    assert_eq!(
                        arr_snapshot(a as *mut u8, 0),
                        arr_snapshot(b as *mut u8, 0),
                        "arrgrowf(NULL,{},{},{})",
                        elemsize,
                        addlen,
                        min_cap
                    );
                    (c.arrfreef)(a);
                    (r.arrfreef)(b);
                }
            }
        }
    }
}

// row 12 ----------------------------------------------------------------------
#[test]
fn cfg_12_arrgrowf_repeated_arrput() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xA11_12);
    unsafe {
        for elemsize in [1usize, 2, 4, 8, 16] {
            // emulate `arrput` exactly: maybegrow(1) then a[length++] = v
            let mut a: *mut u8 = std::ptr::null_mut();
            let mut b: *mut u8 = std::ptr::null_mut();
            for i in 0..64usize {
                let grow = a.is_null() || {
                    let h = read_header(a);
                    h.length + 1 > h.capacity
                };
                if grow {
                    a = (c.arrgrowf)(a as *mut c_void, elemsize, 1, 0) as *mut u8;
                    b = (r.arrgrowf)(b as *mut c_void, elemsize, 1, 0) as *mut u8;
                }
                let val = rng.bytes(elemsize);
                let ha = read_header(a).length;
                let hb = read_header(b).length;
                assert_eq!(ha, hb);
                std::ptr::copy_nonoverlapping(val.as_ptr(), a.add(elemsize * ha), elemsize);
                std::ptr::copy_nonoverlapping(val.as_ptr(), b.add(elemsize * hb), elemsize);
                (*(a.sub(HDR) as *mut Header)).length += 1;
                (*(b.sub(HDR) as *mut Header)).length += 1;
                assert_eq!(
                    arr_snapshot(a, elemsize * (i + 1)),
                    arr_snapshot(b, elemsize * (i + 1)),
                    "after arrput #{} (elemsize={})",
                    i,
                    elemsize
                );
            }
            (c.arrfreef)(a as *mut c_void);
            (r.arrfreef)(b as *mut c_void);
        }
    }
}

// row 13 ----------------------------------------------------------------------
#[test]
fn cfg_13_arrgrowf_setcap_and_noop() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xA11_13);
    unsafe {
        for elemsize in [1usize, 8, 24] {
            let mut a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) as *mut u8;
            let mut b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) as *mut u8;
            // fill the payload so that reallocation must preserve it
            let payload = rng.bytes(elemsize * 4);
            std::ptr::copy_nonoverlapping(payload.as_ptr(), a, payload.len());
            std::ptr::copy_nonoverlapping(payload.as_ptr(), b, payload.len());
            (*(a.sub(HDR) as *mut Header)).length = 4;
            (*(b.sub(HDR) as *mut Header)).length = 4;

            for min_cap in [0usize, 1, 3, 4, 5, 6, 8, 9, 100, 101, 1000, 999] {
                // must be read *before* the call: a successful grow reallocates
                let cap_before = read_header(a).capacity;
                let na = (c.arrgrowf)(a as *mut c_void, elemsize, 0, min_cap) as *mut u8;
                let nb = (r.arrgrowf)(b as *mut c_void, elemsize, 0, min_cap) as *mut u8;
                if min_cap <= cap_before {
                    assert_eq!(na, a, "C must return `a` unchanged for min_cap={}", min_cap);
                    assert_eq!(nb, b, "Rust must return `a` unchanged for min_cap={}", min_cap);
                }
                a = na;
                b = nb;
                assert_eq!(
                    arr_snapshot(a, elemsize * 4),
                    arr_snapshot(b, elemsize * 4),
                    "arrsetcap({}) elemsize={}",
                    min_cap,
                    elemsize
                );
            }
            // addlen path on an existing array
            for addlen in [0usize, 1, 2, 100, 5000] {
                a = (c.arrgrowf)(a as *mut c_void, elemsize, addlen, 0) as *mut u8;
                b = (r.arrgrowf)(b as *mut c_void, elemsize, addlen, 0) as *mut u8;
                assert_eq!(
                    arr_snapshot(a, elemsize * 4),
                    arr_snapshot(b, elemsize * 4),
                    "arraddn({}) elemsize={}",
                    addlen,
                    elemsize
                );
            }
            (c.arrfreef)(a as *mut c_void);
            (r.arrfreef)(b as *mut c_void);
        }
    }
}

// row 14 ----------------------------------------------------------------------
#[test]
fn cfg_14_arrgrowf_free_roundtrip() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xA11_14);
    unsafe {
        // many grow/free cycles: exercises the libc-allocator handoff
        for _ in 0..300 {
            let elemsize = 1 + rng.below(40);
            let n = rng.below(50);
            let mut a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, n, 0) as *mut u8;
            let mut b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, n, 0) as *mut u8;
            for _ in 0..4 {
                let extra = rng.below(30);
                a = (c.arrgrowf)(a as *mut c_void, elemsize, extra, 0) as *mut u8;
                b = (r.arrgrowf)(b as *mut c_void, elemsize, extra, 0) as *mut u8;
                let cap = read_header(a).capacity;
                let fill = rng.bytes(elemsize * cap);
                std::ptr::copy_nonoverlapping(fill.as_ptr(), a, fill.len());
                std::ptr::copy_nonoverlapping(fill.as_ptr(), b, fill.len());
                assert_eq!(
                    arr_snapshot(a, elemsize * cap),
                    arr_snapshot(b, elemsize * cap)
                );
            }
            (c.arrfreef)(a as *mut c_void);
            (r.arrfreef)(b as *mut c_void);
        }
    }
}
