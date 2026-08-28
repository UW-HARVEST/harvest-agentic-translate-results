//! Level 2: `stbds_stralloc` / `stbds_strreset` (the string arena), plus the
//! table-construction entry points `stbds_rand_seed`, `stbds_shmode_func` and
//! `stbds_hmput_default`.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

/// Comparable view of an arena: pointer values differ between libraries, so
/// only the bookkeeping fields plus the block-chain length are compared.
#[derive(Debug, PartialEq, Eq)]
struct ArenaSnap {
    remaining: usize,
    block: u8,
    mode: u8,
    storage_null: bool,
    chain_len: usize,
}

unsafe fn snap_arena(a: *const StringArena) -> ArenaSnap {
    let mut chain_len = 0usize;
    let mut p = (*a).storage as *const StringBlock;
    while !p.is_null() {
        chain_len += 1;
        p = (*p).next;
        assert!(chain_len < 100_000, "arena chain loop");
    }
    ArenaSnap {
        remaining: (*a).remaining,
        block: (*a).block,
        mode: (*a).mode,
        storage_null: (*a).storage.is_null(),
        chain_len,
    }
}

#[repr(C)]
struct StringBlock {
    next: *const StringBlock,
    storage: [c_char; 8],
}

/// Where in the arena the returned string ended up, expressed without any
/// absolute addresses:
///  - `FromHead(off)`: served out of the current head block at byte `off`
///    (the `len <= remaining` path, or the freshly allocated head block).
///  - `Dedicated`: served out of an oversized block of its own, which
///    `stbds_stralloc` splices in behind the head.
#[derive(Debug, PartialEq, Eq)]
enum Placement {
    FromHead(isize),
    Dedicated,
}

unsafe fn placement(arena: *const StringArena, p: *const c_char) -> Placement {
    let head = (*arena).storage as *const StringBlock;
    assert!(!head.is_null(), "arena has no block after stralloc");
    let base = (&(*head).storage) as *const c_char;
    // The head block holds `remaining + <what has been handed out>` bytes; the
    // allocation just made ends exactly at the old watermark, so an offset of
    // `remaining` is the only valid head placement.
    if p == base.offset((*arena).remaining as isize) {
        Placement::FromHead((*arena).remaining as isize)
    } else {
        // Must live in the second block of the chain (spliced in after head).
        let second = (*head).next;
        assert!(!second.is_null(), "string is in neither head nor second block");
        assert_eq!(
            p,
            (&(*second).storage) as *const c_char,
            "dedicated block string is not at its block start"
        );
        Placement::Dedicated
    }
}

#[test]
fn stralloc_sequence_matches() {
    let _g = guard();
    let libs = libs();
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();

    let mut rng = Rng::new(0xA11C);
    let mut lens: Vec<usize> = vec![
        0, 1, 2, 3, 7, 8, 15, 16, 31, 100, 511, 512, 513, 1000, 1023, 1024, 2000, 5000,
    ];
    for _ in 0..200 {
        lens.push(rng.below(700) as usize);
    }

    for (i, &len) in lens.iter().enumerate() {
        let bytes: Vec<u8> = (0..len).map(|k| b'a' + ((k + i) % 26) as u8).collect();
        let mut buf = CStrBuf::from_bytes(&bytes);
        unsafe {
            let cp = libs.c.stralloc(&mut ca, buf.as_ptr());
            let rp = libs.rs.stralloc(&mut ra, buf.as_ptr());

            assert_eq!(
                read_cstr(cp),
                read_cstr(rp),
                "stralloc content mismatch at step {i} (len {len})"
            );
            assert_eq!(
                snap_arena(&ca),
                snap_arena(&ra),
                "arena state mismatch at step {i} (len {len})"
            );
            assert_eq!(
                placement(&ca, cp),
                placement(&ra, rp),
                "stralloc placement mismatch at step {i} (len {len})"
            );
        }
    }

    unsafe {
        libs.c.strreset(&mut ca);
        libs.rs.strreset(&mut ra);
    }
    assert_eq!(unsafe { snap_arena(&ca) }, unsafe { snap_arena(&ra) });
    assert_eq!(unsafe { snap_arena(&ca) }.chain_len, 0);
}

