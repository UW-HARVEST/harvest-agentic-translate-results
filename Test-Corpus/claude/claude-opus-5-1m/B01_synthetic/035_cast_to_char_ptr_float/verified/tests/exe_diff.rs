//! Phase B — valid-path differential tests at the process level.
//!
//! One test per CONFIGS.md row (rows 1–47, 57–58). Every test feeds the same
//! bytes to the C program and the Rust program on stdin and requires their
//! stdout and exit status to be byte-identical.

mod common;

use common::corpus::{self, fixed};
use common::{assert_same, assert_same_all, c_exe, rust_exe};

/// Rows 1–3 — decimal integers: one digit, signed zero, leading zeros.
#[test]
fn row01_03_decimal_integers() {
    assert_same_all(fixed::INTEGERS, "integers");
}

/// Row 4 — decimals with a point and no exponent.
#[test]
fn row04_decimal_with_point() {
    assert_same_all(fixed::DECIMALS, "decimals");
}

/// Row 5 — leading decimal point.
#[test]
fn row05_point_leading() {
    assert_same_all(fixed::POINT_LEADING, "point leading");
}

/// Row 6 — trailing decimal point.
#[test]
fn row06_point_trailing() {
    assert_same_all(fixed::POINT_TRAILING, "point trailing");
}

/// Rows 7–9 — `e`, `E`, and signed exponents.
#[test]
fn row07_09_decimal_exponents() {
    assert_same_all(fixed::EXP_UNSIGNED_E, "exp e");
    assert_same_all(fixed::EXP_UPPER_E, "exp E");
    assert_same_all(fixed::EXP_SIGNED, "exp signed");
}

/// Row 10 — exponent marker with no digits after it. glibc backs the exponent
/// characters out of the subject sequence and converts the mantissa only.
#[test]
fn row10_exponent_without_digits() {
    assert_same_all(fixed::EXP_NO_DIGITS, "exp no digits");
}

/// Row 11 — exponents far wider than any integer type.
#[test]
fn row11_exponent_wider_than_i64() {
    assert_same_all(fixed::EXP_HUGE, "exp huge");
}

/// Row 12 — a zero mantissa must stay zero no matter how large the exponent.
#[test]
fn row12_zero_mantissa_huge_exponent() {
    assert_same_all(fixed::EXP_HUGE_ZERO_MANTISSA, "zero mantissa huge exp");
}

/// Row 13 — plain hexadecimal literals.
#[test]
fn row13_hex_simple() {
    assert_same_all(fixed::HEX_SIMPLE, "hex simple");
}

/// Row 14 — hexadecimal with a point but no binary exponent.
#[test]
fn row14_hex_with_point() {
    assert_same_all(fixed::HEX_POINT, "hex point");
}

/// Row 15 — hexadecimal with `p`/`P` exponents.
#[test]
fn row15_hex_binary_exponent() {
    assert_same_all(fixed::HEX_EXP, "hex exp");
}

/// Row 16 — `p` with no digits after it.
#[test]
fn row16_hex_exponent_without_digits() {
    assert_same_all(fixed::HEX_EXP_NO_DIGITS, "hex exp no digits");
}

/// Row 17 — hex mantissas that fit the accumulator exactly.
#[test]
fn row17_hex_short_mantissa() {
    assert_same_all(fixed::HEX_SHORT_MANTISSA, "hex short mantissa");
}

/// Row 18 — hex mantissas long enough to set the sticky bit, which is where a
/// naive truncating conversion rounds the wrong way.
#[test]
fn row18_hex_sticky_bit() {
    assert_same_all(fixed::HEX_STICKY, "hex sticky");
}

/// Row 19 — hexadecimal overflow to infinity.
#[test]
fn row19_hex_overflow() {
    assert_same_all(fixed::HEX_OVERFLOW, "hex overflow");
}

/// Row 20 — hexadecimal subnormals and underflow.
#[test]
fn row20_hex_subnormal_and_underflow() {
    assert_same_all(fixed::HEX_SUBNORMAL, "hex subnormal");
}

/// Rows 21–22 — `inf` and `infinity` in every letter case, both signs.
#[test]
fn row21_22_infinity_spellings() {
    assert_same_all(fixed::INF, "inf");
    assert_same_all(fixed::INFINITY, "infinity");
}

/// Row 23 — `inf` followed by something that is not the start of `inity`.
#[test]
fn row23_inf_with_trailing_bytes() {
    assert_same_all(fixed::INF_TRAILING, "inf trailing");
}

/// Row 24 — `nan` in every letter case, both signs.
#[test]
fn row24_nan_spellings() {
    assert_same_all(fixed::NAN, "nan");
}

/// Row 25 — `nan` with an n-char-sequence payload.
#[test]
fn row25_nan_with_payload() {
    assert_same_all(fixed::NAN_PAYLOAD, "nan payload");
}

