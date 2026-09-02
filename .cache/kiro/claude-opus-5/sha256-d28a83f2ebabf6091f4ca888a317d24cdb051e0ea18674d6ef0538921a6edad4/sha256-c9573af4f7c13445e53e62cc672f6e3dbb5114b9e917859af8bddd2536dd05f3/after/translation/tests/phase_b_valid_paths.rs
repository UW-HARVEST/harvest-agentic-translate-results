//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row uses many randomised inputs
//! driven by a fixed-seed SplitMix64, and asserts the C `.so` and the Rust
//! `.so` agree byte-for-byte through their exported symbols.

mod common;

use common::*;
use std::ffi::c_char;

/// Assert `foo` agrees for one (buffer, search byte) pair.
fn assert_foo(buf: &CStrBuf, c: u8, ctx: &str) -> i32 {
    let (c_foo, r_foo) = foo_pair();
    let cv = c as c_char;
    let a = unsafe { c_foo(buf.as_ptr(), cv) };
    let b = unsafe { r_foo(buf.as_ptr(), cv) };
    assert_eq!(a, b, "foo divergence ({ctx}): c=0x{c:02x} C={a} Rust={b}");
    a
}

/// Assert `driver` produces identical stdout bytes.
fn assert_driver(buf: &CStrBuf, ctx: &str) -> Vec<u8> {
    let (c_drv, r_drv) = driver_pair();
    let p = buf.as_ptr();
    let out_c = capture_stdout(|| unsafe { c_drv(p) });
    let out_r = capture_stdout(|| unsafe { r_drv(p) });
    assert_eq!(
        out_c,
        out_r,
        "driver stdout divergence ({ctx}):\n  C   = {:?}\n  Rust= {:?}",
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r)
    );
    out_c
}

// ---------------------------------------------------------------------------
// Row 1 — empty string, every non-zero search byte
// ---------------------------------------------------------------------------
#[test]
fn row01_foo_empty_all_search_bytes() {
    let buf = CStrBuf::new(b"");
    for c in 1u8..=255 {
        let n = assert_foo(&buf, c, "row01");
        assert_eq!(n, 0, "empty string must yield 0 for c=0x{c:02x}");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — single byte content × every non-zero search byte
// ---------------------------------------------------------------------------
#[test]
fn row02_foo_single_byte_full_cross_product() {
    for content in 1u8..=255 {
        let buf = CStrBuf::new(&[content]);
        for c in 1u8..=255 {
            let n = assert_foo(&buf, c, "row02");
            let expect = if c == content { 1 } else { 0 };
            assert_eq!(n, expect, "content=0x{content:02x} c=0x{c:02x}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — no match at all, randomised
// ---------------------------------------------------------------------------
#[test]
fn row03_foo_no_match_randomised() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..2000 {
        let c = rng.nonzero_byte();
        let len = rng.below(300);
        // Content drawn from bytes != c and != 0.
        let bytes: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != c {
                    return b;
                }
            })
            .collect();
        let buf = CStrBuf::new(&bytes);
        let n = assert_foo(&buf, c, "row03");
        assert_eq!(n, 0, "no-match case must be 0");
    }
}

// ---------------------------------------------------------------------------
// Rows 4/5/6 — exactly one match, at first / last / interior position
// ---------------------------------------------------------------------------
fn one_match_at(rng: &mut Rng, len: usize, pos: usize, c: u8) -> Vec<u8> {
    let mut bytes: Vec<u8> = (0..len)
        .map(|_| loop {
            let b = rng.nonzero_byte();
            if b != c {
                return b;
            }
        })
        .collect();
    bytes[pos] = c;
    bytes
}

#[test]
fn row04_foo_single_match_first_byte() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..1500 {
        let c = rng.nonzero_byte();
        let len = 1 + rng.below(300);
        let bytes = one_match_at(&mut rng, len, 0, c);
        let buf = CStrBuf::new(&bytes);
        assert_eq!(assert_foo(&buf, c, "row04"), 1);
    }
}

#[test]
fn row05_foo_single_match_last_byte() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..1500 {
        let c = rng.nonzero_byte();
        let len = 1 + rng.below(300);
        let bytes = one_match_at(&mut rng, len, len - 1, c);
        let buf = CStrBuf::new(&bytes);
        assert_eq!(assert_foo(&buf, c, "row05"), 1);
    }
}

#[test]
fn row06_foo_single_match_interior() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..1500 {
        let c = rng.nonzero_byte();
        let len = 3 + rng.below(300);
        let pos = 1 + rng.below(len - 2);
        let bytes = one_match_at(&mut rng, len, pos, c);
        let buf = CStrBuf::new(&bytes);
        assert_eq!(assert_foo(&buf, c, "row06"), 1);
    }
}

