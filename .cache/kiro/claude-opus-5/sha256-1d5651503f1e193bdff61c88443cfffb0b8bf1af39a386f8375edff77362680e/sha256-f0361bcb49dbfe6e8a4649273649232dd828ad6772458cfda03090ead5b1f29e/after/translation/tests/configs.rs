//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares the results byte-for-byte, using
//! many seeded-random inputs per row rather than a single hand-picked value.

mod common;

use common::{Rng, SEED, assert_same, assert_same_bytes, c_impl, rust_impl};
use std::ffi::c_char;

/// Row 1 — `str = NULL`.
#[test]
fn cfg_row01_null_pointer() {
    // Repeated to prove there is no state that makes the second call differ.
    for i in 0..64 {
        assert_same(std::ptr::null(), &format!("row01 null call #{i}"));
    }
}

/// Row 2 — the empty string (length 0, `len == 1`).
#[test]
fn cfg_row02_empty_string() {
    let buf = [0u8];
    for i in 0..64 {
        let got = assert_same_bytes(&buf, &format!("row02 empty #{i}")).unwrap();
        assert_eq!(got, vec![0u8], "row02: empty copy must be exactly one NUL byte");
    }
}

/// Row 3 — length exactly 1, for every one of the 255 possible non-NUL bytes.
#[test]
fn cfg_row03_length_one_all_bytes() {
    for b in 1u16..=255 {
        let buf = [b as u8, 0u8];
        let got = assert_same_bytes(&buf, &format!("row03 byte {b:#04x}")).unwrap();
        assert_eq!(got, vec![b as u8, 0]);
    }
}

/// Row 4 — every length 0..=64 with randomized ASCII content, many seeds.
#[test]
fn cfg_row04_lengths_0_to_64_random_ascii() {
    let mut rng = Rng::new(SEED ^ 0x04);
    for len in 0..=64usize {
        for rep in 0..32 {
            let buf = rng.ascii_cstring(len);
            assert_same_bytes(&buf, &format!("row04 len={len} rep={rep}"));
        }
    }
}

/// Row 5 — machine-word / SIMD `memcpy` boundary lengths.
#[test]
fn cfg_row05_word_and_simd_boundary_lengths() {
    const LENS: &[usize] = &[7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129];
    let mut rng = Rng::new(SEED ^ 0x05);
    for &len in LENS {
        for rep in 0..64 {
            let buf = rng.cstring(len);
            let got = assert_same_bytes(&buf, &format!("row05 len={len} rep={rep}")).unwrap();
            assert_eq!(got.len(), len + 1);
        }
    }
}

/// Row 6 — page-boundary lengths.
#[test]
fn cfg_row06_page_boundary_lengths() {
    const LENS: &[usize] = &[4095, 4096, 4097, 8191, 8192, 8193];
    let mut rng = Rng::new(SEED ^ 0x06);
    for &len in LENS {
        for rep in 0..16 {
            let buf = rng.cstring(len);
            let got = assert_same_bytes(&buf, &format!("row06 len={len} rep={rep}")).unwrap();
            assert_eq!(got.len(), len + 1);
        }
    }
}

/// Row 7 — 1 MiB of randomized non-NUL bytes.
#[test]
fn cfg_row07_one_mib_random() {
    let mut rng = Rng::new(SEED ^ 0x07);
    for rep in 0..4 {
        let buf = rng.cstring(1 << 20);
        let got = assert_same_bytes(&buf, &format!("row07 1MiB rep={rep}")).unwrap();
        assert_eq!(got.len(), (1 << 20) + 1);
    }
}

/// Row 8 — 16 MiB of randomized non-NUL bytes.
#[test]
fn cfg_row08_sixteen_mib_random() {
    let mut rng = Rng::new(SEED ^ 0x08);
    let buf = rng.cstring(16 << 20);
    let got = assert_same_bytes(&buf, "row08 16MiB").unwrap();
    assert_eq!(got.len(), (16 << 20) + 1);
}

