//! Phase B — CONFIGS.md rows 62-68: `stbds_stralloc` / `stbds_strreset`.
mod common;

use common::*;
use std::ffi::{c_char, c_void};

const BLOCKSIZE_MIN: usize = 512;
const BLOCKSIZE_MAX: usize = 1 << 20;

/// Canonical, address-independent description of "where inside the arena did
/// `stbds_stralloc` put this string".
unsafe fn describe(a: *const Arena, p: *mut c_char) -> String {
    let ar = *a;
    let head = ar.storage as *mut StringBlock;
    let loc = if head.is_null() {
        "no-head".to_string()
    } else {
        let head_storage = (head as *mut u8).add(8);
        let next = (*head).next;
        if p as *mut u8 == head_storage.add(ar.remaining) {
            "head+8+remaining".to_string()
        } else if !next.is_null() && p as *mut u8 == (next as *mut u8).add(8) {
            "second-block+8".to_string()
        } else if p as *mut u8 == head_storage {
            "head+8".to_string()
        } else {
            format!("other(head_delta={})", (p as isize) - (head as isize))
        }
    };
    format!("{} loc={} text={}", dump_arena(a), loc, cstr_repr(p))
}

/// Drives the same `stbds_stralloc` sequence on both libraries with two
/// independent arenas and compares the canonical description after each call.
struct ADual<'a> {
    s: &'a Session,
    ca: Box<Arena>,
    ra: Box<Arena>,
    /// (pointer, expected text) for every string handed out so far — re-checked
    /// after every allocation so a clobbering bug cannot hide.
    live_c: Vec<(*mut c_char, Vec<u8>)>,
    live_r: Vec<(*mut c_char, Vec<u8>)>,
    label: String,
    step: usize,
}

impl<'a> ADual<'a> {
    fn new(s: &'a Session, label: &str) -> ADual<'a> {
        ADual {
            s,
            ca: Box::new(Arena::zeroed()),
            ra: Box::new(Arena::zeroed()),
            live_c: Vec::new(),
            live_r: Vec::new(),
            label: label.to_string(),
            step: 0,
        }
    }

    #[track_caller]
    fn alloc(&mut self, text: &[u8]) {
        let mut buf: Vec<u8> = text.to_vec();
        buf.push(0);
        unsafe {
            let cp = (self.s.c.stralloc)(&mut *self.ca, buf.as_mut_ptr() as *mut c_char);
            let rp = (self.s.rust.stralloc)(&mut *self.ra, buf.as_mut_ptr() as *mut c_char);
            let cd = describe(&*self.ca, cp);
            let rd = describe(&*self.ra, rp);
            assert_same(
                &format!(
                    "{} [step {}] stralloc(len={})",
                    self.label, self.step, text.len()
                ),
                &cd,
                &rd,
            );
            self.live_c.push((cp, text.to_vec()));
            self.live_r.push((rp, text.to_vec()));
            self.verify_live();
        }
        self.step += 1;
    }

    #[track_caller]
    fn verify_live(&self) {
        unsafe {
            for (p, want) in self.live_c.iter() {
                let got = std::ffi::CStr::from_ptr(*p).to_bytes();
                assert_eq!(
                    got, &want[..],
                    "{} C arena string clobbered (step {})",
                    self.label, self.step
                );
            }
            for (p, want) in self.live_r.iter() {
                let got = std::ffi::CStr::from_ptr(*p).to_bytes();
                assert_eq!(
                    got, &want[..],
                    "{} RUST arena string clobbered (step {})",
                    self.label, self.step
                );
            }
        }
    }

    #[track_caller]
    fn check_arenas(&mut self, what: &str) {
        unsafe {
            assert_same(
                &format!("{} [step {}] {}", self.label, self.step, what),
                &dump_arena(&*self.ca),
                &dump_arena(&*self.ra),
            );
        }
        self.step += 1;
    }

    fn reset(&mut self) {
        unsafe {
            (self.s.c.strreset)(&mut *self.ca);
            (self.s.rust.strreset)(&mut *self.ra);
        }
        self.live_c.clear();
        self.live_r.clear();
        self.check_arenas("after strreset");
    }
}

impl<'a> Drop for ADual<'a> {
    fn drop(&mut self) {
        unsafe {
            (self.s.c.strreset)(&mut *self.ca);
            (self.s.rust.strreset)(&mut *self.ra);
        }
    }
}

// --- row 62 -------------------------------------------------------------
#[test]
fn cfg_62_stralloc_first_block() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 62);
    for len in [0usize, 1, 2, 7, 100, 510, 511] {
        let mut d = ADual::new(&s, &format!("first-block len={}", len));
        d.check_arenas("fresh zeroed arena");
        let t = rng.cstring(len);
        d.alloc(&t[..len]);
        assert_eq!(d.ca.block, 1, "block counter must advance to 1");
        assert_eq!(d.ca.remaining, BLOCKSIZE_MIN - (len + 1));
    }
}

