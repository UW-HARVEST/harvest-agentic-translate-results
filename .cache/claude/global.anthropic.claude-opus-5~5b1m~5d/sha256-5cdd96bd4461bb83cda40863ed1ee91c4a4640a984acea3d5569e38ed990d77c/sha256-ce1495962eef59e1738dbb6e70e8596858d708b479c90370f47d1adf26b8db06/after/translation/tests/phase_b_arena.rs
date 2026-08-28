//! Phase B — the string arena: `stbds_stralloc` / `stbds_strreset`.
//! Rows C37–C40 of CONFIGS.md and rows E46–E52 of ERRORS.md.
//!
//! Each library gets its own `stbds_string_arena` (the block chain is private
//! heap memory), so the comparison covers: the returned string content, the
//! `remaining` / `block` / `mode` scalars, the block-chain shape, and the fact
//! that every previously returned pointer still holds its string.
mod common;
use common::*;
use std::os::raw::c_char;

struct Arenas<'a> {
    p: &'a Pair,
    pub ca: Box<CArena>,
    pub ra: Box<CArena>,
    /// (returned pointer, expected length, FNV-1a checksum) per library
    hist_c: Vec<(*mut c_char, usize, u64)>,
    hist_r: Vec<(*mut c_char, usize, u64)>,
    op: usize,
}

impl<'a> Arenas<'a> {
    fn new(p: &'a Pair) -> Self {
        Arenas {
            p,
            ca: Box::new(CArena::zeroed()),
            ra: Box::new(CArena::zeroed()),
            hist_c: Vec::new(),
            hist_r: Vec::new(),
            op: 0,
        }
    }

    /// `stbds_stralloc(&arena, s)` on both libraries.
    #[track_caller]
    fn alloc(&mut self, s: &mut Vec<u8>, ctx: &str) {
        self.op += 1;
        assert_eq!(*s.last().unwrap(), 0, "keys must be NUL-terminated");
        let want_len = s.len() - 1;
        let want_sum = fnv(&s[..want_len]);
        unsafe {
            let cp = (self.p.c.stralloc)(&mut *self.ca, s.as_mut_ptr() as *mut c_char);
            let rp = (self.p.r.stralloc)(&mut *self.ra, s.as_mut_ptr() as *mut c_char);
            let (clen, csum) = scan(cp);
            let (rlen, rsum) = scan(rp);
            same_val(
                &format!("{ctx} [op {}] returned string", self.op),
                (clen, csum),
                (rlen, rsum),
            );
            same_val(
                &format!("{ctx} [op {}] returned string == input", self.op),
                (clen, csum),
                (want_len, want_sum),
            );
            self.hist_c.push((cp, want_len, want_sum));
            self.hist_r.push((rp, want_len, want_sum));
            self.check(ctx);
        }
    }

    /// Arena scalars + chain shape (cheap; run after every op).
    #[track_caller]
    fn check(&self, ctx: &str) {
        unsafe {
            same(
                &format!("{ctx} [op {}] arena", self.op),
                &snap_arena(&self.ca),
                &snap_arena(&self.ra),
            );
        }
    }

    /// Every pointer ever returned must still hold its string (the arena never
    /// moves data). Run at the end of a scenario — it is O(total bytes).
    #[track_caller]
    fn check_history(&self, ctx: &str) {
        unsafe {
            for (i, ((cp, cl, cs_), (rp, rl, rs))) in
                self.hist_c.iter().zip(self.hist_r.iter()).enumerate()
            {
                let (gc, hc) = scan(*cp);
                let (gr, hr) = scan(*rp);
                same_val(
                    &format!("{ctx} history #{i}: C vs Rust"),
                    (gc, hc),
                    (gr, hr),
                );
                same_val(
                    &format!("{ctx} history #{i}: content preserved"),
                    (gc, hc),
                    (*cl, *cs_),
                );
                let _ = (rl, rs);
            }
        }
    }

