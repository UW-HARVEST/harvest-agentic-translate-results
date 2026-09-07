//! Phase B — valid-path differential tests for the buffer entry points:
//! `create_numeric_buffer`, `find_value_in_buffer`, and the two composed.
//!
//! CONFIGS.md rows 12–27.

mod common;

use common::*;

const SEEDS: [i32; 11] = [
    0,
    1,
    7,
    42,
    255,
    256,
    -1,
    -7,
    -300,
    i32::MAX,
    i32::MIN,
];

/// Call `create_numeric_buffer` on both libraries into separate buffers padded
/// with canaries, and require identical bytes (including untouched padding).
fn create_both(size: i32, seed: i32, cap: usize) -> (Vec<i8>, Vec<i8>) {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnCreateNumericBuffer>(SYM_CREATE) };
    const PAD: usize = 16;
    let mut cbuf = vec![0x5Au8 as i8; cap + PAD];
    let mut rbuf = vec![0x5Au8 as i8; cap + PAD];
    unsafe {
        c(cbuf.as_mut_ptr(), size, seed);
        r(rbuf.as_mut_ptr(), size, seed);
    }
    (cbuf, rbuf)
}

fn assert_create_eq(size: i32, seed: i32, cap: usize, row: &str) {
    let (cbuf, rbuf) = create_both(size, seed, cap);
    if cbuf != rbuf {
        let i = cbuf.iter().zip(&rbuf).position(|(a, b)| a != b).unwrap();
        panic!(
            "{row}: create_numeric_buffer(size={size}, seed={seed}) differs at index {i}: \
             C {} vs Rust {}",
            cbuf[i], rbuf[i]
        );
    }
}

// ---------------------------------------------------------------------------
// create_numeric_buffer
// ---------------------------------------------------------------------------

#[test]
fn row12_create_non_positive_size_leaves_buffer_untouched() {
    for &size in &[0, -1, -7, -256, i32::MIN, i32::MIN + 1] {
        for &seed in &SEEDS {
            assert_create_eq(size, seed, 64, "row12");
            // and the canary must genuinely be intact on both sides
            let (cbuf, rbuf) = create_both(size, seed, 64);
            assert!(
                cbuf.iter().all(|&b| b == 0x5Au8 as i8),
                "row12: C wrote to the buffer with size={size}"
            );
            assert!(
                rbuf.iter().all(|&b| b == 0x5Au8 as i8),
                "row12: Rust wrote to the buffer with size={size}"
            );
        }
    }
}

#[test]
fn row13_create_size_one() {
    for &seed in &SEEDS {
        assert_create_eq(1, seed, 64, "row13");
    }
}

#[test]
fn row14_create_size_seven() {
    for &seed in &SEEDS {
        assert_create_eq(7, seed, 64, "row14");
    }
}

#[test]
fn row15_create_size_256() {
    for &seed in &SEEDS {
        assert_create_eq(256, seed, 256, "row15");
    }
}

#[test]
fn row16_create_size_1000() {
    for &seed in &SEEDS {
        assert_create_eq(1000, seed, 1000, "row16");
    }
}

#[test]
fn row17_create_random_size_and_seed() {
    let mut rng = Rng::new(0x1701);
    for _ in 0..4000 {
        let size = rng.below(1025) as i32;
        let seed = rng.next_i32();
        assert_create_eq(size, seed, 1024, "row17");
    }
}

// ---------------------------------------------------------------------------
// find_value_in_buffer
// ---------------------------------------------------------------------------

fn assert_find_eq(buf: &[i8], size: usize, needle: i32, row: &str) {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnFindValueInBuffer>(SYM_FIND) };
    let (cv, rv) = unsafe { (c(buf.as_ptr(), size, needle), r(buf.as_ptr(), size, needle)) };
    assert_eq!(
        cv, rv,
        "{row}: find_value_in_buffer(len={size}, needle={needle}) => C {cv} vs Rust {rv}"
    );
}

#[test]
fn row18_find_size_zero() {
    let buf = [1i8, 2, 3, 4];
    for needle in [0, 1, 42, -1, 256, i32::MIN, i32::MAX] {
        assert_find_eq(&buf, 0, needle, "row18");
    }
    // NULL buffer with size 0: neither implementation may dereference.
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnFindValueInBuffer>(SYM_FIND) };
    for needle in [0, 1, -1, i32::MIN] {
        let (cv, rv) = unsafe { (c(std::ptr::null(), 0, needle), r(std::ptr::null(), 0, needle)) };
        assert_eq!(cv, rv, "row18: NULL/0 needle={needle} => C {cv} vs Rust {rv}");
    }
}

#[test]
fn row19_find_size_one() {
    for b in [0i8, 1, -1, 42, 127, -128] {
        let buf = [b];
        for needle in -300..=300 {
            assert_find_eq(&buf, 1, needle, "row19");
        }
    }
}

