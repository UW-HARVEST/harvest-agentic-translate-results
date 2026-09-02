//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Each row drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares results byte-for-byte, using
//! many randomized inputs (fixed seed) rather than one hand-picked value.

mod common;

use common::{assert_same, assert_same_ptr, impls, Rng, SEED};
use std::ffi::c_char;

/// C1 — `len_with_nul == 1`: the empty string.
#[test]
fn cfg_c1_empty() {
    for _ in 0..1000 {
        assert_same(b"\0");
    }
}

/// C2 — `len_with_nul == 2`: exhaustively all 255 legal single-byte contents.
#[test]
fn cfg_c2_all_single_bytes() {
    for b in 1u16..=255 {
        assert_same(&[b as u8, 0]);
    }
}

/// C3 — `len_with_nul == 3`: exhaustively all 255x255 two-byte contents.
#[test]
fn cfg_c3_all_two_byte_pairs() {
    for a in 1u16..=255 {
        for b in 1u16..=255 {
            assert_same(&[a as u8, b as u8, 0]);
        }
    }
}

/// C4 — small `malloc` size classes: payloads 1..=64, many random trials each.
#[test]
fn cfg_c4_small_sizes_sweep() {
    let mut rng = Rng::new(SEED ^ 4);
    for len in 1..=64usize {
        for _ in 0..64 {
            assert_same(&rng.cstring(len));
        }
    }
}

/// C5 — `memcpy`/`malloc` alignment boundaries.
#[test]
fn cfg_c5_alignment_boundaries() {
    let mut rng = Rng::new(SEED ^ 5);
    const LENS: &[usize] = &[
        7, 8, 9, 15, 16, 17, 23, 24, 25, 31, 32, 33, 63, 64, 65, 127, 128, 129,
    ];
    for &len in LENS {
        for _ in 0..256 {
            assert_same(&rng.cstring(len));
        }
    }
}

/// C6 — page boundaries.
#[test]
fn cfg_c6_page_boundaries() {
    let mut rng = Rng::new(SEED ^ 6);
    const LENS: &[usize] = &[4094, 4095, 4096, 4097, 4098, 8191, 8192, 8193];
    for &len in LENS {
        for _ in 0..64 {
            assert_same(&rng.cstring(len));
        }
    }
}

/// C7 — 1 MiB of randomized bytes.
#[test]
fn cfg_c7_one_mib() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..8 {
        assert_same(&rng.cstring(1024 * 1024));
    }
}

/// C8 — past `malloc`'s default mmap threshold (a different allocator path).
#[test]
fn cfg_c8_mmap_threshold() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..2 {
        assert_same(&rng.cstring(16 * 1024 * 1024 + 1));
    }
}

/// C9 — high-bit-only and deliberately invalid UTF-8 contents. A translation
/// that routed through `str`/`CStr` UTF-8 validation would diverge here.
#[test]
fn cfg_c9_non_utf8() {
    let mut rng = Rng::new(SEED ^ 9);

    // High-bit-only payloads of randomized length.
    for _ in 0..2000 {
        let len = rng.below(512) as usize;
        let mut v: Vec<u8> = (0..len).map(|_| 0x80 + (rng.below(128) as u8)).collect();
        v.push(0);
        assert_same(&v);
    }

    // Hand-built invalid UTF-8: lone continuation bytes, truncated sequences,
    // 0xFE/0xFF, overlong forms, surrogate encodings.
    let fixtures: &[&[u8]] = &[
        &[0x80, 0],
        &[0xBF, 0],
        &[0xC0, 0],
        &[0xC0, 0x80, 0],
        &[0xC2, 0],
        &[0xE0, 0x80, 0],
        &[0xE2, 0x82, 0],
        &[0xED, 0xA0, 0x80, 0],
        &[0xF0, 0x9F, 0],
        &[0xF5, 0x80, 0x80, 0x80, 0],
        &[0xFE, 0],
        &[0xFF, 0],
        &[0xFF, 0xFE, 0xFD, 0xFC, 0],
        &[0xF4, 0x90, 0x80, 0x80, 0],
    ];
    for f in fixtures {
        assert_same(f);
    }

    // Long runs of a single high byte, every value 0x80..=0xFF.
    for b in 0x80u16..=0xFF {
        let mut v = vec![b as u8; 300];
        v.push(0);
        assert_same(&v);
    }
}

