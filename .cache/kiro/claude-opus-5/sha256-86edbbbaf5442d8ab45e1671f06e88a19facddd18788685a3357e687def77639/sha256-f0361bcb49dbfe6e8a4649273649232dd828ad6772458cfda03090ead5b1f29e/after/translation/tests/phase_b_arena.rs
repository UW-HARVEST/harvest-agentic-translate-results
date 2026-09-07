//! Phase B — `stbds_stralloc` / `stbds_strreset` (the string arena) and
//! `intput` (the public header's only entry point).
//!
//! CONFIGS.md rows 58-64, 66.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

const SEED: u64 = 0xC0FFEE;
const PIN: usize = 0x3141_5926;

fn empty_arena() -> RawStringArena {
    RawStringArena {
        storage: std::ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    }
}

/// Everything observable about one `stralloc` call.
#[derive(Debug, PartialEq, Eq)]
struct AllocObs {
    /// The stored string, read back through the returned pointer.
    content: Vec<u8>,
    /// `p == (char *)a->storage->storage + a->remaining` — true for the normal
    /// path and for the oversize path when `a->storage` was NULL, false for the
    /// oversize path that splices a second block in behind the head.
    p_is_head_plus_remaining: bool,
    arena: ArenaSnapshot,
}

/// Runs `stralloc(arena, s)` on one library and captures the observables.
unsafe fn alloc_one(
    f: FnStralloc,
    arena: *mut RawStringArena,
    s: &[u8],
) -> (*mut c_char, AllocObs) {
    unsafe {
        let mut buf = s.to_vec();
        let p = f(arena as *mut c_void, buf.as_mut_ptr() as *mut c_char);
        let a = *arena;
        let head_plus = if a.storage.is_null() {
            false
        } else {
            // struct stbds_string_block { next; char storage[8]; } -> storage at +8
            let head_storage = (a.storage as *mut u8).add(8) as *mut c_char;
            p == head_storage.add(a.remaining)
        };
        (
            p,
            AllocObs {
                content: cstr_bytes(p),
                p_is_head_plus_remaining: head_plus,
                arena: snapshot_arena(arena),
            },
        )
    }
}

/// Drives both libraries' `stralloc` with identical arenas and asserts the
/// observables match. Also re-verifies every previously returned pointer.
struct ArenaPair {
    c: Box<RawStringArena>,
    r: Box<RawStringArena>,
    live_c: Vec<(*mut c_char, Vec<u8>)>,
    live_r: Vec<(*mut c_char, Vec<u8>)>,
}

impl ArenaPair {
    fn new() -> ArenaPair {
        ArenaPair {
            c: Box::new(empty_arena()),
            r: Box::new(empty_arena()),
            live_c: Vec::new(),
            live_r: Vec::new(),
        }
    }

    fn alloc(&mut self, s: &[u8], what: &str) {
        let (cl, rl) = libs();
        unsafe {
            let (cp, co) = alloc_one(cl.stralloc, &mut *self.c as *mut RawStringArena, s);
            let (rp, ro) = alloc_one(rl.stralloc, &mut *self.r as *mut RawStringArena, s);
            assert_eq!(co, ro, "{what}: stralloc observables differ for {} bytes", s.len());
            let want = s.split(|&b| b == 0).next().unwrap().to_vec();
            assert_eq!(co.content, want, "{what}: stored content wrong");
            self.live_c.push((cp, want.clone()));
            self.live_r.push((rp, want));
            // every earlier allocation must still read back correctly
            for (p, exp) in &self.live_c {
                assert_eq!(&cstr_bytes(*p), exp, "{what}: C arena corrupted an earlier string");
            }
            for (p, exp) in &self.live_r {
                assert_eq!(&cstr_bytes(*p), exp, "{what}: Rust arena corrupted an earlier string");
            }
        }
    }

    fn set_block(&mut self, block: u8) {
        self.c.block = block;
        self.r.block = block;
    }

