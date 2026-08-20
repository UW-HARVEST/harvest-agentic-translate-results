//! Phase B — CONFIGS.md rows 58..65: the string arena, driven directly through
//! `stbds_stralloc` / `stbds_strreset` with a caller-owned
//! `stbds_string_arena` (the lowest-level string entry points).
mod common;

use common::*;
use core::ffi::{c_char, c_void};

const BLOCKSIZE_MIN: usize = 512;
const BLOCKSIZE_MAX: usize = 1 << 20;

fn blocksize_for(block: u8) -> usize {
    BLOCKSIZE_MIN.wrapping_shl((block >> 1) as u32)
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct ArenaSnap {
    remaining: usize,
    block: u8,
    mode: u8,
    has_storage: bool,
    chain_len: usize,
}

unsafe fn arena_snap(a: *const StringArena) -> ArenaSnap {
    let mut chain_len = 0usize;
    let mut x = (*a).storage;
    while !x.is_null() {
        chain_len += 1;
        assert!(chain_len < 100_000, "arena chain looks circular");
        x = (*x).next;
    }
    ArenaSnap {
        remaining: (*a).remaining,
        block: (*a).block,
        mode: (*a).mode,
        has_storage: !(*a).storage.is_null(),
        chain_len,
    }
}

struct ArenaPair<'l> {
    c: &'l Lib,
    r: &'l Lib,
    ac: Box<StringArena>,
    ar: Box<StringArena>,
    label: String,
    step: usize,
    /// every string handed out so far, and where it landed, so aliasing /
    /// overwrite bugs are caught
    live_c: Vec<(*mut c_char, Vec<u8>)>,
    live_r: Vec<(*mut c_char, Vec<u8>)>,
}

impl<'l> ArenaPair<'l> {
    fn new(c: &'l Lib, r: &'l Lib, label: impl Into<String>) -> ArenaPair<'l> {
        ArenaPair {
            c,
            r,
            ac: Box::new(StringArena::zeroed()),
            ar: Box::new(StringArena::zeroed()),
            label: label.into(),
            step: 0,
            live_c: Vec::new(),
            live_r: Vec::new(),
        }
    }

    fn set_block(&mut self, b: u8) {
        self.ac.block = b;
        self.ar.block = b;
    }

    unsafe fn cmp_state(&mut self, what: &str) {
        let sc = arena_snap(&*self.ac);
        let sr = arena_snap(&*self.ar);
        assert_eq!(
            sc, sr,
            "[{}] step {} ({}): arena state diverged",
            self.label, self.step, what
        );
        self.step += 1;
    }

    /// `stbds_stralloc(&a, str)` on both, comparing the returned string, the
    /// arena bookkeeping, and (where it is meaningful) the offset of the
    /// returned pointer inside the head block.
    unsafe fn alloc(&mut self, text: &[u8]) {
        assert!(!text.contains(&0), "test strings must not contain NUL");
        let mut buf = text.to_vec();
        buf.push(0);
        let len = buf.len();

        // classify the path the C code will take, from the pre-state
        let rem_before = self.ac.remaining;
        let block_before = self.ac.block;
        let had_storage = !self.ac.storage.is_null();
        let will_alloc = len > rem_before;
        let bs = blocksize_for(block_before);
        let big_block = will_alloc && len > bs;
        // only in this one case is the returned pointer NOT inside the head
        // block, so its offset from `storage` is heap-layout dependent
        let offset_meaningless = big_block && had_storage;

        let pc = (self.c.stralloc)(&mut *self.ac, buf.as_mut_ptr() as *mut c_char);
        let pr = (self.r.stralloc)(&mut *self.ar, buf.as_mut_ptr() as *mut c_char);

        assert_eq!(
            cstr_bytes(pc).as_deref(),
            Some(text),
            "[{}] step {}: C returned the wrong string",
            self.label,
            self.step
        );
        assert_eq!(
            cstr_bytes(pr).as_deref(),
            Some(text),
            "[{}] step {}: Rust returned the wrong string",
            self.label,
            self.step
        );
        if !offset_meaningless {
            let dc = (pc as usize).wrapping_sub(self.ac.storage as usize);
            let dr = (pr as usize).wrapping_sub(self.ar.storage as usize);
            assert_eq!(
                dc, dr,
                "[{}] step {}: offset within the head block diverged (len={len}, rem_before={rem_before}, block_before={block_before}, bs={bs})",
                self.label, self.step
            );
            // and it must be consistent with the documented layout
            if !big_block {
                assert_eq!(
                    dc,
                    STRING_BLOCK_HDR + self.ac.remaining,
                    "bump-allocated offset must be 8 + remaining"
                );
            } else {
                assert_eq!(dc, STRING_BLOCK_HDR, "big block payload starts at +8");
            }
        }
        self.live_c.push((pc, text.to_vec()));
        self.live_r.push((pr, text.to_vec()));
        self.cmp_state("stralloc");
    }

    /// Every string previously handed out must still read back correctly.
    unsafe fn verify_all_live(&self) {
        for (p, want) in &self.live_c {
            assert_eq!(
                cstr_bytes(*p).as_deref(),
                Some(want.as_slice()),
                "[{}] a previously allocated C string was clobbered",
                self.label
            );
        }
        for (p, want) in &self.live_r {
            assert_eq!(
                cstr_bytes(*p).as_deref(),
                Some(want.as_slice()),
                "[{}] a previously allocated Rust string was clobbered",
                self.label
            );
        }
    }

    unsafe fn reset(&mut self) {
        (self.c.strreset)(&mut *self.ac);
        (self.r.strreset)(&mut *self.ar);
        self.live_c.clear();
        self.live_r.clear();
        self.cmp_state("strreset");
        assert_eq!(arena_snap(&*self.ac), arena_snap(&*self.ar));
        assert_eq!(self.ac.remaining, 0);
        assert_eq!(self.ac.block, 0);
        assert_eq!(self.ac.mode, 0);
        assert!(self.ac.storage.is_null());
        assert!(self.ar.storage.is_null());
    }
}

