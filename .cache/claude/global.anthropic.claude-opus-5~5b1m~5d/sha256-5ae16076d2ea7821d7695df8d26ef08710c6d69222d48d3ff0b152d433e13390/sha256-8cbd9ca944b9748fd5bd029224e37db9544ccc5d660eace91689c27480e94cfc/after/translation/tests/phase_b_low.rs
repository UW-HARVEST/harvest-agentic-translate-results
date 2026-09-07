//! Phase B — valid-path differential tests for the LOW-LEVEL entry points.
//!
//! Covers CONFIGS.md rows C1–C21 and C57–C63.
//! Every call goes through `dlopen`'d `.so` exports for BOTH libraries.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

const SEEDS: [usize; 5] = [0, 1, 0x3141_5926, usize::MAX, 0x0123_4567_89AB_CDEF];

// ===========================================================================
// C1..C6  stbds_hash_bytes
// ===========================================================================

fn hash_bytes_both(c: &Lib, r: &Lib, buf: &mut [u8], len: usize, seed: usize) {
    let p = buf.as_mut_ptr() as *mut c_void;
    let hc = unsafe { (c.hash_bytes)(p, len, seed) };
    let hr = unsafe { (r.hash_bytes)(p, len, seed) };
    assert_eq!(
        hc, hr,
        "stbds_hash_bytes mismatch: len={} seed={:#x} buf={:02x?}",
        len,
        seed,
        &buf[..len]
    );
}

#[test]
fn c1_hash_bytes_len_zero() {
    let (c, r) = both();
    // len == 0: the pointer is never dereferenced.
    for &seed in &SEEDS {
        let hc = unsafe { (c.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        let hr = unsafe { (r.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        assert_eq!(hc, hr, "hash_bytes(NULL,0,{:#x})", seed);
        let mut b = [0u8; 8];
        hash_bytes_both(&c, &r, &mut b, 0, seed);
    }
}

#[test]
fn c2_c3_c4_hash_bytes_all_lengths_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC0FFEE);
    // Every length 0..=200 exercises every tail `case` and the 8-byte main loop.
    for len in 0..=200usize {
        for _ in 0..24 {
            let mut buf = rng.bytes(len + 8);
            for &seed in &SEEDS {
                hash_bytes_both(&c, &r, &mut buf, len, seed);
            }
            let rs = rng.next_u64() as usize;
            hash_bytes_both(&c, &r, &mut buf, len, rs);
        }
    }
}

#[test]
fn c5_hash_bytes_high_bit_sign_extension() {
    let (c, r) = both();
    // Bytes with bit 7 set at word offsets 3 and 7 make the C `int`
    // arithmetic sign-extend into size_t.
    let mut rng = Rng::new(0x5157);
    for len in 0..=48usize {
        // all-0xFF
        let mut buf = vec![0xFFu8; len + 8];
        for &seed in &SEEDS {
            hash_bytes_both(&c, &r, &mut buf, len, seed);
        }
        // 0x80 only at the sign-bit positions
        let mut buf = vec![0x00u8; len + 8];
        for i in 0..buf.len() {
            if i % 4 == 3 {
                buf[i] = 0x80;
            }
        }
        for &seed in &SEEDS {
            hash_bytes_both(&c, &r, &mut buf, len, seed);
        }
        // random high-bit-heavy
        for _ in 0..8 {
            let mut buf: Vec<u8> = (0..len + 8).map(|_| 0x80 | rng.byte()).collect();
            for &seed in &SEEDS {
                hash_bytes_both(&c, &r, &mut buf, len, seed);
            }
        }
    }
}

#[test]
fn c6_hash_bytes_seed_sweep() {
    let (c, r) = both();
    let mut rng = Rng::new(0xBEEF);
    for _ in 0..400 {
        let len = rng.below(64);
        let mut buf = rng.bytes(len + 8);
        for shift in 0..64u32 {
            let seed = 1usize << shift;
            hash_bytes_both(&c, &r, &mut buf, len, seed);
            hash_bytes_both(&c, &r, &mut buf, len, !seed);
        }
    }
}

// ===========================================================================
// C7..C10  stbds_hash_string
// ===========================================================================

fn hash_string_both(c: &Lib, r: &Lib, s: &mut [u8], seed: usize) {
    assert_eq!(*s.last().unwrap(), 0, "string must be NUL terminated");
    let p = s.as_mut_ptr() as *mut c_char;
    let hc = unsafe { (c.hash_string)(p, seed) };
    let hr = unsafe { (r.hash_string)(p, seed) };
    assert_eq!(
        hc, hr,
        "stbds_hash_string mismatch: seed={:#x} s={:02x?}",
        seed, s
    );
}

#[test]
fn c7_hash_string_empty() {
    let (c, r) = both();
    let mut s = [0u8; 1];
    for &seed in &SEEDS {
        hash_string_both(&c, &r, &mut s, seed);
    }
}

#[test]
fn c8_c10_hash_string_lengths() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1234_5678);
    for len in 0..=255usize {
        for _ in 0..6 {
            let mut s = rng.ascii_cstring(len);
            for &seed in &SEEDS {
                hash_string_both(&c, &r, &mut s, seed);
            }
            let rs = rng.next_u64() as usize;
            hash_string_both(&c, &r, &mut s, rs);
        }
    }
}

