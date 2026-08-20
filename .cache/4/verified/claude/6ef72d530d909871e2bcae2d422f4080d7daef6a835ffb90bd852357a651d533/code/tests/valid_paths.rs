//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1..C15). Every call goes through both
//! shared objects (C and Rust) via `libloading`.

mod common;

use common::*;
use std::collections::HashSet;
use std::ffi::c_char;

// ---------------------------------------------------------------------------
// C1 — empty string
// ---------------------------------------------------------------------------
#[test]
fn cfg_c1_empty_string() {
    assert_same_dup(b"", "C1 empty");

    // Explicitly check the shape of the returned 1-byte block for both libs.
    let src = b"\0";
    let cf = c_strdup();
    let rf = rust_strdup();
    unsafe {
        let c = cf(src.as_ptr() as *const c_char);
        let r = rf(src.as_ptr() as *const c_char);
        assert!(!c.is_null() && !r.is_null(), "C1: NULL returned for \"\"");
        assert_eq!(*c, 0, "C1: C copy of \"\" is not \\0");
        assert_eq!(*r, 0, "C1: Rust copy of \"\" is not \\0");
        assert_eq!(libc::strlen(c), 0);
        assert_eq!(libc::strlen(r), 0);
        libc::free(c as *mut libc::c_void);
        libc::free(r as *mut libc::c_void);
    }
}

// ---------------------------------------------------------------------------
// C2 — every possible single-byte string
// ---------------------------------------------------------------------------
#[test]
fn cfg_c2_all_single_bytes() {
    for b in 1u8..=255 {
        assert_same_dup(&[b], &format!("C2 byte 0x{b:02X}"));
    }
}

// ---------------------------------------------------------------------------
// C3 — randomized printable ASCII, lengths 1..=64
// ---------------------------------------------------------------------------
#[test]
fn cfg_c3_random_ascii_short() {
    let mut rng = Rng::new(SEED ^ 0xC3);
    for i in 0..400 {
        let len = 1 + rng.below(64) as usize;
        let s = rng.ascii_bytes(len);
        assert_same_dup(&s, &format!("C3 iter {i} len {len}"));
    }
}

// ---------------------------------------------------------------------------
// C4 — randomized arbitrary bytes (invalid UTF-8 included), lengths 1..=256
// ---------------------------------------------------------------------------
#[test]
fn cfg_c4_random_binary() {
    let mut rng = Rng::new(SEED ^ 0xC4);
    for i in 0..400 {
        let len = 1 + rng.below(256) as usize;
        let s = rng.nonzero_bytes(len);
        assert_same_dup(&s, &format!("C4 iter {i} len {len}"));
    }
    // Deliberately invalid UTF-8 sequences / high bytes only.
    for pat in [
        vec![0xFFu8; 33],
        vec![0x80u8; 17],
        vec![0xC3u8; 8],
        vec![0xEDu8, 0xA0, 0x80, 0xF5, 0xFE, 0xFF],
        vec![0x7Fu8, 0x0A, 0x0D, 0x09, 0x1B, 0xF4, 0x90, 0x80, 0x80],
    ] {
        assert_same_dup(&pat, "C4 fixed non-utf8 pattern");
    }
}

// ---------------------------------------------------------------------------
// C5 — exhaustive lengths 0..=300
// ---------------------------------------------------------------------------
#[test]
fn cfg_c5_exhaustive_small_lengths() {
    let mut rng = Rng::new(SEED ^ 0xC5);
    for len in 0..=300usize {
        let s = rng.nonzero_bytes(len);
        assert_same_dup(&s, &format!("C5 len {len}"));
    }
}

// ---------------------------------------------------------------------------
// C6 — page / power-of-two boundary lengths
// ---------------------------------------------------------------------------
#[test]
fn cfg_c6_page_boundary_lengths() {
    let mut rng = Rng::new(SEED ^ 0xC6);
    let mut lens: Vec<usize> = Vec::new();
    for base in [1024usize, 2048, 4096, 8192, 16384, 32768, 65536] {
        for d in [-2i64, -1, 0, 1, 2] {
            lens.push((base as i64 + d) as usize);
        }
    }
    lens.push(4093);
    lens.push(4099);
    lens.push(8189);
    lens.push(8195);
    for len in lens {
        let s = rng.nonzero_bytes(len);
        assert_same_dup(&s, &format!("C6 len {len}"));
    }
}

// ---------------------------------------------------------------------------
// C7 — large inputs (1 MiB, 4 MiB)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c7_large_inputs() {
    let mut rng = Rng::new(SEED ^ 0xC7);
    for len in [1usize << 20, (1 << 22) + 7] {
        let s = rng.nonzero_bytes(len);
        assert_same_dup(&s, &format!("C7 len {len}"));
    }
}

