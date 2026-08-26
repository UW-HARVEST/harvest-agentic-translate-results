// Phase C — one differential test per row of ERRORS.md.
//
// Every row constructs the exact invalid input, feeds it to both shared objects
// through their exported `main` symbol *and* to both real executables, and
// asserts that they reject it in the same way.  For the rejection rows the
// expected result is pinned explicitly (`0 0x0p+0 0.0000`, i.e. `scanf` left `f`
// at its initialiser) rather than merely "both failed somehow".

mod common;

use common::{diff_exe, diff_main, main_lines, Rng, Side, SEED};

/// The output the C program produces whenever the `%lf` conversion is rejected:
/// `f` keeps the `0.0f` initialiser.
const REJECTED: &[u8] = b"0 0x0p+0 0.0000";

fn to_vecs(inputs: &[&str]) -> Vec<Vec<u8>> {
    inputs.iter().map(|s| s.as_bytes().to_vec()).collect()
}

/// Asserts C and Rust agree *and* that the shared expected line is the one the C
/// implementation really produces.
fn expect_exact(what: &str, inputs: &[Vec<u8>], expected: &[u8]) {
    diff_main(what, inputs);
    let c = main_lines(Side::C, inputs);
    for (i, line) in c.iter().enumerate() {
        assert_eq!(
            line.as_slice(),
            expected,
            "[{what}] C produced {:?} for input {:?}, expected {:?}",
            String::from_utf8_lossy(line),
            String::from_utf8_lossy(&inputs[i]),
            String::from_utf8_lossy(expected)
        );
    }
    diff_exe(what, inputs);
}

/// Every listed input must be rejected identically by both implementations.
fn expect_rejected(what: &str, inputs: &[&str]) {
    expect_exact(what, &to_vecs(inputs), REJECTED);
}

/// ERRORS row 1 — EOF straight away
fn row_01_empty_input() {
    expect_rejected("row01 empty input", &[""]);
}

/// ERRORS row 2 — white space only, then EOF
fn row_02_whitespace_only() {
    expect_rejected(
        "row02 whitespace only",
        &[
            " ", "  ", "\t", "\n", "\r", "\x0b", "\x0c", " \t\n\x0b\x0c\r", "\n\n\n\n",
            "                                ",
        ],
    );
}

/// ERRORS row 3 — EOF straight after the sign
fn row_03_sign_then_eof() {
    expect_rejected("row03 sign then eof", &["-", "+", "  -", "\n+", "\t\t-"]);
}

/// ERRORS row 4 — the sign is followed by something that cannot start a number
fn row_04_sign_then_non_numeric() {
    expect_rejected(
        "row04 sign then non numeric",
        &[
            "-a", "+z", "- 1", "+ 1", "-\n1", "--1", "++1", "-+1", "+-1", "-e5", "-p1", "-x1",
            "-,5", "-\0", "+\x7f",
        ],
    );
}

/// ERRORS row 5 — the first non-space byte cannot start a number
fn row_05_bad_first_byte() {
    expect_rejected(
        "row05 bad first byte",
        &[
            "a", "@", "e5", "E5", "p1", "P1", "x1", "X1", "z", "/", ":", "[", "`", "{", "~",
            " a", "\t@", "\nq", "'", "\"", ",", ";", "(", ")", "*", "%", "#", "!", "?", "<",
            ">", "=", "|", "\\", "^", "&", "$", "_", "d", "g", "h", "j", "k", "l", "m", "o",
            "q", "r", "s", "t", "u", "v", "w", "y",
        ],
    );
}

/// ERRORS row 6 — a decimal point but no digit anywhere
fn row_06_dot_without_digits() {
    expect_rejected(
        "row06 dot without digits",
        &[
            ".", "-.", "+.", ".e5", ".E5", ".-", ".+", "..", "...", " .", ".e", ".p", "-.e5",
            ".x", ".,", "-..", ".\0",
        ],
    );
}