    fn reset(&mut self, what: &str) {
        let (cl, rl) = libs();
        unsafe {
            (cl.strreset)(&mut *self.c as *mut RawStringArena as *mut c_void);
            (rl.strreset)(&mut *self.r as *mut RawStringArena as *mut c_void);
            let a = snapshot_arena(&*self.c);
            let b = snapshot_arena(&*self.r);
            assert_eq!(a, b, "{what}: strreset left different state");
            assert!(a.storage_null, "{what}: strreset must NULL storage");
            assert_eq!(a.remaining, 0);
            assert_eq!(a.block, 0);
            assert_eq!(a.mode, 0, "strreset memsets the whole arena, mode included");
        }
        self.live_c.clear();
        self.live_r.clear();
    }
}

/// CONFIGS rows 58-59: fresh arena, then many short strings out of one block.
#[test]
fn row58_59_stralloc_fresh_and_fast_path() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xC01);
    let mut ap = ArenaPair::new();
    // first alloc: storage NULL, remaining 0 -> new 512-byte block
    ap.alloc(b"hello\0", "row58 first");
    assert_eq!(ap.c.block, 1, "block must be incremented past the first 512 block");
    assert_eq!(ap.c.remaining, 512 - 6);
    // row 59: len <= remaining fast path, repeatedly
    for _ in 0..200 {
        let n = 1 + rng.below(40);
        let s = rng.cstring(n);
        ap.alloc(&s, "row59 fast path");
    }
    ap.reset("row58_59");
}

/// CONFIGS row 60: `len > blocksize` with `a->storage == NULL`.
#[test]
fn row60_stralloc_oversize_null_storage() {
    let _g = lock();
    for len in [512usize, 513, 1000, 4096, 100_000] {
        let mut ap = ArenaPair::new();
        let mut s = vec![b'x'; len];
        s.push(0);
        ap.alloc(&s, &format!("row60 len={len}"));
        // 512 << 0 == 512, so len >= 512 takes the oversize branch on a fresh
        // arena; storage becomes the new block and remaining stays 0.
        if len >= 512 {
            assert_eq!(ap.c.remaining, 0, "len={len}: oversize branch sets remaining=0");
            assert!(!ap.c.storage.is_null());
        }
        ap.reset(&format!("row60 len={len}"));
    }
}

/// CONFIGS row 61: `len > blocksize` with `a->storage != NULL` — the new block
/// is spliced in *behind* the head and `remaining` is left alone.
#[test]
fn row61_stralloc_oversize_with_existing_head() {
    let _g = lock();
    let mut ap = ArenaPair::new();
    // establish a head block with plenty of remaining
    ap.alloc(b"seed\0", "row61 setup");
    let remaining_before = ap.c.remaining;
    let block_before = ap.c.block;
    // 512 << (1>>1) == 512; ask for more than that
    let mut big = vec![b'B'; 5000];
    big.push(0);
    ap.alloc(&big, "row61 oversize");
    assert_eq!(ap.c.remaining, remaining_before, "remaining must be untouched");
    assert_eq!(ap.c.block, block_before + 1, "block must still be incremented");
    assert_eq!(unsafe { snapshot_arena(&*ap.c) }.block_count, 2);
    // head block still usable afterwards
    ap.alloc(b"after\0", "row61 after");
    ap.reset("row61");
}

