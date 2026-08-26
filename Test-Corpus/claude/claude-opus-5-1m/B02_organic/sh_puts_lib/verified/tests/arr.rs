//! Phase B — CONFIGS.md rows 12..15: `stbds_arrgrowf` / `stbds_arrfreef`.

mod common;
use common::*;
use std::ffi::c_void;

#[derive(Debug, PartialEq, Eq)]
struct HdrSnap {
    is_null: bool,
    same_as_input: bool,
    length: usize,
    capacity: usize,
    table_null: bool,
    temp: isize,
}

unsafe fn hdr_snap(input: *mut c_void, out: *mut c_void) -> HdrSnap {
    if out.is_null() {
        return HdrSnap {
            is_null: true,
            same_as_input: input == out,
            length: 0,
            capacity: 0,
            table_null: true,
            temp: 0,
        };
    }
    let h = header_of_raw(out);
    HdrSnap {
        is_null: false,
        same_as_input: input == out,
        length: (*h).length,
        capacity: (*h).capacity,
        table_null: (*h).hash_table.is_null(),
        temp: (*h).temp,
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn growf_fresh_matrix() {
    let (c, r) = pair();
    for &elemsize in &[1usize, 3, 4, 8, 16, 24, 32] {
        for &addlen in &[0usize, 1, 2, 7] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 17] {
                unsafe {
                    let ac = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ar = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let sc = hdr_snap(std::ptr::null_mut(), ac);
                    let sr = hdr_snap(std::ptr::null_mut(), ar);
                    assert_eq!(
                        sc, sr,
                        "arrgrowf(NULL, {elemsize}, {addlen}, {min_cap}) diverged"
                    );
                    if !ac.is_null() {
                        (c.arrfreef)(ac);
                    }
                    if !ar.is_null() {
                        (r.arrfreef)(ar);
                    }
                }
            }
        }
    }
}

/// `stbds_arrput` spelled out (macro `stbds_arrmaybegrow` + store).
unsafe fn arrput(grow: FnArrGrowf, a: &mut *mut c_void, elemsize: usize, val: &[u8]) {
    let need_grow = if a.is_null() {
        true
    } else {
        let h = header_of_raw(*a);
        (*h).length + 1 > (*h).capacity
    };
    if need_grow {
        *a = grow(*a, elemsize, 1, 0);
    }
    let h = header_of_raw(*a);
    let n = (*h).length;
    std::ptr::copy_nonoverlapping(
        val.as_ptr(),
        (*a as *mut u8).add(n * elemsize),
        elemsize.min(val.len()),
    );
    (*h).length = n + 1;
}

unsafe fn payload(a: *mut c_void, elemsize: usize) -> Vec<u8> {
    if a.is_null() {
        return vec![];
    }
    let h = header_of_raw(a);
    let n = (*h).length * elemsize;
    let mut v = vec![0u8; n];
    std::ptr::copy_nonoverlapping(a as *const u8, v.as_mut_ptr(), n);
    v
}