/// ERRORS row 7 — `"n"` then EOF
fn row_07_n_then_eof() {
    expect_rejected("row07 n then eof", &["n", "N", "-n", "+N", " n"]);
}

/// ERRORS row 8 — `"n"` then a byte other than `a`/`A`
fn row_08_n_then_wrong_byte() {
    expect_rejected(
        "row08 n then wrong byte",
        &["nb", "nB", "n1", "n.", "n ", "n\n", "nn", "no", "N0", "-nx", "n\0"],
    );
}

/// ERRORS row 9 — `"na"` then EOF
fn row_09_na_then_eof() {
    expect_rejected("row09 na then eof", &["na", "nA", "Na", "NA", "-na", "+NA"]);
}

/// ERRORS row 10 — `"na"` then a byte other than `n`/`N`
fn row_10_na_then_wrong_byte() {
    expect_rejected(
        "row10 na then wrong byte",
        &["nax", "na1", "na.", "na ", "na\n", "nam", "NAB", "-naz", "na\0"],
    );
}

/// ERRORS row 11 — `"i"` then EOF
fn row_11_i_then_eof() {
    expect_rejected("row11 i then eof", &["i", "I", "-i", "+I", " i"]);
}

/// ERRORS row 12 — `"i"` then a byte other than `n`/`N`
fn row_12_i_then_wrong_byte() {
    expect_rejected(
        "row12 i then wrong byte",
        &["ix", "i1", "i.", "i ", "i\n", "if", "II", "-iz", "i\0"],
    );
}

/// ERRORS row 13 — `"in"` then EOF
fn row_13_in_then_eof() {
    expect_rejected("row13 in then eof", &["in", "iN", "In", "IN", "-in", "+IN"]);
}

/// ERRORS row 14 — `"in"` then a byte other than `f`/`F`
fn row_14_in_then_wrong_byte() {
    expect_rejected(
        "row14 in then wrong byte",
        &["ing", "in1", "in.", "in ", "in\n", "ini", "INT", "-inz", "in\0"],
    );
}

/// ERRORS row 15 — `"infi"` then EOF: the already-matched `"inf"` is discarded
fn row_15_infi_then_eof() {
    expect_rejected(
        "row15 infi then eof",
        &["infi", "INFI", "-infi", "+InFi", "infin", "infini", "infinit"],
    );
}

/// ERRORS row 16 — `"infi"` then a byte that derails `"nity"`
fn row_16_infi_then_wrong_byte() {
    expect_rejected(
        "row16 infi then wrong byte",
        &[
            "infix", "infiy", "infi1", "infi ", "infinx", "infin1", "infinix", "infini1",
            "infinitx", "infinit1", "infinit ", "-infinitz", "infi\0",
        ],
    );
    // ...whereas a *complete* "infinity" followed by junk is accepted, which is
    // what makes the rows above a real boundary.
    expect_exact(
        "row16 complete infinity then junk",
        &to_vecs(&["infinity1x", "infinityz", "INFINITY(", "infinity."]),
        b"7ff0000000000000 inf inf",
    );
}

/// ERRORS row 17 — truncated `"infinity"` at every length
fn row_17_truncated_infinity() {
    let full = "infinity";
    let mut inputs: Vec<String> = Vec::new();
    for n in 4..full.len() {
        inputs.push(full[..n].to_string());
        inputs.push(format!("-{}", &full[..n]));
        inputs.push(format!("{} ", &full[..n]));
    }
    let refs: Vec<&str> = inputs.iter().map(|s| s.as_str()).collect();
    expect_rejected("row17 truncated infinity", &refs);
}

/// ERRORS row 18 — the accumulated buffer is exactly `0x`/`0X`
fn row_18_bare_hex_prefix() {
    expect_rejected(
        "row18 bare hex prefix",
        &[
            "0x", "0X", "0xg", "0Xg", "0x ", "0x\n", "0xp1", "0xP1", "0x,", "0x-", "0x+",
            "0xx", "0xX", "0x\0", "0xz9", "  0x", "0xp", "0x_",
        ],
    );
}

