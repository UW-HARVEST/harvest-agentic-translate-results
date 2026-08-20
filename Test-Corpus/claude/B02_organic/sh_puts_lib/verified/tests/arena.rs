//! Phase B — CONFIGS.md rows 16..22 (+ ERRORS.md rows 26, 29):
//! `stbds_stralloc` / `stbds_strreset`.

mod common;
use common::*;

/// `stbds_string_block` = `{ struct stbds_string_block *next; char storage[8]; }`
const BLOCK_STORAGE_OFF: usize = 8;

unsafe fn chain(a: &StringArena) -> Vec<*mut u8> {
    let mut v = Vec::new();
    let mut p = a.storage as *mut u8;
    let mut guard = 0;
    while !p.is_null() {
        v.push(p);
        p = *(p as *mut *mut u8);
        guard += 1;
        assert!(guard < 1_000_000, "cyclic block chain");
    }
    v
}

#[derive(Debug, PartialEq, Eq)]
struct ArenaSnap {
    remaining: usize,
    block: u8,
    mode: u8,
    nblocks: usize,
    /// (index of the block that holds the returned string, offset inside its
    /// `storage[]` array) — the raw address itself is not comparable
    loc: Option<(usize, usize)>,
    content: Option<Vec<u8>>,
}

unsafe fn arena_snap(a: &StringArena, p: *mut std::ffi::c_char) -> ArenaSnap {
    let c = chain(a);
    let mut loc = None;
    if !p.is_null() {
        for (i, &b) in c.iter().enumerate() {
            let base = b.add(BLOCK_STORAGE_OFF);
            let off = (p as usize).wrapping_sub(base as usize);
            // heuristically bound the offset by the largest plausible block
            if (p as usize) >= base as usize && off < (1usize << 32) {
                // pick the *smallest* offset among candidate blocks
                if loc.map_or(true, |(_, o): (usize, usize)| off < o) {
                    loc = Some((i, off));
                }
            }
        }
    }
    ArenaSnap {
        remaining: a.remaining,
        block: a.block,
        mode: a.mode,
        nblocks: c.len(),
        loc,
        content: cstr(p),
    }
}

struct DualArena {
    c: StringArena,
    r: StringArena,
}