// ---------------------------------------------------------------------------
// Row 7 — two adjacent matches (exercises the `s++` step landing on a match)
// ---------------------------------------------------------------------------
#[test]
fn row07_foo_two_adjacent_matches() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..1500 {
        let c = rng.nonzero_byte();
        let len = 2 + rng.below(300);
        let pos = rng.below(len - 1);
        let mut bytes = one_match_at(&mut rng, len, pos, c);
        bytes[pos + 1] = c;
        let buf = CStrBuf::new(&bytes);
        assert_eq!(assert_foo(&buf, c, "row07"), 2);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — many scattered matches
// ---------------------------------------------------------------------------
#[test]
fn row08_foo_many_scattered_matches() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..3000 {
        let c = rng.nonzero_byte();
        let len = rng.below(513);
        let mut bytes: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != c {
                    return b;
                }
            })
            .collect();
        let mut expect = 0i32;
        for i in 0..len {
            if rng.below(4) == 0 {
                bytes[i] = c;
                expect += 1;
            }
        }
        let buf = CStrBuf::new(&bytes);
        assert_eq!(assert_foo(&buf, c, "row08"), expect);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — every byte matches, across length boundaries
// ---------------------------------------------------------------------------
const LEN_BOUNDARIES: &[usize] = &[
    1, 2, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 4095, 4096, 4097,
];

