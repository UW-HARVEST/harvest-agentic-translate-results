//! Phase B — valid-path differential tests.
//!
//! One test per row of CONFIGS.md (C1..C22). Every row is driven with many
//! randomized inputs from a fixed-seed SplitMix64 RNG, and both the C `.so`
//! and the Rust `.so` are called through their exported symbols only.

mod common;

use common::*;
use std::ffi::CString;

const SEED: u64 = 0x5DEE_CE66_D;

// ---------------------------------------------------------------------------
// C1 — length-0 haystack, needle swept over every non-zero byte
// ---------------------------------------------------------------------------
#[test]
fn c01_foo_empty_haystack_all_needles() {
    for n in 1u16..=255 {
        let needle = n as u8 as i8;
        let r = diff_foo(b"", needle, "C1 empty");
        assert_eq!(r, 0, "C1: empty haystack must yield 0 for needle {needle}");
    }
}

// ---------------------------------------------------------------------------
// C2 — length-1 haystack: needle matching and not matching, all byte values
// ---------------------------------------------------------------------------
#[test]
fn c02_foo_single_byte_haystack() {
    for b in 1u16..=255 {
        let byte = b as u8;
        let hay = [byte];
        // matching needle
        let r = diff_foo(&hay, byte as i8, "C2 match");
        assert_eq!(r, 1, "C2: single matching byte 0x{byte:02x}");
        // non-matching needle (pick a different non-zero byte deterministically)
        let other = if byte == 1 { 2u8 } else { byte - 1 };
        let r = diff_foo(&hay, other as i8, "C2 nomatch");
        assert_eq!(r, 0, "C2: 0x{other:02x} must not match 0x{byte:02x}");
    }
}

// ---------------------------------------------------------------------------
// C3 — needle absent (0 matches), random ASCII haystacks
// ---------------------------------------------------------------------------
#[test]
fn c03_foo_needle_absent() {
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..200 {
        let len = rng.range(0, 128);
        // Build over a restricted alphabet, then use a needle outside it.
        let hay: Vec<u8> = (0..len).map(|_| rng.range(b'a' as usize, b'z' as usize) as u8).collect();
        let needle = b'Q' as i8;
        let r = diff_foo(&hay, needle, &format!("C3 iter {i}"));
        assert_eq!(r, 0);
        assert_eq!(r, expected_count(&hay, needle));
    }
}

// ---------------------------------------------------------------------------
// C4 — exactly one match at a random position
// ---------------------------------------------------------------------------
#[test]
fn c04_foo_exactly_one_match() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..200 {
        let len = rng.range(1, 200);
        let needle_b = b'#';
        let mut hay: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != needle_b {
                    return b;
                }
            })
            .collect();
        let pos = rng.below(len);
        hay[pos] = needle_b;
        let r = diff_foo(&hay, needle_b as i8, &format!("C4 iter {i} pos {pos}"));
        assert_eq!(r, 1);
        assert_eq!(r, expected_count(&hay, needle_b as i8));
    }
}

// ---------------------------------------------------------------------------
// C5 — many matches, random density
// ---------------------------------------------------------------------------
#[test]
fn c05_foo_many_matches_random_density() {
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..200 {
        let len = rng.range(1, 512);
        let density = rng.range(10, 50); // percent
        let needle_b = b'A';
        let hay: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(100) < density {
                    needle_b
                } else {
                    loop {
                        let b = rng.nonzero_byte();
                        if b != needle_b {
                            return b;
                        }
                    }
                }
            })
            .collect();
        let r = diff_foo(&hay, needle_b as i8, &format!("C5 iter {i}"));
        assert_eq!(r, expected_count(&hay, needle_b as i8));
    }
}

// ---------------------------------------------------------------------------
// C6 — adjacent matches (runs), exercising the `s++` after each hit
// ---------------------------------------------------------------------------
#[test]
fn c06_foo_adjacent_runs() {
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..200 {
        let needle_b = b'Z';
        let mut hay: Vec<u8> = Vec::new();
        let runs = rng.range(1, 8);
        for _ in 0..runs {
            let filler = rng.range(0, 5);
            for _ in 0..filler {
                hay.push(loop {
                    let b = rng.nonzero_byte();
                    if b != needle_b {
                        break b;
                    }
                });
            }
            let run = rng.range(1, 10);
            for _ in 0..run {
                hay.push(needle_b);
            }
        }
        let r = diff_foo(&hay, needle_b as i8, &format!("C6 iter {i}"));
        assert_eq!(r, expected_count(&hay, needle_b as i8));
    }
    // Hand-picked pure-run cases too.
    for n in 1..=64usize {
        let hay = vec![b'A'; n];
        let r = diff_foo(&hay, b'A' as i8, "C6 pure run");
        assert_eq!(r, n as i32);
    }
}

