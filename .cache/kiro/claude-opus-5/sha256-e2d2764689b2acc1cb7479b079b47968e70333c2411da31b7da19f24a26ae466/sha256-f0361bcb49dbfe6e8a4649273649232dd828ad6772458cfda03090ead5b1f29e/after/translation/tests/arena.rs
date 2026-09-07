//! Phase B rows 52-57: `stbds_stralloc` / `stbds_strreset` driven directly on
//! a caller-owned `stbds_string_arena` (the lowest-level entry points of the
//! string-arena subsystem).

mod common;
use common::*;
use std::os::raw::c_char;

const SEED: u64 = 0x5EED_1234;

fn zero_arena() -> StringArena {
    StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 }
}

/// Number of blocks on the arena's chain.
unsafe fn chain_len(a: &StringArena) -> usize {
    unsafe {
        let mut n = 0usize;
        let mut x = a.storage as *const *const u8; // block.next is field 0
        while !x.is_null() {
            n += 1;
            x = *x as *const *const u8;
            if n > 100_000 {
                break;
            }
        }
        n
    }
}

/// Where in the chain the returned pointer lives, without exposing addresses.
///
/// An exact block-start match is checked first: `stbds_stralloc`'s oversized
/// path returns `sb->storage` of a freshly spliced block, and the head-relative
/// offset of such a pointer is a raw heap address difference (legitimately
/// different between the two libraries).
unsafe fn locate(a: &StringArena, p: *const u8) -> String {
    unsafe {
        if a.storage.is_null() {
            return "nostorage".to_string();
        }
        let mut i = 0usize;
        let mut blk = a.storage as *const *const u8;
        while !blk.is_null() {
            if p == (blk as *const u8).add(8) {
                return format!("block{}start", i);
            }
            i += 1;
            blk = *blk as *const *const u8;
            if i > 100_000 {
                break;
            }
        }
        let head_storage = (a.storage as *const u8).add(8);
        let off = p as isize - head_storage as isize;
        if (0..(1isize << 21)).contains(&off) {
            return format!("head+{}", off);
        }
        "elsewhere".to_string()
    }
}

unsafe fn describe_arena(a: &StringArena, p: *const c_char) -> String {
    unsafe {
        format!(
            "remaining={} block={} mode={} chain={} storage={} p={} s={:?}",
            a.remaining,
            a.block,
            a.mode,
            chain_len(a),
            if a.storage.is_null() { "null" } else { "set" },
            locate(a, p as *const u8),
            if p.is_null() {
                "<null>".to_string()
            } else {
                String::from_utf8_lossy(std::ffi::CStr::from_ptr(p).to_bytes()).to_string()
            }
        )
    }
}

struct ArenaPair<'a> {
    c: &'a Lib,
    r: &'a Lib,
    ac: StringArena,
    ar: StringArena,
}

impl<'a> ArenaPair<'a> {
    fn new(c: &'a Lib, r: &'a Lib) -> Self {
        ArenaPair { c, r, ac: zero_arena(), ar: zero_arena() }
    }
    fn alloc(&mut self, s: &str, label: &str) {
        let mut buf: Vec<u8> = s.as_bytes().to_vec();
        buf.push(0);
        let p = buf.as_mut_ptr() as *mut c_char;
        unsafe {
            let pc = (self.c.stralloc)(&mut self.ac, p);
            let pr = (self.r.stralloc)(&mut self.ar, p);
            let dc = describe_arena(&self.ac, pc);
            let dr = describe_arena(&self.ar, pr);
            assert_same(&format!("stralloc {} len={}", label, s.len()), &dc, &dr);
        }
    }
    fn reset(&mut self, label: &str) {
        unsafe {
            (self.c.strreset)(&mut self.ac);
            (self.r.strreset)(&mut self.ar);
            let dc = describe_arena(&self.ac, std::ptr::null());
            let dr = describe_arena(&self.ar, std::ptr::null());
            assert_same(&format!("strreset {}", label), &dc, &dr);
            assert!(self.ac.storage.is_null() && self.ac.remaining == 0);
            assert_eq!(self.ac.block, 0);
            assert_eq!(self.ac.mode, 0);
        }
    }
}

impl<'a> Drop for ArenaPair<'a> {
    fn drop(&mut self) {
        unsafe {
            (self.c.strreset)(&mut self.ac);
            (self.r.strreset)(&mut self.ar);
        }
    }
}