    fn reset(&mut self, ctx: &str) {
        self.check_history(ctx);
        self.op += 1;
        unsafe {
            (self.p.c.strreset)(&mut *self.ca);
            (self.p.r.strreset)(&mut *self.ra);
        }
        self.hist_c.clear();
        self.hist_r.clear();
        unsafe {
            same(
                &format!("{ctx} [op {}] after strreset", self.op),
                &snap_arena(&self.ca),
                &snap_arena(&self.ra),
            );
        }
        // must be fully zeroed
        assert_eq!(self.ca.remaining, 0);
        assert_eq!(self.ca.block, 0);
        assert_eq!(self.ca.mode, 0);
        assert!(self.ca.storage.is_null());
        assert_eq!(self.ra.remaining, 0);
        assert_eq!(self.ra.block, 0);
        assert_eq!(self.ra.mode, 0);
        assert!(self.ra.storage.is_null());
    }

    /// Override the scalar fields of both arenas identically.
    fn preset(&mut self, remaining: usize, block: u8, mode: u8) {
        self.ca.remaining = remaining;
        self.ca.block = block;
        self.ca.mode = mode;
        self.ra.remaining = remaining;
        self.ra.block = block;
        self.ra.mode = mode;
    }
}

/// FNV-1a over a byte slice (cheap stand-in for comparing megabyte payloads).
fn fnv(b: &[u8]) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// strlen + FNV-1a of a NUL-terminated C string.
unsafe fn scan(p: *const c_char) -> (usize, u64) {
    let mut h = 0xcbf29ce484222325u64;
    let mut n = 0usize;
    let mut q = p as *const u8;
    while *q != 0 {
        h ^= *q as u64;
        h = h.wrapping_mul(0x100000001b3);
        n += 1;
        q = q.add(1);
    }
    (n, h)
}

fn cs(n: usize, b: u8) -> Vec<u8> {
    let mut v = vec![b; n];
    v.push(0);
    v
}

// --- C37 / E46 / E47 / E50 : allocation shapes -------------------------------
#[test]
fn c37_stralloc_shapes() {
    let p = fresh_pair(0x37);
    for &len in &[0usize, 1, 2, 10, 100, 500, 510, 511, 512, 513, 1000, 4096] {
        let mut a = Arenas::new(&p);
        let mut s = cs(len, b'x');
        a.alloc(&mut s, &format!("c37 len={len} first"));
        // and again into the same arena
        let mut s2 = cs(len, b'y');
        a.alloc(&mut s2, &format!("c37 len={len} second"));
        a.reset(&format!("c37 len={len}"));
    }
}

#[test]
fn e47_stralloc_first_block() {
    let p = fresh_pair(0x47a);
    let mut a = Arenas::new(&p);
    let mut s = cs(10, b'a');
    a.alloc(&mut s, "e47");
    // fresh 512-byte block: remaining == 512 - 11, block advanced to 1
    same_val("e47 remaining", a.ca.remaining, 512usize - 11);
    same_val("e47 block", a.ca.block, 1u8);
    same_val("e47 remaining parity", a.ca.remaining, a.ra.remaining);
    same_val("e47 block parity", a.ca.block, a.ra.block);
    a.reset("e47");
}

#[test]
fn e50_stralloc_empty_string() {
    let p = fresh_pair(0x50);
    let mut a = Arenas::new(&p);
    for i in 0..600 {
        let mut s = vec![0u8];
        a.alloc(&mut s, &format!("e50 empty#{i}"));
    }
    a.reset("e50");
}

#[test]
fn e46_stralloc_oversized() {
    let p = fresh_pair(0x46a);
    // (a) very first call, storage == NULL -> block becomes the storage,
    //     remaining forced to 0
    {
        let mut a = Arenas::new(&p);
        let mut s = cs(5000, b'q');
        a.alloc(&mut s, "e46 oversized-first");
        same_val("e46 remaining==0", a.ca.remaining, 0usize);
        same_val("e46 remaining parity", a.ca.remaining, a.ra.remaining);
        same_val("e46 block", a.ca.block, a.ra.block);
        a.reset("e46 oversized-first");
    }
    // (b) storage already present -> the big block is spliced in AFTER storage
    {
        let mut a = Arenas::new(&p);
        let mut small = cs(10, b's');
        a.alloc(&mut small, "e46 small");
        let rem_before = a.ca.remaining;
        let mut big = cs(5000, b'B');
        a.alloc(&mut big, "e46 oversized-second");
        same_val("e46 remaining unchanged", a.ca.remaining, rem_before);
        same_val("e46 remaining parity", a.ca.remaining, a.ra.remaining);
        // chain must have grown by exactly one on both sides
        let mut more = cs(20, b'm');
        a.alloc(&mut more, "e46 after-oversized");
        a.reset("e46 oversized-second");
    }
}

