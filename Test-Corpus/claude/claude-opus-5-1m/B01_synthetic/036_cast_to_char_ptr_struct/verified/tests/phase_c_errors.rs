//! Phase C — error-path differential tests.
//!
//! One test per row of ERRORS.md (E1–E13) plus the generic FFI-boundary rows
//! (B1–B6). Because the C `main` **discards** the `scanf` return value, the
//! observable consequence of every rejection is that `x` keeps its initializer
//! `0`; each test therefore asserts both that the two libraries agree *and*
//! that the C really produced the specific documented result (so a test cannot
//! pass by both sides failing in the same uninteresting way).

mod common;

use common::*;

/// The exact line `driver(x)` prints for a given `x`.
fn expected_for_x(x: i32) -> String {
    let mut s = String::new();
    for b in x.to_le_bytes() {
        s.push_str(&format!("{b:02x}"));
    }
    s.push_str("03000000");
    s.push_str("0000000000000040");
    s.push('\n');
    s
}

/// `x` was left untouched by a rejected conversion.
fn untouched() -> String {
    expected_for_x(0)
}

/// Runs one input through both `.so`s and asserts they agree *and* that the C
/// result is exactly `expected`.
#[track_caller]
fn check_input(input: &[u8], expected: &str, row: &str) {
    let run = assert_main_same(input, row);
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        expected,
        "{row}: C result for input {:?} is not the documented one",
        String::from_utf8_lossy(input)
    );
    assert_eq!(run.status, 0, "{row}: main() must return 0");
}

// ---------------------------------------------------------------------------
// E1 — empty input: input failure (scanf -> EOF), x untouched.
// ---------------------------------------------------------------------------
#[test]
fn e1_empty_input() {
    check_input(b"", &untouched(), "E1");
}

// ---------------------------------------------------------------------------
// E2 — whitespace only: EOF after the whitespace skip, x untouched.
// ---------------------------------------------------------------------------
#[test]
fn e2_whitespace_only() {
    for input in [
        &b" "[..],
        b"\t",
        b"\n",
        b"\x0b",
        b"\x0c",
        b"\r",
        b" \t\n\x0b\x0c\r",
        b"\n\n\n\n\n\n\n\n",
        b"                                        ",
    ] {
        check_input(input, &untouched(), "E2");
    }
    // A very long whitespace run (larger than glibc's stdio buffer).
    let long = vec![b' '; 100_000];
    check_input(&long, &untouched(), "E2 long run");
}

// ---------------------------------------------------------------------------
// E3 — first non-whitespace character is neither a digit nor a sign:
//      matching failure (scanf -> 0), x untouched.
// ---------------------------------------------------------------------------
#[test]
fn e3_leading_non_digit() {
    for input in [
        "abc", "x", ".", ",", "!", "?", "/", ":", "e", "E", "A", "z", "#", "*", "(", ")", "[", "-",
        "n", "N", "i", "I", "nan", "inf", "0x", "true", "null", "%d", "'", "\"", "\\", "|", "~",
        "`", "^", "&", "=", "<", ">", "{", "}", ";", "_", "$", "@",
    ] {
        // note: "-" alone is row E4, but it also belongs to this class.
        check_input(input.as_bytes(), &untouched(), "E3");
    }
    // Leading whitespace does not change the outcome.
    for input in ["   abc", "\n\n.", "\t\t\tx", " \r\n?"] {
        check_input(input.as_bytes(), &untouched(), "E3 with whitespace");
    }
}

// ---------------------------------------------------------------------------
// E4 — a lone sign then EOF: matching failure (scanf -> 0, *not* EOF),
//      x untouched.
// ---------------------------------------------------------------------------
#[test]
fn e4_sign_then_eof() {
    for input in ["-", "+", "   -", "\n+", " \t\n\x0b\x0c\r-"] {
        check_input(input.as_bytes(), &untouched(), "E4");
    }
}

