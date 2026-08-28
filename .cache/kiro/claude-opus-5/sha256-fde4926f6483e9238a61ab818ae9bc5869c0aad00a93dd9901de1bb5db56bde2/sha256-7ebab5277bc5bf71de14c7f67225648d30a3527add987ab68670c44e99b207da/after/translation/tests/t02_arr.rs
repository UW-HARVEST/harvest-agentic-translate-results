//! `stbds_arrgrowf` / `stbds_arrfreef` – the dynamic-array primitives that
//! everything else is built on.

mod common;

use common::*;
use std::ffi::c_void;

/// Emulates `stbds_arrput(a, v)` for a `u32` element array.
unsafe fn arrput_u32(grow: FnArrGrowF, a: *mut c_void, v: u32) -> *mut c_void {
    let elemsize = 4usize;
    let mut a = a;
    let need = if a.is_null() {
        true
    } else {
        (*header_of(a)).length + 1 > (*header_of(a)).capacity
    };
    if need {
        a = grow(a, elemsize, 1, 0);
    }
    let len = (*header_of(a)).length;
    *((a as *mut u8).add(elemsize * len) as *mut u32) = v;
    (*header_of(a)).length = len + 1;
    a
}

#[test]
fn arrgrowf_fresh_allocation() {
    let p = load_pair();

    for &elemsize in &[1usize, 2, 4, 8, 16, 20, 64, 100] {
        for &addlen in &[0usize, 1, 2, 3, 5, 17, 1000] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 8, 9, 1024] {
                unsafe {
                    let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let what =
                        format!("arrgrowf(NULL, elemsize={elemsize}, addlen={addlen}, min_cap={min_cap})");
                    // `min_cap == 0 && addlen == 0` requests nothing, so the NULL
                    // input is returned unchanged.
                    assert_eq!(ca.is_null(), ra.is_null(), "{what}: nullness differs");
                    if ca.is_null() {
                        assert!(addlen == 0 && min_cap == 0, "{what}: unexpected NULL");
                        continue;
                    }
                    assert_eq!(dump_header(ca), dump_header(ra), "{what}");
                    (p.c.arrfreef)(ca);
                    (p.r.arrfreef)(ra);
                }
            }
        }
    }
}

#[test]
fn arrgrowf_regrow_sequences() {
    let p = load_pair();

    for &elemsize in &[1usize, 4, 8, 12, 32] {
        unsafe {
            let mut ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            let mut ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            assert_eq!(dump_header(ca), dump_header(ra), "seed alloc e={elemsize}");

            // Walk a long series of grow requests, mutating length the way the
            // stb macros do so that `min_len` keeps moving.
            for step in 0..64usize {
                let addlen = step % 7;
                let min_cap = (step * 3) % 11;
                (*header_of(ca)).length = step % 5;
                (*header_of(ra)).length = step % 5;
                ca = (p.c.arrgrowf)(ca, elemsize, addlen, min_cap);
                ra = (p.r.arrgrowf)(ra, elemsize, addlen, min_cap);
                assert_eq!(
                    dump_header(ca),
                    dump_header(ra),
                    "regrow step={step} e={elemsize} addlen={addlen} min_cap={min_cap}"
                );
            }
            (p.c.arrfreef)(ca);
            (p.r.arrfreef)(ra);
        }
    }
}

#[test]
fn arrput_growth_and_contents() {
    let p = load_pair();
    let elemsize = 4usize;

    unsafe {
        let mut ca: *mut c_void = std::ptr::null_mut();
        let mut ra: *mut c_void = std::ptr::null_mut();

        for i in 0..5000u32 {
            let v = i.wrapping_mul(2_654_435_761);
            ca = arrput_u32(p.c.arrgrowf, ca, v);
            ra = arrput_u32(p.r.arrgrowf, ra, v);
            if i < 64 || i % 251 == 0 {
                assert_eq!(dump_header(ca), dump_header(ra), "arrput header at i={i}");
            }
        }
        assert_eq!(dump_header(ca), dump_header(ra), "arrput final header");
        assert_eq!(
            dump_elems_bytes(ca, elemsize),
            dump_elems_bytes(ra, elemsize),
            "arrput contents"
        );

        (p.c.arrfreef)(ca);
        (p.r.arrfreef)(ra);
    }
}

#[test]
fn arrgrowf_noop_when_capacity_suffices() {
    let p = load_pair();
    let elemsize = 8usize;
    unsafe {
        let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
        let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
        // requesting less than the existing capacity must return the same pointer
        let ca2 = (p.c.arrgrowf)(ca, elemsize, 0, 4);
        let ra2 = (p.r.arrgrowf)(ra, elemsize, 0, 4);
        assert_eq!(ca, ca2, "C returned a new pointer unexpectedly");
        assert_eq!(ra, ra2, "Rust returned a new pointer unexpectedly");
        assert_eq!(dump_header(ca2), dump_header(ra2));
        (p.c.arrfreef)(ca2);
        (p.r.arrfreef)(ra2);
    }
}
