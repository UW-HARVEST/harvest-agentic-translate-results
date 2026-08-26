//! Phase B — valid-path differential tests for the composed pipeline
//! (`scanf("%d", &x)` → `driver(x)` → `print_hex`) reached through the
//! exported `main`, plus the same pipeline through the linked executables.
//!
//! Rows C10–C24 of CONFIGS.md. Each case is run in both shared libraries via
//! `dlopen`/`dlsym` (one fresh process per input, because each library buffers
//! stdin internally) and stdout *and* the return value are compared.

mod common;

use common::*;

const WHITESPACE: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];

/// C10 — plain decimal, no whitespace, no sign, terminated by EOF.
#[test]
fn c10_plain_decimal() {
    for s in [
        "0", "1", "2", "9", "10", "42", "99", "100", "12345", "65535", "1000000", "2147483647",
        "1234567890",
    ] {
        assert_main_same(s.as_bytes(), &format!("C10 {s:?}"));
    }
}

/// C11 — every single whitespace character accepted by `isspace` as a prefix.
#[test]
fn c11_each_whitespace_prefix() {
    for ws in WHITESPACE {
        for n in ["7", "0", "123", "-8", "+9", "2147483647"] {
            let mut input = vec![ws];
            input.extend_from_slice(n.as_bytes());
            assert_main_same(&input, &format!("C11 ws={ws:#04x} n={n}"));
            // Repeated occurrences of the same whitespace character.
            let mut input = vec![ws; 5];
            input.extend_from_slice(n.as_bytes());
            assert_main_same(&input, &format!("C11 ws*5={ws:#04x} n={n}"));
        }
    }
}

/// C12 — a long mixed whitespace run before the number.
#[test]
fn c12_mixed_whitespace_run() {
    let mut input = Vec::new();
    for _ in 0..8 {
        input.extend_from_slice(&WHITESPACE);
    }
    input.extend_from_slice(b"31337");
    assert_main_same(&input, "C12 mixed run");

    assert_main_same(b" \t\n\x0b\x0c\r-42", "C12 mixed then negative");
    assert_main_same(b"\n\n\n\n0", "C12 newlines then zero");
    assert_main_same(b"\r\n+5", "C12 CRLF then signed");
}

/// C13 — explicit signs with in-range magnitudes.
#[test]
fn c13_explicit_signs() {
    for s in [
        "+0",
        "-0",
        "+1",
        "-1",
        "+42",
        "-42",
        "+2147483647",
        "-2147483648",
        "-2147483647",
        "+1000000000",
        "-1000000000",
    ] {
        assert_main_same(s.as_bytes(), &format!("C13 {s:?}"));
    }
}

/// C14 — leading zeros (base 10, so no octal interpretation).
#[test]
fn c14_leading_zeros() {
    for s in [
        "0",
        "00",
        "000000000",
        "0000000005",
        "0777",
        "-000012",
        "+000012",
        "0000002147483647",
    ] {
        assert_main_same(s.as_bytes(), &format!("C14 {s:?}"));
    }
    let mut long_zeros = "0".repeat(100);
    long_zeros.push('7');
    assert_main_same(long_zeros.as_bytes(), "C14 100 zeros then 7");
}

/// C15 — above `INT_MAX` but inside `long`: `strtol` succeeds and the value is
/// truncated by `*ptr = (int) num.l`.
#[test]
fn c15_above_int_max_truncates() {
    for s in [
        "2147483648",
        "2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "6442450944",
        "8589934592",
        "1099511627776",
        "9223372036854775806",
        "9223372036854775807", // exactly LONG_MAX, no ERANGE
    ] {
        assert_main_same(s.as_bytes(), &format!("C15 {s:?}"));
    }
}

/// C16 — above `LONG_MAX`: `strtol` clamps to `LONG_MAX`, truncated to `-1`.
#[test]
fn c16_above_long_max_saturates() {
    for s in [
        "9223372036854775808",
        "9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "99999999999999999999",
        "170141183460469231731687303715884105728",
    ] {
        assert_main_same(s.as_bytes(), &format!("C16 {s:?}"));
    }
}

/// C17 — below `INT_MIN` inside `long`, and below `LONG_MIN` (clamps to
/// `LONG_MIN`, truncated to `0`).
#[test]
fn c17_below_int_min_and_long_min() {
    for s in [
        "-2147483649",
        "-4294967295",
        "-4294967296",
        "-4294967297",
        "-9223372036854775807",
        "-9223372036854775808", // exactly LONG_MIN, no ERANGE
        "-9223372036854775809",
        "-99999999999999999999",
        "-18446744073709551616",
    ] {
        assert_main_same(s.as_bytes(), &format!("C17 {s:?}"));
    }
}

