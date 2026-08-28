//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1..C14). Every row calls the C `.so` and
//! the Rust `.so` through their exported `custom_strdup` symbol and compares the
//! results byte-for-byte. Randomized rows use the fixed-seed PRNG in
//! `common::Rng` so failures reproduce exactly.

mod common;

use common::{Rng, assert_same, assert_same_payload, bytes_with_nul, c_free, libs};
use std::ffi::c_char;

/// Sanity: both `.so`s loaded and both export the symbol.
#[test]
fn libraries_load_and_export_custom_strdup() {
    let l = libs();
    println!("C    .so: {}", l.c_path.display());
    println!("Rust .so: {}", l.rust_path.display());
    // Resolving the symbols already happened in `libs()`; prove they are usable.
    let src = b"loaded\0";
    let (cp, rp) = unsafe { ((l.c)(src.as_ptr() as *const c_char), (l.rust)(src.as_ptr() as *const c_char)) };
    assert!(!cp.is_null() && !rp.is_null());
    unsafe {
        assert_eq!(bytes_with_nul(cp), b"loaded\0".to_vec());
        assert_eq!(bytes_with_nul(rp), b"loaded\0".to_vec());
        c_free(cp);
        c_free(rp);
    }
}

// ---------------------------------------------------------------------------
// C1 — empty string: len == 1, 1-byte malloc, 1-byte memcpy.
// ---------------------------------------------------------------------------
#[test]
fn c1_empty_string() {
    assert_same(b"\0", "C1/empty");

    // Repeat: a 1-byte allocation is the smallest arena request; make sure it is
    // stable across many calls and that the single byte really is the NUL.
    let l = libs();
    for i in 0..500 {
        let src = b"\0";
        let (cp, rp) = unsafe {
            (
                (l.c)(src.as_ptr() as *const c_char),
                (l.rust)(src.as_ptr() as *const c_char),
            )
        };
        assert!(!cp.is_null(), "C1/iter{i}: C returned NULL");
        assert!(!rp.is_null(), "C1/iter{i}: Rust returned NULL");
        unsafe {
            assert_eq!(*cp as u8, 0, "C1/iter{i}: C byte 0");
            assert_eq!(*rp as u8, 0, "C1/iter{i}: Rust byte 0");
            assert_eq!(bytes_with_nul(cp).len(), 1);
            assert_eq!(bytes_with_nul(rp).len(), 1);
            c_free(cp);
            c_free(rp);
        }
    }
}

// ---------------------------------------------------------------------------
// C2 — length exactly 1, swept over all 255 non-NUL byte values.
// ---------------------------------------------------------------------------
#[test]
fn c2_single_byte_all_values() {
    for b in 1u16..=255 {
        let buf = [b as u8, 0u8];
        assert_same(&buf, &format!("C2/byte=0x{b:02X}"));
    }
}

// ---------------------------------------------------------------------------
// C3 — small lengths 2..=64, randomized printable ASCII, many samples each.
// ---------------------------------------------------------------------------
#[test]
fn c3_small_ascii_random() {
    let mut rng = Rng::new();
    for len in 2usize..=64 {
        for s in 0..40 {
            let payload = rng.ascii(len);
            assert_same_payload(&payload, &format!("C3/len={len}/sample={s}"));
        }
    }
}

// ---------------------------------------------------------------------------
// C4 — medium lengths 65..=4096, randomized content, many samples.
// ---------------------------------------------------------------------------
#[test]
fn c4_medium_random() {
    let mut rng = Rng::new();
    for s in 0..1500 {
        let len = rng.in_range(65, 4096);
        let payload = rng.payload(len);
        assert_same_payload(&payload, &format!("C4/sample={s}/len={len}"));
    }
}

// ---------------------------------------------------------------------------
// C5 — page-crossing / power-of-two boundary lengths (and +-1 around them).
// ---------------------------------------------------------------------------
#[test]
fn c5_page_boundary_lengths() {
    let mut rng = Rng::new();
    let mut lens: Vec<usize> = Vec::new();
    for base in [
        1usize, 2, 4, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 511,
        512, 513, 1023, 1024, 1025, 2047, 2048, 2049, 4095, 4096, 4097, 8191, 8192, 8193,
    ] {
        lens.push(base);
    }
    for &len in &lens {
        for s in 0..12 {
            let payload = rng.payload(len);
            assert_same_payload(&payload, &format!("C5/len={len}/sample={s}"));
        }
    }
}

