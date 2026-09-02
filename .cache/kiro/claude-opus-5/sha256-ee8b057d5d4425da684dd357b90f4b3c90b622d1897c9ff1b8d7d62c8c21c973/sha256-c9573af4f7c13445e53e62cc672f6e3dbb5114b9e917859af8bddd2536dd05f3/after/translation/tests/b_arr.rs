//! Phase B — CONFIGS.md rows 16..23: `stbds_arrgrowf` / `stbds_arrfreef`,
//! the lowest-level entry point of the whole library.

mod common;
use common::*;
use std::ffi::c_void;

/// Mirrored plain dynamic array driven through the raw `arrgrowf` export.
struct Arr<'a> {
    l: &'a Pair,
    c: *mut c_void,
    r: *mut c_void,
    elemsize: usize,
}

impl<'a> Arr<'a> {
    fn new(l: &'a Pair, elemsize: usize) -> Arr<'a> {
        Arr {
            l,
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            elemsize,
        }
    }

    /// Raw `arrgrowf` call on both libraries; returns whether each returned the
    /// *same pointer it was given* (the "growth not needed" no-op path).
    unsafe fn grow(&mut self, addlen: usize, min_cap: usize) -> (bool, bool) {
        let (oc, or) = (self.c, self.r);
        self.c = (self.l.c.arrgrowf)(self.c, self.elemsize, addlen, min_cap);
        self.r = (self.l.r.arrgrowf)(self.r, self.elemsize, addlen, min_cap);
        (self.c == oc, self.r == or)
    }

    unsafe fn set_length(&mut self, n: usize) {
        for p in [self.c, self.r] {
            let h = (p as *mut u8).wrapping_sub(HEADER_SIZE) as *mut Header;
            (*h).length = n;
        }
    }

    /// Fill `[0, n*elemsize)` with a deterministic pattern on both sides.
    unsafe fn fill(&mut self, n: usize, tag: u8) {
        for p in [self.c, self.r] {
            let b = p as *mut u8;
            for i in 0..(n * self.elemsize) {
                *b.wrapping_add(i) = (i as u8).wrapping_mul(31).wrapping_add(tag);
            }
        }
    }

    /// `stbds_arrput` for an `elemsize`-byte element.
    unsafe fn put(&mut self, val: &[u8]) {
        assert_eq!(val.len(), self.elemsize);
        // stbds_arrmaybegrow(a, 1)
        let need = self.c.is_null() || {
            let h = header((self.c as *mut u8) as *mut c_void);
            h.length + 1 > h.capacity
        };
        if need {
            self.grow(1, 0);
        }
        for p in [self.c, self.r] {
            let h = (p as *mut u8).wrapping_sub(HEADER_SIZE) as *mut Header;
            let dst = (p as *mut u8).wrapping_add((*h).length * self.elemsize);
            std::ptr::copy_nonoverlapping(val.as_ptr(), dst, self.elemsize);
            (*h).length += 1;
        }
    }

    unsafe fn assert_eq(&self, what: &str) {
        assert_eq!(
            self.c.is_null(),
            self.r.is_null(),
            "[{what}] nullness: C={:?} Rust={:?}",
            self.c,
            self.r
        );
        if self.c.is_null() {
            return;
        }
        let hc = header(self.c);
        let hr = header(self.r);
        assert_eq!(hc.length, hr.length, "[{what}] length");
        assert_eq!(hc.capacity, hr.capacity, "[{what}] capacity");
        assert_eq!(hc.temp, hr.temp, "[{what}] temp");
        assert_eq!(
            hc.hash_table.is_null(),
            hr.hash_table.is_null(),
            "[{what}] hash_table nullness"
        );
        let n = hc.length * self.elemsize;
        let a = std::slice::from_raw_parts(self.c as *const u8, n);
        let b = std::slice::from_raw_parts(self.r as *const u8, n);
        assert_eq!(a, b, "[{what}] payload");
    }

    unsafe fn free(&mut self) {
        if !self.c.is_null() {
            (self.l.c.arrfreef)(self.c);
            (self.l.r.arrfreef)(self.r);
        }
        self.c = std::ptr::null_mut();
        self.r = std::ptr::null_mut();
    }
}

const ELEMSIZES: [usize; 6] = [1, 2, 4, 8, 16, 24];

/// row 16: `a == NULL`, `addlen == 0`, every interesting `min_cap`.
/// Covers ERRORS.md rows 3 and 5 (the `< 4` clamp and the zero-init).
#[test]
fn row_16_grow_from_null_min_cap() {
    let (l, _g) = libs();
    for &es in ELEMSIZES.iter() {
        for &mc in [0usize, 1, 2, 3, 4, 5, 7, 8, 17, 64, 1000].iter() {
            let mut a = Arr::new(l, es);
            let (same_c, same_r) = unsafe { a.grow(0, mc) };
            assert_eq!(same_c, same_r, "es={es} mc={mc} no-op parity");
            unsafe { a.assert_eq(&format!("null grow es={es} mc={mc}")) };
            if mc == 0 {
                // min_len == 0 <= arrcap(NULL) == 0 -> returns NULL unchanged
                assert!(a.c.is_null(), "es={es}: arrgrowf(NULL,e,0,0) must stay NULL");
            } else {
                let h = unsafe { header(a.c) };
                assert_eq!(h.capacity, mc.max(4), "es={es} mc={mc} capacity clamp");
                assert_eq!(h.length, 0);
                assert_eq!(h.temp, 0);
            }
            unsafe { a.free() };
        }
    }
}

/// row 17: `a == NULL`, growth driven by `addlen` with `min_cap == 0`.
#[test]
fn row_17_grow_from_null_addlen() {
    let (l, _g) = libs();
    for &es in ELEMSIZES.iter() {
        for &n in [0usize, 1, 2, 3, 4, 7, 64, 999].iter() {
            let mut a = Arr::new(l, es);
            unsafe { a.grow(n, 0) };
            unsafe { a.assert_eq(&format!("null addlen es={es} n={n}")) };
            if n == 0 {
                assert!(a.c.is_null());
            } else {
                assert_eq!(unsafe { header(a.c) }.capacity, n.max(4));
            }
            unsafe { a.free() };
        }
    }
}

/// row 18: non-null `a`, `min_cap <= cap` -> exact no-op, pointer identity kept.
/// (ERRORS.md row 1.)
#[test]
fn row_18_grow_noop() {
    let (l, _g) = libs();
    for &es in ELEMSIZES.iter() {
        let mut a = Arr::new(l, es);
        unsafe {
            a.grow(0, 10);
            let cap = header(a.c).capacity;
            a.fill(cap, 0x11);
            a.set_length(cap);
            let before_c = header(a.c);
            for mc in 0..=cap {
                let (sc, sr) = a.grow(0, mc);
                assert!(sc, "C must return the same pointer for min_cap={mc} cap={cap}");
                assert!(sr, "Rust must return the same pointer for min_cap={mc} cap={cap}");
                a.assert_eq(&format!("noop es={es} mc={mc}"));
            }
            let after = header(a.c);
            assert_eq!(before_c.capacity, after.capacity);
            assert_eq!(before_c.length, after.length);
            a.free();
        }
    }
}

/// rows 19-20: doubling path vs exact-`min_cap` path on a non-null array.
#[test]
fn row_19_20_grow_doubling_vs_exact() {
    let (l, _g) = libs();
    for &es in ELEMSIZES.iter() {
        for &start in [1usize, 4, 5, 16, 100].iter() {
            for &mc in [0usize, 1, 5, 17, 32, 199, 201, 4096].iter() {
                let mut a = Arr::new(l, es);
                unsafe {
                    a.grow(0, start);
                    let cap0 = header(a.c).capacity;
                    a.fill(cap0, 0x22);
                    a.set_length(cap0);
                    a.grow(0, mc);
                    a.assert_eq(&format!("es={es} start={start} mc={mc}"));
                    let cap1 = header(a.c).capacity;
                    // Mirror the C decision tree exactly.
                    let min_len = cap0; // length + addlen(0)
                    let want_mc = mc.max(min_len);
                    let expect = if want_mc <= cap0 {
                        cap0
                    } else if want_mc < 2 * cap0 {
                        2 * cap0
                    } else if want_mc < 4 {
                        4
                    } else {
                        want_mc
                    };
                    assert_eq!(cap1, expect, "es={es} start={start} mc={mc}");
                    a.free();
                }
            }
        }
    }
}

/// row 21: growth driven by `addlen` when `min_len > min_cap`.
#[test]
fn row_21_grow_addlen_dominates() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xA_0021);
    for &es in ELEMSIZES.iter() {
        for _ in 0..60 {
            let start = rng.below(64) + 1;
            let addlen = rng.below(300);
            let mc = rng.below(8);
            let mut a = Arr::new(l, es);
            unsafe {
                a.grow(0, start);
                let cap0 = header(a.c).capacity;
                a.fill(cap0, 0x33);
                a.set_length(cap0);
                a.grow(addlen, mc);
                a.assert_eq(&format!("es={es} start={start} addlen={addlen} mc={mc}"));
                a.free();
            }
        }
    }
}