#[test]
fn cfg_52_stralloc_blocks() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 52);
    let mut a = ArenaPair::new(&h.c, &h.r);
    for i in 0..3000u64 {
        let n = rng.below(40);
        let s: String = (0..n).map(|_| (b'a' + (rng.byte() % 26)) as char).collect();
        a.alloc(&s, &format!("rand {}", i));
    }
    a.reset("after rand");
}

#[test]
fn cfg_53_stralloc_boundary() {
    let h = setup(0x3141_5926);
    // len = strlen+1, first block is 512 bytes -> boundaries at strlen 510/511/512
    for strlen in [0usize, 1, 2, 509, 510, 511, 512, 513, 1023, 1024, 1025] {
        let mut a = ArenaPair::new(&h.c, &h.r);
        let s: String = std::iter::repeat('z').take(strlen).collect();
        a.alloc(&s, &format!("boundary {}", strlen));
        // then a second, small one, to see how `remaining` was left
        a.alloc("q", &format!("boundary {} follow", strlen));
        a.reset(&format!("boundary {}", strlen));
    }
    // and the same boundaries reached incrementally
    let mut a = ArenaPair::new(&h.c, &h.r);
    for strlen in [500usize, 10, 1, 1, 1, 400, 100, 511, 512] {
        let s: String = std::iter::repeat('y').take(strlen).collect();
        a.alloc(&s, &format!("incr {}", strlen));
    }
    a.reset("incremental");
}

#[test]
fn cfg_54_stralloc_oversized_fresh() {
    let h = setup(0x3141_5926);
    for strlen in [512usize, 600, 1000, 5000, 100_000] {
        let mut a = ArenaPair::new(&h.c, &h.r);
        let s: String = std::iter::repeat('o').take(strlen).collect();
        a.alloc(&s, &format!("oversized fresh {}", strlen));
        assert_eq!(a.ac.remaining, 0, "fresh oversized must leave remaining = 0");
        // a following short string must now allocate a normal block
        a.alloc("tail", &format!("oversized fresh {} tail", strlen));
        a.reset(&format!("oversized fresh {}", strlen));
    }
}

#[test]
fn cfg_55_stralloc_oversized_spliced() {
    let h = setup(0x3141_5926);
    for pre in [1usize, 3, 10] {
        let mut a = ArenaPair::new(&h.c, &h.r);
        for i in 0..pre {
            a.alloc(&format!("pre{}", i), "pre");
        }
        let rem_before = a.ac.remaining;
        let s: String = std::iter::repeat('X').take(2000).collect();
        a.alloc(&s, &format!("spliced pre={}", pre));
        assert_eq!(
            a.ac.remaining, rem_before,
            "splice path must not disturb `remaining`"
        );
        a.alloc("after", "after splice");
        a.reset(&format!("spliced pre={}", pre));
    }
}

#[test]
fn cfg_56_stralloc_saturate() {
    // Drive `block` to its saturation point (blocksize >= STBDS_STRING_ARENA_
    // BLOCKSIZE_MAX == 1<<20, reached at block == 22) and past it.
    let h = setup(0x3141_5926);
    let mut a = ArenaPair::new(&h.c, &h.r);
    for i in 0..70 {
        let rem = a.ac.remaining;
        assert_eq!(rem, a.ar.remaining);
        if rem > 1 {
            // exactly exhaust the current block
            let s: String = std::iter::repeat('s').take(rem - 1).collect();
            a.alloc(&s, &format!("exhaust {}", i));
            assert_eq!(a.ac.remaining, 0);
        } else {
            a.alloc("x", &format!("newblock {}", i));
        }
    }
    assert_eq!(a.ac.block, 22, "C block should saturate at 22");
    assert_eq!(a.ar.block, 22, "Rust block should saturate at 22");
    // stay saturated
    for i in 0..4 {
        let rem = a.ac.remaining;
        let s: String = std::iter::repeat('t').take(rem.max(2) - 1).collect();
        a.alloc(&s, &format!("saturated {}", i));
    }
    assert_eq!(a.ac.block, 22);
    assert_eq!(a.ar.block, 22);
    a.reset("saturated");
}

#[test]
fn cfg_57_strreset_reuse() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 57);
    let mut a = ArenaPair::new(&h.c, &h.r);
    for round in 0..12 {
        let n = 1 + rng.below(400);
        for i in 0..n {
            let l = rng.below(700);
            let s: String = std::iter::repeat('r').take(l).collect();
            a.alloc(&s, &format!("round {} i {}", round, i));
        }
        a.reset(&format!("round {}", round));
        // reset on an already-reset arena
        a.reset(&format!("round {} double", round));
    }
}