// ---------------------------------------------------------------- row 13
#[test]
fn growf_repeated_growth() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x4C1);
    for &elemsize in &[1usize, 4, 8, 16, 24] {
        unsafe {
            let mut ac: *mut c_void = std::ptr::null_mut();
            let mut ar: *mut c_void = std::ptr::null_mut();
            for step in 0..200usize {
                let val = rng.bytes(elemsize);
                arrput(c.arrgrowf, &mut ac, elemsize, &val);
                arrput(r.arrgrowf, &mut ar, elemsize, &val);
                assert_eq!(
                    hdr_snap(std::ptr::null_mut(), ac),
                    hdr_snap(std::ptr::null_mut(), ar),
                    "arrput step {step} elemsize {elemsize} header diverged"
                );
                assert_eq!(
                    payload(ac, elemsize),
                    payload(ar, elemsize),
                    "arrput step {step} elemsize {elemsize} payload diverged"
                );
            }
            // random explicit grows on top of the existing array
            for step in 0..200usize {
                let addlen = rng.below(9);
                let min_cap = rng.below(1024);
                let before_c = payload(ac, elemsize);
                let before_r = payload(ar, elemsize);
                assert_eq!(before_c, before_r);
                // Does the C take the `min_cap <= stbds_arrcap(a)` early return?
                // Only then is the returned pointer required to be the input:
                // when a real `realloc` happens the allocator is free to grow
                // the block in place or move it, and that decision differs
                // between two independently-allocated heaps — so pointer
                // identity may only be compared on the early-return path.
                let must_be_identity = {
                    let h = *header_of_raw(ac);
                    let min_len = h.length.wrapping_add(addlen);
                    let mc = if min_len > min_cap { min_len } else { min_cap };
                    mc <= h.capacity
                };
                let nc = (c.arrgrowf)(ac, elemsize, addlen, min_cap);
                let nr = (r.arrgrowf)(ar, elemsize, addlen, min_cap);
                if must_be_identity {
                    assert_eq!(nc, ac, "C must not reallocate (step {step})");
                    assert_eq!(nr, ar, "RUST must not reallocate (step {step})");
                }
                let sc = hdr_snap(nc, nc);
                let sr = hdr_snap(nr, nr);
                assert_eq!(
                    sc, sr,
                    "explicit grow step {step} (es={elemsize}, addlen={addlen}, min_cap={min_cap}) diverged"
                );
                ac = nc;
                ar = nr;
                assert_eq!(payload(ac, elemsize), before_c, "C lost payload");
                assert_eq!(payload(ar, elemsize), before_r, "RUST lost payload");
            }
            (c.arrfreef)(ac);
            (r.arrfreef)(ar);
        }
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn growf_noop_and_clamp() {
    let (c, r) = pair();
    let elemsize = 16usize;
    unsafe {
        let mut ac = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8);
        let mut ar = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8);
        (*header_of_raw(ac)).length = 3;
        (*header_of_raw(ar)).length = 3;

        // min_cap <= cap  -> identity return (no reallocation at all)
        for &mc in &[0usize, 1, 3, 4, 7, 8] {
            let nc = (c.arrgrowf)(ac, elemsize, 0, mc);
            let nr = (r.arrgrowf)(ar, elemsize, 0, mc);
            assert!(nc == ac, "C reallocated for min_cap={mc}");
            assert!(nr == ar, "RUST reallocated for min_cap={mc}");
            assert_eq!(hdr_snap(ac, nc), hdr_snap(ar, nr));
        }
        // addlen pushes min_len past cap -> grow, doubling floor applies
        for &(addlen, mc) in &[(6usize, 0usize), (100, 0), (0, 9), (0, 17), (2, 200), (1, 3)] {
            let nc = (c.arrgrowf)(ac, elemsize, addlen, mc);
            let nr = (r.arrgrowf)(ar, elemsize, addlen, mc);
            // pointer identity is not comparable across a real realloc
            assert_eq!(
                hdr_snap(nc, nc),
                hdr_snap(nr, nr),
                "clamp/grow addlen={addlen} min_cap={mc}"
            );
            ac = nc;
            ar = nr;
        }
        (c.arrfreef)(ac);
        (r.arrfreef)(ar);
    }

    // the `min_cap < 4 -> 4` floor: only reachable from an empty array
    unsafe {
        for &mc in &[1usize, 2, 3] {
            let nc = (c.arrgrowf)(std::ptr::null_mut(), 8, 0, mc);
            let nr = (r.arrgrowf)(std::ptr::null_mut(), 8, 0, mc);
            assert_eq!(hdr_snap(std::ptr::null_mut(), nc), hdr_snap(std::ptr::null_mut(), nr));
            assert_eq!((*header_of_raw(nc)).capacity, 4, "C floor");
            assert_eq!((*header_of_raw(nr)).capacity, 4, "RUST floor");
            (c.arrfreef)(nc);
            (r.arrfreef)(nr);
        }
    }
}

// ---------------------------------------------------------------- row 15
#[test]
fn growf_payload_roundtrip() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x5D2);
    for _ in 0..60 {
        let elemsize = 1 + rng.below(40);
        let n = rng.below(200);
        unsafe {
            let mut ac: *mut c_void = std::ptr::null_mut();
            let mut ar: *mut c_void = std::ptr::null_mut();
            for _ in 0..n {
                let val = rng.bytes(elemsize);
                arrput(c.arrgrowf, &mut ac, elemsize, &val);
                arrput(r.arrgrowf, &mut ar, elemsize, &val);
            }
            assert_eq!(
                hdr_snap(std::ptr::null_mut(), ac),
                hdr_snap(std::ptr::null_mut(), ar),
                "roundtrip es={elemsize} n={n}"
            );
            assert_eq!(payload(ac, elemsize), payload(ar, elemsize));
            if !ac.is_null() {
                (c.arrfreef)(ac);
            }
            if !ar.is_null() {
                (r.arrfreef)(ar);
            }
        }
    }
}