#[test]
fn stralloc_big_first_allocation() {
    let _g = guard();
    // `len > blocksize` on a fresh arena takes the dedicated-block path that
    // sets `remaining = 0`.
    let libs = libs();
    for len in [513usize, 600, 1024, 1 << 16] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let mut buf = CStrBuf::from_bytes(&vec![b'Z'; len]);
        unsafe {
            let cp = libs.c.stralloc(&mut ca, buf.as_ptr());
            let rp = libs.rs.stralloc(&mut ra, buf.as_ptr());
            assert_eq!(read_cstr(cp), read_cstr(rp), "len={len}");
            assert_eq!(snap_arena(&ca), snap_arena(&ra), "len={len}");
            libs.c.strreset(&mut ca);
            libs.rs.strreset(&mut ra);
        }
    }
}

#[test]
fn stralloc_big_block_inserted_after_head() {
    let _g = guard();
    // Small allocation first (creates a head block with `remaining`), then an
    // oversized one, which is spliced in *after* the head and leaves
    // `remaining` untouched.
    let libs = libs();
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    let mut small = CStrBuf::new("hello");
    let mut big = CStrBuf::from_bytes(&vec![b'q'; 4096]);
    unsafe {
        libs.c.stralloc(&mut ca, small.as_ptr());
        libs.rs.stralloc(&mut ra, small.as_ptr());
        assert_eq!(snap_arena(&ca), snap_arena(&ra));

        let cp = libs.c.stralloc(&mut ca, big.as_ptr());
        let rp = libs.rs.stralloc(&mut ra, big.as_ptr());
        assert_eq!(read_cstr(cp), read_cstr(rp));
        assert_eq!(snap_arena(&ca), snap_arena(&ra));

        // The head block must still serve small allocations afterwards.
        let cp2 = libs.c.stralloc(&mut ca, small.as_ptr());
        let rp2 = libs.rs.stralloc(&mut ra, small.as_ptr());
        assert_eq!(read_cstr(cp2), read_cstr(rp2));
        assert_eq!(snap_arena(&ca), snap_arena(&ra));

        libs.c.strreset(&mut ca);
        libs.rs.strreset(&mut ra);
        assert_eq!(snap_arena(&ca), snap_arena(&ra));
    }
}

#[test]
fn stralloc_block_size_progression() {
    let _g = guard();
    // Repeatedly exhaust the current block so `a->block` climbs through the
    // whole `512 << (block>>1)` ladder up to the 1 MiB cap.
    let libs = libs();
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    let mut buf = CStrBuf::from_bytes(&vec![b'x'; 400]);
    unsafe {
        for step in 0..600 {
            let cp = libs.c.stralloc(&mut ca, buf.as_ptr());
            let rp = libs.rs.stralloc(&mut ra, buf.as_ptr());
            assert_eq!(read_cstr(cp), read_cstr(rp), "step {step}");
            assert_eq!(
                snap_arena(&ca),
                snap_arena(&ra),
                "arena progression mismatch at step {step}"
            );
        }
        assert!(ca.block > 1, "block counter never advanced");
        libs.c.strreset(&mut ca);
        libs.rs.strreset(&mut ra);
    }
}

#[test]
fn strreset_on_empty_arena() {
    let _g = guard();
    let libs = libs();
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    ca.block = 7;
    ca.mode = 3;
    ca.remaining = 99;
    ra.block = 7;
    ra.mode = 3;
    ra.remaining = 99;
    unsafe {
        libs.c.strreset(&mut ca);
        libs.rs.strreset(&mut ra);
    }
    assert_eq!(unsafe { snap_arena(&ca) }, unsafe { snap_arena(&ra) });
    assert_eq!(ca.block, 0);
    assert_eq!(ca.mode, 0);
    assert_eq!(ca.remaining, 0);
}

