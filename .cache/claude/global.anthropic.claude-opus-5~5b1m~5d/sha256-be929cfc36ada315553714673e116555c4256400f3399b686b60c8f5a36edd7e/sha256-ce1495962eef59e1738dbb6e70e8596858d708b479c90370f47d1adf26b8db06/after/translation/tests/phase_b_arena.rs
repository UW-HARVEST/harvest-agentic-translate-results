//! Phase B rows B48-B54: `stbds_stralloc` / `stbds_strreset` driven directly
//! with a caller-owned `stbds_string_arena`.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;

const BLOCKSIZE_MIN: usize = 512;
const BLOCKSIZE_MAX: usize = 1 << 20;

/// number of blocks reachable from `arena.storage`
unsafe fn chain_len(mut p: *mut c_void) -> usize {
    let mut n = 0;
    while !p.is_null() {
        n += 1;
        assert!(n < 10_000, "arena block chain looks cyclic");
        p = *(p as *mut *mut c_void);
    }
    n
}

struct ArenaPair {
    capi: &'static Api,
    rapi: &'static Api,
    ca: Box<StringArena>,
    ra: Box<StringArena>,
    /// every string ever returned, so we can re-verify it later
    cstrs: Vec<(*mut c_char, Vec<u8>)>,
    rstrs: Vec<(*mut c_char, Vec<u8>)>,
}

impl ArenaPair {
    fn new(capi: &'static Api, rapi: &'static Api) -> ArenaPair {
        ArenaPair {
            capi,
            rapi,
            ca: Box::new(StringArena::zeroed()),
            ra: Box::new(StringArena::zeroed()),
            cstrs: Vec::new(),
            rstrs: Vec::new(),
        }
    }

    fn set_block(&mut self, b: u8) {
        self.ca.block = b;
        self.ra.block = b;
    }

    unsafe fn alloc(&mut self, s: &[u8], ctx: &str) {
        let mut cs = s.to_vec();
        cs.push(0);
        let mut rs = s.to_vec();
        rs.push(0);
        let pc = (self.capi.stralloc)(
            &mut *self.ca as *mut StringArena as *mut c_void,
            cs.as_mut_ptr() as *mut c_char,
        );
        let pr = (self.rapi.stralloc)(
            &mut *self.ra as *mut StringArena as *mut c_void,
            rs.as_mut_ptr() as *mut c_char,
        );
        assert_eq!(read_cstr(pc), s.to_vec(), "{ctx}: C returned wrong string");
        assert_eq!(read_cstr(pr), s.to_vec(), "{ctx}: RUST returned wrong string");
        self.cstrs.push((pc, s.to_vec()));
        self.rstrs.push((pr, s.to_vec()));
        self.assert_same(ctx);
    }

    unsafe fn assert_same(&self, ctx: &str) {
        assert_eq!(
            self.ca.remaining, self.ra.remaining,
            "{ctx}: arena.remaining C={} RUST={}",
            self.ca.remaining, self.ra.remaining
        );
        assert_eq!(
            self.ca.block, self.ra.block,
            "{ctx}: arena.block C={} RUST={}",
            self.ca.block, self.ra.block
        );
        assert_eq!(self.ca.mode, self.ra.mode, "{ctx}: arena.mode");
        assert_eq!(
            self.ca.storage.is_null(),
            self.ra.storage.is_null(),
            "{ctx}: arena.storage null-ness"
        );
        assert_eq!(
            chain_len(self.ca.storage),
            chain_len(self.ra.storage),
            "{ctx}: block chain length"
        );
        // every previously returned string must still read back correctly
        for (i, ((pc, want), (pr, _))) in self.cstrs.iter().zip(self.rstrs.iter()).enumerate() {
            assert_eq!(&read_cstr(*pc), want, "{ctx}: C string {i} corrupted");
            assert_eq!(&read_cstr(*pr), want, "{ctx}: RUST string {i} corrupted");
        }
    }

