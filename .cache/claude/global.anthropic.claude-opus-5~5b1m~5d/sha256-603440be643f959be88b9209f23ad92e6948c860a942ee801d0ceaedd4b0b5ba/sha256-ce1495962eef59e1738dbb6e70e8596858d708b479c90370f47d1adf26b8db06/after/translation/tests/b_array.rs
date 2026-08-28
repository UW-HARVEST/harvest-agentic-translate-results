//! Phase B — CONFIGS.md rows C14..C23: `stbds_arrgrowf` / `stbds_arrfreef`,
//! the lowest level of the library (everything else is built on it).

mod common;
use common::*;
use std::ffi::c_void;

/// Observable state of a raw array: the header plus `n` bytes of payload.
unsafe fn snap_arr(a: *mut c_void, payload: usize) -> String {
    if a.is_null() {
        return "ARR=NULL".into();
    }
    unsafe { format!("{} data={}", snap_hdr(a), hex(a as *const u8, payload)) }
}

/// Fill `n` bytes of the array with a deterministic pattern.
unsafe fn fill(a: *mut c_void, n: usize, tag: u8) {
    unsafe {
        let p = a as *mut u8;
        for i in 0..n {
            *p.add(i) = tag.wrapping_add((i as u8).wrapping_mul(31));
        }
    }
}

// ---------------------------------------------------------------------------
// C14 — a==NULL, addlen==0, min_cap==0  =>  returns NULL, no allocation (R2)
// ---------------------------------------------------------------------------
#[test]
fn c14_null_zero_zero_returns_null() {
    let _g = lock();
    let (c, r) = both();
    for &es in &[0usize, 1, 4, 8, 16, 24, 64, 4096] {
        unsafe {
            let cp = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let rp = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(cp.is_null(), "C arrgrowf(NULL,{es},0,0) should be NULL");
            assert!(rp.is_null(), "Rust arrgrowf(NULL,{es},0,0) should be NULL");
        }
    }
}

