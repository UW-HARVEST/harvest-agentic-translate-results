//! Phase B — CONFIGS.md rows 18..24
//! `stbds_arrgrowf` / `stbds_arrfreef`
//!
//! Note the exact C growth model (`lib.c:276-310`):
//! ```text
//! min_len = arrlen(a) + addlen
//! if (min_len > min_cap) min_cap = min_len
//! if (min_cap <= arrcap(a)) return a;            // <-- returns NULL for (NULL,_,0,0)
//! if (min_cap < 2*arrcap(a)) min_cap = 2*arrcap(a);
//! else if (min_cap < 4)      min_cap = 4;
//! capacity = min_cap
//! ```

mod common;
use common::*;
use std::ffi::c_void;

/// The exact capacity the C code will end up with, or `None` when it returns
/// the input pointer unchanged.
fn model(arrlen: usize, arrcap: usize, addlen: usize, min_cap: usize) -> Option<usize> {
    let mut mc = min_cap;
    let min_len = arrlen + addlen;
    if min_len > mc {
        mc = min_len;
    }
    if mc <= arrcap {
        return None;
    }
    if mc < 2 * arrcap {
        mc = 2 * arrcap;
    } else if mc < 4 {
        mc = 4;
    }
    Some(mc)
}

struct ArrPair {
    ac: *mut c_void,
    ar: *mut c_void,
    elemsize: usize,
    ctx: String,
    step: usize,
}

impl ArrPair {
    fn new(elemsize: usize, ctx: impl Into<String>) -> ArrPair {
        ArrPair {
            ac: std::ptr::null_mut(),
            ar: std::ptr::null_mut(),
            elemsize,
            ctx: ctx.into(),
            step: 0,
        }
    }

    unsafe fn header(&self) -> Option<(usize, usize, isize, bool)> {
        if self.ac.is_null() {
            return None;
        }
        let h = (self.ac as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
        Some((
            (*h).length,
            (*h).capacity,
            (*h).temp,
            !(*h).hash_table.is_null(),
        ))
    }

    unsafe fn len(&self) -> usize {
        self.header().map(|h| h.0).unwrap_or(0)
    }
    unsafe fn cap(&self) -> usize {
        self.header().map(|h| h.1).unwrap_or(0)
    }

    /// Calls `stbds_arrgrowf` on both.  Verifies the "returned the input
    /// pointer unchanged" decision agrees, and that it matches the C model.
    unsafe fn grow(&mut self, addlen: usize, min_cap: usize) {
        let p = pair();
        let prev_len = self.len();
        let prev_cap = self.cap();
        let oc = self.ac;
        let or = self.ar;
        self.ac = (p.c.arrgrowf)(self.ac, self.elemsize, addlen, min_cap);
        self.ar = (p.r.arrgrowf)(self.ar, self.elemsize, addlen, min_cap);
        // NOTE: when the function actually reallocates, the returned address is
        // allocator-dependent and *must not* be compared across the two
        // libraries (they have independent heap histories).  What must match is
        // the *decision* to return the input unchanged, plus all observable
        // header/payload state.
        let same = (self.ac == oc, self.ar == or);
        let expect = model(prev_len, prev_cap, addlen, min_cap);
        match expect {
            None => assert!(
                same.0 && same.1,
                "[{}] grow({},{}) from len={} cap={} should return the input unchanged \
                 (C same={}, Rust same={})",
                self.ctx,
                addlen,
                min_cap,
                prev_len,
                prev_cap,
                same.0,
                same.1
            ),
            Some(c) => {
                assert_eq!(
                    self.cap(),
                    c,
                    "[{}] grow({},{}) from len={} cap={} capacity model",
                    self.ctx,
                    addlen,
                    min_cap,
                    prev_len,
                    prev_cap
                );
            }
        }
        self.step += 1;
    }

    unsafe fn check(&self, what: &str) {
        let sc = snap_raw(self.ac, self.elemsize, KeyRepr::Raw);
        let sr = snap_raw(self.ar, self.elemsize, KeyRepr::Raw);
        assert!(
            sc == sr,
            "DIVERGENCE [{}] step {} after `{}`\n  C    = {:#?}\n  Rust = {:#?}",
            self.ctx,
            self.step,
            what,
            sc,
            sr
        );
    }

    unsafe fn set_length(&mut self, n: usize) {
        for a in [self.ac, self.ar] {
            if a.is_null() {
                continue;
            }
            let h = (a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
            (*h).length = n;
        }
    }

    /// Write identical random bytes into `[0, n)` of both arrays.
    unsafe fn fill(&mut self, n: usize, rng: &mut Rng) {
        if self.elemsize == 0 || n == 0 {
            return;
        }
        let bytes = rng.bytes(n * self.elemsize);
        for a in [self.ac, self.ar] {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), a as *mut u8, bytes.len());
        }
    }