    unsafe fn reset(&mut self, ctx: &str) {
        (self.capi.strreset)(&mut *self.ca as *mut StringArena as *mut c_void);
        (self.rapi.strreset)(&mut *self.ra as *mut StringArena as *mut c_void);
        self.cstrs.clear();
        self.rstrs.clear();
        assert_eq!(self.ca.remaining, 0, "{ctx}: C remaining after reset");
        assert_eq!(self.ra.remaining, 0, "{ctx}: RUST remaining after reset");
        assert_eq!(self.ca.block, 0, "{ctx}: C block after reset");
        assert_eq!(self.ra.block, 0, "{ctx}: RUST block after reset");
        assert!(self.ca.storage.is_null() && self.ra.storage.is_null());
        self.assert_same(ctx);
    }
}

/// B48 — fresh zeroed arena, small string → one 512-byte block
#[test]
fn cfg_b48_stralloc_fresh() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut a = ArenaPair::new(c, r);
        a.alloc(b"hello", "B48 first");
        assert_eq!(a.ca.remaining, BLOCKSIZE_MIN - 6, "B48 remaining");
        assert_eq!(a.ca.block, 1, "B48 block incremented");
        assert_eq!(chain_len(a.ca.storage), 1);
        a.alloc(b"world!!", "B48 second (fits)");
        assert_eq!(chain_len(a.ca.storage), 1, "B48 still one block");
        a.reset("B48 reset");
    });
}

/// B49 — fill a block exactly, then allocate again → new block, block index++
#[test]
fn cfg_b49_stralloc_block_chain() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut a = ArenaPair::new(c, r);
        // 512-byte block: allocate 511 bytes + NUL = exactly 512
        a.alloc(&vec![b'a'; 511], "B49 exact fill");
        assert_eq!(a.ca.remaining, 0, "B49 block full");
        assert_eq!(chain_len(a.ca.storage), 1);
        // one more byte → second block (blocksize = 512 << (1>>1) = 512)
        a.alloc(b"x", "B49 second block");
        assert_eq!(chain_len(a.ca.storage), 2);
        assert_eq!(a.ca.block, 2);
        assert_eq!(a.ca.remaining, 512 - 2);
        // fill many blocks
        for i in 0..12 {
            let big = vec![b'b'; a.ca.remaining.max(1) - 1];
            a.alloc(&big, &format!("B49 fill {i}"));
            a.alloc(b"y", &format!("B49 spill {i}"));
        }
        a.reset("B49 reset");
    });
}

/// B50 — `len > blocksize` with `storage == NULL` → dedicated block, remaining=0
#[test]
fn cfg_b50_stralloc_huge_first() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &len in &[512usize, 513, 1000, 4096, 100_000] {
            let mut a = ArenaPair::new(c, r);
            a.alloc(&vec![b'q'; len], &format!("B50 len={len}"));
            if len + 1 > BLOCKSIZE_MIN {
                assert_eq!(a.ca.remaining, 0, "B50 len={len} remaining must be 0");
                assert_eq!(a.ca.block, 1, "B50 len={len} block incremented");
                assert_eq!(chain_len(a.ca.storage), 1);
            }
            a.reset(&format!("B50 len={len} reset"));
        }
    });
}

/// B51 — `len > blocksize` with `storage != NULL` → spliced after head
#[test]
fn cfg_b51_stralloc_huge_after_head() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut a = ArenaPair::new(c, r);
        a.alloc(b"small", "B51 seed block");
        let rem_before = a.ca.remaining;
        assert_eq!(chain_len(a.ca.storage), 1);
        a.alloc(&vec![b'H'; 5000], "B51 huge");
        assert_eq!(chain_len(a.ca.storage), 2, "B51 spliced");
        assert_eq!(
            a.ca.remaining, rem_before,
            "B51 remaining untouched by the huge branch"
        );
        assert_eq!(a.ca.block, 2, "B51 block still incremented");
        // the small block is still the head and still usable
        a.alloc(b"tiny", "B51 after huge");
        assert_eq!(chain_len(a.ca.storage), 2);
        a.alloc(&vec![b'I'; 6000], "B51 huge 2");
        assert_eq!(chain_len(a.ca.storage), 3);
        a.reset("B51 reset");
    });
}

