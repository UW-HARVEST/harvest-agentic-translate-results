//! Phase B — CONFIGS.md rows 43..47: the string arena
//! (`stbds_stralloc` / `stbds_strreset`), driven directly on a caller-owned
//! `stbds_string_arena` (that is how `STBDS_SH_ARENA` maps use it).

mod common;

use common::*;
use std::ffi::{c_char, c_void};

#[repr(C)]
struct StringBlock {
    next: *mut StringBlock,
    storage: [u8; 8],
}

unsafe fn chain(a: &Arena) -> Vec<*mut StringBlock> {
    let mut v = Vec::new();
    let mut x = a.storage as *mut StringBlock;
    while !x.is_null() {
        v.push(x);
        x = (*x).next;
        assert!(v.len() < 10000, "cyclic block list");
    }
    v
}

/// (index of the block that contains `p`, byte offset inside that block)
unsafe fn locate(a: &Arena, p: *mut c_char) -> (isize, isize) {
    let mut best = (-1isize, -1isize);
    for (i, b) in chain(a).iter().enumerate() {
        let start = (*(*b)).storage.as_ptr() as isize;
        let d = p as isize - start;
        if d >= 0 && d < (1 << 21) && (best.0 < 0 || d < best.1) {
            best = (i as isize, d);
        }
    }
    best
}

unsafe fn arena_snapshot(a: &Arena) -> String {
    format!(
        "remaining={} block={} mode={} blocks={}",
        a.remaining,
        a.block,
        a.mode,
        chain(a).len()
    )
}

unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut i = 0isize;
    loop {
        let b = *(p.offset(i) as *const u8);
        if b == 0 {
            return v;
        }
        v.push(b);
        i += 1;
        assert!(i < 1 << 22);
    }
}

struct DualArena<'a> {
    c: Arena,
    r: Arena,
    api_c: &'a Api,
    api_r: &'a Api,
    trace: Vec<String>,
}

impl<'a> DualArena<'a> {
    fn new(api_c: &'a Api, api_r: &'a Api) -> DualArena<'a> {
        let z = Arena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        DualArena {
            c: z,
            r: Arena { ..z },
            api_c,
            api_r,
            trace: Vec::new(),
        }
    }

    unsafe fn alloc(&mut self, s: &[u8]) {
        let mut buf = s.to_vec();
        buf.push(0);
        let p = buf.as_ptr() as *mut c_char;
        let pc = (self.api_c.stralloc)(&mut self.c, p);
        let pr = (self.api_r.stralloc)(&mut self.r, p);
        self.trace.push(format!("stralloc(len={})", s.len() + 1));
        assert_eq!(
            read_cstr(pc),
            s.to_vec(),
            "C returned the wrong content; trace {:?}",
            self.trace
        );
        assert_eq!(
            read_cstr(pr),
            s.to_vec(),
            "Rust returned the wrong content; trace {:?}",
            self.trace
        );
        assert_eq!(
            locate(&self.c, pc),
            locate(&self.r, pr),
            "returned pointer sits at a different position; trace {:?}",
            self.trace
        );
        self.check();
    }

    unsafe fn check(&self) {
        assert_eq!(
            arena_snapshot(&self.c),
            arena_snapshot(&self.r),
            "arena state diverged; trace {:?}",
            self.trace
        );
    }

    unsafe fn reset(&mut self) {
        (self.api_c.strreset)(&mut self.c);
        (self.api_r.strreset)(&mut self.r);
        self.trace.push("strreset".into());
        self.check();
        assert_eq!(self.c.remaining, 0);
        assert_eq!(self.c.block, 0);
        assert!(self.c.storage.is_null());
    }
}

// row 43 -----------------------------------------------------------------------
#[test]
fn cfg_43_stralloc_first_block_boundary() {
    let (c, r, _g) = libs();
    unsafe {
        for len in [0usize, 1, 2, 8, 100, 510, 511, 512, 513, 1024, 100000] {
            let mut a = DualArena::new(c, r);
            a.alloc(&vec![b'x'; len]); // strlen+1 == len+1
            a.reset();
        }
        // remaining boundary: allocate so that `remaining` is exactly the next
        // request length, then one byte more
        for first in [1usize, 100, 510, 511] {
            let mut a = DualArena::new(c, r);
            a.alloc(&vec![b'a'; first]);
            let rem = a.c.remaining;
            assert_eq!(rem, a.r.remaining);
            if rem >= 1 {
                a.alloc(&vec![b'b'; rem - 1]); // len == remaining
                assert_eq!(a.c.remaining, 0);
                a.alloc(b"c"); // forces a new block
            }
            a.reset();
        }
    }
}

