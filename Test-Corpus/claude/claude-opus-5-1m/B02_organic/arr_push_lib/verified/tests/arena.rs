//! Phase B, CONFIGS.md rows 57-64: the string arena driven DIRECTLY through the
//! exported `stbds_stralloc` / `stbds_strreset` (not just via the hash map).

mod common;
use common::*;
use std::ffi::c_void;

/// A `stbds_string_arena` (24 bytes, 8-byte aligned) owned by the test.
#[repr(C, align(8))]
struct Arena([u8; ARENA_SIZE]);

impl Arena {
    fn zeroed() -> Self {
        Arena([0u8; ARENA_SIZE])
    }
    fn ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
    unsafe fn set(&mut self, storage: *mut u8, remaining: usize, block: u8, mode: u8) {
        let p = self.ptr();
        wr_ptr(p, ARENA_STORAGE, storage);
        wr_usize(p, ARENA_REMAINING, remaining);
        wr_u8(p, ARENA_BLOCK, block);
        wr_u8(p, ARENA_MODE, mode);
    }
    unsafe fn block(&mut self) -> u8 {
        rd_u8(self.ptr(), ARENA_BLOCK)
    }
    unsafe fn remaining(&mut self) -> usize {
        rd_usize(self.ptr(), ARENA_REMAINING)
    }
}

/// Call `stbds_stralloc` on both implementations with independent arenas and
/// compare the full observable state.
struct ArenaPair<'a> {
    p: &'a Pair,
    c: Arena,
    rs: Arena,
}

impl<'a> ArenaPair<'a> {
    fn new(p: &'a Pair) -> Self {
        ArenaPair {
            p,
            c: Arena::zeroed(),
            rs: Arena::zeroed(),
        }
    }
    #[track_caller]
    unsafe fn alloc(&mut self, s: &[u8], ctx: &str) -> (*mut i8, *mut i8) {
        let mut cb = s.to_vec();
        cb.push(0);
        let mut rb = cb.clone();
        let ca = (self.p.c.stralloc)(self.c.ptr() as *mut c_void, cb.as_mut_ptr() as *mut i8);
        let ra = (self.p.rs.stralloc)(self.rs.ptr() as *mut c_void, rb.as_mut_ptr() as *mut i8);
        assert_snap_eq(
            &snap_arena(self.c.ptr(), ca as *const u8),
            &snap_arena(self.rs.ptr(), ra as *const u8),
            ctx,
        );
        // the returned string must be an exact copy
        assert_eq_ctx(
            cstr_bytes(ca as *const u8),
            s.to_vec(),
            &format!("{ctx}: C returned string"),
        );
        assert_eq_ctx(
            cstr_bytes(ra as *const u8),
            s.to_vec(),
            &format!("{ctx}: Rust returned string"),
        );
        (ca, ra)
    }
    #[track_caller]
    unsafe fn reset(&mut self, ctx: &str) {
        (self.p.c.strreset)(self.c.ptr() as *mut c_void);
        (self.p.rs.strreset)(self.rs.ptr() as *mut c_void);
        assert_snap_eq(
            &snap_arena(self.c.ptr(), std::ptr::null()),
            &snap_arena(self.rs.ptr(), std::ptr::null()),
            ctx,
        );
        // strreset memsets the whole arena to 0
        assert_eq_ctx(self.c.0.to_vec(), vec![0u8; ARENA_SIZE], &format!("{ctx}: C zeroed"));
        assert_eq_ctx(
            self.rs.0.to_vec(),
            vec![0u8; ARENA_SIZE],
            &format!("{ctx}: Rust zeroed"),
        );
    }
}

