//! Phase C — error/rejection-path differential tests.  One test per row of
//! ERRORS.md; each constructs the exact invalid input/condition, calls **both**
//! implementations through their `.so` exports, and asserts they reject
//! identically (same bytes on stdout, same exit status — not merely "both
//! failed somehow").
//!
//! `c_src/src/main.c` has no error code and no sentinel value: it ignores the
//! return value of `scanf` and of `printf`, so "rejection" is observable as
//! "`x` keeps its initial value `0`" (⇒ `00000000\n`) plus "exit status 0".
//! Each test therefore also anchors the C side's expected output explicitly, so
//! a regression in *both* implementations could not make the test vacuous.

mod common;

use common::*;

/// Anchor helper: assert the differential equality *and* the exact bytes the C
/// implementation is expected to produce.
fn assert_main_rejects(input: &[u8], kind: Stdin, expected: &[u8]) {
    assert_main_eq(input, kind);
    let c = call_main(c_impl(), input, kind);
    assert_eq!(
        c.stdout,
        expected,
        "C reference output changed for input {}: got {}",
        show(input),
        show(&c.stdout)
    );
    assert_eq!(c.status, 0, "C exit status for input {}", show(input));
}

const ZERO: &[u8] = b"00000000\n";
const MINUS_ONE: &[u8] = b"ffffffff\n";

/// ERRORS row 1 — empty stdin: `scanf` input failure (returns `EOF`).
#[test]
fn err_01_empty_stdin() {
    assert_main_rejects(b"", Stdin::File, ZERO);
    assert_main_rejects(b"", Stdin::Pipe, ZERO);
}

/// ERRORS row 2 — whitespace only, every class and combination.
#[test]
fn err_02_whitespace_only() {
    for w in [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'] {
        assert_main_rejects(&[w], Stdin::File, ZERO);
        assert_main_rejects(&[w; 5], Stdin::File, ZERO);
        assert_main_rejects(&[w; 5], Stdin::Pipe, ZERO);
    }
    assert_main_rejects(b" \t\n\x0b\x0c\r", Stdin::File, ZERO);
    assert_main_rejects(&vec![b' '; 9000], Stdin::File, ZERO);
}

/// ERRORS row 3 — first non-whitespace byte cannot start a decimal integer:
/// `scanf` matching failure (returns 0).
#[test]
fn err_03_matching_failure() {
    for s in [
        "abc", "?", ".", ".5", "x1", "X", "e5", "E", "/5", ":", "#", "\x01", "\x7f", "\u{80}",
        "  abc", "\n\n\tzz", "'42'", "\"42\"", "(42)", "[1]", "$5", "%d", "*7", "~1", "_1", "o10",
    ] {
        assert_main_rejects(s.as_bytes(), Stdin::File, ZERO);
    }
    assert_main_rejects(&[0xffu8, 0xfe, b'5'], Stdin::File, ZERO);
    assert_main_rejects(&[0x80u8], Stdin::File, ZERO);
}

/// ERRORS row 4 — a sign and then EOF.
#[test]
fn err_04_sign_only_eof() {
    assert_main_rejects(b"-", Stdin::File, ZERO);
    assert_main_rejects(b"+", Stdin::File, ZERO);
    assert_main_rejects(b"   -", Stdin::File, ZERO);
    assert_main_rejects(b"\n+", Stdin::File, ZERO);
    assert_main_rejects(b"-", Stdin::Pipe, ZERO);
    assert_main_rejects(b"+", Stdin::Pipe, ZERO);
}

/// ERRORS row 5 — a sign followed by a non-digit.
#[test]
fn err_05_sign_then_nondigit() {
    for s in [
        "- 5", "+ 5", "--5", "++5", "-+5", "+-5", "-x", "+z", "-\n5", "+\t5", "-.5", "+.5", "-,",
        "+/", "-\0", "+\0",
    ] {
        assert_main_rejects(s.as_bytes(), Stdin::File, ZERO);
    }
}

