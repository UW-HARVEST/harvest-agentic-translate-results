//! Phase B — CONFIGS.md rows C7..C9: `stbds_arrgrowf` / `stbds_arrfreef`.
mod common;

use common::*;
use std::ffi::c_void;

unsafe fn hdr_of_raw(a: *mut c_void) -> ArrayHeader {
    *(a as *mut ArrayHeader).sub(1)
}

fn dump(a: *mut c_void) -> Vec<String> {
    unsafe {
        if a.is_null() {
            return vec!["NULL".into()];
        }
        let h = hdr_of_raw(a);
        vec![
            format!("length={}", h.length),
            format!("capacity={}", h.capacity),
            format!("temp={}", h.temp),
            format!("has_table={}", !h.hash_table.is_null()),
        ]
    }
}

/// C7 — fresh (`a == NULL`) allocation over the full elemsize × addlen × min_cap
/// cross product.
#[test]
fn cfg_arrgrowf_fresh_matrix() {
    let l = libs();
    for elemsize in [1usize, 2, 4, 8, 16, 24, 32, 64, 4096] {
        for addlen in [0usize, 1, 2, 7, 64] {
            for min_cap in [0usize, 1, 2, 3, 4, 5, 8, 100] {
                unsafe {
                    let ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    assert_same(
                        &format!("arrgrowf(NULL,{elemsize},{addlen},{min_cap})"),
                        &dump(ca),
                        &dump(ra),
                    );
                    // NOTE: `arrgrowf(NULL, es, 0, 0)` returns NULL (min_cap 0
                    // <= arrcap(NULL) 0 -> early return `a`), and
                    // `stbds_arrfreef(NULL)` would `free((char*)NULL - 32)`,
                    // which crashes in *both* libraries (see ERRORS.md E1/E35).
                    if !ca.is_null() {
                        (l.c.arrfreef)(ca);
                    }
                    if !ra.is_null() {
                        (l.r.arrfreef)(ra);
                    }
                }
            }
        }
    }
}

/// C8 — growth chain on an existing array: repeated `arrmaybegrow`-style calls
/// exercising the `min_cap < 2*cap` doubling path and the early-out.
#[test]
fn cfg_arrgrowf_growth_chain() {
    let l = libs();
    for elemsize in [1usize, 4, 8, 16, 40] {
        unsafe {
            let mut ca: *mut c_void = std::ptr::null_mut();
            let mut ra: *mut c_void = std::ptr::null_mut();
            for n in 0..200usize {
                // emulate stbds_arrmaybegrow(a,1) + length++
                let need_c = ca.is_null()
                    || hdr_of_raw(ca).length + 1 > hdr_of_raw(ca).capacity;
                let need_r = ra.is_null()
                    || hdr_of_raw(ra).length + 1 > hdr_of_raw(ra).capacity;
                assert_eq!(need_c, need_r, "grow decision at n={n}");
                if need_c {
                    ca = (l.c.arrgrowf)(ca, elemsize, 1, 0);
                    ra = (l.r.arrgrowf)(ra, elemsize, 1, 0);
                }
                (*(ca as *mut ArrayHeader).sub(1)).length += 1;
                (*(ra as *mut ArrayHeader).sub(1)).length += 1;
                assert_same(
                    &format!("arrgrowf chain elemsize={elemsize} n={n}"),
                    &dump(ca),
                    &dump(ra),
                );
            }
            // explicit arrsetcap-style calls, incl. no-op ones
            for cap in [0usize, 1, 200, 201, 300, 100000, 100000] {
                ca = (l.c.arrgrowf)(ca, elemsize, 0, cap);
                ra = (l.r.arrgrowf)(ra, elemsize, 0, cap);
                assert_same(
                    &format!("arrsetcap({cap}) elemsize={elemsize}"),
                    &dump(ca),
                    &dump(ra),
                );
            }
            (l.c.arrfreef)(ca);
            (l.r.arrfreef)(ra);
        }
    }
}

/// C9 — the payload survives the realloc identically.
#[test]
fn cfg_arrgrowf_payload_survives() {
    let l = libs();
    let mut rng = Rng::new(0xC9_0000);
    let elemsize = 8usize;
    unsafe {
        let mut ca: *mut c_void = std::ptr::null_mut();
        let mut ra: *mut c_void = std::ptr::null_mut();
        let mut expect: Vec<u8> = Vec::new();
        for n in 0..500usize {
            let need = ca.is_null() || hdr_of_raw(ca).length + 1 > hdr_of_raw(ca).capacity;
            if need {
                ca = (l.c.arrgrowf)(ca, elemsize, 1, 0);
                ra = (l.r.arrgrowf)(ra, elemsize, 1, 0);
            }
            (*(ca as *mut ArrayHeader).sub(1)).length += 1;
            (*(ra as *mut ArrayHeader).sub(1)).length += 1;
            let payload = rng.bytes(elemsize);
            std::ptr::copy_nonoverlapping(
                payload.as_ptr(),
                (ca as *mut u8).add(n * elemsize),
                elemsize,
            );
            std::ptr::copy_nonoverlapping(
                payload.as_ptr(),
                (ra as *mut u8).add(n * elemsize),
                elemsize,
            );
            expect.extend_from_slice(&payload);

            let cb = std::slice::from_raw_parts(ca as *const u8, (n + 1) * elemsize);
            let rb = std::slice::from_raw_parts(ra as *const u8, (n + 1) * elemsize);
            assert_eq!(cb, &expect[..], "C payload at n={n}");
            assert_eq!(rb, &expect[..], "RUST payload at n={n}");
            assert_same(&format!("payload chain n={n}"), &dump(ca), &dump(ra));
        }
        (l.c.arrfreef)(ca);
        (l.r.arrfreef)(ra);
    }
}