// ---------------------------------------------------------------------------
// E5 — sign followed by a non-digit: matching failure, x untouched.
// ---------------------------------------------------------------------------
#[test]
fn e5_sign_then_non_digit() {
    for input in [
        "- 5", "-x", "+.", "--1", "++1", "+-1", "-+1", "- ", "+\n", "-\t5", "-.5", "+.5", "-,",
        "- 2147483647", "+abc", "-abc", "-\0", "- -",
    ] {
        check_input(input.as_bytes(), &untouched(), "E5");
    }
}

// ---------------------------------------------------------------------------
// E6 — `%d` is base 10: a hex/octal prefix is not accepted as such.
// ---------------------------------------------------------------------------
#[test]
fn e6_hex_prefix_rejected() {
    // "0x10": the '0' converts, 'x' stops the conversion => x == 0.
    for input in ["0x10", "0X1f", "0xff", "0b1010", "0o17"] {
        check_input(input.as_bytes(), &expected_for_x(0), "E6");
    }
    // Octal-looking input is read as decimal.
    check_input(b"0777", &expected_for_x(777), "E6 octal is decimal");
    check_input(b"010", &expected_for_x(10), "E6 octal is decimal");
    // A hex digit that is not a decimal digit terminates the number.
    check_input(b"9abcdef", &expected_for_x(9), "E6 hex letters stop");
    check_input(b"1e5", &expected_for_x(1), "E6 no exponent for %d");
}

// ---------------------------------------------------------------------------
// E7 — above INT_MAX but inside long: strtol succeeds, (int) truncates.
// ---------------------------------------------------------------------------
#[test]
fn e7_int_overflow_truncates() {
    let cases: [(&str, i32); 8] = [
        ("2147483648", i32::MIN),
        ("2147483649", i32::MIN + 1),
        ("4294967295", -1),
        ("4294967296", 0),
        ("4294967297", 1),
        ("8589934592", 0),
        ("9223372036854775806", -2),
        ("9223372036854775807", -1), // exactly LONG_MAX, no ERANGE
    ];
    for (input, expect) in cases {
        check_input(input.as_bytes(), &expected_for_x(expect), "E7");
    }
}

// ---------------------------------------------------------------------------
// E8 — above LONG_MAX: ERANGE, strtol clamps to LONG_MAX, (int) => -1.
// ---------------------------------------------------------------------------
#[test]
fn e8_long_overflow_saturates_high() {
    for input in [
        "9223372036854775808",
        "9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "99999999999999999999",
        "+99999999999999999999",
        "340282366920938463463374607431768211456",
    ] {
        check_input(input.as_bytes(), &expected_for_x(-1), "E8");
    }
    // A 4000-digit number also clamps to LONG_MAX.
    let huge = "9".repeat(4000);
    check_input(huge.as_bytes(), &expected_for_x(-1), "E8 4000 digits");
    // And a 100000-digit one, well past any internal buffer size.
    let enormous = "7".repeat(100_000);
    check_input(enormous.as_bytes(), &expected_for_x(-1), "E8 100000 digits");
}

// ---------------------------------------------------------------------------
// E9 — below LONG_MIN: ERANGE, clamps to LONG_MIN, (int) => 0.
// ---------------------------------------------------------------------------
#[test]
fn e9_long_overflow_saturates_low() {
    for input in [
        "-9223372036854775809",
        "-9223372036854775810",
        "-18446744073709551616",
        "-99999999999999999999",
        "-340282366920938463463374607431768211456",
    ] {
        check_input(input.as_bytes(), &expected_for_x(0), "E9");
    }
    let huge = format!("-{}", "9".repeat(4000));
    check_input(huge.as_bytes(), &expected_for_x(0), "E9 4000 digits");
    // Exactly LONG_MIN: no ERANGE, still truncates to 0.
    check_input(
        b"-9223372036854775808",
        &expected_for_x(0),
        "E9 exactly LONG_MIN",
    );
}