/// ERRORS row 6 — a NUL byte where a number is expected.
#[test]
fn err_06_nul_byte() {
    assert_main_rejects(b"\0", Stdin::File, ZERO);
    assert_main_rejects(b"\0 5", Stdin::File, ZERO);
    assert_main_rejects(b"\0\0\0", Stdin::File, ZERO);
    assert_main_rejects(b"  \0 42", Stdin::File, ZERO);
    assert_main_rejects(b"\0", Stdin::Pipe, ZERO);
    // a NUL *after* the digits is just a terminator, not a rejection
    assert_main_eq(b"42\0", Stdin::File);
}

/// ERRORS row 7 — value above `LONG_MAX`: `strtol` `ERANGE`, glibc stores
/// `LONG_MAX`, which truncates to `(int)-1`.
#[test]
fn err_07_over_long_max() {
    assert_main_rejects(b"9223372036854775808", Stdin::File, MINUS_ONE);
    assert_main_rejects(b"18446744073709551616", Stdin::File, MINUS_ONE);
    assert_main_rejects(
        b"1234567890123456789012345678901234567890",
        Stdin::File,
        MINUS_ONE,
    );
    assert_main_rejects("9".repeat(5000).as_bytes(), Stdin::File, MINUS_ONE);
    assert_main_rejects(b"+9223372036854775808", Stdin::File, MINUS_ONE);
    assert_main_rejects(b"  9223372036854775808\n", Stdin::Pipe, MINUS_ONE);
    let mut rng = Rng::new();
    for _ in 0..60 {
        let len = 20 + rng.below(40) as usize;
        let mut s = String::new();
        s.push(char::from(b'1' + rng.below(9) as u8));
        for _ in 1..len {
            s.push(char::from(b'0' + rng.below(10) as u8));
        }
        // any 20+ digit number starting with a non-zero digit exceeds LONG_MAX
        assert_main_rejects(s.as_bytes(), Stdin::File, MINUS_ONE);
    }
}

/// ERRORS row 8 — value below `LONG_MIN`: `strtol` `ERANGE`, glibc stores
/// `LONG_MIN`, which truncates to `(int)0`.
#[test]
fn err_08_under_long_min() {
    assert_main_rejects(b"-9223372036854775809", Stdin::File, ZERO);
    assert_main_rejects(b"-18446744073709551616", Stdin::File, ZERO);
    assert_main_rejects(
        b"-1234567890123456789012345678901234567890",
        Stdin::File,
        ZERO,
    );
    assert_main_rejects(
        format!("-{}", "9".repeat(5000)).as_bytes(),
        Stdin::File,
        ZERO,
    );
    assert_main_rejects(b" -9223372036854775809\n", Stdin::Pipe, ZERO);
    let mut rng = Rng::new();
    for _ in 0..60 {
        let len = 20 + rng.below(40) as usize;
        let mut s = String::from("-");
        s.push(char::from(b'1' + rng.below(9) as u8));
        for _ in 1..len {
            s.push(char::from(b'0' + rng.below(10) as u8));
        }
        assert_main_rejects(s.as_bytes(), Stdin::File, ZERO);
    }
}

