//! Phase B — CONFIGS.md rows 16..22: `stbds_arrgrowf` / `stbds_arrfreef`.
mod common;

use common::*;
use core::ffi::c_void;

/// Everything observable about a raw array: its header. The *pointer value*
/// itself is deliberately NOT compared: `realloc` may or may not return the
/// same block, and the two libraries have independent heaps, so pointer
/// identity/movement is allocator noise rather than translated behaviour.
#[derive(Debug, PartialEq, Eq, Clone)]
struct ArrSnap {
    null: bool,
    length: usize,
    capacity: usize,
    has_table: bool,
    temp: isize,
}

unsafe fn arr_snap(a: *mut c_void) -> ArrSnap {
    if a.is_null() {
        return ArrSnap {
            null: true,
            length: 0,
            capacity: 0,
            has_table: false,
            temp: 0,
        };
    }
    let h = hdr_of_arr(a);
    ArrSnap {
        null: false,
        length: (*h).length,
        capacity: (*h).capacity,
        has_table: !(*h).hash_table.is_null(),
        temp: (*h).temp,
    }
}

/// Compare `n` payload bytes, reporting the first divergent offset compactly.
#[track_caller]
unsafe fn cmp_payload(label: &str, step: usize, ac: *mut c_void, ar: *mut c_void, n: usize) {
    if n == 0 || ac.is_null() || ar.is_null() {
        return;
    }
    let a = core::slice::from_raw_parts(ac as *const u8, n);
    let b = core::slice::from_raw_parts(ar as *const u8, n);
    if let Some(i) = (0..n).find(|&i| a[i] != b[i]) {
        panic!(
            "[{label}] step {step}: payload byte {i} of {n} diverged: C={:#04x} Rust={:#04x}",
            a[i], b[i]
        );
    }
}

struct ArrPair<'l> {
    c: &'l Lib,
    r: &'l Lib,
    ac: *mut c_void,
    ar: *mut c_void,
    elemsize: usize,
    label: String,
    step: usize,
}

impl<'l> ArrPair<'l> {
    fn new(c: &'l Lib, r: &'l Lib, elemsize: usize, label: impl Into<String>) -> ArrPair<'l> {
        ArrPair {
            c,
            r,
            ac: core::ptr::null_mut(),
            ar: core::ptr::null_mut(),
            elemsize,
            label: label.into(),
            step: 0,
        }
    }

    /// `stbds_arrgrow(a,addlen,min_cap)`; `payload_len` bytes of element data
    /// are also compared (0 when the array holds nothing yet).
    unsafe fn grow(&mut self, addlen: usize, min_cap: usize, payload_len: usize) {
        let bc = self.ac;
        let br = self.ar;
        self.ac = (self.c.arrgrowf)(self.ac, self.elemsize, addlen, min_cap);
        self.ar = (self.r.arrgrowf)(self.ar, self.elemsize, addlen, min_cap);
        let _ = (bc, br);
        let sc = arr_snap(self.ac);
        let sr = arr_snap(self.ar);
        assert_eq!(
            sc, sr,
            "[{}] step {}: arrgrowf(elemsize={}, addlen={addlen}, min_cap={min_cap}) header diverged",
            self.label, self.step, self.elemsize
        );
        let pl = if self.ac.is_null() { 0 } else { payload_len };
        cmp_payload(&self.label, self.step, self.ac, self.ar, pl);
        self.step += 1;
    }

    unsafe fn set_length(&mut self, n: usize) {
        if !self.ac.is_null() {
            (*hdr_of_arr(self.ac)).length = n;
        }
        if !self.ar.is_null() {
            (*hdr_of_arr(self.ar)).length = n;
        }
    }

    unsafe fn fill(&mut self, n: usize, tag: u64) {
        let mut rc = Rng::new(tag);
        let mut rr = Rng::new(tag);
        for i in 0..n {
            *(self.ac as *mut u8).add(i) = rc.next_u8();
            *(self.ar as *mut u8).add(i) = rr.next_u8();
        }
    }

    unsafe fn free(&mut self) {
        if !self.ac.is_null() {
            (self.c.arrfreef)(self.ac);
        }
        if !self.ar.is_null() {
            (self.r.arrfreef)(self.ar);
        }
        self.ac = core::ptr::null_mut();
        self.ar = core::ptr::null_mut();
    }
}