// ---------------------------------------------------------------------------
// C15 — min_cap 1..3 gets bumped to 4;   C16 — min_cap >= 4 used verbatim
// C17 — addlen > min_cap  =>  min_cap = min_len
// C23 — elemsize == 0
// ---------------------------------------------------------------------------
#[test]
fn c15_c16_c17_c23_fresh_capacity_rules() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xA15_0001);

    let mut cases: Vec<(usize, usize, usize)> = Vec::new(); // (elemsize, addlen, min_cap)
    for &es in &[0usize, 1, 2, 4, 8, 16, 20, 24, 32, 64] {
        for &addlen in &[0usize, 1, 2, 3, 4, 5, 7, 8, 17, 100] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 6, 7, 8, 16, 17, 1000] {
                cases.push((es, addlen, min_cap));
            }
        }
    }
    for _ in 0..500 {
        cases.push((
            (rng.below(9) * 8) as usize,
            rng.below(64) as usize,
            rng.below(256) as usize,
        ));
    }

    for (es, addlen, min_cap) in cases {
        unsafe {
            let cp = (c.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
            let rp = (r.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
            let ctx = format!("arrgrowf(NULL, es={es}, addlen={addlen}, min_cap={min_cap})");
            assert_eq!(cp.is_null(), rp.is_null(), "{ctx}: nullness");
            if cp.is_null() {
                continue;
            }
            eqs(&ctx, &snap_hdr(cp), &snap_hdr(rp));

            // and the documented rule: cap = max(min_len, min_cap) then bumped to 4
            let want = {
                let min_len = addlen;
                let mut mc = if min_len > min_cap { min_len } else { min_cap };
                if mc < 4 {
                    mc = 4;
                }
                mc
            };
            assert_eq!((*header(cp)).capacity, want, "{ctx}: C capacity");
            assert_eq!((*header(rp)).capacity, want, "{ctx}: Rust capacity");
            assert_eq!((*header(cp)).length, 0);
            assert_eq!((*header(cp)).temp, 0);
            assert!((*header(cp)).hash_table.is_null());

            (c.arrfreef)(cp);
            (r.arrfreef)(rp);
        }
    }
}

// ---------------------------------------------------------------------------
// C18 — min_cap <= cap: early-out, identical pointer, header untouched (R1)
// ---------------------------------------------------------------------------
#[test]
fn c18_early_out_returns_same_pointer() {
    let _g = lock();
    let (c, r) = both();
    for &es in &[1usize, 8, 16, 24] {
        unsafe {
            let mut ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 16);
            let mut ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 16);
            (*header(ca)).length = 5;
            (*header(ra)).length = 5;
            fill(ca, es * 16, 0x11);
            fill(ra, es * 16, 0x11);

            // min_len = 5 + addlen; early-out requires max(min_len,min_cap) <= 16
            for &(addlen, min_cap) in &[(0usize, 0usize), (0, 16), (1, 0), (11, 16), (0, 15)] {
                let cbefore = snap_arr(ca, es * 16);
                let cp = (c.arrgrowf)(ca, es, addlen, min_cap);
                let rp = (r.arrgrowf)(ra, es, addlen, min_cap);
                assert_eq!(cp, ca, "C should early-out for ({addlen},{min_cap})");
                assert_eq!(rp, ra, "Rust should early-out for ({addlen},{min_cap})");
                eqs(
                    &format!("early-out es={es} addlen={addlen} min_cap={min_cap}"),
                    &cbefore,
                    &snap_arr(rp, es * 16),
                );
                ca = cp;
                ra = rp;
            }
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}

// ---------------------------------------------------------------------------
// C19 — min_cap < 2*cap  =>  doubling wins
// C20 — min_cap >= 2*cap =>  min_cap wins
// ---------------------------------------------------------------------------
#[test]
fn c19_c20_growth_rules_on_existing_array() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xA19_0002);

    for &es in &[1usize, 4, 8, 16, 20, 24, 32] {
        for &cap0 in &[4usize, 5, 8, 13, 16, 64] {
            for &len0 in &[0usize, 1, 3] {
                for &(addlen, min_cap) in &[
                    (1usize, 0usize),
                    (0, cap0 + 1),
                    (0, 2 * cap0),
                    (0, 2 * cap0 - 1),
                    (0, 2 * cap0 + 1),
                    (0, 4 * cap0),
                    (cap0, 0),
                    (3 * cap0, 0),
                ] {
                    if len0 > cap0 {
                        continue;
                    }
                    unsafe {
                        let ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, cap0);
                        let ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, cap0);
                        let realcap = (*header(ca)).capacity;
                        (*header(ca)).length = len0;
                        (*header(ra)).length = len0;
                        let tag = rng.byte();
                        fill(ca, es * realcap, tag);
                        fill(ra, es * realcap, tag);

                        let cp = (c.arrgrowf)(ca, es, addlen, min_cap);
                        let rp = (r.arrgrowf)(ra, es, addlen, min_cap);
                        let ctx = format!(
                            "es={es} cap0={realcap} len0={len0} addlen={addlen} min_cap={min_cap}"
                        );
                        // old payload must survive the realloc
                        eqs(&ctx, &snap_arr(cp, es * realcap), &snap_arr(rp, es * realcap));

                        // documented rule
                        let want = {
                            let min_len = len0.wrapping_add(addlen);
                            let mut mc = if min_len > min_cap { min_len } else { min_cap };
                            if mc <= realcap {
                                realcap
                            } else {
                                if mc < 2 * realcap {
                                    mc = 2 * realcap;
                                } else if mc < 4 {
                                    mc = 4;
                                }
                                mc
                            }
                        };
                        assert_eq!((*header(cp)).capacity, want, "{ctx}: C capacity");
                        assert_eq!((*header(rp)).capacity, want, "{ctx}: Rust capacity");

                        (c.arrfreef)(cp);
                        (r.arrfreef)(rp);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C21 — the realistic `arrput` growth chain, one element at a time
// C22 — grow-then-free round trip
// ---------------------------------------------------------------------------
#[test]
fn c21_c22_append_one_at_a_time() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xA21_0003);

    for &es in &[1usize, 2, 4, 8, 16, 20, 24, 32, 64] {
        unsafe {
            let mut ca: *mut c_void = std::ptr::null_mut();
            let mut ra: *mut c_void = std::ptr::null_mut();
            for n in 0..70usize {
                // stbds_arrmaybegrow(a,1)
                let need = |a: *mut c_void| -> bool {
                    a.is_null() || (*header(a)).length + 1 > (*header(a)).capacity
                };
                if need(ca) {
                    ca = (c.arrgrowf)(ca, es, 1, 0);
                }
                if need(ra) {
                    ra = (r.arrgrowf)(ra, es, 1, 0);
                }
                // (a)[length++] = v
                let tag = rng.byte();
                for k in 0..es {
                    *(ca as *mut u8).add(es * n + k) = tag.wrapping_add(k as u8);
                    *(ra as *mut u8).add(es * n + k) = tag.wrapping_add(k as u8);
                }
                (*header(ca)).length += 1;
                (*header(ra)).length += 1;

                eqs(
                    &format!("append es={es} n={n}"),
                    &snap_arr(ca, es * (n + 1)),
                    &snap_arr(ra, es * (n + 1)),
                );
            }
            // capacity ladder must be identical and monotone
            assert_eq!((*header(ca)).capacity, (*header(ra)).capacity);
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}

/// Randomized grow/shrink walk over a long chain of `arrgrowf` calls.
#[test]
fn c21_randomized_growth_walks() {
    let _g = lock();
    let (c, r) = both();
    for trial in 0..200u64 {
        let mut rng = Rng::new(0xA21_5000 + trial);
        let es = [1usize, 4, 8, 16, 24, 32][(trial % 6) as usize];
        unsafe {
            let mut ca: *mut c_void = std::ptr::null_mut();
            let mut ra: *mut c_void = std::ptr::null_mut();
            for step in 0..30 {
                let addlen = rng.below(10) as usize;
                let min_cap = rng.below(40) as usize;
                let cp = (c.arrgrowf)(ca, es, addlen, min_cap);
                let rp = (r.arrgrowf)(ra, es, addlen, min_cap);
                assert_eq!(cp.is_null(), rp.is_null(), "trial {trial} step {step}");
                ca = cp;
                ra = rp;
                if ca.is_null() {
                    continue;
                }
                // grow the length the way arraddn would
                let cap = (*header(ca)).capacity;
                let newlen = ((*header(ca)).length + addlen).min(cap);
                (*header(ca)).length = newlen;
                (*header(ra)).length = newlen;
                let tag = rng.byte();
                fill(ca, es * newlen, tag);
                fill(ra, es * newlen, tag);
                eqs(
                    &format!("walk trial={trial} step={step} es={es}"),
                    &snap_arr(ca, es * newlen),
                    &snap_arr(ra, es * newlen),
                );
            }
            if !ca.is_null() {
                (c.arrfreef)(ca);
                (r.arrfreef)(ra);
            }
        }
    }
}