// ---------------------------------------------------------------------------
// C8 — unaligned / interior source pointers
// ---------------------------------------------------------------------------
#[test]
fn cfg_c8_unaligned_source_offsets() {
    let mut rng = Rng::new(SEED ^ 0xC8);
    for off in 0..=16usize {
        for len in [0usize, 1, 7, 15, 16, 17, 31, 33, 63, 65, 127] {
            let payload = rng.nonzero_bytes(len);
            let mut buf: Vec<u8> = rng.nonzero_bytes(off);
            buf.extend_from_slice(&payload);
            buf.push(0);
            // Junk after the terminator must be ignored.
            buf.extend_from_slice(&rng.nonzero_bytes(8));
            unsafe {
                assert_same_dup_raw(
                    buf.as_ptr().add(off) as *const c_char,
                    len,
                    &format!("C8 off {off} len {len}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C9 — trailing garbage after the terminating NUL
// ---------------------------------------------------------------------------
#[test]
fn cfg_c9_trailing_garbage_after_nul() {
    let mut rng = Rng::new(SEED ^ 0xC9);
    for k in 0..=64usize {
        let mut buf = rng.nonzero_bytes(128);
        buf[k] = 0;
        unsafe {
            assert_same_dup_raw(
                buf.as_ptr() as *const c_char,
                k,
                &format!("C9 nul at {k}"),
            );
        }
        // The copies were freed; make sure the source junk is still intact.
        assert!(buf[k + 1..].iter().all(|&b| b != 0), "C9: source clobbered");
    }
}

// ---------------------------------------------------------------------------
// C10 — terminating NUL is the last readable byte before an unmapped page
// ---------------------------------------------------------------------------
#[test]
fn cfg_c10_nul_at_page_edge() {
    let mut rng = Rng::new(SEED ^ 0xCA);
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    unsafe {
        let total = page * 2;
        let base = libc::mmap(
            std::ptr::null_mut(),
            total,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(base, libc::MAP_FAILED, "C10: mmap failed");
        let base = base as *mut u8;
        // Make the second page inaccessible: any read past the NUL faults.
        assert_eq!(
            libc::mprotect(base.add(page) as *mut libc::c_void, page, libc::PROT_NONE),
            0,
            "C10: mprotect failed"
        );

        for len in 0..=64usize {
            // Fill the whole first page with non-zero junk.
            for i in 0..page {
                *base.add(i) = rng.nonzero_byte();
            }
            // NUL exactly at the last readable byte.
            *base.add(page - 1) = 0;
            let src = base.add(page - 1 - len);
            assert_same_dup_raw(
                src as *const c_char,
                len,
                &format!("C10 page-edge len {len}"),
            );
        }

        libc::munmap(base as *mut libc::c_void, total);
    }
}

// ---------------------------------------------------------------------------
// C11 — repeated / interleaved calls, results alive simultaneously
// ---------------------------------------------------------------------------
#[test]
fn cfg_c11_interleaved_repeated_calls() {
    let mut rng = Rng::new(SEED ^ 0xCB);
    let cf = c_strdup();
    let rf = rust_strdup();

    let mut sources: Vec<Vec<u8>> = Vec::new();
    let mut results: Vec<(*mut c_char, *mut c_char, usize)> = Vec::new();

    for _ in 0..500 {
        let len = rng.below(96) as usize;
        let mut s = rng.nonzero_bytes(len);
        s.push(0);
        sources.push(s);
    }

    for (i, s) in sources.iter().enumerate() {
        unsafe {
            let src = s.as_ptr() as *const c_char;
            // Alternate the order the two implementations are invoked in.
            let (c, r) = if i % 2 == 0 {
                let c = cf(src);
                let r = rf(src);
                (c, r)
            } else {
                let r = rf(src);
                let c = cf(src);
                (c, r)
            };
            assert!(!c.is_null() && !r.is_null(), "C11 iter {i}: NULL");
            results.push((c, r, s.len() - 1));
        }
    }

    // All 1000 live blocks must be distinct, and each must hold the right bytes.
    let mut seen: HashSet<usize> = HashSet::new();
    for (i, (c, r, len)) in results.iter().copied().enumerate() {
        unsafe {
            let want = &sources[i][..len + 1];
            let cgot = std::slice::from_raw_parts(c as *const u8, len + 1);
            let rgot = std::slice::from_raw_parts(r as *const u8, len + 1);
            assert_eq!(cgot, want, "C11 iter {i}: C content");
            assert_eq!(rgot, cgot, "C11 iter {i}: Rust content");
        }
        assert!(seen.insert(c as usize), "C11 iter {i}: duplicate C block");
        assert!(seen.insert(r as usize), "C11 iter {i}: duplicate Rust block");
    }
    for (c, r, _) in results {
        unsafe {
            libc::free(c as *mut libc::c_void);
            libc::free(r as *mut libc::c_void);
        }
    }
    // Sources untouched.
    for (i, s) in sources.iter().enumerate() {
        assert_eq!(s[s.len() - 1], 0, "C11 iter {i}: source terminator changed");
    }
}

// ---------------------------------------------------------------------------
// C12 — properties of the returned block (C heap, independent, writable)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c12_result_block_properties() {
    let mut rng = Rng::new(SEED ^ 0xCC);
    let cf = c_strdup();
    let rf = rust_strdup();
    for i in 0..200 {
        let len = rng.below(200) as usize;
        let mut s = rng.nonzero_bytes(len);
        s.push(0);
        let original = s.clone();
        unsafe {
            let src = s.as_ptr() as *const c_char;
            let c = cf(src);
            let r = rf(src);
            assert!(!c.is_null() && !r.is_null(), "C12 iter {i}: NULL");
            assert_ne!(c as *const c_char, src, "C12: C aliased the source");
            assert_ne!(r as *const c_char, src, "C12: Rust aliased the source");

            // The block must be writable for len+1 bytes and writing must not
            // disturb the source (i.e. it is a real copy, not an alias).
            std::ptr::write_bytes(c as *mut u8, 0xA5, len + 1);
            std::ptr::write_bytes(r as *mut u8, 0x5A, len + 1);
            assert_eq!(s, original, "C12 iter {i}: source changed after writing copy");

            // Releasable with the C allocator (would abort on a bad pointer).
            libc::free(c as *mut libc::c_void);
            libc::free(r as *mut libc::c_void);
        }
    }
}

// ---------------------------------------------------------------------------
// C13 — read-only source mapping (proves no write through `str`)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c13_readonly_source() {
    let mut rng = Rng::new(SEED ^ 0xCD);
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    unsafe {
        let base = libc::mmap(
            std::ptr::null_mut(),
            page,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(base, libc::MAP_FAILED, "C13: mmap failed");
        let base = base as *mut u8;
        for len in [0usize, 1, 5, 64, 255, 1000] {
            for i in 0..page {
                *base.add(i) = 0;
            }
            let payload = rng.nonzero_bytes(len);
            std::ptr::copy_nonoverlapping(payload.as_ptr(), base, len);
            *base.add(len) = 0;
            assert_eq!(
                libc::mprotect(base as *mut libc::c_void, page, libc::PROT_READ),
                0,
                "C13: mprotect RO failed"
            );
            assert_same_dup_raw(base as *const c_char, len, &format!("C13 ro len {len}"));
            assert_eq!(
                libc::mprotect(
                    base as *mut libc::c_void,
                    page,
                    libc::PROT_READ | libc::PROT_WRITE
                ),
                0,
                "C13: mprotect RW failed"
            );
        }
        libc::munmap(base as *mut libc::c_void, page);
    }
}

// ---------------------------------------------------------------------------
// C14 — non-heap sources: .rodata literal and stack buffer
// ---------------------------------------------------------------------------
#[test]
fn cfg_c14_non_heap_sources() {
    // static / .rodata
    static LIT: &[u8] = b"hello, \xffworld\x01\x7f\n\0";
    unsafe {
        assert_same_dup_raw(
            LIT.as_ptr() as *const c_char,
            LIT.len() - 1,
            "C14 rodata literal",
        );
    }
    static EMPTY: &[u8] = b"\0";
    unsafe {
        assert_same_dup_raw(EMPTY.as_ptr() as *const c_char, 0, "C14 rodata empty");
    }

    // stack
    let mut rng = Rng::new(SEED ^ 0xCE);
    for len in [0usize, 1, 2, 3, 8, 31, 200] {
        let mut stack_buf = [0u8; 256];
        for i in 0..len {
            stack_buf[i] = rng.nonzero_byte();
        }
        stack_buf[len] = 0;
        unsafe {
            assert_same_dup_raw(
                stack_buf.as_ptr() as *const c_char,
                len,
                &format!("C14 stack len {len}"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C15 — chained duplication across implementations
// ---------------------------------------------------------------------------
#[test]
fn cfg_c15_chained_duplication() {
    let mut rng = Rng::new(SEED ^ 0xCF);
    let cf = c_strdup();
    let rf = rust_strdup();
    for i in 0..200 {
        let len = rng.below(128) as usize;
        let mut s = rng.nonzero_bytes(len);
        s.push(0);
        unsafe {
            let src = s.as_ptr() as *const c_char;
            // C -> Rust and Rust -> C round trips must both reproduce the input.
            let c1 = cf(src);
            let r2 = rf(c1);
            let r1 = rf(src);
            let c2 = cf(r1);
            for (p, who) in [(c1, "C1"), (r2, "R2"), (r1, "R1"), (c2, "C2")] {
                assert!(!p.is_null(), "C15 iter {i}: {who} NULL");
                let got = std::slice::from_raw_parts(p as *const u8, len + 1);
                assert_eq!(got, &s[..len + 1], "C15 iter {i}: {who} content");
            }
            let a = std::slice::from_raw_parts(r2 as *const u8, len + 1);
            let b = std::slice::from_raw_parts(c2 as *const u8, len + 1);
            assert_eq!(a, b, "C15 iter {i}: chain results differ");
            libc::free(c1 as *mut libc::c_void);
            libc::free(r2 as *mut libc::c_void);
            libc::free(r1 as *mut libc::c_void);
            libc::free(c2 as *mut libc::c_void);
        }
    }
}