const ELEMSIZES: &[usize] = &[1, 2, 3, 4, 8, 16, 17, 64];

/// Row 16 — `a == NULL`, `addlen == 0`, every interesting `min_cap`.
#[test]
fn cfg16_arrgrowf_null_min_caps() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for &elemsize in ELEMSIZES {
            for min_cap in [0usize, 1, 2, 3, 4, 5, 7, 8, 9, 100, 4096] {
                let mut p = ArrPair::new(&c, &r, elemsize, format!("es{elemsize}/mc{min_cap}"));
                p.grow(0, min_cap, 0);
                if min_cap == 0 {
                    // row 1 of ERRORS.md: no allocation at all
                    assert!(p.ac.is_null() && p.ar.is_null());
                } else {
                    let cap = (*hdr_of_arr(p.ac)).capacity;
                    assert_eq!(cap, min_cap.max(4), "capacity clamp for min_cap={min_cap}");
                }
                p.free();
            }
        }
    }
}

/// Row 17 — `a == NULL`, `min_cap == 0`, `addlen` drives `min_len`.
#[test]
fn cfg17_arrgrowf_null_addlen() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x17171717);
    unsafe {
        for &elemsize in &[1usize, 4, 16, 64] {
            for addlen in [1usize, 2, 3, 4, 5, 8, 100, 2048] {
                let mut p = ArrPair::new(&c, &r, elemsize, format!("es{elemsize}/al{addlen}"));
                p.grow(addlen, 0, 0);
                assert_eq!((*hdr_of_arr(p.ac)).capacity, addlen.max(4));
                p.free();
            }
            for _ in 0..32 {
                let addlen = rng.range(1, 4096);
                let mut p = ArrPair::new(&c, &r, elemsize, "rand");
                p.grow(addlen, 0, 0);
                p.free();
            }
        }
    }
}

/// Row 18 — early-out when `min_cap <= capacity`: pointer and header must be
/// bit-identical to before.
#[test]
fn cfg18_arrgrowf_early_out() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x18181818);
    unsafe {
        for &elemsize in ELEMSIZES {
            let mut p = ArrPair::new(&c, &r, elemsize, format!("early/es{elemsize}"));
            p.grow(0, 8, 0); // capacity 8
            p.fill(8 * elemsize, 0xEE);
            let cc = p.ac;
            let cr = p.ar;
            for min_cap in [0usize, 1, 4, 7, 8] {
                p.grow(0, min_cap, 8 * elemsize);
                assert_eq!(p.ac, cc, "C array must not move");
                assert_eq!(p.ar, cr, "Rust array must not move");
            }
            // random early-outs
            for _ in 0..16 {
                let mc = rng.below(9);
                p.grow(0, mc, 8 * elemsize);
            }
            p.free();
        }
    }
}

/// Row 19 — repeated doubling (`min_cap == cap + 1`).
#[test]
fn cfg19_arrgrowf_doubling() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for &elemsize in &[1usize, 8, 16, 64] {
            let mut p = ArrPair::new(&c, &r, elemsize, format!("dbl/es{elemsize}"));
            p.grow(0, 1, 0);
            let mut expect = 4usize;
            assert_eq!((*hdr_of_arr(p.ac)).capacity, expect);
            for _ in 0..12 {
                let cap = (*hdr_of_arr(p.ac)).capacity;
                p.fill(cap * elemsize, cap as u64);
                p.grow(0, cap + 1, cap * elemsize);
                expect *= 2;
                assert_eq!(
                    (*hdr_of_arr(p.ac)).capacity,
                    expect,
                    "doubling from {cap}"
                );
            }
            p.free();
        }
    }
}