// ---------------------------------------------------------------------------
// C7 — match at the very first byte
// ---------------------------------------------------------------------------
#[test]
fn c07_foo_match_at_first_byte() {
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..200 {
        let len = rng.range(1, 64);
        let needle_b = b'!';
        let mut hay: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != needle_b {
                    return b;
                }
            })
            .collect();
        hay[0] = needle_b;
        let r = diff_foo(&hay, needle_b as i8, &format!("C7 iter {i}"));
        assert_eq!(r, 1);
    }
}

// ---------------------------------------------------------------------------
// C8 — match at the very last byte (`s++` lands on the terminator)
// ---------------------------------------------------------------------------
#[test]
fn c08_foo_match_at_last_byte() {
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..200 {
        let len = rng.range(1, 64);
        let needle_b = b'~';
        let mut hay: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != needle_b {
                    return b;
                }
            })
            .collect();
        *hay.last_mut().unwrap() = needle_b;
        let r = diff_foo(&hay, needle_b as i8, &format!("C8 iter {i}"));
        assert_eq!(r, 1, "C8: only the final byte matches");
    }
    // The extreme boundary: a 1-byte string that is the needle.
    assert_eq!(diff_foo(b"~", b'~' as i8, "C8 len1"), 1);
}

// ---------------------------------------------------------------------------
// C9 — haystack is entirely the needle
// ---------------------------------------------------------------------------
#[test]
fn c09_foo_all_bytes_match() {
    for byte in [1u8, 0x0a, b'A', b'x', 0x7f, 0x80, 0xff] {
        for len in 1..=64usize {
            let hay = vec![byte; len];
            let r = diff_foo(&hay, byte as i8, "C9");
            assert_eq!(r, len as i32, "C9: byte 0x{byte:02x} len {len}");
        }
    }
}

// ---------------------------------------------------------------------------
// C10 — high-bit / negative needle over non-UTF-8 haystacks
// ---------------------------------------------------------------------------
#[test]
fn c10_foo_high_bit_needles() {
    let mut rng = Rng::new(SEED ^ 10);
    for needle_b in 0x80u16..=0xFF {
        let needle_b = needle_b as u8;
        for i in 0..8 {
            let len = rng.range(1, 96);
            let hay: Vec<u8> = (0..len)
                .map(|_| {
                    if rng.below(4) == 0 {
                        needle_b
                    } else {
                        // biased towards other high-bit bytes
                        let b = 0x80 + (rng.below(0x80) as u8);
                        if b == needle_b {
                            b.wrapping_sub(1).max(0x80)
                        } else {
                            b
                        }
                    }
                })
                .collect();
            let needle = needle_b as i8; // negative on this platform
            assert!(needle < 0, "sanity: 0x{needle_b:02x} is negative as c_char");
            let r = diff_foo(&hay, needle, &format!("C10 needle 0x{needle_b:02x} iter {i}"));
            assert_eq!(r, expected_count(&hay, needle));
        }
    }
}

// ---------------------------------------------------------------------------
// C11 — full random byte haystack x every non-zero needle value
// ---------------------------------------------------------------------------
#[test]
fn c11_foo_full_byte_domain_cross_product() {
    let mut rng = Rng::new(SEED ^ 11);
    // A handful of random haystacks over the whole 0x01..=0xFF alphabet ...
    let haystacks: Vec<Vec<u8>> = (0..6)
        .map(|_| {
            let len = rng.range(1, 600);
            (0..len).map(|_| rng.nonzero_byte()).collect()
        })
        .collect();
    // ... crossed with all 255 possible non-zero needles.
    for hay in &haystacks {
        for n in 1u16..=255 {
            let needle = n as u8 as i8;
            let r = diff_foo(hay, needle, "C11");
            assert_eq!(r, expected_count(hay, needle));
        }
    }
}

// ---------------------------------------------------------------------------
// C12 — large haystacks (64 KiB / 256 KiB)
// ---------------------------------------------------------------------------
#[test]
fn c12_foo_large_haystacks() {
    let mut rng = Rng::new(SEED ^ 12);
    for &len in &[64 * 1024usize, 256 * 1024] {
        let hay: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        for needle_b in [1u8, b'A', b'x', 0x7f, 0xff] {
            let needle = needle_b as i8;
            let r = diff_foo(&hay, needle, &format!("C12 len {len}"));
            assert_eq!(r, expected_count(&hay, needle));
            assert!(r > 0, "C12: expected many matches in a {len}-byte random buffer");
        }
        // Worst case: every byte matches.
        let all = vec![b'A'; len];
        let r = diff_foo(&all, b'A' as i8, &format!("C12 all-match len {len}"));
        assert_eq!(r, len as i32);
    }
}