/// C18 — what terminates the conversion.
#[test]
fn c18_terminator_variants() {
    for s in [
        "5",       // EOF
        "5 ",      // space
        "5\n",     // newline
        "5\t",     // tab
        "5\r\n",   // CRLF
        "5a",      // letter
        "5abc",    // letters
        "5.5",     // decimal point (not part of %d)
        "5,6",     // comma (no grouping flag)
        "5-6",     // sign in the middle
        "5+6",     //
        "12 34",   // a second number
        "12\n34",  //
        "7 8 9",   //
        "5\0rest", // NUL
        "42)",     //
        "42_",     //
    ] {
        assert_main_same(s.as_bytes(), &format!("C18 {s:?}"));
    }
}

/// C19 — very long digit strings.
#[test]
fn c19_very_long_digit_strings() {
    let mut rng = Rng::new(0xC19);

    let long_digits: String = (0..4096)
        .map(|i| {
            if i == 0 {
                // avoid a leading zero for this case
                (b'1' + (rng.below(9) as u8)) as char
            } else {
                (b'0' + (rng.below(10) as u8)) as char
            }
        })
        .collect();
    assert_main_same(long_digits.as_bytes(), "C19 4096 digits");

    let mut neg = String::from("-");
    neg.push_str(&long_digits);
    assert_main_same(neg.as_bytes(), "C19 4096 digits negative");

    let mut zeros = "0".repeat(4096);
    zeros.push_str("123");
    assert_main_same(zeros.as_bytes(), "C19 4096 zeros then 123");

    // Long run of zeros followed by an overflowing tail.
    let mut mixed = "0".repeat(1000);
    mixed.push_str("99999999999999999999");
    assert_main_same(mixed.as_bytes(), "C19 zeros then overflow");

    // Digit string exactly at the LONG_MAX boundary, padded with zeros.
    let mut padded = "0".repeat(300);
    padded.push_str("9223372036854775807");
    assert_main_same(padded.as_bytes(), "C19 padded LONG_MAX");
}

/// Random decimal string spanning every magnitude class.
fn random_decimal(rng: &mut Rng) -> String {
    let sign = match rng.below(3) {
        0 => "",
        1 => "+",
        _ => "-",
    };
    let digits = match rng.below(6) {
        // fits in i32
        0 => format!("{}", rng.next_u32() % 2_147_483_648),
        // full 32-bit range
        1 => format!("{}", rng.next_u32()),
        // beyond i32, inside i64
        2 => format!("{}", rng.next_u64() >> 1),
        // full 64-bit range (often beyond LONG_MAX)
        3 => format!("{}", rng.next_u64()),
        // beyond u64
        4 => format!("{}{}", rng.next_u64(), rng.next_u64()),
        // small
        _ => format!("{}", rng.below(1000)),
    };
    let zeros = "0".repeat(rng.below(4) as usize);
    format!("{sign}{zeros}{digits}")
}

/// C20 — randomized decimal magnitudes (fixed seed).
#[test]
fn c20_random_decimal_magnitudes() {
    let mut rng = Rng::new(0xC20);
    for i in 0..512 {
        let s = random_decimal(&mut rng);
        assert_main_same(s.as_bytes(), &format!("C20 iteration {i}: {s:?}"));
    }
}

/// Random input exercising the cross-product of axes E (whitespace) ×
/// F (sign) × G (digit shape) × H (magnitude) × I (terminator).
fn random_input(rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::new();
    for _ in 0..rng.below(6) {
        v.push(*rng.pick(&WHITESPACE));
    }
    v.extend_from_slice(random_decimal(rng).as_bytes());
    let terminators: [&[u8]; 16] = [
        b"",
        b" ",
        b"\n",
        b"\t",
        b"\r\n",
        b"abc",
        b"x",
        b".5",
        b"-5",
        b"+5",
        b" 7",
        b"\n123",
        b",",
        b"\0",
        b"\xff",
        b")",
    ];
    let terminator: &[u8] = *rng.pick(&terminators);
    v.extend_from_slice(terminator);
    v
}

/// C21 — randomized full cross-product of the input axes (fixed seed).
#[test]
fn c21_random_axis_crossproduct() {
    let mut rng = Rng::new(0xC21);
    for i in 0..512 {
        let input = random_input(&mut rng);
        assert_main_same(&input, &format!("C21 iteration {i}"));
    }
}