/// Row 9 — the complete non-NUL byte domain `1..=255` in one string, plus
/// randomized permutations of it (exercises `char` signedness handling).
#[test]
fn cfg_row09_full_byte_domain() {
    let mut buf: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    buf.push(0);
    let got = assert_same_bytes(&buf, "row09 ascending 1..=255").unwrap();
    assert_eq!(got.len(), 256);
    assert_eq!(&got[..255], &buf[..255]);

    // Descending, and seeded shuffles.
    let mut desc: Vec<u8> = (1u16..=255).rev().map(|b| b as u8).collect();
    desc.push(0);
    assert_same_bytes(&desc, "row09 descending 255..=1");

    let mut rng = Rng::new(SEED ^ 0x09);
    for rep in 0..32 {
        let mut v: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
        for i in (1..v.len()).rev() {
            let j = rng.below(i + 1);
            v.swap(i, j);
        }
        v.push(0);
        assert_same_bytes(&v, &format!("row09 shuffle rep={rep}"));
    }
}

/// Row 10 — only high bytes `0x80..=0xFF`.
#[test]
fn cfg_row10_high_bytes_only() {
    let mut rng = Rng::new(SEED ^ 0x10);
    for rep in 0..128 {
        let len = rng.in_range(1, 512);
        let mut v: Vec<u8> = (0..len).map(|_| 0x80 | (rng.next_u64() as u8 & 0x7F)).collect();
        v.push(0);
        let got = assert_same_bytes(&v, &format!("row10 rep={rep} len={len}")).unwrap();
        assert!(got[..len].iter().all(|b| *b >= 0x80));
    }

    // All-0xFF, at several lengths.
    for len in [1usize, 2, 8, 17, 64, 4096] {
        let mut v = vec![0xFFu8; len];
        v.push(0);
        assert_same_bytes(&v, &format!("row10 all-0xFF len={len}"));
    }
}

/// Row 11 — non-zero garbage AFTER the terminator: exactly `strlen+1` bytes may
/// be copied. If either side over-copied it would also read the garbage, which
/// `assert_same` compares against the faithful `strlen+1` expectation.
#[test]
fn cfg_row11_garbage_after_terminator() {
    let mut rng = Rng::new(SEED ^ 0x11);
    for rep in 0..256 {
        let total = rng.in_range(2, 2048);
        let nul_at = rng.below(total); // terminator somewhere inside the buffer
        let mut buf: Vec<u8> = (0..total).map(|_| rng.non_nul_byte()).collect();
        buf[nul_at] = 0;
        // Trailing bytes after `nul_at` stay non-zero garbage on purpose.
        let got = assert_same_bytes(&buf, &format!("row11 rep={rep} total={total} nul_at={nul_at}"))
            .unwrap();
        assert_eq!(got.len(), nul_at + 1, "row11: copy length must be exactly strlen+1");
        assert_eq!(&got[..nul_at], &buf[..nul_at]);
        assert_eq!(got[nul_at], 0);
    }
}

/// Row 12 — terminator at offset 0 inside a large populated buffer.
#[test]
fn cfg_row12_early_nul_in_large_buffer() {
    let mut rng = Rng::new(SEED ^ 0x12);
    for rep in 0..32 {
        let mut buf: Vec<u8> = (0..(1usize << 16)).map(|_| rng.non_nul_byte()).collect();
        buf[0] = 0;
        let got = assert_same_bytes(&buf, &format!("row12 rep={rep}")).unwrap();
        assert_eq!(got, vec![0u8], "row12: must copy exactly one NUL byte");
    }
}

