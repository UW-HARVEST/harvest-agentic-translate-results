//! Phase B rows 55-62: `stbds_stralloc` / `stbds_strreset` (the string arena)
//! and `strkey`.
mod common;
use common::*;
use std::ffi::c_char;

/// Pointer-free description of where a `stralloc` result landed.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
struct Loc {
    /// index in the block chain whose `storage[0]` equals the returned pointer
    block_start: Option<usize>,
    /// byte offset from the head block's `storage[0]` (only when the pointer is
    /// not a block start, which makes the value deterministic)
    head_off: Option<isize>,
}

unsafe fn locate(a: &StringArena, p: *const c_char) -> Loc {
    let mut idx = None;
    let mut b = a.storage;
    let mut i = 0usize;
    while !b.is_null() && i < 1 << 20 {
        if std::ptr::addr_of!((*b).storage) as *const c_char == p {
            idx = Some(i);
            break;
        }
        b = (*b).next;
        i += 1;
    }
    let head_off = if idx.is_none() && !a.storage.is_null() {
        Some(p as isize - std::ptr::addr_of!((*a.storage).storage) as isize)
    } else {
        None
    };
    Loc {
        block_start: idx,
        head_off,
    }
}

struct ArenaPair {
    ca: StringArena,
    ra: StringArena,
    cstr: Vec<*mut c_char>,
    rstr: Vec<*mut c_char>,
    label: String,
}

impl ArenaPair {
    fn new(label: &str) -> ArenaPair {
        ArenaPair {
            ca: StringArena::zeroed(),
            ra: StringArena::zeroed(),
            cstr: Vec::new(),
            rstr: Vec::new(),
            label: label.to_string(),
        }
    }

    /// `stbds_stralloc` on both arenas; compares the returned string and the
    /// resulting arena state.
    #[track_caller]
    fn alloc(&mut self, s: &[u8]) {
        assert_eq!(*s.last().unwrap(), 0);
        let l = libs();
        let mut cb = s.to_vec();
        let mut rb = s.to_vec();
        unsafe {
            let cp = (l.c.stralloc)(&mut self.ca, cb.as_mut_ptr() as *mut c_char);
            let rp = (l.r.stralloc)(&mut self.ra, rb.as_mut_ptr() as *mut c_char);
            self.cstr.push(cp);
            self.rstr.push(rp);

            let want = &s[..s.len() - 1];
            let cs = std::slice::from_raw_parts(cp as *const u8, want.len());
            let rs = std::slice::from_raw_parts(rp as *const u8, want.len());
            assert_eq!(cs, want, "[{}] C returned wrong content", self.label);
            assert_eq!(rs, want, "[{}] Rust returned wrong content", self.label);
            assert_eq!(*(cp as *const u8).add(want.len()), 0);
            assert_eq!(*(rp as *const u8).add(want.len()), 0);

            let cl = locate(&self.ca, cp);
            let rl = locate(&self.ra, rp);
            assert_eq!(cl, rl, "[{}] placement differs (len={})", self.label, s.len());
        }
        self.assert_same(&format!("after stralloc len={}", s.len()));
    }

    #[track_caller]
    fn assert_same(&self, what: &str) {
        unsafe {
            let c = snap_arena(&self.ca, &self.cstr);
            let r = snap_arena(&self.ra, &self.rstr);
            assert_eq!(c, r, "[{}] {}: arena state diverged", self.label, what);
        }
    }

    fn reset(&mut self) {
        let l = libs();
        unsafe {
            (l.c.strreset)(&mut self.ca);
            (l.r.strreset)(&mut self.ra);
        }
        self.cstr.clear();
        self.rstr.clear();
        self.assert_same("after strreset");
        assert_eq!(self.ca.remaining, 0);
        assert_eq!(self.ca.block, 0);
        assert!(self.ca.storage.is_null());
    }
}

// row 55: fresh arena, one short string
#[test]
fn row55_stralloc_fresh_short() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(55);
    for _ in 0..60 {
        let mut a = ArenaPair::new("stralloc/fresh");
        let n = 1 + rng.below(40) as usize;
        a.alloc(&rng.cstring(n));
        assert_eq!(a.ca.block, 1, "block counter advanced once");
        a.reset();
    }
    // empty string
    let mut a = ArenaPair::new("stralloc/empty");
    a.alloc(b"\0");
    a.reset();
}

// row 56: many short strings -- bump path, then block exhaustion chain
#[test]
fn row56_stralloc_bump_and_new_blocks() {
    let _s = session(0x31415926);
    for &len in &[1usize, 2, 7, 8, 16, 63, 64, 100, 255, 256, 511] {
        let mut a = ArenaPair::new("stralloc/bump");
        let mut rng = Rng::with(560 + len as u64);
        for _ in 0..400 {
            let n = rng.cstring(len);
            a.alloc(&n);
        }
        a.reset();
    }
    // exactly-fitting sizes drive `remaining` to 0
    let mut a = ArenaPair::new("stralloc/exact");
    a.alloc(&vec![b'x'; 511].into_iter().chain([0]).collect::<Vec<u8>>());
    a.alloc(&vec![b'y'; 0].into_iter().chain([0]).collect::<Vec<u8>>());
    for _ in 0..40 {
        a.alloc(b"abc\0");
    }
    a.reset();
}

// row 57: string bigger than blocksize on an EMPTY arena => dedicated block
#[test]
fn row57_stralloc_oversized_on_empty() {
    let _s = session(0x31415926);
    for &len in &[512usize, 513, 600, 1024, 5000, 100_000] {
        let mut a = ArenaPair::new("stralloc/oversized-empty");
        let mut v = vec![b'q'; len];
        v.push(0);
        a.alloc(&v);
        assert_eq!(a.ca.remaining, 0, "oversized-on-empty sets remaining = 0");
        assert_eq!(a.ca.block, 1);
        a.reset();
    }
}