/// row 22: long randomized grow chain via the `arrput` idiom — the composed
/// pipeline, not a single call.
#[test]
fn row_22_grow_chain_randomized() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xA_0022);
    for &es in ELEMSIZES.iter() {
        for round in 0..6 {
            let mut a = Arr::new(l, es);
            unsafe {
                for step in 0..120 {
                    match rng.below(4) {
                        0 => {
                            let addlen = rng.below(9);
                            let mc = rng.below(40);
                            a.grow(addlen, mc);
                            if !a.c.is_null() {
                                let cap = header(a.c).capacity;
                                let len = header(a.c).length;
                                // initialise the freshly exposed tail so the
                                // payload comparison stays meaningful
                                a.fill(cap, (step as u8).wrapping_add(round as u8));
                                let _ = len;
                            }
                        }
                        _ => {
                            let v = rng.bytes(es);
                            a.put(&v);
                        }
                    }
                    a.assert_eq(&format!("chain es={es} round={round} step={step}"));
                }
                a.free();
            }
        }
    }
}

/// row 2 of ERRORS.md: `arrlen(a) + addlen` wrapping `size_t`.
/// The C does no overflow check; both sides must wrap identically and then
/// take the `min_cap <= arrcap` early-out (because the wrapped value is small).
#[test]
fn err_row_2_addlen_wraparound() {
    let (l, _g) = libs();
    let es = 4usize;
    let mut a = Arr::new(l, es);
    unsafe {
        a.grow(0, 8);
        let cap = header(a.c).capacity;
        a.fill(cap, 0x44);
        a.set_length(4);
        // length(4) + addlen == 0 (mod 2^64) -> min_len 0, min_cap 0 -> no-op
        let addlen = usize::MAX - 3;
        let (sc, sr) = a.grow(addlen, 0);
        assert_eq!(sc, sr, "wraparound no-op parity");
        assert!(sc, "wrapped min_len must take the no-op path");
        a.assert_eq("addlen wraparound");
        a.free();
    }
}

/// row 23: grow then `arrfreef` (non-null), repeatedly — must not diverge or
/// corrupt the allocator on either side.
#[test]
fn row_23_grow_then_free() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xA_0023);
    for _ in 0..300 {
        let es = ELEMSIZES[rng.below(ELEMSIZES.len())];
        let mut a = Arr::new(l, es);
        unsafe {
            a.grow(rng.below(20) + 1, rng.below(20));
            let cap = header(a.c).capacity;
            a.fill(cap, 0x55);
            a.set_length(cap);
            a.assert_eq("grow before free");
            a.free();
        }
    }
}