/// Rows 26–27 — every whitespace character, alone and in runs, before the
/// token. `scanf` skips these; `fgets`-style reading would not.
#[test]
fn row26_27_leading_whitespace() {
    assert_same_all(fixed::LEADING_WS, "leading whitespace");
    // every single whitespace byte, individually, before a value
    let mut cases = Vec::new();
    for ws in [" ", "\t", "\n", "\r", "\x0b", "\x0c"] {
        for reps in [1usize, 2, 3, 17] {
            cases.push(format!("{}{}", ws.repeat(reps), "1.5"));
            cases.push(format!("{}{}", ws.repeat(reps), "-inf"));
        }
    }
    // all six whitespace bytes in every order-ish mix
    cases.push(" \t\n\r\x0b\x0c1.5".to_string());
    cases.push("\x0c\x0b\r\n\t 1.5".to_string());
    assert_same_all(cases, "per-character whitespace");
}

/// Rows 28–29 — trailing bytes after a complete token. `scanf` stops at the
/// first byte it cannot use and the program never reads the rest.
#[test]
fn row28_29_trailing_junk() {
    assert_same_all(fixed::TRAILING_JUNK, "trailing junk");
}

/// Rows 30–32 — exact halfway values and one step either side, i.e. the
/// ties-to-even boundary.
#[test]
fn row30_32_ties_to_even() {
    assert_same_all(fixed::TIES, "ties");
    for seed in [11u64, 12, 13] {
        assert_same_all(
            corpus::exact_ties(seed, 400),
            &format!("generated exact ties seed={seed}"),
        );
    }
}

/// Row 33 — `FLT_MAX` and its neighbours, including the full 39-digit exact
/// decimal expansion.
#[test]
fn row33_flt_max_edge() {
    assert_same_all(fixed::FLT_MAX_EDGE, "FLT_MAX edge");
}

/// Row 34 — the first values that overflow to infinity.
#[test]
fn row34_overflow_to_infinity() {
    assert_same_all(fixed::OVERFLOW, "overflow");
}

/// Row 35 — the subnormal range and the underflow-to-zero boundary.
#[test]
fn row35_subnormal_range() {
    assert_same_all(fixed::SUBNORMAL, "subnormal");
}

/// Row 36 — literals with hundreds to thousands of significant digits.
#[test]
fn row36_oversized_literals() {
    assert_same_all(corpus::long_literals(), "long literals");
}

/// Row 37 — random `f32` bit patterns re-rendered three ways and re-read.
#[test]
fn row37_random_roundtrip() {
    for seed in [1u64, 2, 3] {
        assert_same_all(
            corpus::rand_roundtrip(seed, 700),
            &format!("roundtrip seed={seed}"),
        );
    }
}

/// Row 38 — random decimal literals over the whole sign/point/exponent grid.
#[test]
fn row38_random_decimal() {
    for seed in [4u64, 5, 6] {
        assert_same_all(
            corpus::rand_decimal(seed, 2000),
            &format!("random decimal seed={seed}"),
        );
    }
}

/// Row 39 — random hexadecimal literals.
#[test]
fn row39_random_hex() {
    for seed in [7u64, 8, 9] {
        assert_same_all(
            corpus::rand_hex(seed, 2000),
            &format!("random hex seed={seed}"),
        );
    }
}

/// Row 40 — random soup over the float-token alphabet. Most of these are
/// rejections, which is exactly the point: acceptance must agree too.
#[test]
fn row40_random_junk() {
    for seed in [10u64, 20, 30] {
        assert_same_all(
            corpus::rand_junk(seed, 2000),
            &format!("random junk seed={seed}"),
        );
    }
}

/// Row 41 — whitespace ⧺ interesting token ⧺ trailing junk.
#[test]
fn row41_random_wrapped() {
    for seed in [40u64, 50, 60] {
        assert_same_all(
            corpus::rand_wrapped(seed, 2000),
            &format!("random wrapped seed={seed}"),
        );
    }
}

/// Row 42 — random mantissas with exponents straddling the entire `f32` range,
/// so every result class (zero, subnormal, normal, infinity) is produced.
#[test]
fn row42_random_extreme_exponents() {
    for seed in [70u64, 80, 90] {
        assert_same_all(
            corpus::rand_extreme_exp(seed, 2000),
            &format!("extreme exponents seed={seed}"),
        );
    }
}

/// Row 43 — 20–60 digit significands, the near-tie region.
#[test]
fn row43_random_near_ties() {
    for seed in [100u64, 110, 120] {
        assert_same_all(
            corpus::rand_near_tie(seed, 2000),
            &format!("near ties seed={seed}"),
        );
    }
}

/// Row 44 — long hex mantissas, always exercising the sticky-bit path.
#[test]
fn row44_random_sticky_hex() {
    for seed in [130u64, 140, 150] {
        assert_same_all(
            corpus::rand_sticky_hex(seed, 2000),
            &format!("sticky hex seed={seed}"),
        );
    }
}