// --- C38 / E48 : block growth and saturation --------------------------------
#[test]
fn c38_stralloc_block_growth() {
    let p = fresh_pair(0x38);
    let mut a = Arenas::new(&p);
    // Force a new block on every call by asking for more than the current
    // blocksize; this is the fastest way to walk `block` from 0 up to its
    // saturation point.
    let mut blocks_c = Vec::new();
    // ask for slightly more than the current blocksize each time -> one new
    // block (and one ++block) per call. Capped at 512<<6 == 32 KiB so the test
    // stays cheap; e48_stralloc_block_saturates covers block 12..23 by preset.
    for i in 0..14usize {
        let want = (512usize << (i / 2).min(6)) + 1;
        let mut s = cs(want, b'#');
        a.alloc(&mut s, &format!("c38 i={i} len={want}"));
        blocks_c.push(a.ca.block);
        same_val(&format!("c38 i={i} block parity"), a.ca.block, a.ra.block);
        same_val(
            &format!("c38 i={i} remaining parity"),
            a.ca.remaining,
            a.ra.remaining,
        );
    }
    same_val("c38 block advances once per new block", blocks_c[13], 14u8);
    a.reset("c38");
}

#[test]
fn e48_stralloc_block_saturates() {
    let p = fresh_pair(0x48);
    let mut a = Arenas::new(&p);
    // step `block` up one at a time and check the derived blocksize behaviour
    for block in 0u8..=23 {
        a.preset(0, block, 0);
        let mut s = cs(1, b'z');
        a.alloc(&mut s, &format!("e48 block={block}"));
        let expect_next = if (512usize << (block >> 1)) < (1usize << 20) {
            block + 1
        } else {
            block
        };
        same_val(&format!("e48 block={block} -> next"), a.ca.block, expect_next);
        same_val(
            &format!("e48 block={block} parity"),
            (a.ca.block, a.ca.remaining),
            (a.ra.block, a.ra.remaining),
        );
        a.reset(&format!("e48 block={block}"));
    }
}

// --- C39 / E49 : preset arenas, incl. shift-count overflow -------------------
#[test]
fn c39_stralloc_preset_arena() {
    let p = fresh_pair(0x39);
    // `block` values whose derived blocksize stays either small (<= 1 MiB) or
    // collapses to 0 through the x86-64 `shl` count masking; anything in between
    // would make the C code request tens of terabytes and abort in *both*
    // libraries, which cannot be observed in-process.
    const SAFE_BLOCKS: &[u8] = &[
        0, 1, 2, 3, 4, 5, 10, 11, 20, 21, 22, 23, 110, 111, 120, 126, 127, 128, 129, 130, 150,
        151, 238, 239, 250, 254, 255,
    ];
    for &block in SAFE_BLOCKS {
        for &remaining in &[0usize, 1, 2, 5, 11, 100] {
            for &len in &[0usize, 1, 4, 10, 100] {
                let mut a = Arenas::new(&p);
                // create a real block first so that `storage != NULL` whenever
                // `remaining > 0` (otherwise the C dereferences NULL).
                if remaining > 0 {
                    let mut seed = cs(1, b'0');
                    a.alloc(&mut seed, "c39 seed");
                    let real = a.ca.remaining;
                    a.preset(remaining.min(real), block, 0);
                } else {
                    a.preset(0, block, 0);
                }
                let mut s = cs(len, b'K');
                a.alloc(
                    &mut s,
                    &format!("c39 block={block} remaining={remaining} len={len}"),
                );
                a.reset(&format!("c39 block={block} remaining={remaining} len={len}"));
            }
        }
    }
}

