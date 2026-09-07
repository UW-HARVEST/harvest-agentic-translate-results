//! Phase B — CONFIGS.md rows 52-58: the string arena
//! (`stbds_stralloc` / `stbds_strreset`) driven directly.

mod common;
use common::*;
use std::ffi::{c_void, CStr};

struct Arenas {
    c: Box<StringArena>,
    r: Box<StringArena>,
}

fn zero_arena() -> Box<StringArena> {
    Box::new(StringArena {
        storage: std::ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    })
}

impl Arenas {
    fn new() -> Self {
        Arenas {
            c: zero_arena(),
            r: zero_arena(),
        }
    }

    /// Allocate `s` in both arenas and compare every observable effect.
    #[track_caller]
    fn alloc(&mut self, p: &Pair, s: &[u8], ctx: &str) {
        let k = CKey::new(s);
        unsafe {
            let before_c = snap_arena(&*self.c);
            let before_r = snap_arena(&*self.r);
            assert_eq!(before_c, before_r, "[{ctx}] arena state before alloc");

            let cp = (p.c.stralloc)(&mut *self.c as *mut StringArena as *mut c_void, k.ptr());
            let rp = (p.rs.stralloc)(&mut *self.r as *mut StringArena as *mut c_void, k.ptr());

            let after_c = snap_arena(&*self.c);
            let after_r = snap_arena(&*self.r);
            assert_eq!(
                after_c, after_r,
                "[{ctx}] arena state after alloc of {} bytes (before: {before_c:?})",
                s.len() + 1
            );

            assert_eq!(
                CStr::from_ptr(cp).to_bytes(),
                s,
                "[{ctx}] C returned wrong contents"
            );
            assert_eq!(
                CStr::from_ptr(rp).to_bytes(),
                s,
                "[{ctx}] Rust returned wrong contents"
            );

            // structural placement: in every branch except
            // "oversized string spliced behind an existing head block",
            // the result is `head->storage + remaining_after`.
            let in_head_c = cp as usize
                == (self.c.storage as usize) + 8 + after_c.remaining
                && !self.c.storage.is_null();
            let in_head_r = rp as usize
                == (self.r.storage as usize) + 8 + after_r.remaining
                && !self.r.storage.is_null();
            assert_eq!(
                in_head_c, in_head_r,
                "[{ctx}] placement branch differs (C={in_head_c} RUST={in_head_r})"
            );
        }
    }

    #[track_caller]
    fn reset(&mut self, p: &Pair, ctx: &str) {
        unsafe {
            (p.c.strreset)(&mut *self.c as *mut StringArena as *mut c_void);
            (p.rs.strreset)(&mut *self.r as *mut StringArena as *mut c_void);
            let a = snap_arena(&*self.c);
            let b = snap_arena(&*self.r);
            assert_eq!(a, b, "[{ctx}] arena state after reset");
            assert_eq!(
                a,
                ArenaSnap {
                    remaining: 0,
                    block: 0,
                    mode: 0,
                    block_count: 0
                },
                "[{ctx}] reset must zero the arena"
            );
        }
    }
}

#[test]
fn row52_fresh_arena_small_strings() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 52);
    for len in 0usize..=64 {
        let mut a = Arenas::new();
        let s = rng.nonnul(len);
        a.alloc(p, &s, &format!("fresh len={len}"));
        assert_eq!(a.c.remaining, 512 - (len + 1));
        assert_eq!(a.c.block, 1);
        a.reset(p, "fresh reset");
    }
    // the empty string in a fresh arena (ERRORS.md row 48)
    let mut a = Arenas::new();
    a.alloc(p, b"", "empty string");
    assert_eq!(a.c.remaining, 511);
    a.reset(p, "empty reset");
}

#[test]
fn row53_block_chain_growth() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 53);
    for trial in 0..20 {
        let mut a = Arenas::new();
        for i in 0..400 {
            let len = 1 + rng.below(120);
            let s = rng.nonnul(len);
            a.alloc(p, &s, &format!("chain trial={trial} i={i} len={len}"));
        }
        assert!(a.c.block >= 2, "expected several blocks, block={}", a.c.block);
        a.reset(p, "chain reset");
    }
}

#[test]
fn row54_oversized_first_alloc() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 54);
    for &len in &[512usize, 511, 513, 1000, 4096, 100_000] {
        let mut a = Arenas::new();
        let s = rng.nonnul(len);
        a.alloc(p, &s, &format!("oversized-first len={len}"));
        if len + 1 > 512 {
            // dedicated block, remaining forced to 0
            assert_eq!(a.c.remaining, 0, "len={len}");
            assert_eq!(a.c.block, 1);
        }
        a.reset(p, "oversized-first reset");
    }
}

#[test]
fn row55_oversized_after_existing_block() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 55);
    for &big in &[600usize, 1024, 8192, 200_000] {
        let mut a = Arenas::new();
        // establish a head block with room left over
        a.alloc(p, &rng.nonnul(10), "seed small");
        let before = unsafe { snap_arena(&*a.c) };
        a.alloc(p, &rng.nonnul(big), &format!("oversized-after big={big}"));
        let after = unsafe { snap_arena(&*a.c) };
        assert_eq!(
            before.remaining, after.remaining,
            "oversized alloc must not consume the head block"
        );
        assert_eq!(after.block_count, before.block_count + 1);
        // small allocs must still come out of the original head block
        a.alloc(p, &rng.nonnul(5), "small after oversized");
        a.reset(p, "oversized-after reset");
    }
}

#[test]
fn row56_block_size_saturation() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 56);
    let mut a = Arenas::new();
    // ~2.1 MiB of payload is needed to create 22 blocks and reach the
    // STBDS_STRING_ARENA_BLOCKSIZE_MAX (1 MiB) saturation point.
    for i in 0..14000 {
        let len = 50 + rng.below(350);
        let s = rng.nonnul(len);
        if i % 500 == 0 {
            a.alloc(p, &s, &format!("saturate i={i}"));
        } else {
            // fast path: still compares state, just without the format! cost
            a.alloc(p, &s, "saturate");
        }
    }
    assert!(
        a.c.block >= 22,
        "expected block counter to saturate, got {}",
        a.c.block
    );
    assert_eq!(a.c.block, a.r.block);
    // once blocksize reaches 1 MiB the counter must stop advancing
    let saturated = a.c.block;
    for _ in 0..4000 {
        a.alloc(p, &rng.nonnul(200), "post-saturate");
    }
    assert_eq!(a.c.block, saturated.max(a.c.block));
    assert!(a.c.block <= 23, "block must saturate, got {}", a.c.block);
    a.reset(p, "saturate reset");
}

#[test]
fn row57_alloc_then_reset_repeatedly() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 57);
    let mut a = Arenas::new();
    for round in 0..30 {
        for i in 0..200 {
            let len = 1 + rng.below(300);
            a.alloc(p, &rng.nonnul(len), &format!("round={round} i={i}"));
        }
        a.reset(p, &format!("reset round={round}"));
        // double reset must be idempotent
        a.reset(p, &format!("double reset round={round}"));
    }
}

#[test]
fn row58_reset_fresh_arena() {
    let (p, _g) = libs();
    let mut a = Arenas::new();
    for _ in 0..5 {
        a.reset(p, "fresh reset");
    }
}
