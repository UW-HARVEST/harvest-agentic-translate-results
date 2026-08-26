//! Regression tests for every candidate produced by the independent audits of
//! the `fgets` / `strtol` emulation and of the `%.1f` emulation.
//!
//! The audits were performed against C mirrors of the Rust code; these tests
//! re-check each candidate through the *real* differential harness (both `.so`s
//! via `dlsym`, both executables via `exec`), so nothing is taken on trust.

mod common;
use common::*;

fn s(v: &str) -> Vec<u8> {
    v.as_bytes().to_vec()
}
fn pad(c: u8, n: usize, tail: &str) -> Vec<u8> {
    let mut v = vec![c; n];
    v.extend_from_slice(tail.as_bytes());
    v
}

// ---------------------------------------------------------------------------
// Tier 1 — the 99-byte fgets boundary (an off-by-one changes the printed value)
// ---------------------------------------------------------------------------
#[test]
fn audit_t1_fgets_99_byte_boundary() {
    let cases: Vec<Vec<u8>> = vec![
        pad(b' ', 98, "-1\n"),
        pad(b' ', 98, "12\n"),
        pad(b' ', 97, "12\n"),
        pad(b' ', 99, "12\n"),
        pad(b'0', 90, "1234567890\n"),
        pad(b'0', 89, "1234567890\n"),
        pad(b'0', 99, "7\n"),
        pad(b'0', 98, "7\n"),
        pad(b'9', 98, "\n"),
        pad(b'9', 99, "\n"),
        pad(b'9', 105, "\n"),
        vec![b'1'; 99],
        pad(b'a', 98, "\n"),
        pad(b'a', 99, "\n"),
        vec![0u8; 150],
        {
            let mut v = vec![b'1'; 99];
            v.push(0);
            v.push(b'\n');
            v
        },
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_input_matches(c, &format!("audit T1 #{}", i));
    }
}

// ---------------------------------------------------------------------------
// Tier 2 — degenerate streams
// ---------------------------------------------------------------------------
#[test]
fn audit_t2_degenerate_streams() {
    let cases: Vec<Vec<u8>> = vec![
        s(""),
        s("\n"),
        s("7"),
        s("7\n9\n"),
        s("\r\n"),
        s("\n\n"),
        s(" \n7\n"),
        s("\t\x0b\x0c\r 7\n"),
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_input_matches(c, &format!("audit T2 #{}", i));
    }
}

// ---------------------------------------------------------------------------
// Tier 3 — embedded NUL / C-string reconstruction
// ---------------------------------------------------------------------------
#[test]
fn audit_t3_embedded_nul() {
    let cases: Vec<Vec<u8>> = vec![
        b"\x005\n".to_vec(),
        b"5\x00\n".to_vec(),
        b"12\x0034\n".to_vec(),
        b"-\x005\n".to_vec(),
        b" \x005\n".to_vec(),
        b"\x00".to_vec(),
        vec![0, 0, 0],
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_input_matches(c, &format!("audit T3 #{}", i));
    }
}

// ---------------------------------------------------------------------------
// Tier 4 — strtol range boundaries / ERANGE
// ---------------------------------------------------------------------------
#[test]
fn audit_t4_range_boundaries() {
    let mut cases: Vec<Vec<u8>> = vec![
        s("9223372036854775807\n"),
        s("9223372036854775808\n"),
        s("-9223372036854775808\n"),
        s("-9223372036854775809\n"),
        s("-9223372036854775810\n"),
        s("92233720368547758080\n"),
        s("18446744073709551616\n"),
        s("0000000000000000000000000000009223372036854775808\n"),
        s("-0000000000000000000000000000009223372036854775808\n"),
        s("2147483647\n"),
        s("2147483648\n"),
        s("-2147483648\n"),
        s("-2147483649\n"),
    ];
    cases.push(pad(b'9', 40, "\n"));
    cases.push(pad(b'0', 90, "7\n"));
    for (i, c) in cases.iter().enumerate() {
        assert_input_matches(c, &format!("audit T4 #{}", i));
    }
}

// ---------------------------------------------------------------------------
// Tier 5 — strtol syntax edges
// ---------------------------------------------------------------------------
#[test]
fn audit_t5_syntax_edges() {
    let mut cases: Vec<Vec<u8>> = [
        "0x10\n", "  +0012xyz\n", "++5\n", "--5\n", "+-5\n", "- 5\n", "+ 5\n", "-\n", "+\n", "-",
        "  +  \n", "-0\n", "+0\n", "007\n", ".5\n", "5.9\n", "5e3\n", "1,000\n", "1 2\n", "x5\n",
    ]
    .iter()
    .map(|v| s(v))
    .collect();
    cases.push(vec![0xa0, b'7', b'\n']);
    cases.push(vec![0x85, b'7', b'\n']);
    cases.push(vec![0xff, b'7', b'\n']);
    cases.push(vec![b'7', 0xff, b'\n']);
    for (i, c) in cases.iter().enumerate() {
        assert_input_matches(c, &format!("audit T5 #{}", i));
    }
}