// row 58: oversized string on a NON-empty arena => spliced after the head
#[test]
fn row58_stralloc_oversized_after_head() {
    let _s = session(0x31415926);
    let mut a = ArenaPair::new("stralloc/oversized-spliced");
    a.alloc(b"seed\0");
    for &len in &[600usize, 4000, 9000, 70_000] {
        let mut v = vec![b'w'; len];
        v.push(0);
        a.alloc(&v);
        a.alloc(b"tiny\0");
    }
    a.reset();
}

// row 59: block counter across the 1<<20 saturation point.
// The public API can only ever reach `block == 23` (`512 << 11 == 1<<20` stops
// the increment), so the sweep stops well before `512 << (block>>1)` grows into
// an allocation the OS refuses -- at `block >= 46` the C itself dereferences the
// NULL that `realloc` returns and segfaults, which is untestable.
#[test]
fn row59_stralloc_block_counter_saturation() {
    let _s = session(0x31415926);
    for block in 0u8..=26 {
        let mut a = ArenaPair::new("stralloc/block-sweep");
        a.ca.block = block;
        a.ra.block = block;
        a.alloc(b"hello\0");
        assert_eq!(a.ca.block, a.ra.block, "block counter (start={block})");
        if block >= 22 {
            assert_eq!(a.ca.block, block, "counter must saturate at >= 1<<20");
        } else {
            assert_eq!(a.ca.block, block + 1);
        }
        a.alloc(b"world\0");
        a.reset();
    }
}

// row 60: `512 << (block>>1)` where the shift overflows the word.
// * block 110..=127 -> shift 55..=63, `512 << shift` overflows to 0, so every
//   string takes the dedicated-block path (verified against the C: remaining
//   stays 0 and the returned pointer is the block start).
// * block 128..=160 -> shift >= 64, which is UB in C; gcc emits `shl`, whose
//   count is taken mod 64, so the effective blocksize is `512 << ((block>>1)&63)`.
// Neither is reachable through the public API (the counter saturates at 23), but
// the two implementations must still agree bit for bit.
#[test]
fn row60_stralloc_block_shift_overflow() {
    let _s = session(0x31415926);
    for block in 110u8..=127 {
        let mut a = ArenaPair::new(&format!("stralloc/ovf-block={block}"));
        a.ca.block = block;
        a.ra.block = block;
        a.alloc(b"edge\0");
        assert_eq!(a.ca.block, a.ra.block, "block (start={block})");
        assert_eq!(a.ca.remaining, a.ra.remaining, "remaining (start={block})");
        assert_eq!(a.ca.remaining, 0, "blocksize overflowed to 0 (start={block})");
        a.alloc(b"second\0");
        a.reset();
    }
    // shift count >= 64: C masks it to `& 63`
    for block in 128u8..=160 {
        let mut a = ArenaPair::new(&format!("stralloc/ubshift-block={block}"));
        a.ca.block = block;
        a.ra.block = block;
        a.alloc(b"edge\0");
        assert_eq!(a.ca.block, a.ra.block, "block (start={block})");
        assert_eq!(a.ca.remaining, a.ra.remaining, "remaining (start={block})");
        let expect_blocksize = 512usize << (((block >> 1) as u32) & 63);
        assert_eq!(
            a.ca.remaining,
            expect_blocksize - 5,
            "C used blocksize 512 << ((block>>1)&63) (start={block})"
        );
        a.reset();
    }
}

// row 61: strreset on every arena shape
#[test]
fn row61_strreset_shapes() {
    let _s = session(0x31415926);
    // all-zero arena, twice (idempotent)
    let mut a = ArenaPair::new("strreset/zero");
    a.reset();
    a.reset();

    // one block
    let mut b = ArenaPair::new("strreset/one");
    b.alloc(b"one\0");
    b.reset();

    // many blocks
    let mut c = ArenaPair::new("strreset/many");
    let mut rng = Rng::with(61);
    for _ in 0..300 {
        let n = 1 + rng.below(200) as usize;
        c.alloc(&rng.cstring(n));
    }
    c.reset();

    // mixed dedicated + normal blocks
    let mut d = ArenaPair::new("strreset/mixed");
    for i in 0..12 {
        let mut v = vec![b'a' + i as u8; if i % 2 == 0 { 3000 } else { 12 }];
        v.push(0);
        d.alloc(&v);
    }
    d.reset();
    d.reset();
}

// row 62: strkey
#[test]
fn row62_strkey() {
    let _s = session(0x31415926);
    let l = libs();
    let mut rng = Rng::with(62);
    let mut vals: Vec<i32> = vec![0, 1, -1, 42, i32::MAX, i32::MIN, 999_999, -999_999];
    for _ in 0..500 {
        vals.push(rng.next_u64() as i32);
    }
    for v in vals {
        unsafe {
            let cp = (l.c.strkey)(v);
            let rp = (l.r.strkey)(v);
            let cs = std::ffi::CStr::from_ptr(cp).to_bytes().to_vec();
            let rs = std::ffi::CStr::from_ptr(rp).to_bytes().to_vec();
            assert_eq!(cs, rs, "strkey({v}) differs");
            assert_eq!(cs, format!("test_{v}").into_bytes());
            // the same static buffer must be handed back every time
            assert_eq!(cp, (l.c.strkey)(v));
            assert_eq!(rp, (l.r.strkey)(v));
        }
    }
}