/// Row 13 — source pointer at each misalignment 0..=15.
#[test]
fn cfg_row13_source_misalignment() {
    let mut rng = Rng::new(SEED ^ 0x13);
    // A 16-byte-aligned backing allocation so offsets are true misalignments.
    let layout = std::alloc::Layout::from_size_align(4096, 64).unwrap();
    let base = unsafe { std::alloc::alloc(layout) };
    assert!(!base.is_null());
    for off in 0..=15usize {
        for rep in 0..64 {
            let len = rng.in_range(0, 200);
            let start = unsafe { base.add(off) };
            for i in 0..len {
                unsafe { *start.add(i) = rng.non_nul_byte() };
            }
            unsafe { *start.add(len) = 0 };
            let got = assert_same(start as *const c_char, &format!("row13 off={off} len={len} rep={rep}"))
                .unwrap();
            assert_eq!(got.len(), len + 1);
        }
    }
    unsafe { std::alloc::dealloc(base, layout) };
}

/// Row 14 — the terminator is the last readable byte before an unmapped guard
/// page. Any read past the NUL would segfault, so surviving this proves neither
/// implementation over-reads.
#[test]
fn cfg_row14_guard_page_no_overread() {
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let mut rng = Rng::new(SEED ^ 0x14);

    for rep in 0..16 {
        // Two pages; the second is made unreadable.
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                2 * page,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(map, libc::MAP_FAILED, "mmap failed");
        let base = map as *mut u8;
        assert_eq!(unsafe { libc::mprotect(base.add(page) as *mut libc::c_void, page, libc::PROT_NONE) }, 0);

        // Content length chosen so the NUL lands on the final readable byte.
        let len = rng.in_range(0, page - 1);
        let start = unsafe { base.add(page - len - 1) };
        for i in 0..len {
            unsafe { *start.add(i) = rng.non_nul_byte() };
        }
        unsafe { *start.add(len) = 0 }; // last byte of the readable page

        let got = assert_same(start as *const c_char, &format!("row14 rep={rep} len={len}")).unwrap();
        assert_eq!(got.len(), len + 1);

        unsafe { libc::munmap(map, 2 * page) };
    }
}

/// Row 15 — the returned buffer must be releasable with `free`, because the C
/// returns `malloc` memory and that is part of the observable ABI. (Every other
/// test already frees via `libc::free`; this row hammers it to make an
/// allocator mismatch show up as heap corruption / abort.)
#[test]
fn cfg_row15_result_is_free_able() {
    let mut rng = Rng::new(SEED ^ 0x15);
    for imp in [c_impl(), rust_impl()] {
        for rep in 0..2000 {
            let len = rng.in_range(0, 300);
            let buf = rng.cstring(len);
            let p = unsafe { (imp.custom_strdup)(buf.as_ptr() as *const c_char) };
            assert!(!p.is_null(), "{} returned NULL for len={len} rep={rep}", imp.name);
            let copied = unsafe { std::slice::from_raw_parts(p as *const u8, len + 1) };
            assert_eq!(copied, &buf[..], "{} produced a bad copy", imp.name);
            unsafe { libc::free(p as *mut libc::c_void) };
        }
    }
}

/// Row 16 — the result is an independent copy, not an alias of the input.
#[test]
fn cfg_row16_result_is_independent_copy() {
    let mut rng = Rng::new(SEED ^ 0x16);
    for imp in [c_impl(), rust_impl()] {
        for rep in 0..256 {
            let len = rng.in_range(1, 128);
            let mut buf = rng.cstring(len);
            let original = buf.clone();
            let p = unsafe { (imp.custom_strdup)(buf.as_ptr() as *const c_char) };
            assert!(!p.is_null());
            assert_ne!(p as usize, buf.as_ptr() as usize, "{} aliased the input", imp.name);

            // Mutating the input must not disturb the copy.
            for b in buf.iter_mut().take(len) {
                *b = 0x7E;
            }
            let copied = unsafe { std::slice::from_raw_parts(p as *const u8, len + 1) };
            assert_eq!(copied, &original[..], "{} copy changed when input changed (rep={rep})", imp.name);

            // Mutating the copy must not disturb the input.
            unsafe { *(p as *mut u8) = 0x21 };
            assert_eq!(buf[0], 0x7E, "{} input changed when copy changed", imp.name);

            unsafe { libc::free(p as *mut libc::c_void) };
        }
    }
}