// --- row 63 -------------------------------------------------------------
#[test]
fn cfg_63_stralloc_fast_path() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 63);
    let mut d = ADual::new(&s, "fast-path");
    d.alloc(b"seed");
    let block_before = d.ca.block;
    // 40 small strings all fit in the remaining 507 bytes? no - use tiny ones
    for _ in 0..40 {
        let len = rng.range(0, 8);
        let t = rng.cstring(len);
        d.alloc(&t[..len]);
    }
    assert_eq!(
        d.ca.block, block_before,
        "the fast path must not bump the block counter"
    );
    // strings must sit at strictly descending addresses inside the block
    for w in d.live_c.windows(2) {
        assert!(
            (w[1].0 as usize) < (w[0].0 as usize),
            "arena allocations should descend within a block"
        );
    }
}

// --- row 64 -------------------------------------------------------------
#[test]
fn cfg_64_stralloc_block_ladder_and_saturation() {
    let s = session();
    let mut d = ADual::new(&s, "block-ladder");
    // Each string is sized so that `len == strlen+1 == blocksize` exactly:
    // that forces the "new block" branch, consumes the block completely
    // (`remaining` -> 0) and therefore bumps `block` by exactly 1 per call,
    // walking the whole ladder 512 -> 1024 -> ... -> 1 MiB (block 0 .. 22).
    for i in 0..22usize {
        let block = d.ca.block as usize;
        assert_eq!(block, i, "block counter should be exactly {}", i);
        let blocksize = BLOCKSIZE_MIN << (block >> 1);
        assert!(blocksize <= BLOCKSIZE_MAX);
        let t = vec![b'a' + (i % 26) as u8; blocksize - 1];
        d.alloc(&t);
        assert_eq!(d.ca.remaining, 0, "block must be fully consumed at i={}", i);
        assert_eq!(d.ra.remaining, 0);
        assert_eq!(d.ca.block as usize, i + 1);
        assert_eq!(d.ra.block as usize, i + 1);
    }
    assert_eq!(d.ca.block, 22, "block counter must saturate at 22");
    assert_eq!(d.ra.block, 22);
    // once 512 << 11 == 1 MiB == STBDS_STRING_ARENA_BLOCKSIZE_MAX the counter
    // must stop advancing while the blocksize stays at 1 MiB
    for _ in 0..3 {
        let t = vec![b'Z'; BLOCKSIZE_MAX - 1];
        d.alloc(&t);
        assert_eq!(d.ca.block, 22);
        assert_eq!(d.ra.block, 22);
        assert_eq!(d.ca.remaining, 0);
    }
}

// --- row 64 (secondary shape): fixed-size strings, ladder stops early ----
#[test]
fn cfg_64b_stralloc_fixed_size_strings() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 64);
    let mut d = ADual::new(&s, "block-ladder-fixed");
    for i in 0..60usize {
        let t = rng.cstring(400);
        d.alloc(&t[..400]);
        assert!(d.ca.block <= 22, "block counter overshot at i={}", i);
        assert_eq!(d.ca.block, d.ra.block);
        assert_eq!(d.ca.remaining, d.ra.remaining);
    }
    assert!(d.ca.remaining < BLOCKSIZE_MAX);
}

// --- row 65 -------------------------------------------------------------
#[test]
fn cfg_65_stralloc_oversized_on_empty_arena() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 65);
    for len in [512usize, 513, 1000, 4096, 100_000] {
        let mut d = ADual::new(&s, &format!("oversized-empty len={}", len));
        let t = rng.cstring(len);
        d.alloc(&t[..len]);
        assert_eq!(d.ca.remaining, 0, "remaining must be forced to 0");
        assert_eq!(d.ra.remaining, 0);
        assert_eq!(d.ca.block, 1);
        // the next allocation must create a real block
        let t2 = rng.cstring(10);
        d.alloc(&t2[..10]);
    }
}

// --- row 66 -------------------------------------------------------------
#[test]
fn cfg_66_stralloc_oversized_spliced_after_head() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 66);
    for big in [600usize, 1024, 5000, 200_000] {
        let mut d = ADual::new(&s, &format!("oversized-splice big={}", big));
        d.alloc(b"small");
        let rem_before = d.ca.remaining;
        let t = rng.cstring(big);
        d.alloc(&t[..big]);
        assert_eq!(
            d.ca.remaining, rem_before,
            "remaining must be untouched when the block is spliced after the head"
        );
        assert_eq!(d.ra.remaining, rem_before);
        // more small strings still come from the head block
        for _ in 0..5 {
            let t2 = rng.cstring(4);
            d.alloc(&t2[..4]);
        }
        // and another oversized one goes after the head again
        let t3 = rng.cstring(big);
        d.alloc(&t3[..big]);
    }
}