/// Row 20 — `min_cap` larger than `2*cap` wins over doubling.
#[test]
fn cfg20_arrgrowf_min_cap_wins() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x20202020);
    unsafe {
        for &elemsize in ELEMSIZES {
            let mut p = ArrPair::new(&c, &r, elemsize, format!("mc/es{elemsize}"));
            p.grow(0, 4, 0);
            for mult in [3usize, 10, 100] {
                let cap = (*hdr_of_arr(p.ac)).capacity;
                p.grow(0, cap * mult, 0);
                assert_eq!((*hdr_of_arr(p.ac)).capacity, cap * mult);
            }
            p.free();

            for _ in 0..16 {
                let mut q = ArrPair::new(&c, &r, elemsize, "rand-mc");
                q.grow(0, rng.range(1, 64), 0);
                q.grow(0, rng.range(1, 4096), 0);
                q.free();
            }
        }
    }
}

/// Row 21 — `min_len = length + addlen` with a pre-set `length`.
#[test]
fn cfg21_arrgrowf_length_plus_addlen() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x21212121);
    unsafe {
        for &elemsize in ELEMSIZES {
            for _ in 0..24 {
                let mut p = ArrPair::new(&c, &r, elemsize, format!("lenadd/es{elemsize}"));
                let cap0 = rng.range(4, 64);
                p.grow(0, cap0, 0);
                let cap = (*hdr_of_arr(p.ac)).capacity;
                let length = rng.below(cap + 1);
                p.set_length(length);
                let addlen = rng.range(1, 128);
                p.grow(addlen, 0, 0);
                let want = {
                    let min_len = length + addlen;
                    if min_len <= cap {
                        cap
                    } else if min_len < 2 * cap {
                        2 * cap
                    } else {
                        min_len
                    }
                };
                assert_eq!(
                    (*hdr_of_arr(p.ac)).capacity,
                    want,
                    "length={length} addlen={addlen} cap={cap}"
                );
                p.free();
            }
        }
    }
}

/// Row 22 — realloc must preserve the payload across growth, then `arrfreef`.
///
/// Only the bytes that were actually written before a grow are compared; the
/// slack that `realloc` adds is uninitialised in both libraries and therefore
/// legitimately different.
#[test]
fn cfg22_arrgrowf_payload_roundtrip() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x22222222);
    unsafe {
        for &elemsize in ELEMSIZES {
            for trial in 0..16u64 {
                let mut p = ArrPair::new(&c, &r, elemsize, format!("payload/es{elemsize}"));
                p.grow(0, 4, 0);
                // written == number of leading bytes with known contents
                let mut written = (*hdr_of_arr(p.ac)).capacity * elemsize;
                let tag = 0xC0FFEE ^ trial;
                p.fill(written, tag);
                for _ in 0..6 {
                    let cap = (*hdr_of_arr(p.ac)).capacity;
                    let want = rng.range(cap + 1, cap * 3 + 1);
                    // grow() itself compares the `written` prefix on both sides
                    p.grow(0, want, written);
                    // and the prefix must still equal the original pattern
                    let bytes_c = core::slice::from_raw_parts(p.ac as *const u8, written);
                    let bytes_r = core::slice::from_raw_parts(p.ar as *const u8, written);
                    let mut chk = Rng::new(tag);
                    for i in 0..written {
                        let want_byte = chk.next_u8();
                        assert_eq!(bytes_c[i], want_byte, "C payload byte {i} corrupted");
                        assert_eq!(bytes_r[i], want_byte, "Rust payload byte {i} corrupted");
                    }
                    // refill the whole new capacity so the next round has more
                    // known bytes to protect
                    written = (*hdr_of_arr(p.ac)).capacity * elemsize;
                    p.fill(written, tag);
                }
                p.free();
            }
        }
    }
}
