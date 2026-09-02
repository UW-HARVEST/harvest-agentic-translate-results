//! Phase B — CONFIGS.md rows 65..75: the string arena (`stbds_stralloc` /
//! `stbds_strreset`), `stbds_hmfree_func` teardown, and the two test helpers
//! `strkey` / `arr_del`.

mod common;
use common::*;
use std::ffi::c_char;

/// A pair of arenas, one per library, driven in lock-step.
struct Arenas<'a> {
    l: &'a Pair,
    c: Arena,
    r: Arena,
    /// every string handed back, so we can re-verify old contents stay intact
    handed_c: Vec<(*mut c_char, Vec<u8>)>,
    handed_r: Vec<(*mut c_char, Vec<u8>)>,
}

impl<'a> Arenas<'a> {
    fn new(l: &'a Pair) -> Arenas<'a> {
        let zero = Arena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        Arenas {
            l,
            c: zero,
            r: zero,
            handed_c: vec![],
            handed_r: vec![],
        }
    }

    fn with_block(l: &'a Pair, block: u8, mode: u8) -> Arenas<'a> {
        let mut a = Arenas::new(l);
        a.c.block = block;
        a.r.block = block;
        a.c.mode = mode;
        a.r.mode = mode;
        a
    }

    unsafe fn alloc(&mut self, s: &[u8], what: &str) {
        assert_eq!(*s.last().unwrap(), 0);
        let mut bc = s.to_vec();
        let mut br = s.to_vec();
        let pc = (self.l.c.stralloc)(&mut self.c, bc.as_mut_ptr() as *mut c_char);
        let pr = (self.l.r.stralloc)(&mut self.r, br.as_mut_ptr() as *mut c_char);
        assert!(!pc.is_null() && !pr.is_null(), "[{what}] stralloc returned NULL");
        let sc = std::ffi::CStr::from_ptr(pc).to_bytes().to_vec();
        let sr = std::ffi::CStr::from_ptr(pr).to_bytes().to_vec();
        assert_eq!(sc, sr, "[{what}] returned string");
        assert_eq!(&sc[..], &s[..s.len() - 1], "[{what}] returned string content");
        self.handed_c.push((pc, sc));
        self.handed_r.push((pr, sr));
        self.assert_eq(what);
    }

    unsafe fn assert_eq(&self, what: &str) {
        let a = snap_arena(&self.c);
        let b = snap_arena(&self.r);
        assert_eq!(a, b, "[{what}] arena state");
        // every previously returned string must still read back correctly
        for (i, ((pc, ec), (pr, er))) in self.handed_c.iter().zip(self.handed_r.iter()).enumerate() {
            let gc = std::ffi::CStr::from_ptr(*pc).to_bytes();
            let gr = std::ffi::CStr::from_ptr(*pr).to_bytes();
            assert_eq!(gc, &ec[..], "[{what}] C string {i} was clobbered");
            assert_eq!(gr, &er[..], "[{what}] Rust string {i} was clobbered");
        }
    }

    unsafe fn reset(&mut self, what: &str) {
        (self.l.c.strreset)(&mut self.c);
        (self.l.r.strreset)(&mut self.r);
        self.handed_c.clear();
        self.handed_r.clear();
        let a = snap_arena(&self.c);
        let b = snap_arena(&self.r);
        assert_eq!(a, b, "[{what}] arena state after strreset");
        assert_eq!(a.remaining, 0);
        assert_eq!(a.block, 0);
        assert_eq!(a.mode, 0);
        assert!(!a.has_storage);
    }
}

/// row 65: fresh arena, sequential allocations of lengths 1..40.
#[test]
fn row_65_stralloc_fresh_short() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xE_0065);
    for trial in 0..6 {
        let mut a = Arenas::new(l);
        unsafe {
            for i in 0..120 {
                let n = rng.below(40); let s = rng.cstring(n);
                a.alloc(&s, &format!("trial={trial} alloc {i}"));
            }
            a.reset(&format!("trial={trial}"));
        }
    }
}

/// row 66 / ERRORS.md row 46: the very first string is longer than `blocksize`
/// (512) -> dedicated block, `remaining` forced to 0 (empty-arena splice).
#[test]
fn row_66_stralloc_first_oversized() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xE_0066);
    for &len in [512usize, 513, 600, 1024, 4096, 100_000].iter() {
        let mut a = Arenas::new(l);
        unsafe {
            let s = rng.cstring(len);
            a.alloc(&s, &format!("first oversized len={len}"));
            assert_eq!(
                snap_arena(&a.c).remaining,
                0,
                "len={len}: empty-arena oversized splice must zero `remaining`"
            );
            assert_eq!(snap_arena(&a.c).block, 1, "block must have been bumped once");
            // a follow-up short string then needs a brand new block
            let t = rng.cstring(10);
            a.alloc(&t, &format!("after oversized len={len}"));
            a.reset(&format!("len={len}"));
        }
    }
}

