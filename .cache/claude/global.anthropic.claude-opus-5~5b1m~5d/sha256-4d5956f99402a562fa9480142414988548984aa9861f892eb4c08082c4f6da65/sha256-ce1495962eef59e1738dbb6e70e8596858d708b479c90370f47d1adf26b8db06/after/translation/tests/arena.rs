//! Phase B — CONFIGS.md rows 25..33
//! `stbds_stralloc` / `stbds_strreset`

mod common;
use common::*;
use std::ffi::c_char;

#[derive(Debug, PartialEq, Eq, Clone)]
struct ArenaSnap {
    remaining: usize,
    block: u8,
    mode: u8,
    has_storage: bool,
    chain_len: usize,
}

/// `struct stbds_string_block { struct stbds_string_block *next; char storage[8]; }`
unsafe fn chain_len(a: &StringArena) -> usize {
    let mut n = 0usize;
    let mut p = a.storage as *mut *mut u8; // first field is `next`
    while !p.is_null() {
        n += 1;
        if n > 1_000_000 {
            panic!("arena chain looks circular");
        }
        p = *p as *mut *mut u8;
    }
    n
}

unsafe fn asnap(a: &StringArena) -> ArenaSnap {
    ArenaSnap {
        remaining: a.remaining,
        block: a.block,
        mode: a.mode,
        has_storage: !a.storage.is_null(),
        chain_len: chain_len(a),
    }
}

/// A pair of arenas driven in lock-step.
struct ArenaPair {
    ac: StringArena,
    ar: StringArena,
    /// every string handed out so far: (c_ptr, r_ptr, expected bytes)
    live: Vec<(*mut c_char, *mut c_char, Vec<u8>)>,
    ctx: String,
    step: usize,
}

impl ArenaPair {
    fn new(ctx: impl Into<String>) -> ArenaPair {
        ArenaPair {
            ac: StringArena::zeroed(),
            ar: StringArena::zeroed(),
            live: Vec::new(),
            ctx: ctx.into(),
            step: 0,
        }
    }

    unsafe fn check(&self, what: &str) {
        let sc = asnap(&self.ac);
        let sr = asnap(&self.ar);
        assert_eq!(
            sc, sr,
            "DIVERGENCE [{}] step {} after `{}`",
            self.ctx, self.step, what
        );
        // every previously returned string must still read back correctly in
        // BOTH implementations
        for (i, (pc, pr, want)) in self.live.iter().enumerate() {
            let gc = read_cstr(*pc);
            let gr = read_cstr(*pr);
            assert_eq!(
                &gc, want,
                "[{}] step {} live string #{} corrupted in C",
                self.ctx, self.step, i
            );
            assert_eq!(
                &gr, want,
                "[{}] step {} live string #{} corrupted in Rust",
                self.ctx, self.step, i
            );
        }
    }

    /// `s` must be NUL-terminated.
    unsafe fn alloc(&mut self, s: &mut [u8]) {
        let p = pair();
        let len = s.len(); // strlen + 1, exactly what the C computes
        let bc = (self.ac, chain_len(&self.ac));
        let br = (self.ar, chain_len(&self.ar));
        assert_eq!(asnap(&self.ac), asnap(&self.ar), "[{}] pre-state", self.ctx);
        let pc = (p.c.stralloc)(&mut self.ac, s.as_mut_ptr() as *mut c_char);
        let pr = (p.r.stralloc)(&mut self.ar, s.as_mut_ptr() as *mut c_char);
        let payload: Vec<u8> = s[..s.len() - 1].to_vec();
        assert_eq!(read_cstr(pc), payload, "[{}] C returned wrong bytes", self.ctx);
        assert_eq!(
            read_cstr(pr),
            payload,
            "[{}] Rust returned wrong bytes",
            self.ctx
        );
        // The returned address itself is allocator dependent, so instead assert
        // that EACH implementation placed the string exactly where the C source
        // says it must go (which pins down the block/offset arithmetic).
        verify_placement("C", &self.ctx, self.step, bc.0, bc.1, &self.ac, chain_len(&self.ac), pc, len);
        verify_placement("Rust", &self.ctx, self.step, br.0, br.1, &self.ar, chain_len(&self.ar), pr, len);
        self.live.push((pc, pr, payload));
        self.step += 1;
        self.check("stralloc");
    }