#[test]
fn c9_hash_string_high_bytes() {
    let (c, r) = both();
    let mut rng = Rng::new(0x9999);
    for len in 0..=128usize {
        // pure 0x80..0xFF
        let mut s: Vec<u8> = (0..len).map(|_| 0x80 | (rng.byte() & 0x7f)).collect();
        s.push(0);
        for &seed in &SEEDS {
            hash_string_both(&c, &r, &mut s, seed);
        }
        // arbitrary non-zero bytes
        let mut s = rng.cstring(len);
        for &seed in &SEEDS {
            hash_string_both(&c, &r, &mut s, seed);
        }
    }
    // all-0xFF, long
    let mut s = vec![0xFFu8; 300];
    *s.last_mut().unwrap() = 0;
    for &seed in &SEEDS {
        hash_string_both(&c, &r, &mut s, seed);
    }
}

// ===========================================================================
// C11  stbds_rand_seed and the global seed advance
// ===========================================================================

#[test]
fn c11_rand_seed_and_global_advance() {
    let (c, r) = both();
    for &seed in &SEEDS {
        unsafe {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
        }
        // Each fresh table consumes+advances the global seed. Build eight
        // independent maps and compare the `seed` each one captured.
        for k in 0..8u64 {
            // elemsize == keysize == 8 so that the library's `memcpy` defines
            // *every* byte of the element; bytes past `keysize` are caller-
            // owned scratch that the C library legitimately leaves as
            // whatever `realloc` returned.
            let elemsize = 8usize;
            let mut key: u64 = 0xAAAA_0000 + k;
            let hc = unsafe {
                (c.hmput_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    &mut key as *mut u64 as *mut c_void,
                    8,
                    HM_BINARY,
                )
            };
            let hr = unsafe {
                (r.hmput_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    &mut key as *mut u64 as *mut c_void,
                    8,
                    HM_BINARY,
                )
            };
            let sc = unsafe { snap_map(hc, elemsize, KeyKind::Raw, false) };
            let sr = unsafe { snap_map(hr, elemsize, KeyKind::Raw, false) };
            assert_eq!(sc, sr, "seed advance diverged at k={} seed={:#x}", k, seed);
            unsafe {
                (c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
    // restore the default seed for other tests in this binary
    unsafe {
        (c.rand_seed)(0x3141_5926);
        (r.rand_seed)(0x3141_5926);
    }
}

// ===========================================================================
// C12..C19  stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

unsafe fn hdr_len(a: *mut c_void) -> usize {
    ((a as *mut u8).sub(HEADER_SIZE) as *mut usize).read()
}
unsafe fn set_hdr_len(a: *mut c_void, v: usize) {
    ((a as *mut u8).sub(HEADER_SIZE) as *mut usize).write(v)
}
unsafe fn hdr_cap(a: *mut c_void) -> usize {
    ((a as *mut u8).sub(HEADER_SIZE).add(8) as *mut usize).read()
}

#[test]
fn c12_arrgrowf_null_noop() {
    let (c, r) = both();
    // min_cap = 0, addlen = 0 on a NULL array: min_len == 0 <= arrcap(NULL)==0
    // ==> early return of the input pointer (NULL).
    for &elemsize in &[0usize, 1, 4, 8, 24] {
        let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        assert!(ac.is_null() && ar.is_null(), "elemsize={}", elemsize);
    }
}

#[test]
fn c13_c14_arrgrowf_fresh_min_cap_matrix() {
    let (c, r) = both();
    for &elemsize in &[1usize, 4, 8, 12, 16, 24] {
        for &addlen in &[0usize, 1, 2, 3, 4, 5, 7, 100] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 6, 17, 64, 1000] {
                if addlen == 0 && min_cap == 0 {
                    continue; // C12
                }
                let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                let sc = unsafe { snap_arr(ac, elemsize) };
                let sr = unsafe { snap_arr(ar, elemsize) };
                assert_eq!(
                    sc, sr,
                    "arrgrowf(NULL,{},{},{})",
                    elemsize, addlen, min_cap
                );
                unsafe {
                    (c.arrfreef)(ac);
                    (r.arrfreef)(ar);
                }
            }
        }
    }
}

#[test]
fn c15_c16_c17_arrgrowf_existing() {
    let (c, r) = both();
    for &elemsize in &[1usize, 4, 8, 12, 24] {
        for &start in &[4usize, 5, 8, 17, 64] {
            let mut ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, start) };
            let mut ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, start) };
            let cap = unsafe { hdr_cap(ac) };
            assert_eq!(cap, unsafe { hdr_cap(ar) });
            // fill with deterministic bytes so re-allocation copies are checked
            unsafe {
                set_hdr_len(ac, cap);
                set_hdr_len(ar, cap);
                for k in 0..cap * elemsize {
                    *(ac as *mut u8).add(k) = (k as u8).wrapping_mul(37).wrapping_add(11);
                    *(ar as *mut u8).add(k) = (k as u8).wrapping_mul(37).wrapping_add(11);
                }
            }
            for &min_cap in &[
                0usize,
                1,
                cap / 2,
                cap,
                cap + 1,
                2 * cap - 1,
                2 * cap,
                2 * cap + 1,
                4 * cap,
                10 * cap,
            ] {
                let cap_before_c = unsafe { hdr_cap(ac) };
                let cap_before_r = unsafe { hdr_cap(ar) };
                assert_eq!(cap_before_c, cap_before_r);
                let nc = unsafe { (c.arrgrowf)(ac, elemsize, 0, min_cap) };
                let nr = unsafe { (r.arrgrowf)(ar, elemsize, 0, min_cap) };
                ac = nc;
                ar = nr;
                // The early-return branch (min_cap <= arrcap) must leave the
                // capacity untouched in both. (Pointer identity cannot be
                // compared: `realloc` is free to grow in place, so an equal
                // pointer does not imply the early return was taken.)
                let expect_unchanged = min_cap <= cap_before_c;
                if expect_unchanged {
                    assert_eq!(unsafe { hdr_cap(ac) }, cap_before_c, "C grew unexpectedly");
                    assert_eq!(unsafe { hdr_cap(ar) }, cap_before_r, "Rust grew unexpectedly");
                }
                let sc = unsafe { snap_arr(ac, elemsize) };
                let sr = unsafe { snap_arr(ar, elemsize) };
                assert_eq!(
                    sc, sr,
                    "arrgrowf(a,{},0,{}) from cap {}",
                    elemsize, min_cap, cap
                );
            }
            unsafe {
                (c.arrfreef)(ac);
                (r.arrfreef)(ar);
            }
        }
    }
}

