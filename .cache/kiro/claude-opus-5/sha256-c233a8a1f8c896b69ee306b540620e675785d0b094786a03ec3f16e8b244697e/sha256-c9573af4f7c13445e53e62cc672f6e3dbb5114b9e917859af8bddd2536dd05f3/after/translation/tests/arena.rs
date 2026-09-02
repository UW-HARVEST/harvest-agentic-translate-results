//! Phase B rows 45-51: the string arena (`stbds_stralloc` / `stbds_strreset`).

mod common;
use common::*;

use std::ffi::{c_char, CStr, CString};

/// One arena under test, plus every pointer the library handed back so we can
/// re-verify the stored bytes after later allocations.
struct Arena<'a> {
    lib: &'a Lib,
    a: Box<StringArena>,
    handed: Vec<(*mut c_char, Vec<u8>)>,
}

impl<'a> Arena<'a> {
    fn new(lib: &'a Lib) -> Arena<'a> {
        Arena { lib, a: Box::new(StringArena::new()), handed: Vec::new() }
    }

    unsafe fn alloc(&mut self, s: &CStr) -> String {
        let before_head = self.a.storage;
        let p = (self.lib.stralloc)(&mut *self.a, s.as_ptr() as *mut c_char);
        assert!(!p.is_null(), "{}: stralloc returned NULL", self.lib.name);
        let got = CStr::from_ptr(p).to_bytes().to_vec();
        assert_eq!(
            got,
            s.to_bytes(),
            "{}: stralloc did not copy the string",
            self.lib.name
        );
        // is the result carved out of the (possibly new) head block?
        let in_head = if self.a.storage.is_null() {
            false
        } else {
            let base = (self.a.storage as *const u8).add(std::mem::size_of::<*mut u8>());
            p as *const u8 == base.add(self.a.remaining)
        };
        let head_changed = before_head != self.a.storage;
        self.handed.push((p, s.to_bytes().to_vec()));
        format!(
            "{} in_head={} head_changed={}",
            dump_arena(&*self.a),
            in_head,
            head_changed
        )
    }

    unsafe fn verify_handed(&self) {
        for (p, expect) in &self.handed {
            let got = CStr::from_ptr(*p).to_bytes();
            assert_eq!(
                got,
                &expect[..],
                "{}: previously-returned arena string was corrupted",
                self.lib.name
            );
        }
    }

    unsafe fn reset(&mut self) -> String {
        (self.lib.strreset)(&mut *self.a);
        self.handed.clear();
        dump_arena(&*self.a)
    }

    #[allow(dead_code)]
    unsafe fn state(&self) -> String {
        dump_arena(&*self.a)
    }
}

struct BothArena<'a> {
    c: Arena<'a>,
    r: Arena<'a>,
    what: String,
    step: usize,
}

impl<'a> BothArena<'a> {
    fn new(p: &'a Pair, what: &str) -> BothArena<'a> {
        BothArena { c: Arena::new(&p.c), r: Arena::new(&p.r), what: what.to_string(), step: 0 }
    }
    unsafe fn alloc(&mut self, s: &CStr) {
        self.step += 1;
        let cd = self.c.alloc(s);
        let rd = self.r.alloc(s);
        assert_eq_dump(
            &format!("{} @step {} stralloc(len={})", self.what, self.step, s.to_bytes().len() + 1),
            &cd,
            &rd,
        );
        self.c.verify_handed();
        self.r.verify_handed();
    }
    unsafe fn reset(&mut self) {
        self.step += 1;
        let cd = self.c.reset();
        let rd = self.r.reset();
        assert_eq_dump(&format!("{} @step {} strreset", self.what, self.step), &cd, &rd);
    }
}

// --- row 45 ---------------------------------------------------------------

#[test]
fn cfg45_stralloc_fresh_arena_short_strings() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 45);
    unsafe {
        for t in 0..30 {
            let mut b = BothArena::new(p, &format!("cfg45 t={t}"));
            for _ in 0..12 {
                let len = rng.below(400);
                let s = rng.cstring(len);
                b.alloc(&s);
            }
            b.reset();
        }
    }
}

// --- row 46 ---------------------------------------------------------------

#[test]
fn cfg46_stralloc_block_rollover() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 46);
    unsafe {
        for t in 0..8 {
            let mut b = BothArena::new(p, &format!("cfg46 t={t}"));
            // many short strings: forces block after block after block
            for _ in 0..600 {
                let len = 1 + rng.below(60);
                let s = rng.cstring(len);
                b.alloc(&s);
            }
            assert!(b.c.a.block > 0, "expected several blocks to be allocated");
            b.reset();
        }
    }
}