    unsafe fn reset(&mut self) {
        let p = pair();
        (p.c.strreset)(&mut self.ac);
        (p.r.strreset)(&mut self.ar);
        self.live.clear();
        self.step += 1;
        self.check("strreset");
        assert_eq!(self.ac.remaining, 0);
        assert_eq!(self.ac.block, 0);
        assert_eq!(self.ac.mode, 0);
        assert!(self.ac.storage.is_null());
        assert_eq!(self.ar.remaining, 0);
        assert_eq!(self.ar.block, 0);
        assert_eq!(self.ar.mode, 0);
        assert!(self.ar.storage.is_null());
    }
}

const BLOCKSIZE_MIN: usize = 512;
const BLOCKSIZE_MAX: usize = 1 << 20;

/// Exact model of `stbds_stralloc` (`lib.c:881-918`).
#[allow(clippy::too_many_arguments)]
unsafe fn verify_placement(
    who: &str,
    ctx: &str,
    step: usize,
    before: StringArena,
    chain0: usize,
    after: &StringArena,
    chain1: usize,
    p: *mut c_char,
    len: usize,
) {
    let head0 = before.storage as *mut u8;
    let head1 = after.storage as *mut u8;
    let tag = format!("[{}/{}] step {} len={}", ctx, who, step, len);

    if len <= before.remaining {
        // fast path: bump inside the current head block
        assert_eq!(head1, head0, "{}: head must not change", tag);
        assert_eq!(after.block, before.block, "{}: block must not change", tag);
        assert_eq!(chain1, chain0, "{}: chain must not change", tag);
        assert_eq!(
            after.remaining,
            before.remaining - len,
            "{}: remaining",
            tag
        );
        assert_eq!(
            p as usize,
            head0 as usize + 8 + after.remaining,
            "{}: bump offset",
            tag
        );
        return;
    }

    let bs = BLOCKSIZE_MIN << (before.block >> 1);
    let expect_block = if bs < BLOCKSIZE_MAX {
        before.block + 1
    } else {
        before.block
    };
    assert_eq!(after.block, expect_block, "{}: block counter (bs={})", tag, bs);

    if len > bs {
        // oversized: the string gets its own block
        assert_eq!(chain1, chain0 + 1, "{}: chain grows by one", tag);
        if head0.is_null() {
            assert!(!head1.is_null(), "{}: new head", tag);
            assert_eq!(after.remaining, 0, "{}: remaining forced to 0", tag);
            assert_eq!(p as usize, head1 as usize + 8, "{}: p == sb->storage", tag);
        } else {
            assert_eq!(head1, head0, "{}: head unchanged", tag);
            assert_eq!(
                after.remaining, before.remaining,
                "{}: remaining untouched",
                tag
            );
            let spliced = *(head0 as *mut *mut u8);
            assert_eq!(
                p as usize,
                spliced as usize + 8,
                "{}: p must be in head->next",
                tag
            );
        }
    } else {
        // fresh block of `bs` bytes becomes the new head
        assert_eq!(chain1, chain0 + 1, "{}: chain grows by one", tag);
        assert_ne!(head1, head0, "{}: new head block", tag);
        assert_eq!(after.remaining, bs - len, "{}: remaining", tag);
        assert_eq!(
            p as usize,
            head1 as usize + 8 + after.remaining,
            "{}: bump offset in new block",
            tag
        );
    }
}

// -------------------------------------------------------------------- row 25
#[test]
fn c25_stralloc_fresh_small() {
    let mut rng = Rng::new(0x2525);
    for &len in &[0usize, 1, 2, 7, 8, 9, 15, 16, 63, 64, 255, 500, 510, 511] {
        for _ in 0..8 {
            unsafe {
                let mut ap = ArenaPair::new(format!("fresh len={}", len));
                let mut s = rng.cstring(len, ASCII);
                ap.alloc(&mut s);
                // fresh 512-byte block, len+1 bytes consumed
                assert_eq!(ap.ac.remaining, 512 - (len + 1));
                assert_eq!(ap.ac.block, 1);
                assert_eq!(chain_len(&ap.ac), 1);
                ap.reset();
            }
        }
    }
}