// row 44 -----------------------------------------------------------------------
#[test]
fn cfg_44_stralloc_block_growth_chain() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xA_44);
    unsafe {
        let mut a = DualArena::new(c, r);
        // ~4000 allocations of random length: walks the whole
        // 512 << (block>>1) growth chain up to the 1<<20 saturation
        for _ in 0..4000 {
            let n = rng.below(400);
            a.alloc(&vec![b'z'; n]);
        }
        assert!(a.c.block >= 10, "expected the block counter to grow, got {}", a.c.block);
        a.reset();

        // deliberately large allocations to reach the 1MB cap quickly
        let mut a = DualArena::new(c, r);
        for _ in 0..60 {
            a.alloc(&vec![b'q'; 100_000]);
        }
        a.reset();
    }
}

// row 45 -----------------------------------------------------------------------
#[test]
fn cfg_45_stralloc_oversized() {
    let (c, r, _g) = libs();
    unsafe {
        // (a) oversized request on an *empty* arena: storage == NULL branch
        let mut a = DualArena::new(c, r);
        a.alloc(&vec![b'A'; 5000]);
        assert_eq!(a.c.remaining, 0);
        a.alloc(&vec![b'B'; 3]); // now needs a regular block
        a.reset();

        // (b) oversized request on a *non-empty* arena: spliced after the head
        let mut a = DualArena::new(c, r);
        a.alloc(b"small");
        let rem = a.c.remaining;
        a.alloc(&vec![b'C'; 5000]);
        assert_eq!(a.c.remaining, rem, "remaining must be untouched");
        a.alloc(b"tiny");
        a.alloc(&vec![b'D'; 1 << 21]);
        a.reset();
    }
}

// row 46 -----------------------------------------------------------------------
#[test]
fn cfg_46_stralloc_block_field_range() {
    let (c, r, _g) = libs();
    unsafe {
        // `blocksize = 512 << (block>>1)` with a forged `block` field.
        // block <= 30 keeps `blocksize` (<= 16 MiB) allocatable; bigger values
        // make the C's `realloc` fail and then dereference NULL, which is
        // covered by the crash-parity test in Phase C.
        for block in 0..=30u8 {
            let mut a = DualArena::new(c, r);
            a.c.block = block;
            a.r.block = block;
            a.alloc(b"forged-block-field");
            a.check();
            a.reset();
        }
    }
}

// row 47 -----------------------------------------------------------------------
#[test]
fn cfg_47_strreset_shapes() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xA_47);
    unsafe {
        // zeroed arena
        let mut a = DualArena::new(c, r);
        a.reset();
        a.reset();
        // one block
        let mut a = DualArena::new(c, r);
        a.alloc(b"one");
        a.reset();
        // many blocks, including oversized ones spliced into the list
        let mut a = DualArena::new(c, r);
        for i in 0..40 {
            if i % 7 == 0 {
                a.alloc(&vec![b'o'; 2000 + i]);
            } else {
                a.alloc(&vec![b'n'; 400 + rng.below(200)]);
            }
        }
        assert!(chain(&a.c).len() > 5);
        a.reset();
        // reuse after reset
        a.alloc(b"after-reset");
        a.reset();
    }
}

// row 46b ----------------------------------------------------------------------
// `block >= 128` makes the C shift by >= 64 bits (undefined in C, masked by the
// x86 `shl`). Documented in ERRORS.md row 34 - the Rust must behave the same.
#[test]
fn cfg_46b_stralloc_block_field_shift_overflow() {
    let (c, r, _g) = libs();
    unsafe {
        // block>>1 is masked to 6 bits by the hardware shift, so 128..=159 give
        // the same (allocatable) block sizes as 0..=31
        for block in [128u8, 129, 130, 131, 140, 141, 158, 159] {
            let mut a = DualArena::new(c, r);
            a.c.block = block;
            a.r.block = block;
            a.alloc(b"shift-count-out-of-range");
            a.check();
            a.reset();
        }
    }
}

// stress: an ARENA-mode map plus a standalone arena, interleaved -----------------
#[test]
fn cfg_43_47_stralloc_random_stress() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xA_99);
    unsafe {
        for _ in 0..20 {
            let mut a = DualArena::new(c, r);
            for _ in 0..300 {
                let n = match rng.below(10) {
                    0 => 0,
                    1 => 1,
                    2..=6 => rng.below(600),
                    7 | 8 => 500 + rng.below(1200),
                    _ => rng.below(40000),
                };
                let fill: Vec<u8> = (0..n).map(|_| 1 + (rng.byte() % 255)).collect();
                a.alloc(&fill);
                if rng.below(50) == 0 {
                    a.reset();
                }
            }
            a.reset();
        }
        let _ = std::ptr::null::<c_void>();
    }
}