/// ERRORS row 19 — a *signed* bare hex prefix.  The buffer is three characters
/// long here, so this is a different code path from row 18 — and it must still
/// be rejected rather than producing `-0.0`.
fn row_19_signed_bare_hex_prefix() {
    expect_rejected(
        "row19 signed bare hex prefix",
        &[
            "-0x", "+0x", "-0X", "+0X", "-0xz", "+0xg", "-0x ", "+0x\n", "-0xp1", "-0x-",
            "  -0x", "\n+0X", "-0x\0",
        ],
    );
    // The neighbouring accepted cases, to prove the boundary is in the right
    // place: `0x.` / `-0x.` *do* convert (only the leading `0` is consumed).
    expect_exact("row19 boundary 0x.", &to_vecs(&["0x.", "0x.g", "0X."]), REJECTED);
    expect_exact(
        "row19 boundary -0x.",
        &to_vecs(&["-0x.", "-0X.", "-0x.z"]),
        b"8000000000000000 -0x0p+0 -0.0000",
    );
}

/// ERRORS row 20 — a lone NUL byte or high byte
fn row_20_nul_and_high_bytes() {
    let mut inputs: Vec<Vec<u8>> = vec![
        vec![0u8],
        vec![0xffu8],
        vec![0x80u8],
        vec![0xc3, 0xa9],
        vec![0u8, b'1'],
        vec![0xff, b'1'],
        vec![b' ', 0u8],
    ];
    // every non-ASCII byte on its own
    for b in 0x80u8..=0xff {
        inputs.push(vec![b]);
    }
    // every control byte that is not white space
    for b in 0x00u8..0x20 {
        if !matches!(b, b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            inputs.push(vec![b]);
        }
    }
    expect_exact("row20 nul and high bytes", &inputs, REJECTED);
}

/// ERRORS row 21 — oversized inputs
fn row_21_oversized_input() {
    // 100 000 spaces then EOF
    let spaces = vec![b' '; 100_000];
    expect_exact("row21 100k spaces", &[spaces], REJECTED);

    // 10 000 digits with no exponent, and the same with a fraction
    let mut rng = Rng::new(SEED ^ 21_21);
    let mut long: Vec<u8> = Vec::with_capacity(10_001);
    for _ in 0..10_000 {
        long.push(b'0' + (rng.below(10) as u8));
    }
    let mut long_frac = long.clone();
    long_frac.insert(5000, b'.');
    let mut long_zeros = vec![b'0'; 10_000];
    long_zeros.push(b'1');
    diff_main(
        "row21 oversized digit runs",
        &[long, long_frac, long_zeros, vec![b'9'; 5000]],
    );

    // a 10 000-digit exponent
    let mut huge_exp = b"1e".to_vec();
    huge_exp.extend(std::iter::repeat(b'9').take(10_000));
    let mut huge_negexp = b"1e-".to_vec();
    huge_negexp.extend(std::iter::repeat(b'9').take(10_000));
    let mut huge_hexexp = b"0x1p".to_vec();
    huge_hexexp.extend(std::iter::repeat(b'9').take(10_000));
    diff_main(
        "row21 oversized exponents",
        &[huge_exp, huge_negexp, huge_hexexp],
    );
}