/// Row 58 — a fresh arena and one string that fits the first 512-byte block.
#[test]
fn cfg58_arena_single_short_string() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x5858_5858);
    unsafe {
        for len in [0usize, 1, 2, 7, 8, 9, 100, 255, 300, 500, 510, 511] {
            let mut p = ArenaPair::new(&c, &r, format!("one/len{len}"));
            let s = rng.nz_bytes(len);
            p.alloc(&s);
            assert_eq!(p.ac.block, 1, "first block bumps `block` to 1");
            assert_eq!(p.ac.remaining, 512 - (len + 1));
            p.verify_all_live();
            p.reset();
        }
        for _ in 0..256 {
            let len = rng.below(500);
            let mut p = ArenaPair::new(&c, &r, "one/rand");
            let s = rng.nz_bytes(len);
            p.alloc(&s);
            p.verify_all_live();
            p.reset();
        }
    }
}

/// Row 59 — many strings, exhausting block after block (`block` 0→1→2→…).
#[test]
fn cfg59_arena_many_blocks() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x5959_5959);
    unsafe {
        for trial in 0..24u64 {
            let mut p = ArenaPair::new(&c, &r, format!("many/{trial}"));
            let lo = 1 + (trial as usize % 5) * 20;
            let hi = lo + 40;
            for _ in 0..300 {
                let s = { let n = rng.range(lo, hi); rng.nz_bytes(n) };
                p.alloc(&s);
            }
            p.verify_all_live();
            assert!(p.ac.block >= 2, "block counter should have advanced");
            p.reset();
        }
    }
}

/// Row 60 — `len > blocksize` on a fresh arena (`storage == NULL`).
#[test]
fn cfg60_arena_big_block_fresh() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x6060_6060);
    unsafe {
        for len in [512usize, 513, 600, 1024, 4096, 65536] {
            let mut p = ArenaPair::new(&c, &r, format!("big/fresh/len{len}"));
            let s = rng.nz_bytes(len);
            p.alloc(&s);
            // big-block path with storage == NULL: next = 0, remaining = 0
            assert_eq!(p.ac.remaining, 0, "big block leaves remaining == 0");
            assert_eq!(p.ac.chain_len_now(), 1);
            assert_eq!(p.ac.block, 1);
            p.verify_all_live();
            p.reset();
        }
    }
}

trait ChainLen {
    unsafe fn chain_len_now(&self) -> usize;
}
impl ChainLen for StringArena {
    unsafe fn chain_len_now(&self) -> usize {
        let mut n = 0;
        let mut x = self.storage;
        while !x.is_null() {
            n += 1;
            x = (*x).next;
        }
        n
    }
}

/// Row 61 — `len > blocksize` when a normal block already exists: the big block
/// is spliced in AFTER the head and `remaining` is preserved.
#[test]
fn cfg61_arena_big_block_after_head() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x6161_6161);
    unsafe {
        for big in [600usize, 1024, 2048, 70000] {
            let mut p = ArenaPair::new(&c, &r, format!("big/after/len{big}"));
            // establish a head block with plenty of room left
            p.alloc(&rng.nz_bytes(10));
            let rem = p.ac.remaining;
            let chain = p.ac.chain_len_now();
            let blk = p.ac.block;
            // this must exceed `blocksize_for(blk)`; bump `block` down if needed
            if big <= blocksize_for(blk) {
                continue;
            }
            p.alloc(&rng.nz_bytes(big));
            assert_eq!(p.ac.remaining, rem, "big block must not touch `remaining`");
            assert_eq!(p.ac.chain_len_now(), chain + 1);
            p.verify_all_live();
            // the head block is still usable
            p.alloc(&rng.nz_bytes(5));
            p.verify_all_live();
            p.reset();
        }
    }
}