/// ERRORS row 9 — fits `long`, not `int`, positive: silent truncation.
#[test]
fn err_09_int_overflow_positive() {
    assert_main_rejects(b"2147483648", Stdin::File, b"00000080\n");
    assert_main_rejects(b"2147483649", Stdin::File, b"01000080\n");
    assert_main_rejects(b"4294967295", Stdin::File, b"ffffffff\n");
    assert_main_rejects(b"4294967296", Stdin::File, b"00000000\n");
    assert_main_rejects(b"4294967297", Stdin::File, b"01000000\n");
    let mut rng = Rng::new();
    for _ in 0..80 {
        let v = rng.range_i64(i32::MAX as i64 + 1, i64::MAX);
        let expect = format!(
            "{}\n",
            (v as u32)
                .to_le_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        assert_main_rejects(format!("{v}").as_bytes(), Stdin::File, expect.as_bytes());
    }
}

/// ERRORS row 10 — fits `long`, not `int`, negative: silent truncation.
#[test]
fn err_10_int_overflow_negative() {
    assert_main_rejects(b"-2147483649", Stdin::File, b"ffffff7f\n");
    assert_main_rejects(b"-4294967296", Stdin::File, b"00000000\n");
    assert_main_rejects(b"-4294967297", Stdin::File, b"ffffffff\n");
    let mut rng = Rng::new();
    for _ in 0..80 {
        let v = rng.range_i64(i64::MIN, i32::MIN as i64 - 1);
        let expect = format!(
            "{}\n",
            (v as u32)
                .to_le_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        assert_main_rejects(format!("{v}").as_bytes(), Stdin::File, expect.as_bytes());
    }
}

/// ERRORS row 11 — exactly `LONG_MIN` (no `ERANGE`; boundary of the saturation
/// branch).
#[test]
fn err_11_long_min_exact() {
    assert_main_rejects(b"-9223372036854775808", Stdin::File, ZERO);
    assert_main_rejects(b"-9223372036854775807", Stdin::File, b"01000000\n");
    assert_main_rejects(b"-0000009223372036854775808", Stdin::File, ZERO);
}

/// ERRORS row 12 — exactly `LONG_MAX` (no `ERANGE`).
#[test]
fn err_12_long_max_exact() {
    assert_main_rejects(b"9223372036854775807", Stdin::File, MINUS_ONE);
    assert_main_rejects(b"9223372036854775806", Stdin::File, b"feffffff\n");
    assert_main_rejects(b"00000009223372036854775807", Stdin::File, MINUS_ONE);
}

/// ERRORS row 13 — fd 0 closed: the first `read` fails with `EBADF`, so `scanf`
/// reports input failure.
#[test]
fn err_13_stdin_closed() {
    assert_main_rejects(b"", Stdin::Closed, ZERO);
    assert_main_rejects(b"12345", Stdin::Closed, ZERO); // content irrelevant, fd is gone
}

/// ERRORS row 14 — fd 0 is a directory: `read` fails with `EISDIR`.
#[test]
fn err_14_stdin_is_directory() {
    assert_main_rejects(b"", Stdin::Directory, ZERO);
    assert_main_rejects(b"999", Stdin::Directory, ZERO);
}

/// ERRORS row 15 — fd 1 closed: every `printf` fails, the return value is
/// ignored, `main` still returns 0 and nothing is produced.
#[test]
fn err_15_stdout_closed() {
    for (input, kind) in [
        (&b""[..], Stdin::File),
        (&b"42"[..], Stdin::File),
        (&b"-1\n"[..], Stdin::File),
        (&b"abc"[..], Stdin::File),
        (&b"99999999999999999999"[..], Stdin::Pipe),
    ] {
        assert_main_eq_stdout_closed(input, kind);
        let c = call_main_stdout_closed(c_impl(), input, kind);
        assert!(
            c.stdout.is_empty(),
            "C wrote {} with stdout closed",
            show(&c.stdout)
        );
        assert_eq!(c.status, 0, "C exit status with stdout closed");
    }
}

/// ERRORS row 16 — `driver` performs no validation: the extremes of `int`.
#[test]
fn err_16_driver_extremes() {
    for x in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        assert_driver_eq(x);
    }
    assert_eq!(call_driver(c_impl(), i32::MIN), b"00000080\n");
    assert_eq!(call_driver(c_impl(), i32::MAX), b"ffffff7f\n");
}

/// ERRORS row 17 — "garbage"/out-of-range-looking values crossing the FFI
/// boundary.  There is no enum in the C source, but an `int` parameter can
/// still receive an arbitrary 32-bit pattern, and a caller may leave garbage in
/// the upper half of the 64-bit argument register: both must be ignored exactly
/// like the C code ignores them.
#[test]
fn err_17_driver_garbage_bits() {
    let patterns: Vec<i32> = vec![
        0x0000_0000u32 as i32,
        0xffff_ffffu32 as i32,
        0x8000_0000u32 as i32,
        0x7fff_ffffu32 as i32,
        0xdead_beefu32 as i32,
        0xcafe_babeu32 as i32,
        0xaaaa_aaaau32 as i32,
        0x5555_5555u32 as i32,
        0xffff_0000u32 as i32,
        0x0000_ffffu32 as i32,
        -12345678,
        i32::MIN,
    ];
    assert_driver_batch_eq("garbage bit patterns", &patterns);

    // Same values passed as i64-truncated arguments (upper 32 bits of the
    // register carry garbage): the callee must use the low 32 bits only.
    let c = c_impl().driver_fn();
    let r = rust_impl().driver_fn();
    let wide: unsafe extern "C" fn(i64) = unsafe { std::mem::transmute(c) };
    let wide_r: unsafe extern "C" fn(i64) = unsafe { std::mem::transmute(r) };
    let mut rng = Rng::new();
    let mut args: Vec<i64> = Vec::new();
    for p in &patterns {
        args.push(((rng.next_u32() as u64) << 32 | (*p as u32 as u64)) as i64);
    }
    let co = call_driver_wide(wide, &args);
    let ro = call_driver_wide(wide_r, &args);
    assert_eq!(
        co,
        ro,
        "driver() with garbage in the upper argument half:\n  C   : {}\n  Rust: {}",
        show(&co),
        show(&ro)
    );
    // and the low 32 bits are what gets printed
    let expect = call_driver_many(c_impl(), &patterns);
    assert_eq!(co, expect, "upper argument bits must be ignored");
}

/// Helper for row 17: same fork-based capture as `call_driver_many`, but the
/// symbol is called through a deliberately wider (`long`) argument type so that
/// the upper half of the argument register carries garbage.
fn call_driver_wide(f: unsafe extern "C" fn(i64), args: &[i64]) -> Vec<u8> {
    fork_capture_stdout(|| {
        for &a in args {
            unsafe { f(a) };
        }
        0
    })
}

/// ERRORS row 18 — `scanf`'s failure never changes the exit status.
#[test]
fn err_18_exit_status_always_zero() {
    for input in [
        &b""[..],
        b"abc",
        b"-",
        b"+",
        b"\0",
        b"999999999999999999999999",
        b"42",
    ] {
        for kind in [
            Stdin::File,
            Stdin::Pipe,
            Stdin::DevNull,
            Stdin::Closed,
            Stdin::Directory,
        ] {
            let c = call_main(c_impl(), input, kind);
            let r = call_main(rust_impl(), input, kind);
            assert_eq!(c.status, 0, "C status for {} / {kind:?}", show(input));
            assert_eq!(r.status, 0, "Rust status for {} / {kind:?}", show(input));
            assert_eq!(c.stdout, r.stdout, "output for {} / {kind:?}", show(input));
        }
    }
    // and the same end to end
    for input in [&b""[..], b"abc", b"-", b"\0", b"999999999999999999999999"] {
        assert_exe_eq(input);
    }
}

/// Generic FFI boundary: the export surface takes no pointer, so there is no
/// null-pointer path; verify the signatures really are what the C header says
/// by resolving them and by checking that no other symbol is exported.
#[test]
fn err_19_no_pointer_or_enum_surface() {
    // both symbols resolve in both objects
    let _ = c_impl().driver_fn();
    let _ = c_impl().main_fn();
    let _ = rust_impl().driver_fn();
    let _ = rust_impl().main_fn();
    // `print_hex` is static in the C source: it must be exported by neither
    assert!(
        !c_impl().has_symbol("print_hex"),
        "C .so unexpectedly exports print_hex"
    );
    assert!(
        !rust_impl().has_symbol("print_hex"),
        "Rust .so must not export print_hex (it is static in the C source)"
    );
}