/// C22 — the same corpus driven through the linked executables
/// (`add_executable(driver src/main.c)` vs `[[bin]] driver`), comparing stdout
/// and the process exit status.
#[test]
fn c22_executables_end_to_end() {
    let mut rng = Rng::new(0xC22);
    for i in 0..256 {
        let input = random_input(&mut rng);
        let run = assert_exe_same(&input, &format!("C22 iteration {i}"));
        assert_eq!(run.status, 0, "C22: C program exit status must be 0");
        assert_eq!(run.stdout.len(), 33, "C22: unexpected output length");
    }
    // Plus the deterministic cases from the other rows.
    for s in [
        "1",
        "-1",
        "0",
        "2147483647",
        "-2147483648",
        "99999999999999999999",
        "-99999999999999999999",
        "4294967296",
        "abc",
        "",
        "  +5",
        "-",
        "0777",
        "0x10",
    ] {
        assert_exe_same(s.as_bytes(), &format!("C22 fixed {s:?}"));
    }
}

/// C23 — trailing newline present or absent, stdin/stdout being pipes (so C
/// stdout is fully buffered and only drained at exit).
#[test]
fn c23_trailing_newline_and_pipe_buffering() {
    for base in ["7", "-7", "0", "2147483647", "99999999999999999999", "abc"] {
        let with_nl = format!("{base}\n");
        let a = assert_exe_same(base.as_bytes(), "C23 without newline");
        let b = assert_exe_same(with_nl.as_bytes(), "C23 with newline");
        assert_eq!(
            a, b,
            "C23: a trailing newline must not change the result for {base:?}"
        );

        // Same through the exported `main` in both .so's.
        let c = assert_main_same(base.as_bytes(), "C23 so without newline");
        let d = assert_main_same(with_nl.as_bytes(), "C23 so with newline");
        assert_eq!(c, d, "C23: .so results differ by trailing newline");
        assert_eq!(
            a.stdout, c.stdout,
            "C23: executable and .so disagree for {base:?}"
        );
    }
}

/// C24 — the FFI return value of the exported `main` is 0 for every input
/// class, in both libraries.
#[test]
fn c24_main_return_value() {
    let mut rng = Rng::new(0xC24);
    let mut inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"   ".to_vec(),
        b"abc".to_vec(),
        b"-".to_vec(),
        b"+".to_vec(),
        b"0".to_vec(),
        b"5".to_vec(),
        b"-5".to_vec(),
        b"99999999999999999999".to_vec(),
        b"\0".to_vec(),
        b"\xff\xfe".to_vec(),
    ];
    for _ in 0..64 {
        inputs.push(random_input(&mut rng));
    }
    for input in inputs {
        let c = run_main_via_so(c_lib(), &input);
        let r = run_main_via_so(rust_lib(), &input);
        assert_eq!(c.status, 0, "C main() must return 0 for {input:?}");
        assert_eq!(
            r.status, c.status,
            "C24: main() return value diverged for {input:?}"
        );
        assert_eq!(c.stdout, r.stdout, "C24: stdout diverged for {input:?}");
    }
}

/// C29 — stdin delivered in separate chunks, so a single conversion has to
/// span several `read` calls: the number must not be cut short at a chunk
/// boundary, and a conversion that already has digits must keep waiting for
/// more input rather than stopping at the boundary.
#[test]
fn c29_stdin_arrives_in_chunks() {
    // Digits split across two reads: the value is 1234, not 12.
    let run = assert_main_chunked_same(&[b"12", b"34 "], 120, "C29 split digits");
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "d2040000030000000000000000000040\n",
        "C29: the C library did not join the digits across reads (expected 1234)"
    );

    // Sign, digits and terminator each in their own chunk.
    let run = assert_main_chunked_same(&[b"-", b"7", b"6", b"x"], 80, "C29 split sign");
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "b4ffffff030000000000000000000040\n",
        "C29: expected -76"
    );

    // Whitespace, then the number much later.
    assert_main_chunked_same(&[b"   ", b"\n\t", b"42", b""], 80, "C29 late number");
    // A leading chunk that cannot match: the failure must not wait for more.
    assert_main_chunked_same(&[b"x", b"5"], 80, "C29 early mismatch");
    // Overflowing value split across chunks.
    assert_main_chunked_same(
        &[b"999999999", b"99999999999", b" "],
        80,
        "C29 split overflow",
    );
    // One byte at a time.
    assert_main_chunked_same(&[b"1", b"2", b"3", b"4", b"5", b"\n"], 40, "C29 byte by byte");
}