#[test]
fn row20_find_match_at_first_interior_last() {
    let n = 64usize;
    for pos in [0usize, 1, 31, n - 2, n - 1] {
        let mut buf = vec![0x11i8; n];
        buf[pos] = 0x77;
        assert_find_eq(&buf, n, 0x77, "row20");
        // also confirm the returned index is the *first* match
        let mut buf2 = vec![0x11i8; n];
        buf2[pos] = 0x77;
        if pos + 1 < n {
            buf2[pos + 1] = 0x77;
        }
        assert_find_eq(&buf2, n, 0x77, "row20");
    }
}

#[test]
fn row21_find_needle_absent() {
    let buf = vec![0x11i8; 128];
    for needle in [0, 1, 0x77, 255, -2, 1000] {
        assert_find_eq(&buf, buf.len(), needle, "row21");
    }
}

#[test]
fn row22_find_needle_sweep_all_bytes() {
    // Buffer holding every byte value exactly once.
    let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    for needle in 0..=255 {
        assert_find_eq(&buf, 256, needle, "row22");
    }
    // Same buffer reversed, so the match index differs per needle.
    let rev: Vec<i8> = buf.iter().rev().copied().collect();
    for needle in 0..=255 {
        assert_find_eq(&rev, 256, needle, "row22");
    }
}

#[test]
fn row23_find_needle_above_uchar_range() {
    let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    for needle in 256..=511 {
        assert_find_eq(&buf, 256, needle, "row23");
    }
    for needle in [1000, 65536, 65536 + 42, 0x7FFF_FF2A, 0x0001_0100] {
        assert_find_eq(&buf, 256, needle, "row23");
    }
}

#[test]
fn row24_find_negative_needles() {
    let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    for needle in -256..=-1 {
        assert_find_eq(&buf, 256, needle, "row24");
    }
    for needle in [i32::MIN, i32::MIN + 1, -1000, -65536, -65535] {
        assert_find_eq(&buf, 256, needle, "row24");
    }
}

#[test]
fn row25_find_signed_char_boundary() {
    let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    for needle in [126, 127, 128, 129, 254, 255, -128, -127, -129, -1, 0] {
        assert_find_eq(&buf, 256, needle, "row25");
    }
}

#[test]
fn row26_find_random() {
    let mut rng = Rng::new(0x2601);
    for _ in 0..8000 {
        let size = rng.below(4097) as usize;
        // Bias the alphabet so matches are common for some draws and rare for
        // others.
        let alphabet = 1 + rng.below(256) as u32;
        let buf: Vec<i8> = (0..size)
            .map(|_| (rng.next_u32() % alphabet) as u8 as i8)
            .collect();
        let needle = if rng.next_u64() & 3 == 0 {
            rng.next_i32()
        } else {
            (rng.next_u32() % 300) as i32 - 40
        };
        assert_find_eq(&buf, size, needle, "row26");
    }
}

// ---------------------------------------------------------------------------
// Composed pipeline: create_numeric_buffer -> find_value_in_buffer
// ---------------------------------------------------------------------------

#[test]
fn row27_composed_create_then_find() {
    let p = Pair::load();
    let (cc, rc) = unsafe { p.both::<FnCreateNumericBuffer>(SYM_CREATE) };
    let (cf, rf) = unsafe { p.both::<FnFindValueInBuffer>(SYM_FIND) };
    let mut rng = Rng::new(0x2701);

    for _ in 0..3000 {
        let size = rng.below(600) as i32;
        let seed = rng.next_i32();
        let cap = 600usize;
        let mut cbuf = vec![0i8; cap];
        let mut rbuf = vec![0i8; cap];
        unsafe {
            cc(cbuf.as_mut_ptr(), size, seed);
            rc(rbuf.as_mut_ptr(), size, seed);
        }
        assert_eq!(cbuf, rbuf, "row27: filled buffers differ (size={size}, seed={seed})");

        let len = size.max(0) as usize;
        for _ in 0..4 {
            let needle = if rng.next_u64() & 1 == 0 {
                (rng.next_u32() % 256) as i32
            } else {
                rng.next_i32()
            };
            // Cross-check every pairing: each library's search over each
            // library's buffer must agree.
            let a = unsafe { cf(cbuf.as_ptr(), len, needle) };
            let b = unsafe { rf(rbuf.as_ptr(), len, needle) };
            let a2 = unsafe { cf(rbuf.as_ptr(), len, needle) };
            let b2 = unsafe { rf(cbuf.as_ptr(), len, needle) };
            assert_eq!(
                (a, a2),
                (b, b2),
                "row27: composed search disagreed (size={size}, seed={seed}, needle={needle})"
            );
        }
    }
}
