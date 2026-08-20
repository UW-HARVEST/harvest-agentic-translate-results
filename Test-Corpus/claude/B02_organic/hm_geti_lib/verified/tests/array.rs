//! Phase B — CONFIGS.md rows 13..20: `stbds_arrgrowf` / `stbds_arrfreef`
//! driven exactly like the `stbds_arrmaybegrow` / `stbds_arrput` macros.

mod common;
use common::*;
use std::ffi::c_void;

const ELEMSIZES: [usize; 8] = [1, 2, 4, 8, 16, 24, 32, 64];

/// Fills elements `[0, len)` with a deterministic pattern so that snapshots
/// never look at uninitialised heap.
unsafe fn fill(a: *mut c_void, elemsize: usize, from: usize, to: usize, tag: u8) {
    for i in from..to {
        let p = (a as *mut u8).wrapping_add(i * elemsize);
        for b in 0..elemsize {
            *p.wrapping_add(b) = tag.wrapping_add((i as u8).wrapping_mul(7)).wrapping_add(b as u8);
        }
    }
}

// --------------------------------------------------------------- row 13
#[test]
fn row13_arrgrowf_null_nogrow_returns_null() {
    let (c, r) = load_both();
    unsafe {
        for &es in &ELEMSIZES {
            let cv = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let rv = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            diff_eq_val(
                &format!("row13 arrgrowf(NULL,{es},0,0) null-ness"),
                cv.is_null(),
                rv.is_null(),
            );
            assert!(cv.is_null(), "C must return NULL (early return path)");
        }
    }
}

// --------------------------------------------------------------- row 14
#[test]
fn row14_arrgrowf_fresh_min_cap_clamp() {
    let (c, r) = load_both();
    unsafe {
        for &es in &ELEMSIZES {
            for &min_cap in &[1usize, 2, 3, 4, 5, 7, 8, 9, 100, 1000] {
                let ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap);
                let ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap);
                diff_eq(
                    &format!("row14 arrgrowf(NULL,{es},0,{min_cap})"),
                    &snapshot_array(ca, es, 0),
                    &snapshot_array(ra, es, 0),
                );
                (c.arrfreef)(ca);
                (r.arrfreef)(ra);
            }
        }
    }
}

// --------------------------------------------------------------- row 15
#[test]
fn row15_arrgrowf_fresh_addlen_drives_min_len() {
    let (c, r) = load_both();
    unsafe {
        for &es in &ELEMSIZES {
            for &addlen in &[1usize, 2, 3, 4, 5, 17, 63, 64, 65] {
                for &min_cap in &[0usize, 1, 4, 100] {
                    let ca = (c.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    let ra = (r.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    diff_eq(
                        &format!("row15 arrgrowf(NULL,{es},{addlen},{min_cap})"),
                        &snapshot_array(ca, es, 0),
                        &snapshot_array(ra, es, 0),
                    );
                    (c.arrfreef)(ca);
                    (r.arrfreef)(ra);
                }
            }
        }
    }
}

// ---------------------------------------------------------- rows 16..18
#[test]
fn row16_18_arrgrowf_existing_capacity_rules() {
    let (c, r) = load_both();
    unsafe {
        for &es in &[1usize, 4, 8, 24] {
            // build an array with capacity `cap0` and length `len0`
            for &(len0, cap_req) in &[(0usize, 4usize), (3, 4), (4, 8), (7, 8), (10, 16), (16, 16)] {
                let mut ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, cap_req);
                let mut ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, cap_req);
                (*header(ca)).length = len0;
                (*header(ra)).length = len0;
                fill(ca, es, 0, len0, 0x10);
                fill(ra, es, 0, len0, 0x10);
                let cap = arr_cap(ca);
                diff_eq_val("row16 setup capacity", cap, arr_cap(ra));

                // row 16: min_cap <= cap  -> identical pointer, header untouched
                for &mc in &[0usize, 1, cap / 2, cap] {
                    let cb = (c.arrgrowf)(ca, es, 0, mc);
                    let rb = (r.arrgrowf)(ra, es, 0, mc);
                    diff_eq_val(
                        &format!("row16 identity es={es} len={len0} cap={cap} mc={mc}"),
                        cb == ca,
                        rb == ra,
                    );
                    assert_eq!(cb, ca, "C must return the same pointer");
                    diff_eq(
                        &format!("row16 es={es} len={len0} cap={cap} mc={mc}"),
                        &snapshot_array(cb, es, len0),
                        &snapshot_array(rb, es, len0),
                    );
                }

                // row 17: cap < min_cap < 2*cap  -> capacity doubles
                // row 18: min_cap >= 2*cap       -> capacity == min_cap
                for &mc in &[cap + 1, cap + 2, 2 * cap - 1, 2 * cap, 2 * cap + 1, 4 * cap + 3] {
                    ca = (c.arrgrowf)(ca, es, 0, mc);
                    ra = (r.arrgrowf)(ra, es, 0, mc);
                    diff_eq(
                        &format!("row17/18 es={es} len={len0} mc={mc}"),
                        &snapshot_array(ca, es, len0),
                        &snapshot_array(ra, es, len0),
                    );
                }
                (c.arrfreef)(ca);
                (r.arrfreef)(ra);
            }
        }
    }
}

// --------------------------------------------------------------- row 19
#[test]
fn row19_arrgrowf_randomized_push_pipeline() {
    let (c, r) = load_both();
    let mut rng = Rng::new(19);
    unsafe {
        for &es in &[1usize, 4, 8, 16, 24] {
            let mut ca: *mut c_void = std::ptr::null_mut();
            let mut ra: *mut c_void = std::ptr::null_mut();
            let mut len = 0usize;
            for step in 0..200 {
                let n = 1 + rng.below(5) as usize;
                // `stbds_arrmaybegrow(a,n)`
                let need_grow = ca.is_null() || (*header(ca)).length + n > (*header(ca)).capacity;
                let need_grow_r = ra.is_null() || (*header(ra)).length + n > (*header(ra)).capacity;
                diff_eq_val(&format!("row19 es={es} step={step} maybegrow"), need_grow, need_grow_r);
                if need_grow {
                    ca = (c.arrgrowf)(ca, es, n, 0);
                    ra = (r.arrgrowf)(ra, es, n, 0);
                }
                // `stbds_arraddnindex`: length += n, then the caller writes
                (*header(ca)).length += n;
                (*header(ra)).length += n;
                let tag = rng.byte();
                fill(ca, es, len, len + n, tag);
                fill(ra, es, len, len + n, tag);
                len += n;
                diff_eq(
                    &format!("row19 es={es} step={step} len={len}"),
                    &snapshot_array(ca, es, len),
                    &snapshot_array(ra, es, len),
                );
            }
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}

// --------------------------------------------------------------- row 20
#[test]
fn row20_arrfreef() {
    let (c, r) = load_both();
    unsafe {
        for &es in &ELEMSIZES {
            // fresh
            let ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            diff_eq("row20 fresh", &snapshot_array(ca, es, 0), &snapshot_array(ra, es, 0));
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);

            // after several grows
            let mut ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let mut ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            for k in 1..8 {
                ca = (c.arrgrowf)(ca, es, k, 0);
                ra = (r.arrgrowf)(ra, es, k, 0);
                (*header(ca)).length += k;
                (*header(ra)).length += k;
            }
            let l = arr_len(ca) as usize;
            fill(ca, es, 0, l, 0x33);
            fill(ra, es, 0, l, 0x33);
            diff_eq("row20 grown", &snapshot_array(ca, es, l), &snapshot_array(ra, es, l));
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}