// ---------------------------------------------------------------------------
// E10 — the conversion stops at the first non-digit; the rest is never read.
// ---------------------------------------------------------------------------
#[test]
fn e10_trailing_garbage() {
    let cases: [(&str, i32); 12] = [
        ("5abc", 5),
        ("12 34", 12),
        ("7-9", 7),
        ("42)", 42),
        ("42_", 42),
        ("1.5", 1),
        ("2,000", 2),
        ("99+1", 99),
        ("-13xyz", -13),
        ("+13xyz", 13),
        ("0zzz", 0),
        ("2147483647!!!", i32::MAX),
    ];
    for (input, expect) in cases {
        check_input(input.as_bytes(), &expected_for_x(expect), "E10");
    }
    // Only the first token is consumed even with a lot of trailing data.
    let mut input = b"8".to_vec();
    input.extend_from_slice(&vec![b'z'; 100_000]);
    check_input(&input, &expected_for_x(8), "E10 huge trailing garbage");
}

// ---------------------------------------------------------------------------
// E11 — NUL bytes and non-UTF-8 bytes are neither whitespace nor digits.
// ---------------------------------------------------------------------------
#[test]
fn e11_nul_and_non_utf8_bytes() {
    let cases: [(&[u8], i32); 12] = [
        (b"\0", 0),
        (b"\0\0\0", 0),
        (b"\xff\xfe", 0),
        (b"\x80", 0),
        (b"\xc3", 0),
        (b"\xc3\xa9", 0),
        (b"4\x002", 4),
        (b"5\xff9", 5),
        (b"\x01\x02", 0),
        (b"\x7f", 0),
        (b"-\xff", 0),
        (b" \0 5", 0),
    ];
    for (input, expect) in cases {
        check_input(input, &expected_for_x(expect), "E11");
    }
    // A random byte soup must never crash either implementation and must agree.
    let mut rng = Rng::new(0xE11);
    for i in 0..256 {
        let len = rng.below(24) as usize;
        let input: Vec<u8> = (0..len).map(|_| (rng.next_u32() & 0xff) as u8).collect();
        let run = assert_main_same(&input, &format!("E11 random bytes {i}"));
        assert_eq!(run.status, 0, "E11: main() must return 0");
    }
}

// ---------------------------------------------------------------------------
// E12 — `print_hex`'s `len <= 0` guard is unreachable: the function is
//       `static` in C and therefore exported by neither library.
// ---------------------------------------------------------------------------
#[test]
fn e12_print_hex_not_exported() {
    assert!(
        !c_impl().exports_print_hex(),
        "the C .so unexpectedly exports print_hex"
    );
    assert!(
        !rust_impl().exports_print_hex(),
        "the Rust .so exports print_hex although the C one does not"
    );
}

// ---------------------------------------------------------------------------
// E13 — `driver` rejects nothing: every int is accepted and produces output.
// ---------------------------------------------------------------------------
#[test]
fn e13_driver_accepts_every_int() {
    let mut rng = Rng::new(0xE13);
    let mut values: Vec<i64> = vec![
        i32::MIN as i64,
        i32::MIN as i64 + 1,
        -1,
        0,
        1,
        i32::MAX as i64 - 1,
        i32::MAX as i64,
    ];
    for _ in 0..512 {
        values.push(rng.next_i32() as i64);
    }
    let out = assert_driver_batch_same(&values, false, "E13");
    for (i, line) in out.iter().enumerate() {
        let v = values[i];
        assert_eq!(
            line.len(),
            33,
            "driver({v}) must always print 32 hex digits + \\n"
        );
        assert!(
            line[..32].iter().all(|b| b.is_ascii_hexdigit()),
            "driver({v}) printed a non-hex byte"
        );
    }
}

// ---------------------------------------------------------------------------
// B1/B2 — no pointer and no length parameters exist in the exported API,
//         so null/zero/oversized-length arguments are not constructible.
// ---------------------------------------------------------------------------
#[test]
fn b1_no_pointer_parameters() {
    // The exported surface is exactly `void driver(int)` and `int main(void)`.
    let syms = exported_symbols(c_lib());
    assert_eq!(
        syms,
        vec!["driver".to_string(), "main".to_string()],
        "the C .so's exported surface changed; re-derive ERRORS.md rows B1/B2"
    );
    // The only pointer/length-taking function is static and unexported.
    assert!(!c_impl().exports_print_hex());
    assert!(!rust_impl().exports_print_hex());
}