// ---------------------------------------------------------------------------
// C6 — full byte alphabet: every value 0x01..=0xFF, plus random permutations.
// ---------------------------------------------------------------------------
#[test]
fn c6_all_byte_values() {
    let alphabet: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    assert_eq!(alphabet.len(), 255);
    assert_same_payload(&alphabet, "C6/ascending-alphabet");

    let descending: Vec<u8> = alphabet.iter().rev().copied().collect();
    assert_same_payload(&descending, "C6/descending-alphabet");

    // Randomized permutations (Fisher-Yates with the seeded PRNG).
    let mut rng = Rng::new();
    for s in 0..300 {
        let mut v = alphabet.clone();
        for i in (1..v.len()).rev() {
            let j = rng.below(i + 1);
            v.swap(i, j);
        }
        assert_same_payload(&v, &format!("C6/permutation={s}"));
    }

    // Repeated alphabet, so the payload spans many pages while still covering
    // every byte value.
    let mut long: Vec<u8> = Vec::new();
    for _ in 0..40 {
        long.extend_from_slice(&alphabet);
    }
    assert_same_payload(&long, "C6/alphabet-x40");
}

// ---------------------------------------------------------------------------
// C7 — high-bit / non-UTF-8 payloads only (negative as signed char).
// ---------------------------------------------------------------------------
#[test]
fn c7_high_bytes_non_utf8() {
    let mut rng = Rng::new();

    // Explicitly invalid UTF-8 sequences: lone continuation bytes, truncated
    // multi-byte starters, overlong encodings, surrogate-range encodings.
    let handpicked: [&[u8]; 8] = [
        &[0x80],
        &[0xBF, 0xBF, 0xBF],
        &[0xC0, 0xAF],
        &[0xE0, 0x80, 0xAF],
        &[0xED, 0xA0, 0x80],
        &[0xF4, 0x90, 0x80, 0x80],
        &[0xFE, 0xFF],
        &[0xFF; 64],
    ];
    for (i, p) in handpicked.iter().enumerate() {
        assert_same_payload(p, &format!("C7/handpicked={i}"));
    }

    for s in 0..800 {
        let len = rng.in_range(1, 3000);
        let payload = rng.high_bytes(len);
        assert_same_payload(&payload, &format!("C7/random={s}/len={len}"));
    }
}

// ---------------------------------------------------------------------------
// C8 — large allocations (glibc mmap path rather than the arena path).
// ---------------------------------------------------------------------------
#[test]
fn c8_large_strings() {
    let mut rng = Rng::new();
    for &len in &[
        64 * 1024usize,
        128 * 1024,
        1024 * 1024,
        1024 * 1024 + 1,
        4 * 1024 * 1024,
        8 * 1024 * 1024,
    ] {
        // Fill with a random-but-cheap repeating pattern; every byte non-NUL.
        let seed = rng.next_u64();
        let mut payload = vec![0u8; len];
        let mut x = seed | 1;
        for slot in payload.iter_mut() {
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            *slot = ((x >> 33) as u8) | 1; // guarantees non-zero
        }
        assert_same_payload(&payload, &format!("C8/len={len}"));
    }
}

// ---------------------------------------------------------------------------
// C9 — NUL as the last readable byte before an unmapped guard page.
//
// Proves neither implementation reads past the terminator: any over-read would
// touch the PROT_NONE page and take SIGSEGV.
// ---------------------------------------------------------------------------
#[test]
fn c9_string_flush_against_guard_page() {
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    assert!(page >= 1024);

    let mut rng = Rng::new();

    // Payload lengths that place the NUL exactly at the last byte of the
    // readable page, including lengths that leave the *start* unaligned.
    for &payload_len in &[0usize, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128] {
        let total = 2 * page;
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                total,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED, "mmap failed");

        // Make the second page inaccessible: it is the guard.
        let guard = unsafe { (base as *mut u8).add(page) };
        let rc = unsafe { libc::mprotect(guard as *mut libc::c_void, page, libc::PROT_NONE) };
        assert_eq!(rc, 0, "mprotect failed");

        // Place payload + NUL so the NUL is the final readable byte.
        let start = unsafe { (base as *mut u8).add(page - payload_len - 1) };
        let payload = rng.payload(payload_len);
        unsafe {
            std::ptr::copy_nonoverlapping(payload.as_ptr(), start, payload_len);
            *start.add(payload_len) = 0;
        }

        let l = libs();
        let src = start as *const c_char;
        let (cp, rp) = unsafe { ((l.c)(src), (l.rust)(src)) };
        assert!(!cp.is_null(), "C9/len={payload_len}: C returned NULL");
        assert!(!rp.is_null(), "C9/len={payload_len}: Rust returned NULL");

        let mut expected = payload.clone();
        expected.push(0);
        unsafe {
            assert_eq!(
                bytes_with_nul(cp),
                expected,
                "C9/len={payload_len}: C mismatch"
            );
            assert_eq!(
                bytes_with_nul(rp),
                expected,
                "C9/len={payload_len}: Rust mismatch"
            );
            c_free(cp);
            c_free(rp);
        }

        unsafe {
            assert_eq!(libc::munmap(base, total), 0, "munmap failed");
        }
    }
}

