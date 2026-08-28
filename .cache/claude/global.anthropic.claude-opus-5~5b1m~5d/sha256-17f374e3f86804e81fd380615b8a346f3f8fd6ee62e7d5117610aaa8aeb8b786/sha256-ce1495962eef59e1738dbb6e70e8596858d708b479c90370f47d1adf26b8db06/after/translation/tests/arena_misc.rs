//! Phase B — CONFIGS.md rows 57..=64: the string arena (`stbds_stralloc` /
//! `stbds_strreset`) driven directly with a caller-owned arena, plus `strkey`
//! and `arr_del`.

mod common;
use common::*;
use std::ffi::c_char;

const BLOCKSIZE_MIN: usize = 512;
const BLOCKSIZE_MAX: usize = 1 << 20;

struct DualArena {
    ac: StringArena,
    ar: StringArena,
}

impl DualArena {
    fn new() -> Self {
        DualArena {
            ac: StringArena::new(),
            ar: StringArena::new(),
        }
    }
    /// `stbds_stralloc` on both libraries; compares arena state and the
    /// returned string contents.
    fn alloc(&mut self, s: &mut [u8], what: &str) {
        assert_eq!(*s.last().unwrap(), 0);
        let p = common::libs();
        let pc = unsafe { (p.c.stralloc)(&raw mut self.ac, s.as_mut_ptr() as *mut c_char) };
        let pr = unsafe { (p.r.stralloc)(&raw mut self.ar, s.as_mut_ptr() as *mut c_char) };
        let gc = unsafe { cstr_bytes(pc) };
        let gr = unsafe { cstr_bytes(pr) };
        assert_eq!(gc, gr, "stralloc contents diverged: {what}");
        assert_eq!(
            gc.as_deref(),
            Some(&s[..s.len() - 1]),
            "stralloc did not return the input string: {what}"
        );
        let sc = unsafe { arena_snap(&raw const self.ac) };
        let sr = unsafe { arena_snap(&raw const self.ar) };
        assert_eq!(sc, sr, "arena state diverged: {what}");
        // the returned pointer must lie inside the head block for the
        // bump-allocated case, which `remaining` already captures
    }
    fn snap(&self) -> (ArenaSnap, ArenaSnap) {
        unsafe {
            (
                arena_snap(&raw const self.ac),
                arena_snap(&raw const self.ar),
            )
        }
    }
    fn reset(&mut self) {
        let p = common::libs();
        unsafe {
            (p.c.strreset)(&raw mut self.ac);
            (p.r.strreset)(&raw mut self.ar);
        }
        let (a, b) = self.snap();
        assert_eq!(a, b, "arena state diverged after strreset");
        assert_eq!(a.remaining, 0);
        assert_eq!(a.block, 0);
        assert_eq!(a.mode, 0);
        assert_eq!(a.chain, 0);
        assert!(a.storage_null);
    }
}

fn mkstr(len: usize, tag: u8) -> Vec<u8> {
    let mut v: Vec<u8> = (0..len)
        .map(|i| 0x41 + ((i as u8).wrapping_add(tag) % 26))
        .collect();
    v.push(0);
    v
}

/// row 57 — fresh arena, short string: one 512-byte block, `remaining` trace
#[test]
fn r57_stralloc_fresh_short() {
    let _g = lock_libs();
    for len in [0usize, 1, 2, 7, 8, 100, 510, 511] {
        let mut a = DualArena::new();
        let mut s = mkstr(len, len as u8);
        a.alloc(&mut s, &format!("fresh len={len}"));
        let (sc, _) = a.snap();
        assert_eq!(sc.chain, 1, "one block expected");
        assert_eq!(sc.remaining, BLOCKSIZE_MIN - (len + 1));
        assert_eq!(sc.block, 1, "block must have been incremented");
        // a second string that still fits must be bump-allocated from the same
        // block (no new block, so `block` stays put)
        let len2 = 20usize;
        if len2 + 1 <= sc.remaining {
            let mut s2 = mkstr(len2, 7);
            a.alloc(&mut s2, "second, same block");
            let (sc2, _) = a.snap();
            assert_eq!(sc2.chain, 1);
            assert_eq!(sc2.block, 1, "no new block -> block unchanged");
            assert_eq!(sc2.remaining, sc.remaining - (len2 + 1));
        } else {
            // ... otherwise a fresh block is allocated and `block` advances
            let mut s2 = mkstr(len2, 7);
            a.alloc(&mut s2, "second, new block");
            let (sc2, _) = a.snap();
            assert_eq!(sc2.chain, 2);
            assert_eq!(sc2.block, 2);
        }
        a.reset();
    }
}