// ---------------------------------------------------------------------------
// B3 — no enum-typed parameter exists; `driver`'s `int` accepts every bit
//      pattern, so there is no "invalid variant" to reject. Verified by
//      sweeping values that would be out-of-range for any enum.
// ---------------------------------------------------------------------------
#[test]
fn b3_no_enum_parameters() {
    assert!(
        !std::fs::read_to_string(manifest_dir().join("c_src/src/main.c"))
            .expect("read C source")
            .contains("enum"),
        "the C source now declares an enum; ERRORS.md row B3 must be re-derived"
    );
    // Values that no sane enum would define, passed across the FFI boundary.
    assert_driver_batch_same(
        &[-1, -2, 255, 256, 999, 0x7fff_ffff, -0x8000_0000, 0x1000_0000],
        false,
        "B3 out-of-range 'enum' values",
    );
}

// ---------------------------------------------------------------------------
// B4 — one step past every interesting int boundary.
// ---------------------------------------------------------------------------
#[test]
fn b4_int_extremes() {
    let mut values = Vec::new();
    for anchor in [
        i32::MIN as i64,
        i16::MIN as i64,
        i8::MIN as i64,
        -1,
        0,
        1,
        i8::MAX as i64,
        u8::MAX as i64,
        i16::MAX as i64,
        u16::MAX as i64,
        i32::MAX as i64,
        0x1_0000,
        0x100,
    ] {
        for delta in [-2i64, -1, 0, 1, 2] {
            let v = anchor.wrapping_add(delta);
            values.push(v as i32); // wraps exactly like the C conversion
        }
    }
    let values: Vec<i64> = values.into_iter().map(|v| v as i64).collect();
    assert_driver_batch_same(&values, false, "B4");
}

// ---------------------------------------------------------------------------
// B5 — `driver` takes a 32-bit `int`: garbage in the upper half of the
//      argument register must be ignored identically by both libraries.
// ---------------------------------------------------------------------------
#[test]
fn b5_upper_half_of_arg_register_ignored() {
    let mut rng = Rng::new(0xB5);
    let mut wides: Vec<i64> = vec![
        0x0000_0001_0000_0000u64 as i64,
        0xffff_ffff_0000_0000u64 as i64,
        0xdead_beef_0000_002au64 as i64,
        0x7fff_ffff_ffff_ffffu64 as i64,
        (-1i64),
        i64::MIN,
    ];
    for _ in 0..128 {
        wides.push(rng.next_u64() as i64);
    }

    // Same symbol, called through `fn(i64)`: C and Rust must agree.
    let wide_out = assert_driver_batch_same(&wides, true, "B5 64-bit argument");

    // And both must behave exactly as if only the low 32 bits had been passed.
    let narrowed: Vec<i64> = wides.iter().map(|&w| w as i32 as i64).collect();
    let narrow_out = assert_driver_batch_same(&narrowed, false, "B5 truncated argument");
    for (i, w) in wides.iter().enumerate() {
        assert_eq!(
            wide_out[i], narrow_out[i],
            "the upper half of the argument register was not ignored for {w:#018x}"
        );
    }
}

// ---------------------------------------------------------------------------
// B6 — the exported `main` returns exactly 0 for every rejected input class.
// ---------------------------------------------------------------------------
#[test]
fn b6_main_returns_zero() {
    for input in [
        &b""[..],
        b" ",
        b"\n",
        b"abc",
        b"-",
        b"+",
        b"- 5",
        b"0x10",
        b"\0",
        b"\xff",
        b"99999999999999999999",
        b"-99999999999999999999",
        b"5abc",
    ] {
        let c = run_main_via_so(c_lib(), input);
        let r = run_main_via_so(rust_lib(), input);
        assert_eq!(c.status, 0, "C main() returned {} for {input:?}", c.status);
        assert_eq!(
            r.status, c.status,
            "return value diverged for {input:?}: C={} Rust={}",
            c.status, r.status
        );
        assert_eq!(c.stdout, r.stdout, "stdout diverged for {input:?}");
    }
}