#[test]
fn c18_c19_arrgrowf_push_chain() {
    let (c, r) = both();
    let mut rng = Rng::new(0xABCD);
    for &elemsize in &[1usize, 4, 8, 12, 16, 24] {
        let mut ac: *mut c_void = std::ptr::null_mut();
        let mut ar: *mut c_void = std::ptr::null_mut();
        for step in 0..600usize {
            // stbds_arrmaybegrow(a, 1)
            let addn = 1 + rng.below(3);
            unsafe {
                if ac.is_null() || hdr_len(ac) + addn > hdr_cap(ac) {
                    ac = (c.arrgrowf)(ac, elemsize, addn, 0);
                }
                if ar.is_null() || hdr_len(ar) + addn > hdr_cap(ar) {
                    ar = (r.arrgrowf)(ar, elemsize, addn, 0);
                }
                let l = hdr_len(ac);
                assert_eq!(l, hdr_len(ar));
                set_hdr_len(ac, l + addn);
                set_hdr_len(ar, l + addn);
                for k in 0..addn * elemsize {
                    let b = rng.byte();
                    *(ac as *mut u8).add(l * elemsize + k) = b;
                    *(ar as *mut u8).add(l * elemsize + k) = b;
                }
            }
            let sc = unsafe { snap_arr(ac, elemsize) };
            let sr = unsafe { snap_arr(ar, elemsize) };
            assert_eq!(sc, sr, "push chain elemsize={} step={}", elemsize, step);
        }
        unsafe {
            (c.arrfreef)(ac);
            (r.arrfreef)(ar);
        }
    }
}

// ===========================================================================
// C20  arr_push
// ===========================================================================