/// Rows 45–46 — embedded NUL bytes and raw non-UTF-8 input. The C reads bytes,
/// so the Rust translation must not go through `String`.
#[test]
fn row45_46_binary_input() {
    assert_same_all(corpus::binary_inputs(), "binary inputs");
    assert_same_all(fixed::NUL_AND_BINARY, "nul bytes");
    // every single byte value on its own
    let singles: Vec<Vec<u8>> = (0u8..=255).map(|b| vec![b]).collect();
    assert_same_all(singles, "every single byte");
    // every byte value followed by a digit, and preceded by a digit
    let pairs: Vec<Vec<u8>> = (0u8..=255).flat_map(|b| [vec![b, b'1'], vec![b'1', b]]).collect();
    assert_same_all(pairs, "every byte adjacent to a digit");
}

/// Row 47 — `\r\n` line endings and a very long single line.
#[test]
fn row47_line_endings_and_long_lines() {
    let mut cases: Vec<Vec<u8>> = vec![
        b"1.5\r\n".to_vec(),
        b"\r\n1.5".to_vec(),
        b"\r\n\r\n-2.5\r\n".to_vec(),
        b"1.5\n\r".to_vec(),
    ];
    cases.push({
        let mut v = Vec::new();
        v.extend(std::iter::repeat(b'0').take(65536));
        v.extend_from_slice(b"1.5");
        v
    });
    cases.push({
        let mut v = b"1.".to_vec();
        v.extend(std::iter::repeat(b'9').take(65536));
        v
    });
    cases.push({
        let mut v = b"0x1.".to_vec();
        v.extend(std::iter::repeat(b'f').take(65536));
        v.extend_from_slice(b"p0");
        v
    });
    assert_same_all(cases, "line endings and long lines");
}

/// Row 57 — stdin closed rather than merely empty. `read` fails instead of
/// returning EOF, and the program must still print `+0.0f` and exit 0.
#[test]
fn row57_closed_stdin() {
    use std::process::{Command, Stdio};
    let mut outs = Vec::new();
    for exe in [c_exe(), rust_exe()] {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec 0<&- ; exec {}", exe.display()))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("spawn via sh");
        outs.push((out.status.code(), out.stdout));
    }
    assert_eq!(
        outs[0], outs[1],
        "closed-stdin divergence: C={:?} RUST={:?}",
        outs[0], outs[1]
    );
    // and stdin pointing at /dev/null
    assert_same(b"", "empty stdin");
}

/// Row 58 — stdin delivered one byte at a time with a flush between each, so
/// every `read` returns a short count. This is what exercises the retry loop
/// in `ByteReader::getc`.
#[test]
fn row58_byte_at_a_time_stdin() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    for text in ["  1.5", "0x1.8p3", "-infinity", "nan(1)", "1e-45", ""] {
        let mut outs = Vec::new();
        for exe in [c_exe(), rust_exe()] {
            let mut child = Command::new(&exe)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn");
            let mut stdin = child.stdin.take().unwrap();
            let bytes = text.as_bytes().to_vec();
            let w = std::thread::spawn(move || {
                for b in bytes {
                    if stdin.write_all(&[b]).is_err() {
                        return;
                    }
                    let _ = stdin.flush();
                    std::thread::sleep(std::time::Duration::from_micros(200));
                }
            });
            let out = child.wait_with_output().expect("wait");
            let _ = w.join();
            outs.push((out.status.code(), out.stdout));
        }
        assert_eq!(
            outs[0], outs[1],
            "byte-at-a-time divergence for {text:?}: C={:?} RUST={:?}",
            outs[0], outs[1]
        );
    }
}

/// Boundary neighbours: for a spread of `f32` values, the exact decimal
/// expansion of the value, of its predecessor and of its successor must all
/// convert identically. This is the "one step past a documented range" check
/// applied across the whole representable range rather than at one point.
#[test]
fn test_boundary_neighbours() {
    let mut cases = Vec::new();
    for bits in [
        0x0000_0001u32,
        0x0000_0002,
        0x007f_ffff,
        0x0080_0000,
        0x0080_0001,
        0x3f80_0000,
        0x4b80_0000,
        0x7f7f_fffe,
        0x7f7f_ffff,
    ] {
        for b in [bits.wrapping_sub(1), bits, bits + 1] {
            let v = f32::from_bits(b);
            cases.push(format!("{:?}", v));
            cases.push(format!("{:.*e}", 45, f64::from(v)));
            cases.push(format!("{:.*}", 60, f64::from(v)));
        }
    }
    assert_same_all(cases, "boundary neighbours");
}

/// The whole fixed corpus in one go, as a catch-all regression net.
#[test]
fn test_entire_fixed_corpus() {
    assert_same_all(corpus::all_fixed(), "entire fixed corpus");
}