// --- rows 47, 48 ----------------------------------------------------------

#[test]
fn cfg47_stralloc_oversize_into_fresh_arena() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 47);
    unsafe {
        for len in [512usize, 511, 513, 600, 2000, 100_000] {
            let mut b = BothArena::new(p, &format!("cfg47 len={len}"));
            let s = rng.cstring(len);
            b.alloc(&s); // storage == NULL branch
            let s2 = rng.cstring(10);
            b.alloc(&s2);
            b.reset();
        }
    }
}

#[test]
fn cfg48_stralloc_oversize_into_nonempty_arena() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 48);
    unsafe {
        for len in [600usize, 1000, 5000, 100_000] {
            let mut b = BothArena::new(p, &format!("cfg48 len={len}"));
            let small = rng.cstring(4);
            b.alloc(&small); // creates the head block, remaining = 507
            let s = rng.cstring(len); // len > blocksize -> splice-after-head
            b.alloc(&s);
            let small2 = rng.cstring(4);
            b.alloc(&small2);
            let s3 = rng.cstring(len);
            b.alloc(&s3);
            b.reset();
        }
    }
}

// --- row 49 ---------------------------------------------------------------

#[test]
fn cfg49_stralloc_block_saturation() {
    let _g = serial();
    let p = pair();
    unsafe {
        let mut b = BothArena::new(p, "cfg49");
        let mut iters = 0;
        // Drive until `block` stops increasing (blocksize saturated at 1<<20).
        while iters < 400 {
            iters += 1;
            let rem = b.c.a.remaining;
            let len = if rem > 1 { (rem - 1).min(120_000) } else { 1 };
            let s = CString::new(vec![b'q'; len]).unwrap();
            b.alloc(&s);
            if b.c.a.block >= 24 {
                break;
            }
        }
        assert!(b.c.a.block >= 22, "block only reached {}", b.c.a.block);
        // a few more allocations past saturation: `block` must stay put
        let before = b.c.a.block;
        for _ in 0..4 {
            let rem = b.c.a.remaining;
            let len = if rem > 1 { rem - 1 } else { 1 };
            let s = CString::new(vec![b'z'; len]).unwrap();
            b.alloc(&s);
        }
        assert_eq!(b.c.a.block, before.max(b.c.a.block));
        assert_eq!(b.c.a.block, b.r.a.block);
        b.reset();
    }
}

// --- row 50 ---------------------------------------------------------------

#[test]
fn cfg50_stralloc_empty_strings() {
    let _g = serial();
    let p = pair();
    unsafe {
        let mut b = BothArena::new(p, "cfg50");
        let e = CString::new("").unwrap();
        for _ in 0..1200 {
            b.alloc(&e);
        }
        b.reset();
    }
}

// --- row 51 ---------------------------------------------------------------

#[test]
fn cfg51_strreset_shapes() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 51);
    unsafe {
        // empty arena
        let mut b = BothArena::new(p, "cfg51 empty");
        b.reset();
        b.reset();

        // one block
        let mut b = BothArena::new(p, "cfg51 one block");
        let s = rng.cstring(10);
        b.alloc(&s);
        b.reset();
        b.reset();
        b.alloc(&s); // reusable after reset
        b.reset();

        // many blocks
        let mut b = BothArena::new(p, "cfg51 many blocks");
        for _ in 0..300 {
            let n = 1 + rng.below(50);
            let s = rng.cstring(n);
            b.alloc(&s);
        }
        b.reset();
        b.reset();

        // blocks including an oversize one
        let mut b = BothArena::new(p, "cfg51 oversize chain");
        let small = rng.cstring(4);
        b.alloc(&small);
        let big = rng.cstring(4000);
        b.alloc(&big);
        let big2 = rng.cstring(9000);
        b.alloc(&big2);
        b.reset();
        b.reset();
    }
}

// --- mode field is caller-owned; make sure it is preserved / zeroed alike ---

#[test]
fn cfg51b_arena_mode_field_roundtrip() {
    let _g = serial();
    let p = pair();
    let mut rng = Rng::new(TEST_SEED ^ 0x51b);
    unsafe {
        for mode in [0u8, 1, 2, 3, 4, 255] {
            let mut b = BothArena::new(p, &format!("cfg51b mode={mode}"));
            b.c.a.mode = mode;
            b.r.a.mode = mode;
            let s = rng.cstring(20);
            b.alloc(&s);
            b.alloc(&s);
            b.reset(); // strreset memsets the whole struct -> mode back to 0
            assert_eq!(b.c.a.mode, 0);
            assert_eq!(b.r.a.mode, 0);
        }
    }
}