/// Row 62 — `len == remaining` exactly, and `len == remaining + 1`.
#[test]
fn cfg62_arena_exact_fit_boundary() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x6262_6262);
    unsafe {
        for delta in [0isize, 1, -1] {
            for first in [0usize, 1, 100, 400] {
                let mut p =
                    ArenaPair::new(&c, &r, format!("fit/d{delta}/f{first}"));
                p.alloc(&rng.nz_bytes(first));
                let rem = p.ac.remaining;
                let target_len = (rem as isize + delta) as usize; // includes the NUL
                if target_len == 0 {
                    continue;
                }
                let chain = p.ac.chain_len_now();
                let s = rng.nz_bytes(target_len - 1);
                p.alloc(&s);
                if delta <= 0 {
                    assert_eq!(p.ac.chain_len_now(), chain, "must fit in the head block");
                    assert_eq!(p.ac.remaining, rem - target_len);
                } else {
                    assert_eq!(p.ac.chain_len_now(), chain + 1, "must need a new block");
                }
                p.verify_all_live();
                p.reset();
            }
        }
    }
}

/// Row 63 — every `a->block` preset from 0..=24 (and the `BLOCKSIZE_MAX`
/// saturation of `++a->block`).
#[test]
fn cfg63_arena_preset_block() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x6363_6363);
    unsafe {
        for b in 0u8..=24 {
            let bs = blocksize_for(b);
            let mut p = ArenaPair::new(&c, &r, format!("preset/b{b}"));
            p.set_block(b);
            // a string that fits the (large) block => normal new-block path
            p.alloc(&rng.nz_bytes(64));
            let saturated = bs >= BLOCKSIZE_MAX;
            assert_eq!(
                p.ac.block,
                if saturated { b } else { b + 1 },
                "++a->block must be skipped once blocksize >= 1<<20 (b={b}, bs={bs})"
            );
            assert_eq!(p.ac.remaining, bs - 65);
            p.verify_all_live();
            p.reset();
        }
    }
}

/// Row 64 — 2000 random-length strings in one arena (mixed bump / new-block /
/// big-block), then reset. Every handed-out string is re-verified.
#[test]
fn cfg64_arena_fuzz() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x6464_6464);
    unsafe {
        let mut p = ArenaPair::new(&c, &r, "fuzz");
        for i in 0..2000usize {
            let len = match i % 20 {
                0 => rng.range(600, 1500), // big-block path
                1 => 0,
                _ => rng.range(1, 400),
            };
            let s = rng.nz_bytes(len);
            p.alloc(&s);
            if i % 200 == 0 {
                p.verify_all_live();
            }
        }
        p.verify_all_live();
        p.reset();

        // and a second fuzz run with the arena reused after a reset
        for round in 0..8 {
            for _ in 0..200 {
                let s = { let n = rng.range(0, 700); rng.nz_bytes(n) };
                p.alloc(&s);
            }
            p.verify_all_live();
            p.reset();
            let _ = round;
        }
    }
}

/// Row 65 — `stbds_strreset` on an empty arena, a 1-block arena, a many-block
/// arena, and twice in a row.
#[test]
fn cfg65_strreset_shapes() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x6565_6565);
    unsafe {
        // empty
        let mut p = ArenaPair::new(&c, &r, "reset/empty");
        p.reset();
        p.reset();
        p.reset();
        // one block
        p.alloc(&rng.nz_bytes(10));
        p.reset();
        p.reset();
        // many blocks
        for _ in 0..200 {
            p.alloc(&{ let n = rng.range(1, 300); rng.nz_bytes(n) });
        }
        assert!(p.ac.chain_len_now() > 1);
        p.reset();
        p.reset();
        // big block + normal blocks
        p.alloc(&rng.nz_bytes(4000));
        p.alloc(&rng.nz_bytes(10));
        p.alloc(&rng.nz_bytes(9000));
        p.reset();
        // arena is fully reusable after reset
        p.alloc(&rng.nz_bytes(20));
        assert_eq!(p.ac.block, 1);
        assert_eq!(p.ac.remaining, 512 - 21);
        p.reset();
        let _ = &mut p as *mut _ as *mut c_void;
    }
}