/// CONFIGS row 62: the `512 << (block>>1)` progression, including the
/// shift that wraps the block size to 0 for `block >= 110`.
#[test]
fn row62_stralloc_block_progression() {
    let _g = lock();
    // natural progression: allocate strings that always exceed `remaining`
    let mut ap = ArenaPair::new();
    let mut last_block = 0u8;
    for i in 0..40usize {
        // 400 bytes always exceeds what is left of a 512-byte block after one
        // 400-byte string, forcing a new block every other allocation.
        let mut s = vec![b'p'; 400 + i];
        s.push(0);
        ap.alloc(&s, &format!("row62 natural i={i}"));
        assert!(ap.c.block >= last_block);
        last_block = ap.c.block;
    }
    assert!(last_block > 1, "block never advanced (got {last_block})");
    ap.reset("row62 natural");

    // explicit block values: 0..=20 keeps `512 << (block>>1)` <= 512 KiB
    for block in 0..=20u8 {
        for len in [1usize, 7, 8, 9, 300, 511, 512, 513, 5000] {
            let mut ap = ArenaPair::new();
            ap.set_block(block);
            let mut s = vec![b'q'; len];
            s.push(0);
            ap.alloc(&s, &format!("row62 block={block} len={len}"));
            ap.reset(&format!("row62 block={block} len={len}"));
        }
    }

    // block >= 110 => `512 << (block>>1)` shifts by >= 55 and wraps to 0, so
    // every request takes the oversize branch.
    for block in 110..=127u8 {
        for len in [1usize, 8, 100, 5000] {
            let mut ap = ArenaPair::new();
            ap.set_block(block);
            let mut s = vec![b'w'; len];
            s.push(0);
            ap.alloc(&s, &format!("row62 wrap block={block} len={len}"));
            assert_eq!(ap.c.remaining, 0, "wrapped blocksize must take the oversize branch");
            ap.reset(&format!("row62 wrap block={block} len={len}"));
        }
    }
}

/// CONFIGS rows 63-64: mixed block chains through `strreset`, and reset of an
/// empty arena.
#[test]
fn row63_64_strreset() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xC02);

    // row 64: empty arena
    let mut ap = ArenaPair::new();
    ap.reset("row64 empty");
    // and again, twice in a row
    ap.reset("row64 empty again");

    // row 63: mixed normal + oversize blocks
    for round in 0..10usize {
        let mut ap = ArenaPair::new();
        for i in 0..60usize {
            let n = if (i + round) % 7 == 0 {
                2000 + rng.below(3000)
            } else {
                1 + rng.below(60)
            };
            let s = rng.cstring(n);
            ap.alloc(&s, &format!("row63 round={round} i={i}"));
        }
        let (a, b) = unsafe { (snapshot_arena(&*ap.c), snapshot_arena(&*ap.r)) };
        assert_eq!(a, b, "row63 round={round}: arena state differs");
        assert!(a.block_count > 1, "expected a multi-block chain");
        ap.reset(&format!("row63 round={round}"));
    }

    // a non-zero `mode` is also cleared by strreset (it memsets the whole struct)
    let mut ap = ArenaPair::new();
    ap.c.mode = 3;
    ap.r.mode = 3;
    ap.alloc(b"abc\0", "row63 mode");
    ap.reset("row63 mode cleared");
}

/// CONFIGS row 66 / ERRORS row 26: `intput` for every `num` that does not trip
/// one of its asserts. `intput` has no return value, so the observable is
/// (a) that it returns at all, and (b) that it consumed the global hash seed
/// identically in both libraries.
#[test]
fn row66_intput_non_aborting() {
    let _g = lock();
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0xC03);
    let mut nums: Vec<i32> = vec![0, 1, 2, 7, 8, 10, 12, -1, -9, -11, i32::MAX, i32::MIN, 100000];
    for _ in 0..200 {
        let n = rng.next_u32() as i32;
        if n != 9 && n != 11 {
            nums.push(n);
        }
    }
    for num in nums {
        unsafe {
            (c.rand_seed)(PIN);
            (r.rand_seed)(PIN);
            (c.intput)(num);
            (r.intput)(num);
            // observable: how far each library advanced the global hash seed
            let ct = (c.shmode_func)(16, 0);
            let rt = (r.shmode_func)(16, 0);
            let cs = snapshot_map(ct, 16, 0, KeyKind::Bytes);
            let rs = snapshot_map(rt, 16, 0, KeyKind::Bytes);
            assert_eq!(
                cs.seed, rs.seed,
                "intput({num}) consumed the global seed differently: C={:#x} RUST={:#x}",
                cs.seed, rs.seed
            );
            (c.hmfree_func)((ct as *mut u8).sub(16) as *mut c_void, 16);
            (r.hmfree_func)((rt as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}