// ---------------------------------------------------------------------------
// C13 — driver("")
// ---------------------------------------------------------------------------
#[test]
fn c13_driver_empty_input() {
    let out = diff_driver(b"", "C13");
    assert_eq!(out, b"A: 0\nx: 0\n", "C13: exact stdout bytes");
}

// ---------------------------------------------------------------------------
// C14 — driver: only 'A'
// ---------------------------------------------------------------------------
#[test]
fn c14_driver_only_a() {
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..64 {
        let n = rng.range(1, 40);
        let mut hay = vec![b'A'; n];
        // sprinkle filler that is neither 'A' nor 'x'
        for _ in 0..rng.range(0, 20) {
            let pos = rng.below(hay.len());
            hay.insert(pos, b'-');
        }
        let out = diff_driver(&hay, &format!("C14 iter {i}"));
        assert_eq!(out, format!("A: {n}\nx: 0\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// C15 — driver: only 'x'
// ---------------------------------------------------------------------------
#[test]
fn c15_driver_only_x() {
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..64 {
        let n = rng.range(1, 40);
        let mut hay = vec![b'x'; n];
        for _ in 0..rng.range(0, 20) {
            let pos = rng.below(hay.len());
            hay.insert(pos, b'.');
        }
        let out = diff_driver(&hay, &format!("C15 iter {i}"));
        assert_eq!(out, format!("A: 0\nx: {n}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// C16 — driver: both 'A' and 'x', randomized
// ---------------------------------------------------------------------------
#[test]
fn c16_driver_both_needles_random() {
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..200 {
        let len = rng.range(0, 256);
        let hay: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => b'A',
                1 => b'x',
                _ => rng.ascii_byte(),
            })
            .collect();
        let out = diff_driver(&hay, &format!("C16 iter {i}"));
        let na = hay.iter().filter(|&&b| b == b'A').count();
        let nx = hay.iter().filter(|&&b| b == b'x').count();
        assert_eq!(out, format!("A: {na}\nx: {nx}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// C17 — driver: neither needle, incl. case-sensitivity near-misses
// ---------------------------------------------------------------------------
#[test]
fn c17_driver_neither_needle_case_sensitivity() {
    for hay in [
        &b"aaaa"[..],
        &b"XXXX"[..],
        &b"aXaXaX"[..],
        &b"the quick brown dog"[..],
        &b"BCDEFG"[..],
        &b"\x01\x02\x03\x7f"[..],
    ] {
        let out = diff_driver(hay, "C17");
        assert_eq!(
            out, b"A: 0\nx: 0\n",
            "C17: {:?} must count zero of both (case sensitive)",
            preview(hay)
        );
    }
    // Mixed case where only the exact case counts.
    let out = diff_driver(b"aAaAxXxX", "C17 mixed");
    assert_eq!(out, b"A: 2\nx: 2\n");
}

// ---------------------------------------------------------------------------
// C18 — driver: adjacent runs and first/last-byte matches
// ---------------------------------------------------------------------------
#[test]
fn c18_driver_runs_and_edges() {
    for (hay, want) in [
        (&b"AAAA"[..], "A: 4\nx: 0\n"),
        (&b"xxxx"[..], "A: 0\nx: 4\n"),
        (&b"AxAxAx"[..], "A: 3\nx: 3\n"),
        (&b"AAAxxx"[..], "A: 3\nx: 3\n"),
        (&b"A"[..], "A: 1\nx: 0\n"),
        (&b"x"[..], "A: 0\nx: 1\n"),
        (&b"Ax"[..], "A: 1\nx: 1\n"),
        (&b"xA"[..], "A: 1\nx: 1\n"),
        (&b"-A"[..], "A: 1\nx: 0\n"),
        (&b"A-"[..], "A: 1\nx: 0\n"),
        (&b"-----x"[..], "A: 0\nx: 1\n"),
        (&b"x-----"[..], "A: 0\nx: 1\n"),
    ] {
        let out = diff_driver(hay, "C18");
        assert_eq!(out, want.as_bytes(), "C18: input {:?}", preview(hay));
    }
}

// ---------------------------------------------------------------------------
// C19 — driver: non-UTF-8 input
// ---------------------------------------------------------------------------
#[test]
fn c19_driver_non_utf8_input() {
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..100 {
        let len = rng.range(1, 200);
        let hay: Vec<u8> = (0..len)
            .map(|_| match rng.below(5) {
                0 => b'A',
                1 => b'x',
                _ => 0x80 + rng.below(0x80) as u8, // invalid UTF-8 bytes
            })
            .collect();
        assert!(
            std::str::from_utf8(&hay).is_err() || hay.iter().all(|&b| b < 0x80),
            "sanity"
        );
        let out = diff_driver(&hay, &format!("C19 iter {i}"));
        let na = hay.iter().filter(|&&b| b == b'A').count();
        let nx = hay.iter().filter(|&&b| b == b'x').count();
        assert_eq!(out, format!("A: {na}\nx: {nx}\n").into_bytes());
    }
    // Guaranteed-invalid UTF-8 with needles at the edges.
    let out = diff_driver(b"A\xff\xfe\x80x", "C19 fixed");
    assert_eq!(out, b"A: 1\nx: 1\n");
}

// ---------------------------------------------------------------------------
// C20 — driver: large input, multi-digit counts
// ---------------------------------------------------------------------------
#[test]
fn c20_driver_large_input_multidigit() {
    let mut rng = Rng::new(SEED ^ 20);
    let len = 128 * 1024usize;
    let hay: Vec<u8> = (0..len)
        .map(|_| match rng.below(3) {
            0 => b'A',
            1 => b'x',
            _ => rng.ascii_byte(),
        })
        .collect();
    let na = hay.iter().filter(|&&b| b == b'A').count();
    let nx = hay.iter().filter(|&&b| b == b'x').count();
    let out = diff_driver(&hay, "C20");
    assert_eq!(out, format!("A: {na}\nx: {nx}\n").into_bytes());
    assert!(na > 9999 && nx > 9999, "C20: expected 5+ digit counts, got {na}/{nx}");

    // Exact digit-width boundaries: 9/10, 99/100, 999/1000 occurrences.
    for n in [9usize, 10, 99, 100, 999, 1000] {
        let hay = vec![b'A'; n];
        let out = diff_driver(&hay, "C20 widths");
        assert_eq!(out, format!("A: {n}\nx: 0\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// C21 — composed pipeline: driver's printed numbers == foo's return values,
//       cross-checked between the two libraries in all four combinations.
// ---------------------------------------------------------------------------
#[test]
fn c21_driver_and_foo_consistency_cross_library() {
    let mut rng = Rng::new(SEED ^ 21);
    let l = libs();
    for i in 0..150 {
        let len = rng.range(0, 300);
        let hay: Vec<u8> = (0..len)
            .map(|_| match rng.below(5) {
                0 => b'A',
                1 => b'x',
                _ => rng.nonzero_byte(),
            })
            .collect();
        let cs = CString::new(hay.clone()).unwrap();

        let (c_a, c_x, rs_a, rs_x) = unsafe {
            let p = cs.as_ptr();
            (
                (l.c.foo)(p, b'A' as i8),
                (l.c.foo)(p, b'x' as i8),
                (l.rs.foo)(p, b'A' as i8),
                (l.rs.foo)(p, b'x' as i8),
            )
        };
        assert_eq!((c_a, c_x), (rs_a, rs_x), "C21 iter {i}: foo mismatch");

        let c_out = capture_stdout(|| unsafe { (l.c.driver)(cs.as_ptr()) });
        let rs_out = capture_stdout(|| unsafe { (l.rs.driver)(cs.as_ptr()) });
        assert_eq!(c_out, rs_out, "C21 iter {i}: driver stdout mismatch");

        // The wrapper must be composed from exactly these low-level results.
        let want = format!("A: {c_a}\nx: {c_x}\n").into_bytes();
        assert_eq!(c_out, want, "C21 iter {i}: C driver vs C foo");
        assert_eq!(rs_out, want, "C21 iter {i}: Rust driver vs C foo");
    }
}

// ---------------------------------------------------------------------------
// C22 — foo driven with exactly the needles the wrapper hard-codes
// ---------------------------------------------------------------------------
#[test]
fn c22_foo_with_driver_needles() {
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..300 {
        let len = rng.range(0, 400);
        let hay: Vec<u8> = (0..len)
            .map(|_| match rng.below(3) {
                0 => b'A',
                1 => b'x',
                _ => rng.nonzero_byte(),
            })
            .collect();
        for needle_b in [b'A', b'x'] {
            let r = diff_foo(&hay, needle_b as i8, &format!("C22 iter {i}"));
            assert_eq!(r, expected_count(&hay, needle_b as i8));
        }
    }
}

// ---------------------------------------------------------------------------
// Sanity: both libraries really are two distinct objects on disk.
// ---------------------------------------------------------------------------
#[test]
fn c00_harness_loads_two_distinct_libraries() {
    let l = libs();
    assert_ne!(l.c.path, l.rs.path);
    assert!(l.c.path.to_string_lossy().contains("c_src"), "{:?}", l.c.path);
    assert!(l.rs.path.to_string_lossy().contains("target"), "{:?}", l.rs.path);
    assert_ne!(
        l.c.foo as usize, l.rs.foo as usize,
        "the two `foo` symbols must resolve to different code"
    );
    assert_ne!(l.c.driver as usize, l.rs.driver as usize);
}