// -------------------------------------------------------------------- row 26
#[test]
fn c26_stralloc_oversized_first() {
    let mut rng = Rng::new(0x2626);
    // len (== strlen+1) must exceed blocksize 512
    for &payload in &[512usize, 513, 1023, 1024, 4096, 5000] {
        unsafe {
            let mut ap = ArenaPair::new(format!("oversized-first payload={}", payload));
            let mut s = rng.cstring(payload, ASCII);
            ap.alloc(&mut s);
            // a->storage was NULL -> new block becomes the head, remaining := 0
            assert_eq!(ap.ac.remaining, 0, "remaining must be forced to 0");
            assert_eq!(ap.ac.block, 1, "block still incremented");
            assert_eq!(chain_len(&ap.ac), 1);
            // a following small string therefore needs a brand new block
            let mut t = rng.cstring(10, ASCII);
            ap.alloc(&mut t);
            assert_eq!(chain_len(&ap.ac), 2);
            ap.reset();
        }
    }
}

// -------------------------------------------------------------------- row 27
#[test]
fn c27_stralloc_many_small() {
    let mut rng = Rng::new(0x2727);
    unsafe {
        let mut ap = ArenaPair::new("many-small");
        for i in 0..600 {
            let len = 1 + (i % 40);
            let mut s = rng.cstring(len, ASCII);
            ap.alloc(&mut s);
        }
        assert!(ap.ac.block >= 8, "expected several block growths, got {}", ap.ac.block);
        ap.reset();
    }
}

// -------------------------------------------------------------------- row 28
#[test]
fn c28_stralloc_mixed_oversized_after() {
    let mut rng = Rng::new(0x2828);
    unsafe {
        let mut ap = ArenaPair::new("mixed");
        // establish a head block first
        let mut s = rng.cstring(10, ASCII);
        ap.alloc(&mut s);
        let rem_before = ap.ac.remaining;
        // now an oversized string: spliced in AFTER the head, `remaining`
        // deliberately left untouched by the C code
        let mut big = rng.cstring(4096, ASCII);
        ap.alloc(&mut big);
        assert_eq!(
            ap.ac.remaining, rem_before,
            "the oversized-after path must not touch `remaining`"
        );
        assert_eq!(chain_len(&ap.ac), 2);
        // more small strings keep coming out of the (still current) head block
        for _ in 0..20 {
            let mut t = rng.cstring(5, ASCII);
            ap.alloc(&mut t);
        }
        // interleave more oversized ones
        for k in 0..6 {
            let mut b = rng.cstring(3000 + k * 700, ASCII);
            ap.alloc(&mut b);
        }
        ap.reset();
    }
}

// -------------------------------------------------------------------- row 29
#[test]
fn c29_stralloc_exact_exhaustion() {
    let mut rng = Rng::new(0x2929);
    unsafe {
        let mut ap = ArenaPair::new("exact");
        let mut s = rng.cstring(100, ASCII);
        ap.alloc(&mut s);
        let rem = ap.ac.remaining; // 512 - 101 = 411
        assert_eq!(rem, 411);
        // consume exactly the rest (len == remaining)
        let mut t = rng.cstring(rem - 1, ASCII);
        ap.alloc(&mut t);
        assert_eq!(ap.ac.remaining, 0);
        assert_eq!(chain_len(&ap.ac), 1);
        // one more byte must open a new block
        let mut u = rng.cstring(0, ASCII);
        ap.alloc(&mut u);
        assert_eq!(chain_len(&ap.ac), 2);
        assert_eq!(ap.ac.remaining, 512 - 1);
        ap.reset();
    }
}