/// row 67 / ERRORS.md row 46: oversized string into a NON-empty arena — the new
/// block is spliced in *after* the head (`sb->next = a->storage->next`).
#[test]
fn row_67_stralloc_oversized_nonempty() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xE_0067);
    let mut a = Arenas::new(l);
    unsafe {
        // establish a head block first
        let s = rng.cstring(20);
        a.alloc(&s, "head");
        assert!(snap_arena(&a.c).remaining > 0);
        let mut saw_splice = false;
        for i in 0..12 {
            let big = rng.cstring(2000 + i * 500);
            let before = snap_arena(&a.c);
            let blocksize = 512usize << (before.block >> 1);
            a.alloc(&big, &format!("oversized-nonempty {i}"));
            let after = snap_arena(&a.c);
            if big.len() > before.remaining && big.len() > blocksize {
                // dedicated block spliced in after the head; because the arena
                // is non-empty, `remaining` is left alone
                saw_splice = true;
                assert_eq!(
                    after.remaining, before.remaining,
                    "i={i}: the non-empty oversized splice must not touch `remaining`"
                );
                assert_eq!(after.chain_len, before.chain_len + 1, "i={i}: chain +1");
            } else if big.len() > before.remaining {
                // blocksize has grown past the string: ordinary new block
                assert_eq!(after.remaining, blocksize - big.len(), "i={i}: new block");
                assert_eq!(after.chain_len, before.chain_len + 1, "i={i}: chain +1");
            } else {
                // fits in the current block
                assert_eq!(after.remaining, before.remaining - big.len(), "i={i}: in-place");
                assert_eq!(after.chain_len, before.chain_len, "i={i}: chain unchanged");
            }
            // interleave short allocations, which keep using the head block
            let sh = rng.cstring(5);
            a.alloc(&sh, &format!("short after oversized {i}"));
        }
        assert!(saw_splice, "the non-empty oversized-splice branch was never reached");
        a.reset("oversized-nonempty");
    }
}

/// row 68 / ERRORS.md row 47: exhaust block after block so `a->block` climbs
/// (blocksize progression 512,512,1024,1024,2048,...).
#[test]
fn row_68_stralloc_block_progression() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xE_0068);
    let mut a = Arenas::new(l);
    unsafe {
        let mut max_block = 0u8;
        for i in 0..400 {
            let n = 300 + rng.below(200); let s = rng.cstring(n);
            a.alloc(&s, &format!("progression {i}"));
            max_block = max_block.max(snap_arena(&a.c).block);
        }
        assert!(
            max_block >= 8,
            "expected the block counter to climb past 8, got {max_block}"
        );
        a.reset("progression");
    }
}

/// row 69: pre-seeded `a->block` (with `remaining == 0`), covering the whole
/// blocksize ladder including the shift counts that are UB in C and must still
/// agree bit-for-bit with what the compiler actually emits.
#[test]
fn row_69_stralloc_preseeded_block() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xE_0069);
    for &block in [
        0u8, 1, 2, 3, 4, 5, 8, 10, 15, 20, 21, 22, 23, 24, 30, 126, 127, 128, 129, 130, 254, 255,
    ]
    .iter()
    {
        for &mode in [0u8, 1, 2, 3].iter() {
            let mut a = Arenas::with_block(l, block, mode);
            unsafe {
                for i in 0..6 {
                    let n = rng.below(60); let s = rng.cstring(n);
                    a.alloc(&s, &format!("block={block} mode={mode} alloc {i}"));
                }
                // and one oversized string in the same arena
                let big = rng.cstring(1500);
                a.alloc(&big, &format!("block={block} mode={mode} oversized"));
                assert_eq!(
                    snap_arena(&a.c).mode,
                    mode,
                    "stralloc must never touch arena.mode"
                );
                a.reset(&format!("block={block} mode={mode}"));
            }
        }
    }
}

/// row 70 / ERRORS.md row 48: the empty string still consumes one byte, so 600
/// of them roll a 512-byte block over.
#[test]
fn row_70_stralloc_empty_strings() {
    let (l, _g) = libs();
    let mut a = Arenas::new(l);
    unsafe {
        let empty = vec![0u8];
        for i in 0..600 {
            a.alloc(&empty, &format!("empty {i}"));
        }
        assert!(snap_arena(&a.c).chain_len >= 2, "must have rolled over a block");
        a.reset("empty strings");
    }
}