    unsafe fn free(&mut self) {
        let p = pair();
        if !self.ac.is_null() {
            (p.c.arrfreef)(self.ac);
        }
        if !self.ar.is_null() {
            (p.r.arrfreef)(self.ar);
        }
        self.ac = std::ptr::null_mut();
        self.ar = std::ptr::null_mut();
    }
}

// -------------------------------------------------------------------- row 18
#[test]
fn c18_arrgrowf_from_null_cross_product() {
    for &elemsize in &[1usize, 2, 4, 8, 12, 16, 24, 32, 64] {
        for &addlen in &[0usize, 1, 2, 7, 100] {
            for &min_cap in &[0usize, 1, 2, 4, 5, 100] {
                unsafe {
                    let mut ap = ArrPair::new(
                        elemsize,
                        format!("null es={} addlen={} min_cap={}", elemsize, addlen, min_cap),
                    );
                    ap.grow(addlen, min_cap);
                    ap.check("grow from NULL");
                    match model(0, 0, addlen, min_cap) {
                        None => {
                            assert!(ap.ac.is_null(), "must stay NULL");
                            assert!(ap.ar.is_null(), "must stay NULL");
                        }
                        Some(_) => {
                            assert!(!ap.ac.is_null());
                            assert!(!ap.ar.is_null());
                            assert_eq!(ap.len(), 0);
                            let (_, _, temp, has_ht) = ap.header().unwrap();
                            assert_eq!(temp, 0, "fresh array temp must be 0");
                            assert!(!has_ht, "fresh array hash_table must be NULL");
                        }
                    }
                    ap.free();
                }
            }
        }
    }
}

// -------------------------------------------------------------------- row 19
#[test]
fn c19_arrgrowf_repeated_doubling() {
    let mut rng = Rng::new(0x1919);
    for &elemsize in &[1usize, 8, 16, 24] {
        unsafe {
            let mut ap = ArrPair::new(elemsize, format!("doubling es={}", elemsize));
            let mut len = 0usize;
            let mut caps = Vec::new();
            for i in 0..64 {
                ap.grow(1, 0);
                len += 1;
                ap.set_length(len);
                // initialise the brand-new element in BOTH before comparing,
                // otherwise we would be comparing malloc garbage
                let pat = rng.bytes(elemsize);
                for a in [ap.ac, ap.ar] {
                    std::ptr::copy_nonoverlapping(
                        pat.as_ptr(),
                        (a as *mut u8).add((len - 1) * elemsize),
                        elemsize,
                    );
                }
                ap.check(&format!("append {}", i));
                caps.push(ap.cap());
            }
            // the C growth schedule for addlen=1,min_cap=0 starting from NULL
            assert_eq!(&caps[0..8], &[4, 4, 4, 4, 8, 8, 8, 8]);
            assert_eq!(caps[8], 16);
            assert_eq!(caps[16], 32);
            assert_eq!(caps[32], 64);
            ap.free();
        }
    }
}