/// C10 — misaligned input pointer: the same logical string read at byte offsets
/// 0..=15 inside an over-aligned backing buffer.
#[test]
fn cfg_c10_misaligned_input() {
    let mut rng = Rng::new(SEED ^ 10);
    let i = impls();

    #[repr(align(64))]
    struct Aligned([u8; 1024]);

    for _ in 0..500 {
        let len = rng.below(256) as usize;
        let payload: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();

        for off in 0..16usize {
            let mut buf = Aligned([0u8; 1024]);
            buf.0[off..off + len].copy_from_slice(&payload);
            buf.0[off + len] = 0;
            let start = &buf.0[off..off + len + 1];
            assert_same_ptr(i, start.as_ptr() as *const c_char, Some(start));
        }
    }
}

/// C11 — ownership shape: fresh, distinct, `free`-able allocations. (Also
/// covers ERRORS.md row G6.) `assert_same_ptr` checks distinctness and calls
/// `free`; here the two results are additionally held alive at once.
#[test]
fn cfg_c11_result_is_free_able() {
    let mut rng = Rng::new(SEED ^ 11);
    let i = impls();

    for _ in 0..500 {
        let n = rng.below(200) as usize;
        let input = rng.cstring(n);
        let p = input.as_ptr() as *const c_char;

        let a = unsafe { (i.rust)(p) };
        let b = unsafe { (i.rust)(p) };
        assert!(!a.is_null() && !b.is_null());
        assert_ne!(a, b, "two calls must return distinct buffers");

        let ca = unsafe { (i.c)(p) };
        let cb = unsafe { (i.c)(p) };
        assert!(!ca.is_null() && !cb.is_null());
        assert_ne!(ca, cb, "two C calls must return distinct buffers");

        // All four alive simultaneously, then all released with libc free.
        for q in [a, b, ca, cb] {
            let n = unsafe { libc::strlen(q) };
            assert_eq!(n + 1, input.len());
            let bytes = unsafe { std::slice::from_raw_parts(q as *const u8, n + 1) };
            assert_eq!(bytes, &input[..]);
        }
        for q in [a, b, ca, cb] {
            unsafe { libc::free(q as *mut libc::c_void) };
        }
    }
}

/// C12 — the NUL terminator is the last readable byte before an unmapped guard
/// page. If either implementation read past the terminator it would segfault.
#[test]
fn cfg_c12_guard_page() {
    let mut rng = Rng::new(SEED ^ 12);
    let i = impls();
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;

    unsafe {
        let region = libc::mmap(
            std::ptr::null_mut(),
            page * 2,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(region, libc::MAP_FAILED, "mmap failed");
        // Make the second page inaccessible.
        assert_eq!(
            libc::mprotect(region.byte_add(page), page, libc::PROT_NONE),
            0,
            "mprotect failed"
        );
        let base = region as *mut u8;

        for len in 0..=64usize {
            for _ in 0..16 {
                // Place payload + NUL so the NUL sits at the final byte of page 1.
                let start = base.add(page - (len + 1));
                for k in 0..len {
                    *start.add(k) = rng.nonzero_byte();
                }
                *start.add(len) = 0;
                let slice = std::slice::from_raw_parts(start, len + 1);
                assert_same_ptr(i, start as *const c_char, Some(slice));
            }
        }

        libc::munmap(region, page * 2);
    }
}

/// C13 — 2000 interleaved C/Rust calls with all results held alive at once,
/// proving neither implementation carries hidden state between calls and that
/// they do not interfere through the shared allocator.
#[test]
fn cfg_c13_interleaved_stateful() {
    let mut rng = Rng::new(SEED ^ 13);
    let i = impls();

    let mut inputs: Vec<Vec<u8>> = Vec::new();
    let mut live: Vec<(*mut c_char, usize)> = Vec::new();

    for iter in 0..2000usize {
        let n = rng.below(300) as usize;
        let input = rng.cstring(n);
        let p = input.as_ptr() as *const c_char;
        // Alternate which implementation goes first.
        let (a, b) = if iter % 2 == 0 {
            (unsafe { (i.c)(p) }, unsafe { (i.rust)(p) })
        } else {
            let r = unsafe { (i.rust)(p) };
            (unsafe { (i.c)(p) }, r)
        };
        assert!(!a.is_null() && !b.is_null());
        let idx = inputs.len();
        inputs.push(input);
        live.push((a, idx));
        live.push((b, idx));
    }

    // Validate every buffer only now that thousands of allocations are live.
    for (p, idx) in &live {
        let n = unsafe { libc::strlen(*p) };
        let bytes = unsafe { std::slice::from_raw_parts(*p as *const u8, n + 1) };
        assert_eq!(bytes, &inputs[*idx][..], "buffer corrupted while live");
    }
    for (p, _) in live {
        unsafe { libc::free(p as *mut libc::c_void) };
    }
}

/// C14 — free-form property sweep: random length 0..=8192, random contents.
#[test]
fn cfg_c14_property_sweep() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..5000 {
        let len = rng.below(8193) as usize;
        assert_same(&rng.cstring(len));
    }
}
