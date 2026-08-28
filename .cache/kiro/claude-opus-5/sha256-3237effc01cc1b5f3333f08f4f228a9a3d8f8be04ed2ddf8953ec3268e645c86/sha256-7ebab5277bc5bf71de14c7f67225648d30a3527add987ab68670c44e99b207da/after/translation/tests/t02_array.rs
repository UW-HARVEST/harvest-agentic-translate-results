//! Level 2: the dynamic-array primitives `stbds_arrgrowf` / `stbds_arrfreef`.

mod common;

use common::*;
use std::ffi::c_void;

/// Snapshot of a *raw* array pointer: header plus `length*elemsize` payload bytes.
unsafe fn snap_arr(a: *mut c_void, elemsize: usize) -> Vec<u8> {
    unsafe {
        let mut out = Vec::new();
        if a.is_null() {
            out.push(0);
            return out;
        }
        out.push(1);
        snapshot_header(&mut out, a);
        let len = header(a).length;
        out.extend_from_slice(std::slice::from_raw_parts(a as *const u8, len * elemsize));
        out
    }
}

#[test]
fn arrgrowf_from_null() {
    let p = load_pair();
    for &elemsize in &[1usize, 2, 4, 8, 12, 16, 24, 64, 100] {
        for &addlen in &[0usize, 1, 2, 3, 4, 5, 7, 8, 100, 1000] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 8, 16, 999] {
                let ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                let ra = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                // NOTE: with addlen==0 && min_cap==0 the C code short-circuits
                // on `min_cap <= stbds_arrcap(a)` and hands back NULL.
                assert_eq!(
                    ca.is_null(),
                    ra.is_null(),
                    "arrgrowf(NULL, {elemsize}, {addlen}, {min_cap}) null-ness differs"
                );
                let cs = unsafe { snap_arr(ca, elemsize) };
                let rs = unsafe { snap_arr(ra, elemsize) };
                assert_bytes_eq(
                    &format!("arrgrowf(NULL, {elemsize}, {addlen}, {min_cap})"),
                    &cs,
                    &rs,
                );
                if !ca.is_null() {
                    unsafe {
                        (p.c.arrfreef)(ca);
                        (p.r.arrfreef)(ra);
                    }
                }
            }
        }
    }
}

/// Replicates `stbds_arrput` on an `i32` array using only exported symbols.
struct Arr {
    a: *mut c_void,
}

impl Arr {
    fn new() -> Arr {
        Arr {
            a: std::ptr::null_mut(),
        }
    }
    unsafe fn push(&mut self, grow: FnGrow, v: i32) {
        unsafe {
            // stbds_arrmaybegrow(a,1)
            let need = if self.a.is_null() {
                true
            } else {
                let h = header(self.a);
                h.length + 1 > h.capacity
            };
            if need {
                self.a = grow(self.a, 4, 1, 0);
            }
            let h = (self.a as *mut u8).sub(HDR) as *mut ArrayHeader;
            *((self.a as *mut i32).add((*h).length)) = v;
            (*h).length += 1;
        }
    }
}

type FnGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;

#[test]
fn arrput_growth_sequence() {
    let p = load_pair();
    let mut ca = Arr::new();
    let mut ra = Arr::new();
    for i in 0..5000i32 {
        unsafe {
            ca.push(p.c.arrgrowf, i.wrapping_mul(7));
            ra.push(p.r.arrgrowf, i.wrapping_mul(7));
        }
        let cs = unsafe { snap_arr(ca.a, 4) };
        let rs = unsafe { snap_arr(ra.a, 4) };
        assert_bytes_eq(&format!("arrput sequence at i={i}"), &cs, &rs);
    }
    unsafe {
        (p.c.arrfreef)(ca.a);
        (p.r.arrfreef)(ra.a);
    }
}

#[test]
fn arrsetcap_then_grow() {
    let p = load_pair();
    for &start in &[0usize, 1, 3, 4, 5, 9, 17, 33, 1000] {
        let mut ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), 8, 0, start) };
        let mut ra = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), 8, 0, start) };
        assert_bytes_eq(
            &format!("arrsetcap({start})"),
            &unsafe { snap_arr(ca, 8) },
            &unsafe { snap_arr(ra, 8) },
        );
        for &(addlen, min_cap) in &[
            (1usize, 0usize),
            (0, 0),
            (1, 0),
            (3, 0),
            (0, 2),
            (8, 0),
            (1, 1000),
            (1, 0),
        ] {
            ca = unsafe { (p.c.arrgrowf)(ca, 8, addlen, min_cap) };
            ra = unsafe { (p.r.arrgrowf)(ra, 8, addlen, min_cap) };
            assert_bytes_eq(
                &format!("arrgrowf(start={start}, addlen={addlen}, min_cap={min_cap})"),
                &unsafe { snap_arr(ca, 8) },
                &unsafe { snap_arr(ra, 8) },
            );
        }
        unsafe {
            (p.c.arrfreef)(ca);
            (p.r.arrfreef)(ra);
        }
    }
}

#[test]
fn arrgrowf_no_op_when_capacity_suffices() {
    let p = load_pair();
    let ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), 4, 0, 64) };
    let ra = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), 4, 0, 64) };
    // must return the same pointer untouched
    let ca2 = unsafe { (p.c.arrgrowf)(ca, 4, 0, 10) };
    let ra2 = unsafe { (p.r.arrgrowf)(ra, 4, 0, 10) };
    assert_eq!(ca, ca2, "C returned a new pointer");
    assert_eq!(ra, ra2, "Rust returned a new pointer");
    assert_bytes_eq(
        "arrgrowf no-op",
        &unsafe { snap_arr(ca2, 4) },
        &unsafe { snap_arr(ra2, 4) },
    );
    unsafe {
        (p.c.arrfreef)(ca2);
        (p.r.arrfreef)(ra2);
    }
}