// -------------------------------------------------------------------- row 20
#[test]
fn c20_arrgrowf_min_cap_branches() {
    for &elemsize in &[1usize, 8, 16] {
        for &start in &[4usize, 8, 16, 100] {
            unsafe {
                let mut ap =
                    ArrPair::new(elemsize, format!("mincap es={} start={}", elemsize, start));
                ap.grow(0, start);
                ap.check("initial");
                let cap = ap.cap();
                assert_eq!(cap, start);
                // min_cap <= cap  -> identity return, nothing changes
                for &mc in &[0usize, 1, cap / 2, cap - 1, cap] {
                    let before = ap.ac;
                    ap.grow(0, mc);
                    assert_eq!(ap.ac, before, "min_cap={} <= cap={} must be a no-op", mc, cap);
                    ap.check("no-op grow");
                }
                // cap+1 -> the `min_cap < 2*cap` branch bumps it to 2*cap
                ap.grow(0, cap + 1);
                assert_eq!(ap.cap(), 2 * cap, "doubling branch");
                ap.check("doubled");
                // 2*cap2+1 -> exceeds doubling, min_cap wins verbatim
                let cap2 = ap.cap();
                let want = 2 * cap2 + 1;
                ap.grow(0, want);
                assert_eq!(ap.cap(), want, "explicit min_cap branch");
                ap.grow(0, 4 * want);
                assert_eq!(ap.cap(), 4 * want);
                ap.check("final");
                ap.free();
            }
        }
    }
}

// -------------------------------------------------------------------- row 21
#[test]
fn c21_arrgrowf_zero_elemsize() {
    for &min_cap in &[0usize, 1, 4, 100, 1000] {
        unsafe {
            let mut ap = ArrPair::new(0, format!("es=0 min_cap={}", min_cap));
            ap.grow(0, min_cap);
            ap.check("grow 1");
            ap.grow(3, min_cap);
            ap.check("grow 2");
            ap.grow(0, 0);
            ap.check("grow 3");
            ap.free();
        }
    }
}

// -------------------------------------------------------------------- row 22
#[test]
fn c22_arrgrowf_preserves_payload() {
    let mut rng = Rng::new(0x2222);
    for &elemsize in &[1usize, 3, 8, 16, 32] {
        unsafe {
            let mut ap = ArrPair::new(elemsize, format!("payload es={}", elemsize));
            ap.grow(0, 8);
            ap.set_length(8);
            ap.fill(8, &mut rng);
            ap.check("initial fill");
            for step in 0..12 {
                let n = ap.len();
                ap.grow(1, 0);
                // payload [0, n) must have survived the realloc identically
                ap.check(&format!("after grow {}", step));
                ap.set_length(n + 1);
                let pat = rng.bytes(elemsize);
                for a in [ap.ac, ap.ar] {
                    std::ptr::copy_nonoverlapping(
                        pat.as_ptr(),
                        (a as *mut u8).add(n * elemsize),
                        elemsize,
                    );
                }
                ap.check(&format!("append {}", step));
            }
            ap.free();
        }
    }
}

// -------------------------------------------------------------------- row 23
#[test]
fn c23_arrgrowf_randomized_sequences() {
    let mut rng = Rng::new(0x2323);
    for seq in 0..256 {
        let elemsize = *rng.pick(&[1usize, 2, 4, 8, 12, 16, 24, 32]);
        unsafe {
            let mut ap = ArrPair::new(elemsize, format!("seq {} es={}", seq, elemsize));
            let mut len = 0usize;
            for _ in 0..24 {
                let addlen = *rng.pick(&[0usize, 1, 2, 3, 8, 17]);
                let min_cap = *rng.pick(&[0usize, 1, 2, 4, 9, 33]);
                ap.grow(addlen, min_cap);
                let cap = ap.cap();
                len = (len + addlen).min(cap);
                ap.set_length(len);
                ap.fill(len, &mut rng);
                ap.check("randomized step");
            }
            ap.free();
        }
    }
}

// -------------------------------------------------------------------- row 24
#[test]
fn c24_arrgrowf_addlen_dominates() {
    let mut rng = Rng::new(0x2424);
    for &elemsize in &[1usize, 8, 16] {
        unsafe {
            let mut ap = ArrPair::new(elemsize, format!("addlen-dom es={}", elemsize));
            ap.grow(1000, 0);
            assert_eq!(ap.cap(), 1000);
            ap.set_length(1000);
            ap.fill(1000, &mut rng);
            ap.check("length=1000");
            // min_len = 1000+1 > min_cap 0 -> min_cap = 1001, but 1001 < 2*1000
            // so it doubles to 2000
            ap.grow(1, 0);
            assert_eq!(ap.cap(), 2000);
            ap.check("doubled to 2000");
            // huge addlen with min_cap 0
            ap.grow(100_000, 0);
            assert_eq!(ap.cap(), 101_000);
            ap.check("huge");
            ap.free();
        }
    }
}