/// Row 17 — many simultaneously live results; no shared scratch buffer.
#[test]
fn cfg_row17_many_live_results() {
    let mut rng = Rng::new(SEED ^ 0x17);
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for i in 0..256usize {
        let n = rng.in_range(1, 64);
        let mut v = rng.cstring(n);
        // Make each input unique by stamping the index into the first bytes.
        v[0] = (i % 255) as u8 + 1;
        inputs.push(v);
    }

    for imp in [c_impl(), rust_impl()] {
        let mut live: Vec<*mut c_char> = Vec::new();
        for inp in &inputs {
            let p = unsafe { (imp.custom_strdup)(inp.as_ptr() as *const c_char) };
            assert!(!p.is_null());
            live.push(p);
        }
        // All 256 results must be distinct pointers and still hold their data.
        let mut seen: Vec<usize> = live.iter().map(|p| *p as usize).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "{} returned duplicate pointers", imp.name);

        for (inp, p) in inputs.iter().zip(live.iter()) {
            let n = inp.len();
            let copied = unsafe { std::slice::from_raw_parts(*p as *const u8, n) };
            assert_eq!(copied, &inp[..], "{} clobbered an earlier result", imp.name);
        }
        for p in live {
            unsafe { libc::free(p as *mut libc::c_void) };
        }
    }
}

/// Row 18 — concurrent calls from 8 threads (reentrancy).
#[test]
fn cfg_row18_concurrent_calls() {
    let c = c_impl();
    let r = rust_impl();
    let mut handles = Vec::new();
    for t in 0..8u64 {
        handles.push(std::thread::spawn(move || {
            let mut rng = Rng::new(SEED ^ 0x18 ^ (t << 32));
            for rep in 0..2000 {
                let len = rng.in_range(0, 256);
                let buf = rng.cstring(len);
                for imp in [c, r] {
                    let p = unsafe { (imp.custom_strdup)(buf.as_ptr() as *const c_char) };
                    assert!(!p.is_null(), "{} NULL in thread {t} rep {rep}", imp.name);
                    let copied = unsafe { std::slice::from_raw_parts(p as *const u8, len + 1) };
                    assert_eq!(copied, &buf[..], "{} bad copy in thread {t} rep {rep}", imp.name);
                    unsafe { libc::free(p as *mut libc::c_void) };
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("thread panicked");
    }
}

/// Row 19 — unrestricted seeded property sweep mixing every axis, including
/// occasional NULL inputs, so the branchy and straight-line paths interleave.
#[test]
fn cfg_row19_property_sweep() {
    let mut rng = Rng::new(SEED ^ 0x19);
    for i in 0..20_000usize {
        if rng.below(50) == 0 {
            assert_same(std::ptr::null(), &format!("row19 #{i} null"));
            continue;
        }
        let len = rng.in_range(0, 4096);
        let mut buf: Vec<u8> = Vec::with_capacity(len + 1);
        // Mix content classes so value-dependent bugs are reachable.
        let class = rng.below(4);
        for _ in 0..len {
            buf.push(match class {
                0 => rng.ascii_byte(),
                1 => 0x80 | (rng.next_u64() as u8 & 0x7F),
                2 => rng.non_nul_byte(),
                _ => 0xFF,
            });
        }
        buf.push(0);
        // Sometimes append garbage past the terminator.
        if rng.below(3) == 0 {
            for _ in 0..rng.in_range(1, 64) {
                buf.push(rng.non_nul_byte());
            }
        }
        assert_same_bytes(&buf[..], &format!("row19 #{i} len={len} class={class}"));
    }
}