#[test]
fn c20_arr_push() {
    let (c, r) = both();
    for &n in &[
        0i32, -1, -50, -1000, 1, 2, 49, 50, 51, 99, 100, 101, 149, 150, 500, 2000,
    ] {
        unsafe {
            (c.arr_push)(n);
            (r.arr_push)(n);
        }
    }
    // arr_push has no observable output; reaching here means both agreed on
    // "no crash, no assert" for every input.
}

// ===========================================================================
// C21  strkey
// ===========================================================================

#[test]
fn c21_strkey() {
    let (c, r) = both();
    let check = |n: c_int| {
        let pc = unsafe { (c.strkey)(n) };
        let pr = unsafe { (r.strkey)(n) };
        let sc = unsafe { cstr_bytes(pc) };
        let sr = unsafe { cstr_bytes(pr) };
        assert_eq!(
            sc,
            sr,
            "strkey({}) : C={:?} RUST={:?}",
            n,
            String::from_utf8_lossy(&sc),
            String::from_utf8_lossy(&sr)
        );
        // also sanity-check against the C format
        assert_eq!(sc, format!("test_{}", n).into_bytes(), "strkey({})", n);
    };
    for n in -300i32..=300 {
        check(n);
    }
    for &n in &[
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        -1_000_000,
        1_000_000,
        999_999_999,
        -999_999_999,
    ] {
        check(n);
    }
    let mut rng = Rng::new(0x2222);
    for _ in 0..2000 {
        check(rng.next_u32() as i32);
    }
}

// ===========================================================================
// C57..C63  stbds_stralloc / stbds_strreset
// ===========================================================================

struct ArenaPair {
    ac: StringArena,
    ar: StringArena,
}

impl ArenaPair {
    fn new() -> ArenaPair {
        ArenaPair {
            ac: StringArena::new(),
            ar: StringArena::new(),
        }
    }
    fn alloc(&mut self, c: &Lib, r: &Lib, s: &mut [u8], what: &str) {
        assert_eq!(*s.last().unwrap(), 0);
        let pc = unsafe { (c.stralloc)(&mut self.ac, s.as_mut_ptr() as *mut c_char) };
        let pr = unsafe { (r.stralloc)(&mut self.ar, s.as_mut_ptr() as *mut c_char) };
        assert!(!pc.is_null() && !pr.is_null(), "{}", what);
        // string contents must match
        assert_eq!(
            unsafe { cstr_bytes(pc) },
            unsafe { cstr_bytes(pr) },
            "stralloc content: {}",
            what
        );
        assert_eq!(
            unsafe { cstr_bytes(pc) },
            s[..s.len() - 1].to_vec(),
            "stralloc round trip: {}",
            what
        );
        let sc = unsafe { snap_arena(&self.ac) };
        let sr = unsafe { snap_arena(&self.ar) };
        assert_eq!(sc, sr, "arena state after {}", what);
        // The normal (non-oversize) path must place the string at exactly
        // `storage->storage + remaining` (after `remaining` was decremented).
        // The oversize path returns a pointer into a *different* block, so all
        // we can require is that BOTH libraries agree on which path was taken.
        let head_c =
            unsafe { (self.ac.storage as *const u8).add(8).add(self.ac.remaining) } as *const u8;
        let head_r =
            unsafe { (self.ar.storage as *const u8).add(8).add(self.ar.remaining) } as *const u8;
        assert_eq!(
            pc as *const u8 == head_c,
            pr as *const u8 == head_r,
            "stralloc placement path disagreement: {}",
            what
        );
    }
    fn reset(&mut self, c: &Lib, r: &Lib) {
        unsafe {
            (c.strreset)(&mut self.ac);
            (r.strreset)(&mut self.ar);
        }
        assert_eq!(unsafe { snap_arena(&self.ac) }, unsafe {
            snap_arena(&self.ar)
        });
        assert_eq!(self.ac, self.ar, "arena fully zeroed identically");
    }
}

#[test]
fn c57_stralloc_fresh_short() {
    let (c, r) = both();
    let mut ap = ArenaPair::new();
    let mut s = b"hello\0".to_vec();
    ap.alloc(&c, &r, &mut s, "fresh short");
    assert_eq!(ap.ac.block, 1);
    assert_eq!(ap.ac.remaining, 512 - 6);
    ap.reset(&c, &r);
}