/// ERRORS row 22 — one step past the representable range: `ERANGE`, but the
/// conversion still succeeds
fn row_22_one_past_range() {
    expect_exact(
        "row22 overflow +",
        &to_vecs(&["1e309", "1e400", "1e99999", "0x1p1024", "0x1p2000", "1.8e308"]),
        b"7ff0000000000000 inf inf",
    );
    expect_exact(
        "row22 overflow -",
        &to_vecs(&["-1e309", "-1e400", "-0x1p1024", "-1.8e308"]),
        b"fff0000000000000 -inf -inf",
    );
    expect_exact(
        "row22 underflow +",
        &to_vecs(&["1e-400", "1e-99999", "0x1p-1080", "2e-324", "0x1p-1075"]),
        REJECTED,
    );
    expect_exact(
        "row22 underflow -",
        &to_vecs(&["-1e-400", "-1e-99999", "-0x1p-1080", "-2e-324"]),
        b"8000000000000000 -0x0p+0 -0.0000",
    );
    // exactly at the boundary (still finite / still subnormal)
    expect_exact(
        "row22 max normal",
        &to_vecs(&["1.7976931348623157e308"]),
        b"7fefffffffffffff 0x1.fffffffffffffp+1023 179769313486231570814527423731704356798070567525844996598917476803157260780028538760589558632766878171540458953514382464234321326889464182768467546703537516986049910576551282076245490090389328944075868508455133942304583236903222948165808559332123348274797826204144723168738177180919299881250404026184124858368.0000",
    );
    expect_exact(
        "row22 min subnormal",
        &to_vecs(&["5e-324", "0x1p-1074", "3e-324"]),
        b"1 0x0.0000000000001p-1022 0.0000",
    );
}

/// ERRORS row 23 — `driver` rejects nothing; the out-of-range "enum" values for a
/// `double` are the non-finite exponent fields, checked here through the C ABI
/// with the sign/mantissa corners.
fn row_23_driver_accepts_every_bit_pattern() {
    // (the exhaustive sweep lives in ffi_driver.rs; this pins the corner values)
    let bits: Vec<u64> = vec![
        0x0000_0000_0000_0000,
        0x8000_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        0xffff_ffff_ffff_ffff,
        0x7ff8_0000_0000_0000,
        0xfff8_0000_0000_0000,
        0x000f_ffff_ffff_ffff,
        0x0010_0000_0000_0000,
        0x7fef_ffff_ffff_ffff,
    ];
    common::diff_driver_bits("row23 corner bit patterns", &bits);
}

fn main() {
    common::run_suite(
        "errors",
        &[
            ("row_01_empty_input", row_01_empty_input),
            ("row_02_whitespace_only", row_02_whitespace_only),
            ("row_03_sign_then_eof", row_03_sign_then_eof),
            ("row_04_sign_then_non_numeric", row_04_sign_then_non_numeric),
            ("row_05_bad_first_byte", row_05_bad_first_byte),
            ("row_06_dot_without_digits", row_06_dot_without_digits),
            ("row_07_n_then_eof", row_07_n_then_eof),
            ("row_08_n_then_wrong_byte", row_08_n_then_wrong_byte),
            ("row_09_na_then_eof", row_09_na_then_eof),
            ("row_10_na_then_wrong_byte", row_10_na_then_wrong_byte),
            ("row_11_i_then_eof", row_11_i_then_eof),
            ("row_12_i_then_wrong_byte", row_12_i_then_wrong_byte),
            ("row_13_in_then_eof", row_13_in_then_eof),
            ("row_14_in_then_wrong_byte", row_14_in_then_wrong_byte),
            ("row_15_infi_then_eof", row_15_infi_then_eof),
            ("row_16_infi_then_wrong_byte", row_16_infi_then_wrong_byte),
            ("row_17_truncated_infinity", row_17_truncated_infinity),
            ("row_18_bare_hex_prefix", row_18_bare_hex_prefix),
            (
                "row_19_signed_bare_hex_prefix",
                row_19_signed_bare_hex_prefix,
            ),
            ("row_20_nul_and_high_bytes", row_20_nul_and_high_bytes),
            ("row_21_oversized_input", row_21_oversized_input),
            ("row_22_one_past_range", row_22_one_past_range),
            (
                "row_23_driver_accepts_every_bit_pattern",
                row_23_driver_accepts_every_bit_pattern,
            ),
        ],
    );
}