// ---------------------------------------------------------------------------
// rows 57-58 — fresh arena, one short string / the empty string
// ---------------------------------------------------------------------------
#[test]
fn cfg57_58_stralloc_fresh_arena() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for s in [
            &b""[..],
            &b"a"[..],
            &b"hello"[..],
            &b"0123456789"[..],
            &vec![b'x'; 100][..],
            &vec![b'y'; 510][..], // len 511 <= 512
            &vec![b'z'; 511][..], // len 512 == blocksize
        ] {
            let mut ap = ArenaPair::new(p);
            let ctx = format!("fresh arena, len={}", s.len() + 1);
            ap.alloc(s, &ctx);
            // block was bumped 0 -> 1, remaining = 512 - len
            assert_eq_ctx(ap.c.block(), ap.rs.block(), &format!("{ctx}: block"));
            assert_eq_ctx(ap.c.remaining(), ap.rs.remaining(), &format!("{ctx}: remaining"));
            assert_eq!(ap.c.block(), 1);
            assert_eq!(ap.c.remaining(), 512 - (s.len() + 1));
            ap.reset(&format!("{ctx}: reset"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 59 — fill a block, then spill into the next (block 0->1->2->...)
// ---------------------------------------------------------------------------
#[test]
fn cfg59_stralloc_block_chain() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let mut r = Rng::new(0x590059);
    unsafe {
        let mut ap = ArenaPair::new(p);
        for i in 0..400usize {
            let n = r.range(1, 40);
            let s: Vec<u8> = (0..n).map(|k| b'A' + ((i + k) % 26) as u8).collect();
            ap.alloc(&s, &format!("chain alloc {i} len={n}"));
        }
        // several blocks must have been allocated by now
        let blocks = arena_blocks(&mut ap.c);
        assert_eq_ctx(blocks, arena_blocks(&mut ap.rs), "chain length");
        assert!(blocks > 3, "expected multiple blocks, got {blocks}");
        eprintln!("cfg59: {blocks} blocks, block={} ", ap.c.block());
        ap.reset("chain reset");
    }
}

unsafe fn arena_blocks(a: &mut Arena) -> usize {
    let mut blk = rd_ptr(a.ptr(), ARENA_STORAGE);
    let mut n = 0;
    while !blk.is_null() {
        n += 1;
        blk = rd_ptr(blk, 0);
    }
    n
}

// ---------------------------------------------------------------------------
// row 60 — oversize string on a fresh arena (storage == NULL path)
// ---------------------------------------------------------------------------
#[test]
fn cfg60_stralloc_oversize_no_storage() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for len in [512usize, 513, 600, 1024, 4096, 100_000] {
            let mut ap = ArenaPair::new(p);
            let s = vec![b'Q'; len];
            let ctx = format!("oversize fresh len={}", len + 1);
            ap.alloc(&s, &ctx);
            // storage was NULL => sb->next = 0, storage = sb, remaining = 0
            assert_eq_ctx(ap.c.remaining(), ap.rs.remaining(), &format!("{ctx}: remaining"));
            assert_eq!(ap.c.remaining(), 0, "oversize+no-storage must set remaining=0");
            assert_eq!(arena_blocks(&mut ap.c), 1);
            assert_eq_ctx(ap.c.block(), ap.rs.block(), &format!("{ctx}: block"));
            assert_eq!(ap.c.block(), 1, "block is bumped even on the oversize path");
            ap.reset(&format!("{ctx}: reset"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 61 — oversize string when storage != NULL (splice after the head)
// ---------------------------------------------------------------------------
#[test]
fn cfg61_stralloc_oversize_with_storage() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for big in [513usize, 1500, 5000] {
            let mut ap = ArenaPair::new(p);
            // 1) a normal allocation so that storage != NULL and remaining > 0
            ap.alloc(b"seed", "pre-alloc");
            let rem_before_c = ap.c.remaining();
            let rem_before_rs = ap.rs.remaining();
            assert_eq!(rem_before_c, rem_before_rs);
            let blocks_before = arena_blocks(&mut ap.c);
            // 2) an oversize string: gets its own block spliced in AFTER the head
            let s = vec![b'B'; big];
            let ctx = format!("oversize with storage, len={}", big + 1);
            ap.alloc(&s, &ctx);
            // remaining is deliberately NOT touched on this path
            assert_eq_ctx(ap.c.remaining(), ap.rs.remaining(), &format!("{ctx}: remaining"));
            assert_eq!(
                ap.c.remaining(),
                rem_before_c,
                "the oversize+storage path must leave `remaining` untouched"
            );
            assert_eq!(arena_blocks(&mut ap.c), blocks_before + 1);
            assert_eq_ctx(
                arena_blocks(&mut ap.c),
                arena_blocks(&mut ap.rs),
                &format!("{ctx}: chain length"),
            );
            // 3) the head block is still the allocation block
            ap.alloc(b"after", &format!("{ctx}: subsequent small alloc"));
            ap.reset(&format!("{ctx}: reset"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 62 — pre-set `block`, incl. the 1<<20 saturation at block >= 22
// ---------------------------------------------------------------------------
#[test]
fn cfg62_stralloc_block_field_and_saturation() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for block in 0u8..=24 {
            let expect_blocksize = 512usize << (block >> 1);
            for &len_minus1 in &[0usize, 1, 100, 511, 512] {
                let mut ap = ArenaPair::new(p);
                ap.c.set(std::ptr::null_mut(), 0, block, 0);
                ap.rs.set(std::ptr::null_mut(), 0, block, 0);
                let s = vec![b'k'; len_minus1];
                let ctx = format!("block={block} len={}", len_minus1 + 1);
                ap.alloc(&s, &ctx);
                assert_eq_ctx(ap.c.block(), ap.rs.block(), &format!("{ctx}: block"));
                assert_eq_ctx(ap.c.remaining(), ap.rs.remaining(), &format!("{ctx}: remaining"));
                // the exact C rule: ++block iff blocksize < 1<<20, i.e. block < 22
                let expect_block = if expect_blocksize < (1 << 20) {
                    block + 1
                } else {
                    block
                };
                assert_eq!(
                    ap.c.block(),
                    expect_block,
                    "{ctx}: block must saturate at 23 (blocksize {expect_blocksize})"
                );
                ap.reset(&format!("{ctx}: reset"));
            }
        }
        // saturation is sticky: repeatedly forcing a new block never goes past 23
        let mut ap = ArenaPair::new(p);
        ap.c.set(std::ptr::null_mut(), 0, 21, 0);
        ap.rs.set(std::ptr::null_mut(), 0, 21, 0);
        for i in 0..6 {
            // force a new block each time by exhausting `remaining`
            let (cst, cbl) = (rd_ptr(ap.c.ptr(), ARENA_STORAGE), ap.c.block());
            let (rst, rbl) = (rd_ptr(ap.rs.ptr(), ARENA_STORAGE), ap.rs.block());
            ap.c.set(cst, 0, cbl, 0);
            ap.rs.set(rst, 0, rbl, 0);
            ap.alloc(b"x", &format!("saturate {i}"));
            assert_eq_ctx(ap.c.block(), ap.rs.block(), &format!("saturate {i}: block"));
            assert!(ap.c.block() <= 23, "block ran past 23: {}", ap.c.block());
        }
        ap.reset("saturate reset");
    }
}

// ---------------------------------------------------------------------------
// row 63 — 400 randomized mixed-length allocations on one arena
// ---------------------------------------------------------------------------
#[test]
fn cfg63_stralloc_random_mixed() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    for seed in [0x630063u64, 0x630064, 0x630065] {
        let mut r = Rng::new(seed);
        unsafe {
            let mut ap = ArenaPair::new(p);
            for i in 0..400usize {
                let n = r.range(1, 2000);
                let s: Vec<u8> = (0..n).map(|_| 0x21 + (r.u64() % 0x5e) as u8).collect();
                ap.alloc(&s, &format!("seed={seed:#x} mixed {i} len={n}"));
            }
            eprintln!(
                "cfg63 seed={seed:#x}: blocks={} block={} remaining={}",
                arena_blocks(&mut ap.c),
                ap.c.block(),
                ap.c.remaining()
            );
            ap.reset(&format!("seed={seed:#x} reset"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 64 — strreset with 0 / 1 / many blocks
// ---------------------------------------------------------------------------
#[test]
fn cfg64_strreset() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        // 0 blocks: already-zeroed arena
        let mut ap = ArenaPair::new(p);
        ap.reset("reset of a zeroed arena");
        // and again (idempotent)
        ap.reset("reset twice");

        // 1 block
        let mut ap = ArenaPair::new(p);
        ap.alloc(b"one", "single");
        assert_eq!(arena_blocks(&mut ap.c), 1);
        ap.reset("reset 1 block");

        // many blocks including oversize splices
        let mut r = Rng::new(0x640064);
        let mut ap = ArenaPair::new(p);
        for i in 0..120usize {
            let n = if i % 7 == 0 { r.range(600, 3000) } else { r.range(1, 60) };
            let s: Vec<u8> = (0..n).map(|_| b'm').collect();
            ap.alloc(&s, &format!("many {i} len={n}"));
        }
        let nb = arena_blocks(&mut ap.c);
        assert_eq_ctx(nb, arena_blocks(&mut ap.rs), "many: chain length");
        assert!(nb > 10, "expected many blocks, got {nb}");
        ap.reset("reset many blocks");
        // arena is reusable after reset
        ap.alloc(b"reused", "after reset");
        assert_eq!(ap.c.block(), 1);
        ap.reset("final reset");

        // strreset on a non-zero arena whose storage is NULL: loop never runs
        let mut ap = ArenaPair::new(p);
        ap.c.set(std::ptr::null_mut(), 1234, 9, 3);
        ap.rs.set(std::ptr::null_mut(), 1234, 9, 3);
        ap.reset("reset with storage=NULL but non-zero fields");
    }
}
