//! Phase B — valid-path differential tests, one `#[test]` per row of
//! `CONFIGS.md`. Every row drives BOTH shared libraries through their exported
//! symbols and compares stdout byte-for-byte (plus the exit status for `main`).

mod common;

use common::{assert_driver_eq, assert_driver_eq_all, assert_main_eq, Rng};

// ---------------------------------------------------------------------------
// `driver(int x)` — the lowest-level entry point, called directly
// ---------------------------------------------------------------------------

/// Row 1 — `x = 0`.
#[test]
fn row01_driver_zero() {
    assert_driver_eq(0);
}

/// Row 2 — small positive values (no overflow).
#[test]
fn row02_driver_small_positive() {
    let mut rng = Rng::new(0x5EED_0002);
    let xs: Vec<i32> = (0..500).map(|_| rng.range(1, 1000) as i32).collect();
    assert_driver_eq_all(&xs);
}

/// Row 3 — small negative values (no overflow).
#[test]
fn row03_driver_small_negative() {
    let mut rng = Rng::new(0x5EED_0003);
    let xs: Vec<i32> = (0..500).map(|_| rng.range(-1000, -1) as i32).collect();
    assert_driver_eq_all(&xs);
}

/// Row 4 — values where the printed result sits around zero.
#[test]
fn row04_driver_around_zero_result() {
    assert_driver_eq_all(&[1, -1, 2, -2, 149, 150, 151, -149, -150, -151, -200]);
}

/// Row 5 — the positive extremes and the `2*x` overflow threshold.
#[test]
fn row05_driver_positive_extremes() {
    assert_driver_eq_all(&[
        i32::MAX,
        i32::MAX - 1,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MAX - 300,
        i32::MAX - 299,
    ]);
}

/// Row 6 — the negative extremes and the `2*x` underflow threshold.
#[test]
fn row06_driver_negative_extremes() {
    assert_driver_eq_all(&[
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        i32::MIN / 2 - 1,
        i32::MIN + 300,
        i32::MIN + 299,
    ]);
}

/// Row 7 — the whole window where `2*x` fits but `y += 300` overflows.
#[test]
fn row07_driver_plus300_overflow_window() {
    let mut xs: Vec<i32> = (1_073_741_674..=1_073_741_823).collect();
    // One value on either side of the window.
    xs.push(1_073_741_673);
    xs.push(1_073_741_824);
    assert_driver_eq_all(&xs);
}

/// Row 8 — `2*x` overflows.
#[test]
fn row08_driver_double_overflows() {
    let mut rng = Rng::new(0x5EED_0008);
    let xs: Vec<i32> = (0..500)
        .map(|_| rng.range(1_073_741_824, i32::MAX as i64) as i32)
        .collect();
    assert_driver_eq_all(&xs);
}

/// Row 9 — `2*x` underflows.
#[test]
fn row09_driver_double_underflows() {
    let mut rng = Rng::new(0x5EED_0009);
    let xs: Vec<i32> = (0..500)
        .map(|_| rng.range(i32::MIN as i64, -1_073_741_824) as i32)
        .collect();
    assert_driver_eq_all(&xs);
}

/// Row 10 — powers of two, both signs, including the sign bit.
#[test]
fn row10_driver_powers_of_two() {
    let mut xs = Vec::new();
    for k in 0..32 {
        let v = 1i64 << k;
        xs.push(v as i32);
        xs.push((-v) as i32);
        xs.push((v - 1) as i32);
    }
    assert_driver_eq_all(&xs);
}

/// Row 11 — uniform random over the entire `i32` range.
#[test]
fn row11_driver_uniform_random_i32() {
    let mut rng = Rng::new(0x5EED_0011);
    let xs: Vec<i32> = (0..2000).map(|_| rng.next_i32()).collect();
    assert_driver_eq_all(&xs);
}

/// Row 12 — notable raw bit patterns passed as `int`.
#[test]
fn row12_driver_bit_patterns() {
    let xs: Vec<i32> = [
        0x8000_0000u32,
        0xFFFF_FFFF,
        0x7FFF_FFFF,
        0xAAAA_AAAA,
        0x5555_5555,
        0x0000_0001,
        0xFFFF_FFFE,
        0x8000_0001,
    ]
    .iter()
    .map(|&p| p as i32)
    .collect();
    assert_driver_eq_all(&xs);
}

// ---------------------------------------------------------------------------
// `main()` — the composed `scanf` → `driver` → `printf` pipeline
// ---------------------------------------------------------------------------

/// Row 13 — plain decimal digits, EOF-terminated.
#[test]
fn row13_main_plain_digits_eof_terminated() {
    let mut rng = Rng::new(0x5EED_0013);
    for _ in 0..200 {
        let v = rng.range(0, 1_000_000_000);
        assert_main_eq(format!("{v}").as_bytes());
    }
}

/// Row 14 — newline / CRLF terminated.
#[test]
fn row14_main_newline_terminated() {
    let mut rng = Rng::new(0x5EED_0014);
    for _ in 0..80 {
        let v = rng.range(0, 1_000_000_000);
        assert_main_eq(format!("{v}\n").as_bytes());
        assert_main_eq(format!("{v}\r\n").as_bytes());
    }
}

/// Row 15 — explicit `+` / `-` signs.
#[test]
fn row15_main_explicit_signs() {
    let mut rng = Rng::new(0x5EED_0015);
    for _ in 0..80 {
        let v = rng.range(0, 3_000_000_000);
        assert_main_eq(format!("+{v}").as_bytes());
        assert_main_eq(format!("-{v}").as_bytes());
        assert_main_eq(format!("+{v}\n").as_bytes());
        assert_main_eq(format!("-{v}\n").as_bytes());
    }
}