// ---------------------------------------------------------------------------
// C10 — unaligned source pointers (SIMD prologue of strlen/memcpy).
// ---------------------------------------------------------------------------
#[test]
fn c10_unaligned_source_offsets() {
    let mut rng = Rng::new();

    for offset in 0usize..=16 {
        for s in 0..60 {
            let len = rng.in_range(1, 600);
            let payload = rng.payload(len);

            // Over-allocate then start the string `offset` bytes in. The Vec's
            // own allocation is 16-byte aligned from glibc, so `offset`
            // directly controls the misalignment.
            let mut buf = vec![0xAAu8; offset];
            buf.extend_from_slice(&payload);
            buf.push(0);

            let src = unsafe { buf.as_ptr().add(offset) } as *const c_char;
            let l = libs();
            let (cp, rp) = unsafe { ((l.c)(src), (l.rust)(src)) };
            assert!(!cp.is_null() && !rp.is_null());

            let mut expected = payload.clone();
            expected.push(0);
            unsafe {
                assert_eq!(
                    bytes_with_nul(cp),
                    expected,
                    "C10/offset={offset}/sample={s}: C mismatch"
                );
                assert_eq!(
                    bytes_with_nul(rp),
                    expected,
                    "C10/offset={offset}/sample={s}: Rust mismatch"
                );
                c_free(cp);
                c_free(rp);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C11 — many interleaved calls; statelessness and allocator non-interference.
// ---------------------------------------------------------------------------
#[test]
fn c11_many_interleaved_calls() {
    let mut rng = Rng::new();
    let l = libs();

    let mut sources: Vec<Vec<u8>> = Vec::new();
    let mut live: Vec<(*mut c_char, *mut c_char, usize)> = Vec::new();

    for i in 0..2000 {
        let len = rng.in_range(0, 300);
        let mut buf = rng.payload(len);
        buf.push(0);

        let src = buf.as_ptr() as *const c_char;
        // Interleave: C then Rust, and on odd iterations Rust then C, so any
        // ordering-dependent allocator interaction is exercised both ways.
        let (cp, rp) = if i % 2 == 0 {
            unsafe { ((l.c)(src), (l.rust)(src)) }
        } else {
            let r = unsafe { (l.rust)(src) };
            let c = unsafe { (l.c)(src) };
            (c, r)
        };
        assert!(!cp.is_null() && !rp.is_null(), "C11/iter={i}");

        sources.push(buf);
        live.push((cp, rp, i));

        // Free some results early (shuffled order) while others stay live, so
        // the heap is fragmented and interleaved between the two libraries.
        if live.len() > 64 {
            let victim = rng.below(live.len());
            let (c, r, idx) = live.swap_remove(victim);
            let expected = {
                let s = &sources[idx];
                let n = s.iter().position(|&b| b == 0).unwrap();
                let mut v = s[..n].to_vec();
                v.push(0);
                v
            };
            unsafe {
                assert_eq!(bytes_with_nul(c), expected, "C11/late-check C idx={idx}");
                assert_eq!(bytes_with_nul(r), expected, "C11/late-check Rust idx={idx}");
                c_free(c);
                c_free(r);
            }
        }
    }

    // Drain the rest, verifying contents survived unrelated allocation traffic.
    for (c, r, idx) in live {
        let expected = {
            let s = &sources[idx];
            let n = s.iter().position(|&b| b == 0).unwrap();
            let mut v = s[..n].to_vec();
            v.push(0);
            v
        };
        unsafe {
            assert_eq!(bytes_with_nul(c), expected, "C11/drain C idx={idx}");
            assert_eq!(bytes_with_nul(r), expected, "C11/drain Rust idx={idx}");
            c_free(c);
            c_free(r);
        }
    }
}

// ---------------------------------------------------------------------------
// C12 — the result is an independent copy, not an alias of the input.
// ---------------------------------------------------------------------------
#[test]
fn c12_result_is_independent_copy() {
    let mut rng = Rng::new();
    let l = libs();

    for s in 0..300 {
        let len = rng.in_range(1, 512);
        let payload = rng.payload(len);
        let mut buf = payload.clone();
        buf.push(0);

        let (cp, rp) = unsafe {
            (
                (l.c)(buf.as_ptr() as *const c_char),
                (l.rust)(buf.as_ptr() as *const c_char),
            )
        };
        assert!(!cp.is_null() && !rp.is_null());

        let mut expected = payload.clone();
        expected.push(0);

        // Mutate the *source* after the call: copies must be unaffected.
        for b in buf.iter_mut().take(len) {
            *b = b.wrapping_add(7) | 1;
        }
        unsafe {
            assert_eq!(bytes_with_nul(cp), expected, "C12/s={s}: C copy changed");
            assert_eq!(bytes_with_nul(rp), expected, "C12/s={s}: Rust copy changed");
        }

        // Mutate the copies: the source must be unaffected, and the two copies
        // must be independent of each other.
        let snapshot = buf.clone();
        unsafe {
            for i in 0..len {
                *cp.add(i as isize as usize) = 0x41 as c_char;
            }
        }
        assert_eq!(buf, snapshot, "C12/s={s}: writing C copy touched source");
        unsafe {
            assert_eq!(
                bytes_with_nul(rp),
                expected,
                "C12/s={s}: writing C copy touched Rust copy"
            );
            c_free(cp);
            c_free(rp);
        }
    }
}

// ---------------------------------------------------------------------------
// C13 — allocator compatibility: results are libc-`free`-able.
// ---------------------------------------------------------------------------
#[test]
fn c13_returned_pointer_is_free_able() {
    let mut rng = Rng::new();
    let l = libs();

    // Cover arena sizes and mmap sizes, allocating and freeing repeatedly. If
    // the Rust side had used the Rust global allocator, `free` on its pointer
    // would corrupt or abort here.
    for &len in &[0usize, 1, 8, 24, 120, 1000, 5000, 100_000, 200_000, 1_000_000] {
        for _ in 0..50 {
            let mut buf = rng.payload(len);
            buf.push(0);
            let src = buf.as_ptr() as *const c_char;
            let (cp, rp) = unsafe { ((l.c)(src), (l.rust)(src)) };
            assert!(!cp.is_null() && !rp.is_null(), "C13/len={len}");
            unsafe {
                assert_eq!(bytes_with_nul(cp), bytes_with_nul(rp), "C13/len={len}");
                c_free(cp);
                c_free(rp);
            }
        }
    }

    // Also exercise malloc_usable_size-style expectations indirectly: allocate
    // many, free all, allocate again — a mismatched allocator shows up as a
    // crash or a corrupted second round.
    let mut batch = Vec::new();
    for _ in 0..500 {
        let mut buf = rng.payload(64);
        buf.push(0);
        let src = buf.as_ptr() as *const c_char;
        let (cp, rp) = unsafe { ((l.c)(src), (l.rust)(src)) };
        batch.push((cp, rp, buf));
    }
    for (cp, rp, buf) in batch {
        let expected = {
            let n = buf.iter().position(|&b| b == 0).unwrap();
            let mut v = buf[..n].to_vec();
            v.push(0);
            v
        };
        unsafe {
            assert_eq!(bytes_with_nul(cp), expected);
            assert_eq!(bytes_with_nul(rp), expected);
            c_free(cp);
            c_free(rp);
        }
    }
}

// ---------------------------------------------------------------------------
// C14 — embedded NUL truncates at the first terminator.
// ---------------------------------------------------------------------------
#[test]
fn c14_embedded_nul_truncates() {
    let l = libs();

    let buf = b"abc\0def\0";
    let (cp, rp) = unsafe {
        (
            (l.c)(buf.as_ptr() as *const c_char),
            (l.rust)(buf.as_ptr() as *const c_char),
        )
    };
    assert!(!cp.is_null() && !rp.is_null());
    unsafe {
        assert_eq!(bytes_with_nul(cp), b"abc\0".to_vec(), "C14: C");
        assert_eq!(bytes_with_nul(rp), b"abc\0".to_vec(), "C14: Rust");
        // Exactly 4 bytes copied, not 8: byte 4 of the copy is *not* 'd'.
        c_free(cp);
        c_free(rp);
    }

    // Leading NUL: nothing but the terminator is copied.
    assert_same(b"\0ignored-tail\0", "C14/leading-nul");

    // Randomized: NUL at a random position, arbitrary trailing garbage.
    let mut rng = Rng::new();
    for s in 0..400 {
        let head_len = rng.in_range(0, 200);
        let tail_len = rng.in_range(1, 200);
        let head = rng.payload(head_len);
        let tail = rng.payload(tail_len);

        let mut buf = head.clone();
        buf.push(0);
        buf.extend_from_slice(&tail);
        buf.push(0);

        assert_same(&buf, &format!("C14/random={s}/head={head_len}"));
    }
}