/// rows 71-72 / ERRORS.md row 49: `strreset` on a zeroed arena, on a one-block
/// arena, and on a many-block arena.
#[test]
fn row_71_72_strreset() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xE_0071);

    // zeroed arena: pure no-op loop
    let mut a = Arenas::new(l);
    unsafe {
        a.reset("zeroed");
        a.reset("zeroed twice");
    }

    // one block
    unsafe {
        let s = rng.cstring(10);
        a.alloc(&s, "one block");
        assert_eq!(snap_arena(&a.c).chain_len, 1);
        a.reset("one block");
    }

    // many blocks
    unsafe {
        for i in 0..60 {
            let n = 400 + rng.below(100); let s = rng.cstring(n);
            a.alloc(&s, &format!("many {i}"));
        }
        assert!(snap_arena(&a.c).chain_len >= 5);
        a.reset("many blocks");
        // reusable after reset
        for i in 0..20 {
            let n = rng.below(30); let s = rng.cstring(n);
            a.alloc(&s, &format!("reuse {i}"));
        }
        a.reset("after reuse");
    }

    // a pre-seeded arena that never allocated anything
    for &block in [0u8, 7, 22, 255].iter() {
        let mut b = Arenas::with_block(l, block, 3);
        unsafe { b.reset(&format!("preseeded block={block} unused")) };
    }
}

/// row 73 / ERRORS.md rows 7-8: every `hmfree_func` shape.
#[test]
fn row_73_hmfree_func_shapes() {
    let (l, _g) = libs();
    let elemsize = 16usize;

    unsafe {
        // (a) p == NULL -> immediate return on both
        (l.c.hmfree_func)(std::ptr::null_mut(), elemsize);
        (l.r.hmfree_func)(std::ptr::null_mut(), elemsize);

        // (b) hash_table == NULL (plain arrgrowf array)
        for lib in [&l.c, &l.r] {
            let raw = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let h = (raw as *mut u8).wrapping_sub(HEADER_SIZE) as *mut Header;
            (*h).length = 3;
            std::ptr::write_bytes(raw as *mut u8, 0, elemsize * 3);
            assert!((*h).hash_table.is_null());
            (lib.hmfree_func)(raw, elemsize);
        }
    }

    // (c) every string.mode, with entries present
    for &shmode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA].iter() {
        seed_both(l, 0xCCCC);
        let mut rng = Rng::new(0xE_0073 + shmode as u64);
        let mut keep_c = Keep::new();
        let mut keep_r = Keep::new();
        let mut m = Map::from_shmode(l, elemsize, 8, HM_STRING, shmode, Pay::Raw);
        unsafe {
            let mut seen: Vec<Vec<u8>> = vec![];
            for _ in 0..25 {
                // >= 8 payload bytes: SH_NONE memcpy's `keysize` (8) bytes
                let n = 9 + rng.below(20); let k = rng.cstring(n);
                if seen.contains(&k) {
                    continue;
                }
                seen.push(k.clone());
                let kc = keep_c.add(&k);
                let kr = keep_r.add(&k);
                let v = rng.bytes(8);
                m.put(kc, kr, &v, 8);
            }
            // teardown through the real export; must not double-free or leak
            // differently (ASan-free smoke test: the allocator would abort)
            m.free();
        }
    }
}

/// row 74: `strkey` — the shared 256-byte static buffer.
#[test]
fn row_74_strkey() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xE_0074);
    let mut vals: Vec<i32> = vec![0, 1, -1, 7, 42, 99999, i32::MAX, i32::MIN];
    for _ in 0..300 {
        vals.push(rng.next_u32() as i32);
    }
    unsafe {
        for n in vals {
            let pc = (l.c.strkey)(n);
            let pr = (l.r.strkey)(n);
            let sc = std::ffi::CStr::from_ptr(pc).to_bytes().to_vec();
            let sr = std::ffi::CStr::from_ptr(pr).to_bytes().to_vec();
            assert_eq!(sc, sr, "strkey({n})");
            assert_eq!(sc, format!("test_{n}").into_bytes(), "strkey({n}) content");
            // the pointer must be stable across calls (module-static buffer)
            let pc2 = (l.c.strkey)(n);
            let pr2 = (l.r.strkey)(n);
            assert_eq!(pc, pc2, "C strkey buffer must be static");
            assert_eq!(pr, pr2, "Rust strkey buffer must be static");
        }
    }
}

/// row 75: `arr_del` — the only symbol in the public header.  It is observably
/// a no-op (allocate / arrdel / arrdelswap / free), so the contract is "must
/// complete without aborting or corrupting the heap", for every `int`.
#[test]
fn row_75_arr_del() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xE_0075);
    let mut vals: Vec<i32> = vec![0, 1, -1, 2, 3, 4, 255, 256, i32::MAX, i32::MIN];
    for _ in 0..500 {
        vals.push(rng.next_u32() as i32);
    }
    unsafe {
        for n in vals {
            (l.c.arr_del)(n);
            (l.r.arr_del)(n);
        }
    }
    // heap sanity after 1000+ alloc/free cycles in both libraries
    let mut a = Arenas::new(l);
    unsafe {
        let s = b"post-arr_del\0";
        a.alloc(s, "after arr_del");
        a.reset("after arr_del");
    }
}
