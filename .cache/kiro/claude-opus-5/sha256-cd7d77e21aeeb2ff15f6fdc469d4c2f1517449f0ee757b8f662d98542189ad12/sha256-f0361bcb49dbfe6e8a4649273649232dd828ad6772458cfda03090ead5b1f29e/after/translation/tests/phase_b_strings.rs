//! Phase B — CONFIGS.md rows 74-81: the string-arena entry points
//! `stbds_stralloc` / `stbds_strreset`, driven directly.

mod common;

use common::*;

struct ArenaPair {
    c: Arena,
    r: Arena,
}

#[derive(PartialEq, Eq, Debug)]
struct ArenaState {
    remaining: usize,
    block: u8,
    mode: u8,
    has_storage: bool,
    block_count: usize,
    /// bytes of the string just returned (None when nothing was returned yet)
    returned: Option<Vec<u8>>,
}

unsafe fn blocks(a: &Arena) -> usize {
    let mut n = 0usize;
    let mut p = a.storage as *const u8;
    while !p.is_null() {
        n += 1;
        p = std::ptr::read_unaligned(p as *const *const u8);
        assert!(n < 1_000_000, "arena block list looks circular");
    }
    n
}

unsafe fn cbytes(p: *const u8) -> Vec<u8> {
    let mut v = Vec::new();
    let mut q = p;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

impl ArenaPair {
    fn new(mode: u8) -> ArenaPair {
        let mut c = Arena::zeroed();
        let mut r = Arena::zeroed();
        c.mode = mode;
        r.mode = mode;
        ArenaPair { c, r }
    }

    #[track_caller]
    fn alloc(&mut self, what: &str, s: &[u8]) {
        let (lc, lr) = libs();
        let (cs, rs) = unsafe {
            let mut cbuf = s.to_vec();
            let mut rbuf = s.to_vec();
            let cp = (lc.stralloc)(&mut self.c, cbuf.as_mut_ptr() as *mut _);
            let rp = (lr.stralloc)(&mut self.r, rbuf.as_mut_ptr() as *mut _);
            (cbytes(cp as *const u8), cbytes(rp as *const u8))
        };
        let cst = unsafe {
            ArenaState {
                remaining: self.c.remaining,
                block: self.c.block,
                mode: self.c.mode,
                has_storage: !self.c.storage.is_null(),
                block_count: blocks(&self.c),
                returned: Some(cs),
            }
        };
        let rst = unsafe {
            ArenaState {
                remaining: self.r.remaining,
                block: self.r.block,
                mode: self.r.mode,
                has_storage: !self.r.storage.is_null(),
                block_count: blocks(&self.r),
                returned: Some(rs),
            }
        };
        same(what, &cst, &rst);
        // and the returned text must equal the input text
        let expect = &s[..s.len() - 1];
        assert_eq!(
            cst_ret(&cst),
            expect,
            "{what}: C returned the wrong string content"
        );
    }

    #[track_caller]
    fn reset(&mut self, what: &str) {
        let (lc, lr) = libs();
        unsafe {
            (lc.strreset)(&mut self.c);
            (lr.strreset)(&mut self.r);
        }
        let cst = unsafe {
            ArenaState {
                remaining: self.c.remaining,
                block: self.c.block,
                mode: self.c.mode,
                has_storage: !self.c.storage.is_null(),
                block_count: blocks(&self.c),
                returned: None,
            }
        };
        let rst = unsafe {
            ArenaState {
                remaining: self.r.remaining,
                block: self.r.block,
                mode: self.r.mode,
                has_storage: !self.r.storage.is_null(),
                block_count: blocks(&self.r),
                returned: None,
            }
        };
        same(what, &cst, &rst);
        assert_eq!(cst.remaining, 0);
        assert_eq!(cst.block, 0);
        assert_eq!(cst.mode, 0);
        assert!(!cst.has_storage);
    }
}

fn cst_ret(s: &ArenaState) -> &[u8] {
    s.returned.as_deref().unwrap_or(&[])
}

/// rows 74-76: short strings, one block then spilling into the next.
#[test]
fn cfg_74_76_stralloc_short() {
    let _g = lock();
    let mut rng = Rng::new(0x74);

    // row 74: a single short string into a fresh arena, for several lengths
    for n in [0usize, 1, 2, 7, 8, 100, 511, 512, 513] {
        let mut a = ArenaPair::new(3);
        let s = rng.ascii_cstring(n);
        a.alloc(&format!("row74 n={n}"), &s);
        a.reset(&format!("row74 n={n} reset"));
    }

    // row 76: many short strings, walking `remaining` down and rolling over
    for &len in &[1usize, 8, 63, 64, 200] {
        let mut a = ArenaPair::new(3);
        for i in 0..400 {
            let s = rng.ascii_cstring(len);
            a.alloc(&format!("row76 len={len} i={i}"), &s);
        }
        a.reset(&format!("row76 len={len} reset"));
    }

    // row 76 with random lengths
    let mut a = ArenaPair::new(3);
    for i in 0..2000 {
        let s = rng.ascii_cstring_range(0, 500);
        a.alloc(&format!("row76 rnd{i}"), &s);
    }
    a.reset("row76 rnd reset");
}

/// rows 75, 77, 78: the oversize-block branch.
#[test]
fn cfg_75_77_78_stralloc_oversize() {
    let _g = lock();
    let mut rng = Rng::new(0x75);

    // row 78: oversize as the very first allocation (storage == NULL)
    for n in [513usize, 1000, 4096, 100_000] {
        let mut a = ArenaPair::new(2);
        let s = rng.ascii_cstring(n);
        a.alloc(&format!("row78 n={n}"), &s);
        a.reset(&format!("row78 n={n} reset"));
    }

    // row 75: len > 512 but < 1<<20 at block == 0, right at the boundary
    for n in [511usize, 512, 513, 1023, 1024, 1025] {
        let mut a = ArenaPair::new(2);
        let s = rng.ascii_cstring(n);
        a.alloc(&format!("row75 n={n}"), &s);
        let s2 = rng.ascii_cstring(n);
        a.alloc(&format!("row75 n={n} second"), &s2);
        a.reset(&format!("row75 n={n} reset"));
    }

    // row 77: interleaved short / oversize
    let mut a = ArenaPair::new(3);
    for i in 0..600 {
        let n = if i % 4 == 0 {
            rng.range(600, 5000)
        } else {
            rng.range(0, 200)
        };
        let s = rng.ascii_cstring(n);
        a.alloc(&format!("row77 i={i} n={n}"), &s);
    }
    a.reset("row77 reset");
}

/// row 79: 4000 random strings; walks the `block` counter far up.
#[test]
fn cfg_79_stralloc_block_saturation() {
    let _g = lock();
    let mut rng = Rng::new(0x79);
    let mut a = ArenaPair::new(3);
    for i in 0..4000 {
        let s = rng.ascii_cstring_range(0, 3000);
        a.alloc(&format!("row79 i={i}"), &s);
    }
    assert!(a.c.block > 1, "row79 did not advance the arena block counter");
    a.reset("row79 reset");

    // deterministically push `block` to saturation: each allocation of exactly
    // `blocksize` bytes forces a fresh block.
    let mut a = ArenaPair::new(3);
    let mut n = 512usize;
    for i in 0..40 {
        let s = rng.ascii_cstring(n);
        a.alloc(&format!("row79 sat i={i} n={n}"), &s);
        if n < (1 << 21) {
            n *= 2;
        }
    }
    a.reset("row79 sat reset");
}

/// rows 80-81: reset then reuse; reset of zeroed / single / multi-block arenas.
#[test]
fn cfg_80_81_strreset() {
    let _g = lock();
    let mut rng = Rng::new(0x80);

    // row 81: zeroed arena (no-op), twice
    let mut a = ArenaPair::new(0);
    a.reset("row81 zeroed");
    a.reset("row81 zeroed again");

    // row 81: single block
    let mut a = ArenaPair::new(1);
    let s = rng.ascii_cstring(10);
    a.alloc("row81 single", &s);
    a.reset("row81 single reset");

    // row 81: multi block
    let mut a = ArenaPair::new(3);
    for i in 0..300 {
        let s = rng.ascii_cstring_range(0, 900);
        a.alloc(&format!("row81 multi{i}"), &s);
    }
    a.reset("row81 multi reset");

    // row 80: allocate / reset / re-allocate cycles
    let mut a = ArenaPair::new(3);
    for cycle in 0..12 {
        for i in 0..80 {
            let s = rng.ascii_cstring_range(0, 1200);
            a.alloc(&format!("row80 cycle{cycle} i={i}"), &s);
        }
        a.reset(&format!("row80 cycle{cycle} reset"));
    }
}