// --- row 67 -------------------------------------------------------------
#[test]
fn cfg_67_stralloc_remaining_boundaries() {
    let s = session();
    // repeated empty strings
    let mut d = ADual::new(&s, "empty-strings");
    for _ in 0..600 {
        d.alloc(b"");
    }
    drop(d);

    // len exactly == remaining  (strlen == remaining-1) -> fast path, rem -> 0
    for delta in [-1i64, 0, 1] {
        let mut d = ADual::new(&s, &format!("boundary delta={}", delta));
        d.alloc(b"x");
        let rem = d.ca.remaining as i64;
        assert_eq!(d.ra.remaining as i64, rem);
        // strlen chosen so that len == strlen+1 == rem + delta
        let strlen = rem + delta - 1;
        assert!(strlen >= 0);
        let t = vec![b'z'; strlen as usize];
        d.alloc(&t);
        let t2 = vec![b'y'; 3];
        d.alloc(&t2);
    }
}

// --- row 68 -------------------------------------------------------------
#[test]
fn cfg_68_strreset() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 68);

    // zeroed arena
    let mut d = ADual::new(&s, "reset-empty");
    d.reset();
    d.reset();
    drop(d);

    // one block
    let mut d = ADual::new(&s, "reset-one-block");
    d.alloc(b"hello");
    d.reset();
    d.reset();
    // usable again after reset
    d.alloc(b"again");
    d.reset();
    drop(d);

    // many blocks
    let mut d = ADual::new(&s, "reset-many-blocks");
    for _ in 0..30 {
        let t = rng.cstring(400);
        d.alloc(&t[..400]);
    }
    d.reset();
    drop(d);

    // oversized-block chain
    let mut d = ADual::new(&s, "reset-oversized-chain");
    d.alloc(b"head");
    for _ in 0..10 {
        let t = rng.cstring(3000);
        d.alloc(&t[..3000]);
    }
    d.reset();
    d.alloc(b"post-reset");
    d.reset();
    drop(d);
}

// --- randomized mixed arena workload ------------------------------------
#[test]
fn cfg_62_68_random_arena_workload() {
    let s = session();
    for trial in 0..6 {
        let mut rng = Rng::new(TEST_SEED ^ 6268 ^ trial);
        let mut d = ADual::new(&s, &format!("random-arena trial={}", trial));
        for _ in 0..400 {
            match rng.below(20) {
                0 => d.reset(),
                1 | 2 => {
                    let len = rng.range(512, 4096);
                    let t = rng.cstring(len);
                    d.alloc(&t[..len]);
                }
                _ => {
                    let len = rng.range(0, 600);
                    let t = rng.cstring(len);
                    d.alloc(&t[..len]);
                }
            }
        }
    }
}

// --- stralloc driven through the SH_ARENA map path ----------------------
#[test]
fn cfg_39_arena_through_map() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 3939);
    let lay = L_STR;
    unsafe {
        let mut cp = (s.c.shmode_func)(lay.elemsize, SH_ARENA);
        let mut rp = (s.rust.shmode_func)(lay.elemsize, SH_ARENA);
        let mut bufs: Vec<Box<[u8]>> = Vec::new();
        for i in 0..120usize {
            let len = match i % 5 {
                0 => 3,
                1 => 200,
                2 => 520,
                3 => 1500,
                _ => 40,
            };
            let mut t = rng.cstring(len);
            t.pop();
            t.extend_from_slice(format!("|{}", i).as_bytes());
            t.push(0);
            let mut b = t.into_boxed_slice();
            let p = b.as_mut_ptr() as *mut c_char;
            bufs.push(b);
            let v = rng.bytes(lay.elemsize - 8);
            cp = map_put_string(s.c, cp, lay, p, &v, HM_STRING);
            rp = map_put_string(s.rust, rp, lay, p, &v, HM_STRING);
            assert_same(
                &format!("SH_ARENA map put #{}", i),
                &dump_map(cp, DumpOpts::strptr(lay.elemsize)),
                &dump_map(rp, DumpOpts::strptr(lay.elemsize)),
            );
        }
        // look everything up again through the arena-owned keys
        for b in bufs.iter() {
            let p = b.as_ptr() as *mut c_void;
            let (c2, ci) = map_geti(s.c, cp, lay, p, HM_STRING);
            let (r2, ri) = map_geti(s.rust, rp, lay, p, HM_STRING);
            cp = c2;
            rp = r2;
            assert_eq!(ci, ri);
            assert!(ci >= 0);
        }
        map_free(s.c, cp, lay);
        map_free(s.rust, rp, lay);
    }
}