impl DualArena {
    fn new() -> Self {
        DualArena { c: StringArena::new(), r: StringArena::new() }
    }
    fn with_block(block: u8) -> Self {
        let mut d = Self::new();
        d.c.block = block;
        d.r.block = block;
        d
    }
    fn alloc(&mut self, s: &[u8], ctx: &str) {
        let (c, r) = pair();
        let buf = CBuf::cstr(s);
        unsafe {
            let pc = (c.stralloc)(&mut self.c, buf.as_char());
            let pr = (r.stralloc)(&mut self.r, buf.as_char());
            let sc = arena_snap(&self.c, pc);
            let sr = arena_snap(&self.r, pr);
            assert_eq!(sc, sr, "stralloc diverged ({ctx}) len={}", s.len());
            assert_eq!(sc.content.as_deref(), Some(s), "content wrong ({ctx})");
        }
    }
    fn reset(&mut self, ctx: &str) {
        let (c, r) = pair();
        unsafe {
            (c.strreset)(&mut self.c);
            (r.strreset)(&mut self.r);
            let sc = arena_snap(&self.c, std::ptr::null_mut());
            let sr = arena_snap(&self.r, std::ptr::null_mut());
            assert_eq!(sc, sr, "strreset diverged ({ctx})");
            assert_eq!(
                sc,
                ArenaSnap {
                    remaining: 0,
                    block: 0,
                    mode: 0,
                    nblocks: 0,
                    loc: None,
                    content: None
                },
                "arena not zeroed ({ctx})"
            );
        }
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn stralloc_first_small() {
    let mut a = DualArena::new();
    a.alloc(b"hello", "first small");
    assert_eq!(a.c.block, 1);
    assert_eq!(a.c.remaining, 512 - 6);
    a.reset("after first small");
}

// ---------------------------------------------------------------- row 17
#[test]
fn stralloc_many_small() {
    let mut rng = Rng::new(0x11A);
    let mut a = DualArena::new();
    for i in 0..2000usize {
        let len = rng.below(65);
        let s = rng.cbytes(len, 0x21, 0x7e);
        a.alloc(&s, &format!("many_small #{i}"));
    }
    a.reset("after many small");
}

// ---------------------------------------------------------------- row 18
#[test]
fn stralloc_big_block() {
    // fresh arena (storage == NULL) and len > blocksize -> dedicated block,
    // remaining forced to 0
    for len in [512usize, 513, 1024, 8192, 100_000] {
        let mut a = DualArena::new();
        let s = vec![b'x'; len];
        a.alloc(&s, &format!("big on fresh len={len}"));
        assert_eq!(a.c.remaining, 0, "remaining must be 0 (len={len})");
        assert_eq!(a.c.block, 1);
        a.reset(&format!("after big len={len}"));
    }
    // len == blocksize exactly is NOT "big" (`len > blocksize`)
    let mut a = DualArena::new();
    a.alloc(&vec![b'y'; 511], "len 512 incl. NUL == blocksize");
    assert_eq!(a.c.remaining, 0);
    a.reset("boundary");
}

// ---------------------------------------------------------------- row 19
#[test]
fn stralloc_big_block_after_small() {
    let mut a = DualArena::new();
    a.alloc(b"small", "seed the arena");
    let before = a.c.remaining;
    a.alloc(&vec![b'z'; 40_000], "big after small");
    assert_eq!(a.c.remaining, before, "remaining must be untouched");
    a.alloc(b"another small", "small after big");
    a.reset("after mixed");
}

// ---------------------------------------------------------------- row 20
#[test]
fn stralloc_mixed_sizes() {
    let sizes = [0usize, 1, 2, 7, 8, 63, 64, 255, 510, 511, 512, 513, 1024, 2048, 100_000];
    let mut rng = Rng::new(0x22B);
    for round in 0..12usize {
        let mut a = DualArena::new();
        for i in 0..40usize {
            let len = sizes[rng.below(sizes.len())];
            let s: Vec<u8> = (0..len).map(|k| b'a' + ((k + i) % 26) as u8).collect();
            a.alloc(&s, &format!("mixed r{round} #{i} len={len}"));
        }
        a.reset(&format!("mixed round {round}"));
    }
}

// ---------------------------------------------------------------- row 21
#[test]
fn stralloc_block_field_matrix() {
    // `blocksize = 512u << (a->block >> 1)` — the shift count is not bounded by
    // the C code, so `a->block >= 110` makes the shift overflow / wrap.  Both
    // implementations must agree.  Block values whose blocksize would be
    // >= 32 MiB are skipped: the C code would attempt (and possibly fail) a
    // multi-gigabyte malloc, which is an allocator-capacity question rather
    // than a translation question.
    let mut skipped = 0usize;
    let mut tested = 0usize;
    for b in 0u8..=255 {
        let k = (b >> 1) as u32;
        let shift = k & 63;
        let bs = 512u64.checked_shl(shift).map(|v| v as u128).unwrap_or(0);
        let bs = if 9 + shift >= 64 { 0u128 } else { bs };
        if bs > (32 << 20) {
            skipped += 1;
            continue;
        }
        tested += 1;
        let mut a = DualArena::with_block(b);
        a.alloc(b"key", &format!("block={b}"));
        a.alloc(b"another key that is a bit longer", &format!("block={b} 2nd"));
        a.reset(&format!("block={b}"));
    }
    assert!(tested >= 100, "only {tested} block values tested ({skipped} skipped)");
}

// ---------------------------------------------------------------- row 22 / ERRORS 29
#[test]
fn strreset_empty() {
    let mut a = DualArena::new();
    a.reset("fresh arena, storage == NULL");
    a.reset("double reset");
    // and with a non-zero mode/block preserved-then-cleared
    let mut b = DualArena::with_block(7);
    b.c.mode = 3;
    b.r.mode = 3;
    b.reset("reset clears block+mode");
}

#[test]
fn strreset_many() {
    let mut rng = Rng::new(0x33C);
    for round in 0..8usize {
        let mut a = DualArena::new();
        let n = 1 + rng.below(300);
        for i in 0..n {
            let len = if i % 17 == 0 { 5000 + rng.below(1000) } else { rng.below(80) };
            let s = vec![b'q'; len];
            a.alloc(&s, &format!("reset_many r{round} #{i}"));
        }
        a.reset(&format!("reset_many round {round}"));
    }
}
