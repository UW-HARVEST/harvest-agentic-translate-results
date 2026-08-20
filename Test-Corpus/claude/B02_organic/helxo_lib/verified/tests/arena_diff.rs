//! Phase B — CONFIGS.md rows C35..C38: the string arena
//! (`stbds_stralloc` / `stbds_strreset`).
mod common;

use common::*;
use std::ffi::{c_char, c_void};

/// Blocks are `struct { next; char storage[8]; }` — `storage` sits at offset 8.
const BLOCK_STORAGE_OFF: usize = 8;

/// Address-independent description of an arena plus the pointer `stralloc`
/// just returned.
unsafe fn dump(a: &StringArena, p: *mut c_char, len: usize) -> Vec<String> {
    let mut out = vec![
        format!("remaining={}", a.remaining),
        format!("block={}", a.block),
        format!("mode={}", a.mode),
        format!("storage_null={}", a.storage.is_null()),
    ];
    // block chain length
    let mut n = 0usize;
    let mut b = a.storage as *mut *mut c_void;
    while !b.is_null() && n < 10_000 {
        n += 1;
        b = *b as *mut *mut c_void;
    }
    out.push(format!("chain_len={n}"));
    if p.is_null() {
        out.push("p=NULL".into());
        return out;
    }
    // where did the string land?
    let head = a.storage as *mut u8;
    let expect_in_head = head.wrapping_add(BLOCK_STORAGE_OFF + a.remaining);
    out.push(format!(
        "p_loc={}",
        if p as *mut u8 == expect_in_head {
            "head+8+remaining"
        } else {
            "separate_block"
        }
    ));
    let bytes = std::slice::from_raw_parts(p as *const u8, len);
    out.push(format!("content={bytes:02x?}"));
    out
}

fn mkstr(rng: &mut Rng, len: usize) -> Vec<u8> {
    rng.cstring(len)
}

/// C35 — length matrix on a fresh arena, in sequence (block boundary = 512).
#[test]
fn cfg_stralloc_length_matrix() {
    let l = libs();
    let mut rng = Rng::new(0x35_0000);
    for start_mode in [0u8, 1, 2, 3, 44] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        ca.mode = start_mode;
        ra.mode = start_mode;
        for len in [
            0usize, 1, 2, 7, 8, 9, 15, 16, 63, 64, 100, 255, 256, 500, 509, 510, 511, 512, 513,
            1000, 1023, 1024, 1025, 4096,
        ] {
            let mut s = mkstr(&mut rng, len);
            unsafe {
                let cp = (l.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
                let rp = (l.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
                assert_same(
                    &format!("stralloc len={len} mode={start_mode}"),
                    &dump(&ca, cp, len + 1),
                    &dump(&ra, rp, len + 1),
                );
            }
        }
        unsafe {
            (l.c.strreset)(&mut ca);
            (l.r.strreset)(&mut ra);
            assert_same(
                "strreset after length matrix",
                &dump(&ca, std::ptr::null_mut(), 0),
                &dump(&ra, std::ptr::null_mut(), 0),
            );
        }
    }
}

/// C36 — the dedicated-block (`len > blocksize`) path, on empty and populated
/// arenas, mixed with small allocations.
#[test]
fn cfg_stralloc_oversize_mix() {
    let l = libs();
    let mut rng = Rng::new(0x36_0000);
    // (a) oversize first, on an *empty* arena  -> becomes the head, remaining=0
    // (b) oversize after a small one           -> spliced after the head
    for first_small in [false, true] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let mut step = 0;
        if first_small {
            let mut s = mkstr(&mut rng, 10);
            unsafe {
                let cp = (l.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
                let rp = (l.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
                assert_same(
                    &format!("oversize-mix small first_small={first_small}"),
                    &dump(&ca, cp, 11),
                    &dump(&ra, rp, 11),
                );
            }
        }
        for len in [600usize, 5, 4096, 6, 1 << 16, 7, 1 << 20, 8, (1 << 20) + 5, 9] {
            step += 1;
            let mut s = mkstr(&mut rng, len);
            unsafe {
                let cp = (l.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
                let rp = (l.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
                assert_same(
                    &format!("oversize-mix step={step} len={len} first_small={first_small}"),
                    &dump(&ca, cp, len + 1),
                    &dump(&ra, rp, len + 1),
                );
            }
        }
        unsafe {
            (l.c.strreset)(&mut ca);
            (l.r.strreset)(&mut ra);
        }
    }
}

/// C37 — many small allocations: `a->block` walks 0 -> 22 and saturates once
/// `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)`.
#[test]
fn cfg_stralloc_block_saturation() {
    let l = libs();
    let mut rng = Rng::new(0x37_0000);
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    for i in 0..4000usize {
        let len = 200 + rng.below(400);
        let mut s = mkstr(&mut rng, len);
        unsafe {
            let cp = (l.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
            let rp = (l.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
            assert_same(
                &format!("block saturation i={i} len={len}"),
                &dump(&ca, cp, len + 1),
                &dump(&ra, rp, len + 1),
            );
        }
    }
    assert!(ca.block >= 20, "expected the block counter to saturate, got {}", ca.block);
    assert_eq!(ca.block, ra.block);
    unsafe {
        (l.c.strreset)(&mut ca);
        (l.r.strreset)(&mut ra);
    }
}

/// C38 — `stbds_strreset` zeroes every field and the arena is reusable after.
#[test]
fn cfg_strreset_and_reuse() {
    let l = libs();
    let mut rng = Rng::new(0x38_0000);
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    for round in 0..6usize {
        for i in 0..40usize {
            let len = rng.below(700);
            let mut s = mkstr(&mut rng, len);
            unsafe {
                let cp = (l.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
                let rp = (l.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
                assert_same(
                    &format!("reuse round={round} i={i} len={len}"),
                    &dump(&ca, cp, len + 1),
                    &dump(&ra, rp, len + 1),
                );
            }
        }
        unsafe {
            (l.c.strreset)(&mut ca);
            (l.r.strreset)(&mut ra);
        }
        for (name, a) in [("C", &ca), ("RUST", &ra)] {
            assert!(a.storage.is_null(), "{name} storage not cleared");
            assert_eq!(a.remaining, 0, "{name} remaining not cleared");
            assert_eq!(a.block, 0, "{name} block not cleared");
            assert_eq!(a.mode, 0, "{name} mode not cleared");
        }
        assert_same(
            &format!("post-reset round={round}"),
            &unsafe { dump(&ca, std::ptr::null_mut(), 0) },
            &unsafe { dump(&ra, std::ptr::null_mut(), 0) },
        );
    }
}