/// B52 — pre-set `block` field: `blocksize = 512 << (block>>1)`, saturating the
/// `++a->block` at `blocksize >= 1<<20`
#[test]
fn cfg_b52_stralloc_block_field() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for b in 0u8..=24 {
            let mut a = ArenaPair::new(c, r);
            a.set_block(b);
            let blocksize = BLOCKSIZE_MIN << (b >> 1);
            a.alloc(b"probe", &format!("B52 block={b}"));
            let expect_block = if blocksize < BLOCKSIZE_MAX { b + 1 } else { b };
            assert_eq!(
                a.ca.block, expect_block,
                "B52 block={b} blocksize={blocksize}"
            );
            assert_eq!(a.ca.remaining, blocksize - 6, "B52 block={b} remaining");
            a.reset(&format!("B52 block={b} reset"));
        }
        // saturation: repeated allocations must stop incrementing block
        let mut a = ArenaPair::new(c, r);
        a.set_block(21);
        for i in 0..6 {
            let rem = a.ca.remaining;
            a.alloc(&vec![b'z'; rem.max(1) - 1], &format!("B52 sat fill {i}"));
            a.alloc(b"w", &format!("B52 sat spill {i}"));
        }
        assert_eq!(a.ca.block, 22, "B52 block saturates at 22 (512<<11 == 1<<20 == MAX)");
        a.reset("B52 sat reset");
    });
}

/// B53 — 300 random strings of mixed length into one arena
#[test]
fn cfg_b53_stralloc_random() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for seed in 1u64..=3 {
            let mut rng = Rng::new(53 + seed);
            let mut a = ArenaPair::new(c, r);
            for i in 0..300 {
                let l = rng.below(900);
                let s = rng.ascii(l);
                a.alloc(&s, &format!("B53 s={seed} i={i} len={l}"));
            }
            a.reset(&format!("B53 s={seed} reset"));
        }
    });
}

/// B54 — `strreset` on populated / empty / twice
#[test]
fn cfg_b54_strreset() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        // empty (already-zeroed) arena
        let mut a = ArenaPair::new(c, r);
        a.reset("B54 reset empty");
        a.reset("B54 reset empty twice");
        // populated, multi-block
        for i in 0..8 {
            a.alloc(&vec![b'p'; 400], &format!("B54 fill {i}"));
        }
        assert!(chain_len(a.ca.storage) > 1);
        a.reset("B54 reset populated");
        a.reset("B54 reset populated twice");
        // reusable after reset
        a.alloc(b"reused", "B54 after reset");
        assert_eq!(a.ca.block, 1);
        a.reset("B54 final reset");
    });
}

/// B32b — empty string keys and single-byte strings
#[test]
fn cfg_b53b_stralloc_edge_lengths() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut a = ArenaPair::new(c, r);
        a.alloc(b"", "B53b empty");
        assert_eq!(a.ca.remaining, BLOCKSIZE_MIN - 1, "B53b empty consumes 1");
        for i in 0..600 {
            a.alloc(b"", &format!("B53b empty {i}"));
        }
        a.reset("B53b reset");
        // lengths exactly at the block boundary
        for &l in &[509usize, 510, 511, 512, 513] {
            let mut a = ArenaPair::new(c, r);
            a.alloc(&vec![b'e'; l], &format!("B53b boundary {l}"));
            a.alloc(b"x", &format!("B53b boundary {l} follow"));
            a.reset(&format!("B53b boundary {l} reset"));
        }
    });
}