/// row 58 — fresh arena, string longer than the block size: dedicated block,
/// `a->storage == NULL` branch, `remaining` forced to 0
#[test]
fn r58_stralloc_fresh_oversized() {
    let _g = lock_libs();
    for len in [512usize, 513, 1000, 4096, 100_000] {
        let mut a = DualArena::new();
        let mut s = mkstr(len, 3);
        a.alloc(&mut s, &format!("fresh oversized len={len}"));
        let (sc, _) = a.snap();
        assert_eq!(sc.chain, 1);
        assert_eq!(sc.remaining, 0, "dedicated-block path sets remaining = 0");
        assert_eq!(sc.block, 1);
        a.reset();
    }
}

/// row 59 — non-empty arena + over-sized string: the dedicated block is spliced
/// in *after* the head block and `remaining` is left untouched
#[test]
fn r59_stralloc_oversized_after_head() {
    let _g = lock_libs();
    let mut a = DualArena::new();
    let mut small = mkstr(10, 1);
    a.alloc(&mut small, "head block");
    let (before, _) = a.snap();
    assert_eq!(before.chain, 1);

    let mut big = mkstr(5000, 2);
    a.alloc(&mut big, "oversized after head");
    let (after, _) = a.snap();
    assert_eq!(after.chain, 2, "the dedicated block must be spliced in");
    assert_eq!(
        after.remaining, before.remaining,
        "remaining must be untouched on the oversized-with-head path"
    );
    assert_eq!(after.block, before.block + 1);

    // the head block must still be usable
    let mut more = mkstr(20, 5);
    a.alloc(&mut more, "bump after splice");
    let (after2, _) = a.snap();
    assert_eq!(after2.chain, 2);
    assert_eq!(after2.remaining, after.remaining - 21);
    a.reset();
}

/// row 60 / ERRORS row 36 — `block` grows until `blocksize` saturates at 1<<20,
/// after which `a->block` is frozen.
#[test]
fn r60_stralloc_block_saturation() {
    let _g = lock_libs();
    let mut a = DualArena::new();
    let mut expect_block = 0u8;
    for step in 0..26 {
        let (sc, sr) = a.snap();
        assert_eq!(sc, sr);
        assert_eq!(sc.block, expect_block, "step {step}");
        let blocksize = BLOCKSIZE_MIN << (sc.block >> 1);
        // a string that exactly fills a fresh block: len == blocksize
        let mut s = mkstr(blocksize - 1, step as u8);
        a.alloc(&mut s, &format!("saturation step {step} blocksize={blocksize}"));
        let (sc2, _) = a.snap();
        assert_eq!(sc2.remaining, 0, "block should be exactly consumed");
        if blocksize < BLOCKSIZE_MAX {
            expect_block += 1;
        }
        assert_eq!(sc2.block, expect_block);
    }
    let (fin, _) = a.snap();
    assert_eq!(fin.block, 22, "block must saturate at 22");
    assert_eq!(BLOCKSIZE_MIN << (fin.block >> 1), BLOCKSIZE_MAX);
    a.reset();
}