// ---------------------------------------------------------------------------
// Signed-overflow inputs reachable from stdin (`bedrooms += extra_bedrooms`)
// ---------------------------------------------------------------------------
#[test]
fn audit_stdin_reachable_overflow() {
    let mut cases: Vec<Vec<u8>> = vec![
        s("2147483643\n"),
        s("2147483647\n"),
        s("-2147483648\n"),
        s("-2147483647\n"),
        s("1073741824\n"),
        s("-1073741824\n"),
        s("1234567890\n"),
    ];
    for v in 2147483642i64..=2147483647 {
        cases.push(format!("{}\n", v).into_bytes());
    }
    for (i, c) in cases.iter().enumerate() {
        assert_input_matches(c, &format!("audit overflow #{}", i));
    }
}

// ---------------------------------------------------------------------------
// Every candidate `f64` produced by the independent `%.1f` audit, given by bit
// pattern, plus the sign-flipped variant of each.  Tiers, in order:
//   T1 values where only the `< 2^53` guard keeps the fast path correct,
//   T2 exact 1-decimal ties (`odd/4`) decided by `format!("{:.1}")`,
//   T3 ties where the fast path *does* fire (hardware ties-to-even must agree
//      with decimal half-even),
//   T4 zero/sign/subnormal, T5 NaN/Inf, T6 one-ulp neighbours of near-ties,
//   T7 huge magnitudes whose exact expansion is hundreds of digits.
// ---------------------------------------------------------------------------
const AUDIT_F64_BITS: &[u64] = &[
    // T1
    0x430999999999999C, 0x430999999999999F, 0x43099999999999A1, 0x43099999999999A4,
    0xC3099999999999A4, 0x433FFFFFFFFFFFFF, 0x4340000000000001, 0x431FFFFFFFFFFFFF,
    0xC31FFFFFFFFFFFFF, 0x7E37E43C8800759C, 0x7FEFFFFFFFFFFFFF, 0xFFEFFFFFFFFFFFFF,
    0x7FE1CCF385EBC8A0, 0x430999999999999A,
    // T2
    0x3FD0000000000000, 0xBFD0000000000000, 0x3FF4000000000000, 0xBFF4000000000000,
    0x4002000000000000, 0x400A000000000000, 0x4011000000000000, 0x4020800000000000,
    0x412E848080000000, 0x3FE8000000000000, 0xBFE8000000000000, 0x3FFC000000000000,
    0x4006000000000000, 0x400E000000000000, 0x4021800000000000, 0x412E848180000000,
    0x430FFFFFFFFFFFFE,
    // T3
    0x42F999999999999C, 0xC2F999999999999C, 0x42F99999999999A4, 0x42FFFFFFFFFFFFFC,
    0x4300000000000002, 0x4309999999999996, 0xC309999999999996, 0x42FC6BF526340004,
    0x43010D9316EC0006, 0x4309999999999999,
    // T4
    0x8000000000000000, 0x0000000000000000, 0xBFA47AE147AE147B, 0x8000000000000001,
    0x0000000000000001, 0x800FFFFFFFFFFFFF, 0x0010000000000000, 0x81A56E1FC2F8F359,
    0xBFA9999999999999, 0xBFA999999999999A,
    // T5
    0x7FF8000000000000, 0xFFF8000000000000, 0x7FF8000000000001, 0x7FF0000000000001,
    0xFFF0000000000001, 0x7FF0000000000000, 0xFFF0000000000000,
    // T6
    0x3FCFFFFFFFFFFFFF, 0x3FD0000000000001, 0xBFD0000000000001, 0x3FC3333333333333,
    0x3FD6666666666666, 0x3FDCCCCCCCCCCCCD, 0x4021E66666666666, 0x4005666666666666,
    0x3FB9999999999999, 0x3FB999999999999A, 0x3FB999999999999B, 0x4004000000000000,
    0x400C000000000000, 0x4012000000000000,
    // T7
    0x44B52D02C7E14AF6, 0x7FE0000000000000, 0x7FEFFFFFFFFFFFFE, 0x430C6BF526340000,
    0x4341C37937E08000, 0x4376345785D8A000, 0x4330000000000000,
];

#[test]
fn audit_f64_candidates() {
    let mut vals: Vec<f64> = Vec::new();
    for &b in AUDIT_F64_BITS {
        vals.push(f64::from_bits(b));
        vals.push(f64::from_bits(b ^ 0x8000_0000_0000_0000)); // sign-flipped
    }
    // several integer-field contexts so `%d` and `%.1f` are checked together
    let mut cases: Vec<(House, i32)> = Vec::new();
    for &v in &vals {
        cases.push((House::new(2, 5, v), 7));
        cases.push((House::new(0, 0, v), 0));
        cases.push((House::new(i32::MAX, i32::MIN, v), i32::MAX));
    }
    assert_run_batch(&cases, "audit f64 candidates");
    assert_run_twice_batch(&cases, "audit f64 candidates x2");
}