/// Row 16 — every whitespace class glibc's `%d` skips, singly and mixed.
#[test]
fn row16_main_leading_whitespace_classes() {
    let mut rng = Rng::new(0x5EED_0016);
    let classes: [&[u8]; 8] = [
        b" ", b"\t", b"\n", b"\x0b", b"\x0c", b"\r", b" \t\n\x0b\x0c\r", b"\n\n\n",
    ];
    for ws in classes {
        for _ in 0..10 {
            let v = rng.range(i32::MIN as i64, i32::MAX as i64);
            let mut input = ws.to_vec();
            input.extend_from_slice(format!("{v}").as_bytes());
            assert_main_eq(&input);
        }
    }
}

/// Row 17 — whitespace run crossing the internal read chunk.
#[test]
fn row17_main_long_whitespace_run() {
    for n in [4095usize, 4096, 4097, 10_000] {
        let mut input = vec![b' '; n];
        input.extend_from_slice(b"12345");
        assert_main_eq(&input);
        let mut input = vec![b'\n'; n];
        input.extend_from_slice(b"-67890");
        assert_main_eq(&input);
    }
}

/// Row 18 — leading zeros and signed zero.
#[test]
fn row18_main_leading_zeros() {
    for s in [
        "0",
        "-0",
        "+0",
        "00",
        "0000000042",
        "-0000042",
        "+0000042",
        "000000000000000000000000000000",
        "-000000000000000000000000000001",
    ] {
        assert_main_eq(s.as_bytes());
    }
}

/// Row 19 — digit-count sweep from 1 to 19 digits.
#[test]
fn row19_main_digit_count_sweep() {
    let mut rng = Rng::new(0x5EED_0019);
    for len in 1..=19u32 {
        for _ in 0..4 {
            let mut s = String::new();
            for i in 0..len {
                let d = if i == 0 {
                    rng.range(1, 9)
                } else {
                    rng.range(0, 9)
                };
                s.push((b'0' + d as u8) as char);
            }
            assert_main_eq(s.as_bytes());
            assert_main_eq(format!("-{s}").as_bytes());
        }
    }
}

/// Row 20 — the `int` / `long` magnitude class boundaries, as text.
#[test]
fn row20_main_magnitude_boundaries() {
    for s in [
        "2147483646",
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483647",
        "-2147483648",
        "-2147483649",
        "-2147483650",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967296",
        "-4294967297",
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551616",
        "99999999999999999999999999",
    ] {
        assert_main_eq(s.as_bytes());
    }
}

/// Row 21 — digit run at / beyond the internal read chunk size.
#[test]
fn row21_main_long_digit_runs() {
    for n in [4095usize, 4096, 4097, 10_000] {
        assert_main_eq(&vec![b'9'; n]);
        let mut neg = vec![b'-'];
        neg.extend(std::iter::repeat(b'1').take(n));
        assert_main_eq(&neg);
        // Leading zeros then a small value, crossing the chunk boundary.
        let mut zeros = vec![b'0'; n];
        zeros.extend_from_slice(b"77");
        assert_main_eq(&zeros);
    }
}

/// Row 22 — a valid value followed by trailing garbage.
#[test]
fn row22_main_trailing_garbage() {
    for s in [
        "12abc", "1 2", "5)", "42\n7", "7\t8", "-3.5", "9e9", "0x10", "123,456", "42#",
    ] {
        assert_main_eq(s.as_bytes());
    }
}

/// Row 23 — property loop over random `i32` values rendered as text.
#[test]
fn row23_main_random_i32_text() {
    let mut rng = Rng::new(0x5EED_0023);
    for _ in 0..500 {
        let v = rng.next_i32();
        assert_main_eq(format!("{v}").as_bytes());
    }
}

/// Row 24 — property loop over random `i64` values (out of `int` range).
#[test]
fn row24_main_random_i64_text() {
    let mut rng = Rng::new(0x5EED_0024);
    for _ in 0..500 {
        let v = rng.next_i64();
        assert_main_eq(format!("{v}").as_bytes());
    }
}

/// Row 28 — byte-level fuzz of the `%d` conversion: random strings drawn from
/// the alphabet that drives every branch of the conversion (digits, signs, all
/// whitespace classes, letters, punctuation, NUL and high bytes).
#[test]
fn row28_main_random_byte_strings() {
    let alphabet: &[u8] = b"0123456789+-  \t\n\r\x0b\x0cabxeE.,\x00\xff/";
    let mut rng = Rng::new(0x5EED_0028);
    for _ in 0..400 {
        let len = rng.range(0, 24) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| alphabet[rng.range(0, alphabet.len() as i64 - 1) as usize])
            .collect();
        assert_main_eq(&input);
    }
}

/// Row 29 — the same fuzz, but every input is prefixed with a long run so it
/// straddles the internal read chunk.
#[test]
fn row29_main_random_byte_strings_across_chunk() {
    let alphabet: &[u8] = b"0123456789+- \t\nabx.";
    let mut rng = Rng::new(0x5EED_0029);
    for _ in 0..40 {
        let pad = rng.range(4090, 4100) as usize;
        let mut input: Vec<u8> = vec![b' '; pad];
        let len = rng.range(1, 12) as usize;
        input.extend((0..len).map(|_| alphabet[rng.range(0, alphabet.len() as i64 - 1) as usize]));
        assert_main_eq(&input);
    }
}

/// Row 30 — a strided sweep across the whole `int` domain: 65536 values spaced
/// by a prime-ish stride, so every wrap-around region of `2*x + 300` is hit.
#[test]
fn row30_driver_strided_full_domain_sweep() {
    let xs: Vec<i32> = (0..65_536i64)
        .map(|k| (i32::MIN as i64 + k * 65_537) as i32)
        .collect();
    assert_driver_eq_all(&xs);
}