/// row 61 — random mixture of short and long strings
#[test]
fn r61_stralloc_random_mix() {
    let _g = lock_libs();
    let mut rng = Rng::new(0xA7EA_0061);
    for trial in 0..30 {
        let mut a = DualArena::new();
        for i in 0..60 {
            let len = match rng.below(10) {
                0 => 0,
                1 => 1,
                2..=6 => rng.below(300),
                7 | 8 => 400 + rng.below(400),
                _ => 1000 + rng.below(4000),
            };
            let mut s = mkstr(len, (i ^ trial) as u8);
            a.alloc(&mut s, &format!("trial {trial} i {i} len {len}"));
        }
        a.reset();
    }
}

/// row 62 — `stbds_strreset` with 0, 1 and many blocks
#[test]
fn r62_strreset_variants() {
    let _g = lock_libs();
    // 0 blocks
    {
        let mut a = DualArena::new();
        a.reset();
        a.reset(); // idempotent
    }
    // 1 block
    {
        let mut a = DualArena::new();
        let mut s = mkstr(5, 0);
        a.alloc(&mut s, "one block");
        a.reset();
    }
    // a non-zero `mode` field must also be zeroed by the memset
    for mode in [1u8, 2, 3, 7, 255] {
        let mut a = DualArena::new();
        a.ac.mode = mode;
        a.ar.mode = mode;
        let mut s = mkstr(30, mode);
        a.alloc(&mut s, "arena with mode set");
        let (sc, _) = a.snap();
        assert_eq!(sc.mode, mode, "stralloc must not touch `mode`");
        a.reset(); // asserts mode == 0 afterwards
    }
    // many blocks (mixture of regular and dedicated)
    {
        let mut a = DualArena::new();
        for i in 0..40 {
            let len = if i % 3 == 0 { 4000 } else { 480 };
            let mut s = mkstr(len, i as u8);
            a.alloc(&mut s, "many blocks");
        }
        let (sc, _) = a.snap();
        assert!(sc.chain > 5, "expected a long block chain, got {}", sc.chain);
        a.reset();
        // reusable after reset
        let mut s = mkstr(9, 9);
        a.alloc(&mut s, "after reset");
        let (sc2, _) = a.snap();
        assert_eq!(sc2.chain, 1);
        assert_eq!(sc2.block, 1);
        a.reset();
    }
}

/// row 63 — `strkey`
#[test]
fn r63_strkey() {
    let _g = lock_libs();
    let p = common::libs();
    let mut rng = Rng::new(0xA7EA_0063);
    let mut cases: Vec<i32> = vec![
        0,
        1,
        -1,
        9,
        10,
        -9,
        -10,
        99,
        100,
        999,
        1000,
        123456789,
        -123456789,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    for _ in 0..2000 {
        cases.push(rng.next_u32() as i32);
    }
    for &n in &cases {
        let sc = unsafe { cstr_bytes((p.c.strkey)(n)) };
        let sr = unsafe { cstr_bytes((p.r.strkey)(n)) };
        assert_eq!(sc, sr, "strkey({n}) diverged");
        assert_eq!(
            sc.as_deref().map(|b| String::from_utf8_lossy(b).to_string()),
            Some(format!("test_{n}")),
            "strkey({n}) wrong"
        );
    }
}

/// row 64 — `arr_del`, the only symbol declared in `include/lib.h`.
/// It returns `void` and has no reachable side effects, so the differential
/// property is "both complete without faulting or corrupting the heap".
#[test]
fn r64_arr_del() {
    let _g = lock_libs();
    let p = common::libs();
    let mut rng = Rng::new(0xA7EA_0064);
    let mut cases: Vec<i32> = vec![0, 1, -1, 2, 3, 4, i32::MIN, i32::MAX, -2147483647];
    for _ in 0..3000 {
        cases.push(rng.next_u32() as i32);
    }
    for &n in &cases {
        unsafe { (p.c.arr_del)(n) };
        unsafe { (p.r.arr_del)(n) };
    }
    // interleave with other allocations to catch heap corruption
    for &n in cases.iter().take(200) {
        unsafe { (p.c.arr_del)(n) };
        let junk: Vec<u8> = vec![0xAB; 1024];
        unsafe { (p.r.arr_del)(n) };
        assert_eq!(junk[512], 0xAB);
    }
}