#[test]
fn row09_foo_all_bytes_match() {
    let mut rng = Rng::new(SEED ^ 9);
    for &len in LEN_BOUNDARIES {
        for _ in 0..16 {
            let c = rng.nonzero_byte();
            let bytes = vec![c; len];
            let buf = CStrBuf::new(&bytes);
            assert_eq!(assert_foo(&buf, c, "row09"), len as i32, "len={len}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — high-bit (negative c_char) search bytes over arbitrary content
// ---------------------------------------------------------------------------
#[test]
fn row10_foo_high_bit_search_bytes() {
    let mut rng = Rng::new(SEED ^ 10);
    for c in 0x80u8..=0xFF {
        for _ in 0..12 {
            let len = rng.below(400);
            let bytes = rng.bytes(len);
            let buf = CStrBuf::new(&bytes);
            let expect = bytes.iter().filter(|&&b| b == c).count() as i32;
            assert_eq!(assert_foo(&buf, c, "row10"), expect, "c=0x{c:02x}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — boundary search bytes 0x01 and 0x7F
// ---------------------------------------------------------------------------
#[test]
fn row11_foo_boundary_search_bytes() {
    let mut rng = Rng::new(SEED ^ 11);
    for &c in &[0x01u8, 0x7Fu8, 0x80u8, 0xFFu8] {
        for _ in 0..300 {
            let len = rng.below(400);
            let mut bytes = rng.bytes(len);
            // Sprinkle the target byte in so matches actually occur.
            for i in 0..len {
                if rng.below(5) == 0 {
                    bytes[i] = c;
                }
            }
            let buf = CStrBuf::new(&bytes);
            let expect = bytes.iter().filter(|&&b| b == c).count() as i32;
            assert_eq!(assert_foo(&buf, c, "row11"), expect, "c=0x{c:02x}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — start alignment 0..=63 (glibc strchr reads aligned words)
// ---------------------------------------------------------------------------
#[test]
fn row12_foo_start_alignment_sweep() {
    let mut rng = Rng::new(SEED ^ 12);
    for offset in 0..64usize {
        for _ in 0..40 {
            let c = rng.nonzero_byte();
            let len = rng.below(200);
            let mut bytes = rng.bytes(len);
            for i in 0..len {
                if rng.below(6) == 0 {
                    bytes[i] = c;
                }
            }
            let buf = CStrBuf::with_alignment(&bytes, offset);
            let expect = bytes.iter().filter(|&&b| b == c).count() as i32;
            assert_eq!(
                assert_foo(&buf, c, "row12"),
                expect,
                "offset={offset} len={len} c=0x{c:02x}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — length boundaries with random content and random search byte
// ---------------------------------------------------------------------------
#[test]
fn row13_foo_length_boundaries() {
    let mut rng = Rng::new(SEED ^ 13);
    for &len in LEN_BOUNDARIES {
        for _ in 0..40 {
            let c = rng.nonzero_byte();
            let mut bytes = rng.bytes(len);
            for i in 0..len {
                if rng.below(7) == 0 {
                    bytes[i] = c;
                }
            }
            let buf = CStrBuf::new(&bytes);
            let expect = bytes.iter().filter(|&&b| b == c).count() as i32;
            assert_eq!(assert_foo(&buf, c, "row13"), expect, "len={len}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — broad property sweep over arbitrary (non-UTF-8) byte content
// ---------------------------------------------------------------------------
#[test]
fn row14_foo_broad_property_sweep() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..4000 {
        let c = rng.nonzero_byte();
        let len = rng.below(1025);
        let bytes = rng.bytes(len);
        let buf = CStrBuf::new(&bytes);
        let expect = bytes.iter().filter(|&&b| b == c).count() as i32;
        assert_eq!(assert_foo(&buf, c, "row14"), expect);
    }
}

// ---------------------------------------------------------------------------
// Row 15 — the two bytes `driver` hard-codes, exercised through `foo` directly
// ---------------------------------------------------------------------------
#[test]
fn row15_foo_driver_search_bytes() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..3000 {
        let len = rng.below(600);
        let mut bytes = rng.bytes(len);
        for i in 0..len {
            match rng.below(8) {
                0 => bytes[i] = b'A',
                1 => bytes[i] = b'x',
                _ => {}
            }
        }
        let buf = CStrBuf::new(&bytes);
        for &c in &[b'A', b'x'] {
            let expect = bytes.iter().filter(|&&b| b == c).count() as i32;
            assert_eq!(assert_foo(&buf, c, "row15"), expect);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 16-21 — `driver` output shapes / printf digit widths
// ---------------------------------------------------------------------------
#[test]
fn row16_driver_zero_counts() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..200 {
        let len = rng.below(200);
        let bytes: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != b'A' && b != b'x' {
                    return b;
                }
            })
            .collect();
        let buf = CStrBuf::new(&bytes);
        let out = assert_driver(&buf, "row16");
        assert_eq!(out, b"A: 0\nx: 0\n");
    }
}

#[test]
fn row17_driver_one_a_no_x() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..200 {
        let len = 1 + rng.below(200);
        let mut bytes: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != b'A' && b != b'x' {
                    return b;
                }
            })
            .collect();
        bytes[rng.below(len)] = b'A';
        let buf = CStrBuf::new(&bytes);
        assert_eq!(assert_driver(&buf, "row17"), b"A: 1\nx: 0\n");
    }
}

#[test]
fn row18_driver_no_a_one_x() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..200 {
        let len = 1 + rng.below(200);
        let mut bytes: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != b'A' && b != b'x' {
                    return b;
                }
            })
            .collect();
        bytes[rng.below(len)] = b'x';
        let buf = CStrBuf::new(&bytes);
        assert_eq!(assert_driver(&buf, "row18"), b"A: 0\nx: 1\n");
    }
}

#[test]
fn row19_driver_two_digit_differing_counts() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..150 {
        let na = 10 + rng.below(90);
        let nx = 10 + rng.below(90);
        if na == nx {
            continue;
        }
        let mut bytes = vec![b'.'; 256];
        // Place `na` 'A's and `nx` 'x's at distinct random slots.
        let mut slots: Vec<usize> = (0..256).collect();
        for i in (1..slots.len()).rev() {
            let j = rng.below(i + 1);
            slots.swap(i, j);
        }
        for &s in slots.iter().take(na) {
            bytes[s] = b'A';
        }
        for &s in slots.iter().skip(na).take(nx) {
            bytes[s] = b'x';
        }
        let buf = CStrBuf::new(&bytes);
        let out = assert_driver(&buf, "row19");
        assert_eq!(out, format!("A: {na}\nx: {nx}\n").into_bytes());
    }
}

#[test]
fn row20_driver_wide_digit_counts() {
    // 3-, 4- and 5-digit counts.
    for &(na, nx) in &[
        (100usize, 999usize),
        (1000, 1234),
        (10000, 99999),
        (12345, 100),
        (0, 54321),
    ] {
        let mut bytes = Vec::with_capacity(na + nx);
        bytes.extend(std::iter::repeat(b'A').take(na));
        bytes.extend(std::iter::repeat(b'x').take(nx));
        let buf = CStrBuf::new(&bytes);
        let out = assert_driver(&buf, "row20");
        assert_eq!(out, format!("A: {na}\nx: {nx}\n").into_bytes());
    }
}

#[test]
fn row21_driver_empty_input() {
    let buf = CStrBuf::new(b"");
    assert_eq!(assert_driver(&buf, "row21"), b"A: 0\nx: 0\n");
}

#[test]
fn row22_driver_broad_property_sweep() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..400 {
        let len = rng.below(2049);
        let bytes = rng.bytes(len);
        let buf = CStrBuf::new(&bytes);
        let na = bytes.iter().filter(|&&b| b == b'A').count();
        let nx = bytes.iter().filter(|&&b| b == b'x').count();
        let out = assert_driver(&buf, "row22");
        assert_eq!(out, format!("A: {na}\nx: {nx}\n").into_bytes());
    }
}

#[test]
fn row23_driver_alignment_sweep() {
    let mut rng = Rng::new(SEED ^ 23);
    for offset in 0..64usize {
        let len = rng.below(300);
        let mut bytes = rng.bytes(len);
        for i in 0..len {
            match rng.below(6) {
                0 => bytes[i] = b'A',
                1 => bytes[i] = b'x',
                _ => {}
            }
        }
        let buf = CStrBuf::with_alignment(&bytes, offset);
        let na = bytes.iter().filter(|&&b| b == b'A').count();
        let nx = bytes.iter().filter(|&&b| b == b'x').count();
        let out = assert_driver(&buf, "row23");
        assert_eq!(out, format!("A: {na}\nx: {nx}\n").into_bytes(), "off={offset}");
    }
}

// ---------------------------------------------------------------------------
// Row 24 — composed pipeline: driver's printed digits must equal the
// low-level `foo` results, in BOTH libraries.
// ---------------------------------------------------------------------------
#[test]
fn row24_composed_driver_matches_low_level_foo() {
    let (c_foo, r_foo) = foo_pair();
    let (c_drv, r_drv) = driver_pair();
    let mut rng = Rng::new(SEED ^ 24);

    for _ in 0..300 {
        let len = rng.below(800);
        let mut bytes = rng.bytes(len);
        for i in 0..len {
            match rng.below(5) {
                0 => bytes[i] = b'A',
                1 => bytes[i] = b'x',
                _ => {}
            }
        }
        let buf = CStrBuf::new(&bytes);
        let p = buf.as_ptr();

        let ca = unsafe { c_foo(p, b'A' as c_char) };
        let cx = unsafe { c_foo(p, b'x' as c_char) };
        let ra = unsafe { r_foo(p, b'A' as c_char) };
        let rx = unsafe { r_foo(p, b'x' as c_char) };
        assert_eq!((ca, cx), (ra, rx), "low-level foo divergence");

        let out_c = capture_stdout(|| unsafe { c_drv(p) });
        let out_r = capture_stdout(|| unsafe { r_drv(p) });
        assert_eq!(out_c, out_r, "driver stdout divergence");
        assert_eq!(out_c, format!("A: {ca}\nx: {cx}\n").into_bytes());
        assert_eq!(out_r, format!("A: {ra}\nx: {rx}\n").into_bytes());
    }
}