#[test]
fn e49_stralloc_block_shift_overflow() {
    let p = fresh_pair(0x49);
    // (block>>1) >= 64 : the C shift count is taken mod 64
    for &block in &[128u8, 129, 130, 131, 150, 151, 238, 250, 254, 255] {
        let mut a = Arenas::new(&p);
        a.preset(0, block, 0);
        let mut s = cs(7, b'V');
        a.alloc(&mut s, &format!("e49 block={block}"));
        same_val(
            &format!("e49 block={block} scalars"),
            (a.ca.block, a.ca.remaining),
            (a.ra.block, a.ra.remaining),
        );
        // a second allocation exercises whichever branch the first one chose
        let mut s2 = cs(3, b'W');
        a.alloc(&mut s2, &format!("e49 block={block} second"));
        a.reset(&format!("e49 block={block}"));
    }
}

// --- C40 / E51 / E52 : strreset ---------------------------------------------
#[test]
fn c40_strreset_shapes() {
    let p = fresh_pair(0x40);
    // (a) never-used arena
    {
        let mut a = Arenas::new(&p);
        a.reset("c40 empty");
        a.reset("c40 empty twice");
    }
    // (b) one block
    {
        let mut a = Arenas::new(&p);
        let mut s = cs(10, b'1');
        a.alloc(&mut s, "c40 one");
        a.reset("c40 one");
    }
    // (c) many blocks
    {
        let mut a = Arenas::new(&p);
        for i in 0..30 {
            let mut s = cs(400 + i * 7, b'2');
            a.alloc(&mut s, &format!("c40 many#{i}"));
        }
        a.reset("c40 many");
    }
    // (d) chain that includes an oversized spliced block
    {
        let mut a = Arenas::new(&p);
        let mut s = cs(5, b'3');
        a.alloc(&mut s, "c40 mixed small");
        let mut b = cs(9000, b'4');
        a.alloc(&mut b, "c40 mixed big");
        let mut c = cs(5, b'5');
        a.alloc(&mut c, "c40 mixed small2");
        a.reset("c40 mixed");
    }
    // (e) mode field is also zeroed
    {
        let mut a = Arenas::new(&p);
        a.preset(0, 7, 3);
        let mut s = cs(4, b'6');
        a.alloc(&mut s, "c40 mode");
        a.reset("c40 mode");
    }
}

#[test]
fn e51_strreset_empty() {
    let p = fresh_pair(0x51);
    let mut a = Arenas::new(&p);
    for b in [0u8, 1, 7, 22, 255] {
        a.preset(0, b, 2);
        a.reset(&format!("e51 block={b}"));
    }
}

#[test]
fn e52_strreset_chain() {
    let p = fresh_pair(0x52);
    let mut rng = Rng::new(0x52);
    for round in 0..10 {
        let mut a = Arenas::new(&p);
        for i in 0..40 {
            let n = rng.below(1200);
            let mut s = cs(n, b'a'.wrapping_add((i % 20) as u8));
            a.alloc(&mut s, &format!("e52 r={round} i={i} n={n}"));
        }
        a.reset(&format!("e52 r={round}"));
    }
}

// --- randomized arena property test -----------------------------------------
#[test]
fn c37_stralloc_random() {
    let p = fresh_pair(0x37f);
    let mut rng = Rng::new(0x37f);
    for round in 0..12 {
        let mut a = Arenas::new(&p);
        for i in 0..60 {
            let n = match rng.below(5) {
                0 => 0,
                1 => 1 + rng.below(16),
                2 => 480 + rng.below(64),
                3 => rng.below(2200),
                _ => rng.below(200),
            };
            let mut s = rng.cstring(n, ASCII);
            a.alloc(&mut s, &format!("c37r round={round} i={i} n={n}"));
        }
        a.reset(&format!("c37r round={round}"));
    }
}