#[test]
fn shmode_func_matches() {
    let _g = guard();
    let libs = libs();
    for seed in [0usize, 1, 0x3141_5926, usize::MAX, 0xABCD_EF01] {
        for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for fmt in [Fmt::BinaryKV, Fmt::StrKV, Fmt::Binary2KV] {
                unsafe {
                    libs.c.rand_seed(seed);
                    libs.rs.rand_seed(seed);
                    let a = libs.c.shmode_func(fmt.elemsize(), mode);
                    let b = libs.rs.shmode_func(fmt.elemsize(), mode);
                    assert_eq!(
                        snap_hm(a, fmt),
                        snap_hm(b, fmt),
                        "shmode_func(elemsize={}, mode={mode}) seed={seed:#x} mismatch",
                        fmt.elemsize()
                    );
                    libs.c.hmfree_func(a.sub(fmt.elemsize()) as *mut c_void, fmt.elemsize());
                    libs.rs
                        .hmfree_func(b.sub(fmt.elemsize()) as *mut c_void, fmt.elemsize());
                }
            }
        }
    }
}

#[test]
fn rand_seed_advances_identically() {
    let _g = guard();
    // `stbds_make_hash_index` mixes and re-stores the global seed; repeated
    // table creation must produce the same seed sequence in both libraries.
    let libs = libs();
    unsafe {
        libs.c.rand_seed(0x3141_5926);
        libs.rs.rand_seed(0x3141_5926);
        let mut cseeds = Vec::new();
        let mut rseeds = Vec::new();
        for _ in 0..64 {
            let a = libs.c.shmode_func(8, SH_NONE);
            let b = libs.rs.shmode_func(8, SH_NONE);
            cseeds.push(snap_hm(a, Fmt::BinaryKV).table.unwrap().seed);
            rseeds.push(snap_hm(b, Fmt::BinaryKV).table.unwrap().seed);
            libs.c.hmfree_func(a.sub(8) as *mut c_void, 8);
            libs.rs.hmfree_func(b.sub(8) as *mut c_void, 8);
        }
        assert_eq!(cseeds, rseeds, "global seed sequence diverged");
        // Sanity: the sequence really does move.
        assert!(cseeds.windows(2).any(|w| w[0] != w[1]));
    }
}

#[test]
fn hmput_default_matches() {
    let _g = guard();
    let libs = libs();
    for fmt in [Fmt::BinaryKV, Fmt::StrKV, Fmt::Binary2KV] {
        let es = fmt.elemsize();
        unsafe {
            // From NULL.
            let a = libs.c.hmput_default(std::ptr::null_mut(), es);
            let b = libs.rs.hmput_default(std::ptr::null_mut(), es);
            assert_eq!(snap_hm(a, fmt), snap_hm(b, fmt), "hmput_default(NULL, {es})");

            // Idempotent second call on a non-empty map.
            let a2 = libs.c.hmput_default(a as *mut c_void, es);
            let b2 = libs.rs.hmput_default(b as *mut c_void, es);
            assert_eq!(a, a2, "C hmput_default moved the map");
            assert_eq!(b, b2, "Rust hmput_default moved the map");
            assert_eq!(snap_hm(a2, fmt), snap_hm(b2, fmt));

            // Set the default-value slot, as `stbds_hmdefault` does: t[-1].
            // (Skipped for `char *` keys: a bit pattern is not a valid
            // pointer, and the slot below `t` holds no key anyway.)
            if fmt != Fmt::StrKV {
                let cslot = a2.sub(es);
                let rslot = b2.sub(es);
                std::ptr::write_bytes(cslot, 0x5A, es);
                std::ptr::write_bytes(rslot, 0x5A, es);
                assert_eq!(snap_hm(a2, fmt), snap_hm(b2, fmt));
            }

            libs.c.hmfree_func(a2.sub(es) as *mut c_void, es);
            libs.rs.hmfree_func(b2.sub(es) as *mut c_void, es);
        }
    }
}