// -------------------------------------------------------------------- row 30
#[test]
fn c30_stralloc_block_saturation() {
    let mut rng = Rng::new(0x3030);
    unsafe {
        let mut ap = ArenaPair::new("saturation");
        // Always ask for slightly more than the current blocksize so every call
        // takes the oversized path and therefore bumps `a->block` by one.
        for i in 0..27 {
            let bs = BLOCKSIZE_MIN << (ap.ac.block >> 1);
            let mut s = rng.cstring(bs + 100, ASCII);
            ap.alloc(&mut s);
            assert_eq!(ap.ac.block, ap.ar.block, "block divergence at call {}", i);
        }
        assert_eq!(
            ap.ac.block, 22,
            "block must saturate at 22 (512<<11 == 1<<20 is not < 1<<20)"
        );
        assert_eq!(ap.ar.block, 22);
        // further calls must not bump it any more
        for _ in 0..3 {
            let mut s = rng.cstring(BLOCKSIZE_MAX + 100, ASCII);
            ap.alloc(&mut s);
            assert_eq!(ap.ac.block, 22);
            assert_eq!(ap.ar.block, 22);
        }
        ap.reset();
    }

    // and the *normal* path also saturates: 4999-byte payloads
    unsafe {
        let mut ap = ArenaPair::new("saturation-normal");
        for _ in 0..40 {
            let mut s = rng.cstring(4999, ASCII);
            ap.alloc(&mut s);
        }
        assert_eq!(ap.ac.block, ap.ar.block);
        assert!(ap.ac.block >= 14, "block should have climbed, got {}", ap.ac.block);
        ap.reset();
    }
}

// -------------------------------------------------------------------- row 31
#[test]
fn c31_stralloc_high_bytes() {
    let mut rng = Rng::new(0x3131);
    let high: Vec<u8> = (0x01u8..=0xFFu8).collect();
    unsafe {
        let mut ap = ArenaPair::new("high-bytes");
        for i in 0..300 {
            let len = 1 + (i * 7) % 90;
            let mut s = rng.cstring(len, &high);
            ap.alloc(&mut s);
        }
        ap.reset();
    }
}

// -------------------------------------------------------------------- row 32
#[test]
fn c32_strreset_shapes() {
    let mut rng = Rng::new(0x3232);
    unsafe {
        // empty arena
        let mut ap = ArenaPair::new("reset-empty");
        ap.check("initial");
        ap.reset();
        ap.reset();

        // single block
        let mut s = rng.cstring(10, ASCII);
        ap.alloc(&mut s);
        ap.reset();

        // many blocks
        for i in 0..200 {
            let mut t = rng.cstring(1 + i % 60, ASCII);
            ap.alloc(&mut t);
        }
        ap.reset();

        // chain containing an oversized block
        let mut a1 = rng.cstring(10, ASCII);
        ap.alloc(&mut a1);
        let mut a2 = rng.cstring(9000, ASCII);
        ap.alloc(&mut a2);
        let mut a3 = rng.cstring(20, ASCII);
        ap.alloc(&mut a3);
        ap.reset();

        // reuse after reset behaves exactly like a fresh arena
        let mut a4 = rng.cstring(10, ASCII);
        ap.alloc(&mut a4);
        assert_eq!(ap.ac.block, 1);
        assert_eq!(ap.ac.remaining, 512 - 11);
        ap.reset();
    }
}

// -------------------------------------------------------------------- row 33
#[test]
fn c33_stralloc_randomized() {
    let mut rng = Rng::new(0x3333);
    for seq in 0..512 {
        unsafe {
            let mut ap = ArenaPair::new(format!("rand seq {}", seq));
            let nops = 1 + rng.below(24);
            for _ in 0..nops {
                match rng.below(20) {
                    0 => ap.reset(),
                    1..=3 => {
                        // oversized-ish
                        let len = 400 + rng.below(5000);
                        let mut s = rng.cstring(len, HIGHBYTES);
                        ap.alloc(&mut s);
                    }
                    _ => {
                        let len = rng.below(200);
                        let mut s = rng.cstring(len, ASCII);
                        ap.alloc(&mut s);
                    }
                }
            }
            ap.reset();
        }
    }
}