#[test]
fn err_44_stralloc_empty_string() {
    let h = setup(0x3141_5926);
    let mut a = ArenaPair::new(&h.c, &h.r);
    for i in 0..600 {
        a.alloc("", &format!("empty {}", i));
    }
    a.reset("empty");
}

#[test]
fn err_45_strreset_empty() {
    let h = setup(0x3141_5926);
    let mut a = ArenaPair::new(&h.c, &h.r);
    a.reset("virgin");
    a.reset("virgin twice");
    a.alloc("x", "after virgin reset");
    a.reset("after use");
}

#[test]
fn err_43_stralloc_block_saturate() {
    // Same as cfg_56 but asserting the exact block sequence, step by step.
    let h = setup(0x3141_5926);
    let mut a = ArenaPair::new(&h.c, &h.r);
    let mut cseq = String::new();
    let mut rseq = String::new();
    for i in 0..60 {
        let rem = a.ac.remaining;
        if rem > 1 {
            let s: String = std::iter::repeat('b').take(rem - 1).collect();
            a.alloc(&s, &format!("seq {}", i));
        } else {
            a.alloc("z", &format!("seq {}", i));
        }
        cseq.push_str(&format!("{},", a.ac.block));
        rseq.push_str(&format!("{},", a.ar.block));
    }
    assert_eq!(cseq, rseq, "block growth sequence mismatch");
    a.reset("seq");
}

#[test]
fn err_41_stralloc_oversized() {
    // `len > blocksize`: the oversized block is spliced in *after* the head
    // (so `remaining` is untouched), except on a fresh arena where
    // `remaining` is explicitly set to 0.
    let h = setup(0x3141_5926);
    for block_pushes in [0usize, 1, 2, 4] {
        let mut a = ArenaPair::new(&h.c, &h.r);
        // grow `block` first so `blocksize` is larger than 512
        for i in 0..block_pushes {
            let rem = a.ac.remaining;
            let n = if rem > 1 { rem - 1 } else { 1 };
            let s: String = std::iter::repeat('p').take(n).collect();
            a.alloc(&s, &format!("grow block {}", i));
        }
        let blocksize = 512usize << ((a.ac.block as usize) >> 1);
        let fresh = a.ac.storage.is_null();
        let rem_before = a.ac.remaining;
        // strlen == blocksize  =>  len == blocksize + 1 > blocksize
        let s: String = std::iter::repeat('O').take(blocksize).collect();
        a.alloc(&s, &format!("oversized block={} ", a.ac.block));
        if fresh {
            assert_eq!(a.ac.remaining, 0, "fresh arena must get remaining = 0");
        } else {
            assert_eq!(
                a.ac.remaining, rem_before,
                "splice must leave `remaining` alone"
            );
        }
        // exactly at the boundary: strlen == blocksize-1 => len == blocksize, NOT oversized
        let s2: String = std::iter::repeat('N').take(blocksize.saturating_sub(1)).collect();
        a.alloc(&s2, "at boundary");
        a.reset(&format!("oversized pushes={}", block_pushes));
    }
}

#[test]
fn err_42_stralloc_first() {
    // First call on a zeroed arena: `len > 0 == remaining` -> a 512-byte block
    // is allocated and `remaining` becomes 512 - len.
    let h = setup(0x3141_5926);
    for strlen in [0usize, 1, 2, 10, 100, 510, 511] {
        let mut a = ArenaPair::new(&h.c, &h.r);
        assert!(a.ac.storage.is_null() && a.ac.remaining == 0 && a.ac.block == 0);
        let s: String = std::iter::repeat('f').take(strlen).collect();
        a.alloc(&s, &format!("first strlen={}", strlen));
        assert_eq!(
            a.ac.remaining,
            512 - (strlen + 1),
            "remaining after the first {}-byte string",
            strlen + 1
        );
        assert_eq!(a.ac.remaining, a.ar.remaining);
        assert_eq!(a.ac.block, 1, "block must have been bumped to 1");
        assert_eq!(a.ar.block, 1);
        assert!(!a.ac.storage.is_null() && !a.ar.storage.is_null());
        a.reset(&format!("first strlen={}", strlen));
    }
}