#[test]
fn c58_stralloc_many_short() {
    let (c, r) = both();
    let mut rng = Rng::new(0x3333);
    let mut ap = ArenaPair::new();
    for i in 0..4000usize {
        let n = rng.below(80);
        let mut s = rng.ascii_cstring(n);
        ap.alloc(&c, &r, &mut s, &format!("many short #{}", i));
    }
    ap.reset(&c, &r);
}

#[test]
fn c59_stralloc_oversize_empty_arena() {
    let (c, r) = both();
    let mut ap = ArenaPair::new();
    // len > 512 on a fresh arena -> oversize path, storage installed,
    // remaining set to 0.
    let mut s = vec![b'x'; 5000];
    s.push(0);
    ap.alloc(&c, &r, &mut s, "oversize fresh");
    assert_eq!(ap.ac.remaining, 0);
    assert_eq!(ap.ac.block, 1);
    assert!(!ap.ac.storage.is_null());
    ap.reset(&c, &r);
}

#[test]
fn c60_stralloc_oversize_splice() {
    let (c, r) = both();
    let mut ap = ArenaPair::new();
    let mut small = b"abc\0".to_vec();
    ap.alloc(&c, &r, &mut small, "warm up");
    let rem_before = ap.ac.remaining;
    // now an oversize string with a non-NULL a->storage -> splice path,
    // `remaining` is left untouched.
    let mut big = vec![b'y'; 4096];
    big.push(0);
    ap.alloc(&c, &r, &mut big, "oversize splice");
    assert_eq!(ap.ac.remaining, rem_before, "remaining must be untouched");
    assert_eq!(unsafe { snap_arena(&ap.ac) }.nblocks, 2);
    // and the head block is still usable
    let mut small2 = b"def\0".to_vec();
    ap.alloc(&c, &r, &mut small2, "after splice");
    ap.reset(&c, &r);
}

#[test]
fn c61_stralloc_empty_string() {
    let (c, r) = both();
    let mut ap = ArenaPair::new();
    let mut e = b"\0".to_vec();
    ap.alloc(&c, &r, &mut e, "empty on fresh arena");
    for i in 0..200 {
        let mut e = b"\0".to_vec();
        ap.alloc(&c, &r, &mut e, &format!("empty warm #{}", i));
    }
    ap.reset(&c, &r);
}

#[test]
fn c62_stralloc_block_saturation() {
    let (c, r) = both();
    let mut ap = ArenaPair::new();
    // Each iteration requests exactly the whole next block, so `remaining`
    // hits 0 and the following call allocates a new (larger) block. This walks
    // a->block from 0 up past the 1<<20 saturation point.
    for step in 0..26usize {
        let blocksize: usize = 512usize << (ap.ac.block as usize >> 1);
        let mut s = vec![b'z'; blocksize - 1];
        s.push(0);
        ap.alloc(&c, &r, &mut s, &format!("saturation step {}", step));
        assert_eq!(ap.ac.remaining, 0);
        assert_eq!(ap.ac.block, ap.ar.block);
        if step >= 22 {
            assert_eq!(ap.ac.block, 22, "block must saturate at 22");
        }
    }
    ap.reset(&c, &r);
}

#[test]
fn c63_strreset_shapes() {
    let (c, r) = both();
    // (a) fresh / already-zero arena: idempotent no-op
    let mut ap = ArenaPair::new();
    ap.reset(&c, &r);
    ap.reset(&c, &r);
    // (b) single block
    let mut s = b"one\0".to_vec();
    ap.alloc(&c, &r, &mut s, "single");
    ap.reset(&c, &r);
    // (c) multi block
    for i in 0..40usize {
        let mut s = vec![b'q'; 400];
        s.push(0);
        ap.alloc(&c, &r, &mut s, &format!("multi {}", i));
    }
    assert!(unsafe { snap_arena(&ap.ac) }.nblocks > 1);
    ap.reset(&c, &r);
    // (d) arena containing a spliced oversize block
    let mut s = b"head\0".to_vec();
    ap.alloc(&c, &r, &mut s, "head");
    let mut big = vec![b'B'; 9000];
    big.push(0);
    ap.alloc(&c, &r, &mut big, "spliced");
    ap.reset(&c, &r);
    // (e) randomized mixture, repeated
    let mut rng = Rng::new(0x4444);
    for round in 0..12usize {
        for i in 0..60usize {
            let n = if rng.below(5) == 0 {
                600 + rng.below(4000)
            } else {
                rng.below(120)
            };
            let mut s = rng.ascii_cstring(n);
            ap.alloc(&c, &r, &mut s, &format!("mix r{} i{}", round, i));
        }
        ap.reset(&c, &r);
    }
}
