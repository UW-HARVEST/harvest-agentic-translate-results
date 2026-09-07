//! Phase C — error-path differential tests, one test per row of ERRORS.md.
//!
//! The library's only rejection result is the exact byte string
//! `"An error occurred\n"` on stdout with no state mutation, so "same error" is
//! asserted as *that exact sentinel*, not merely "both failed somehow".

mod harness;

use harness::*;
use std::ffi::c_int;

const ERR: &str = "An error occurred\n";
const SEED: u64 = 0xBADC0DE_5EED;

/// Assert both libraries reject, with the identical sentinel output.
#[track_caller]
fn assert_rejected(label: &str, op: Op) {
    let out = assert_same_one(label, op);
    assert_eq!(
        String::from_utf8_lossy(&out),
        ERR,
        "[{label}] expected the rejection sentinel, got other output"
    );
}

/// Assert both libraries accept: 2 `run`s => 8 report lines, and no sentinel.
/// Returns the raw captured bytes.
#[track_caller]
fn assert_accepted(label: &str, op: Op) -> Vec<u8> {
    let out = assert_same_one(label, op);
    let text = String::from_utf8_lossy(&out);
    assert!(
        !text.contains("An error occurred"),
        "[{label}] unexpectedly rejected: {text:?}"
    );
    assert_eq!(
        text.lines().count(),
        8,
        "[{label}] expected 8 report lines (driver runs run() twice), got {text:?}"
    );
    drop(text);
    out
}

/// `driver(s)` must parse to `parsed`: it calls `run()` twice, so `bedrooms`
/// must move by exactly `2 * parsed` (wrapping i32 arithmetic, as in C).
#[track_caller]
fn assert_parses_as(label: &str, s: &str, parsed: i32) {
    let out = assert_accepted(label, drv(s));
    let expected = parsed.wrapping_mul(2);
    assert_eq!(
        bedrooms_delta(&out),
        expected,
        "[{label}] driver({s:?}) should parse as {parsed} (bedrooms delta {expected})"
    );
}

// -------------------------------------------------------------------------
// E1 — empty string: strtol performs no conversion, endp == str
// -------------------------------------------------------------------------
#[test]
fn err_e1_empty_string() {
    assert_rejected("E1 driver(\"\")", drv(""));
}

// -------------------------------------------------------------------------
// E2 — all-whitespace
// -------------------------------------------------------------------------
#[test]
fn err_e2_whitespace_only() {
    for s in [
        " ", "  ", "\t", "\n", "\r", "\u{b}", "\u{c}", " \t\n\u{b}\u{c}\r ", "\t\t\t\t\t\t\t\t",
    ] {
        assert_rejected(&format!("E2 driver({s:?})"), drv(s));
    }
}

// -------------------------------------------------------------------------
// E3 — non-numeric leading character
// -------------------------------------------------------------------------
#[test]
fn err_e3_non_numeric_prefix() {
    for s in [
        "abc", "x1", "?", "one", "NaN", "nan", "inf", "INF", "infinity", "null", "#5", "_7",
        "\u{7f}", "\u{1}", "a2147483647", " abc 12",
    ] {
        assert_rejected(&format!("E3 driver({s:?})"), drv(s));
    }
    // Non-ASCII / high bytes must also be rejected identically.
    for b in [0x80u8, 0xC3, 0xFF, 0x01, 0x7F] {
        assert_rejected(
            &format!("E3 driver(byte {b:#04x})"),
            Op::Driver(vec![b, b'1', b'2']),
        );
    }
}

// -------------------------------------------------------------------------
// E4 — lone / doubled sign
// -------------------------------------------------------------------------
#[test]
fn err_e4_lone_or_double_sign() {
    for s in [
        "+", "-", "++", "--", "+-", "-+", "++1", "--1", "+-1", "-+1", " - 1", " + 1", "+ 1",
        "- 1", "+\t1", "-a", "+a", "   +", "   -",
    ] {
        assert_rejected(&format!("E4 driver({s:?})"), drv(s));
    }
}

// -------------------------------------------------------------------------
// E5 — non-decimal numeric forms (base is hard-coded to 10)
// -------------------------------------------------------------------------
#[test]
fn err_e5_non_decimal_forms() {
    // These never start a base-10 conversion -> rejected.
    for s in ["x1A", ".5", "-.5", "e5", "E5", "'1'", "\"1\"", "$5", "%5", "b101"] {
        assert_rejected(&format!("E5 reject driver({s:?})"), drv(s));
    }
    // ...but a leading '0' *does* convert (to 0), so these are accepted with 0.
    // base 10 stops at the first non-decimal digit, so "0x1A" -> 0 but "08" -> 8.
    for (s, parsed) in [
        ("0x1A", 0),
        ("0X1A", 0),
        ("0b101", 0),
        ("0b", 0),
        ("0o17", 0),
        ("08", 8),
        ("09", 9),
        ("019", 19),
    ] {
        assert_parses_as(&format!("E5 accept driver({s:?})"), s, parsed);
    }
}

// -------------------------------------------------------------------------
// E6 — ERANGE above LONG_MAX
// -------------------------------------------------------------------------
#[test]
fn err_e6_erange_above_long_max() {
    let mut cases: Vec<String> = vec![
        "9223372036854775808".into(),  // LONG_MAX + 1
        "9223372036854775809".into(),
        "99999999999999999999".into(),
        "+9223372036854775808".into(),
        format!("{}", "9".repeat(400)),
        format!(" \t+{}", "1".repeat(64)),
        format!("{}abc", "9".repeat(30)), // ERANGE even with trailing garbage
    ];
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..32 {
        let digits: String = (0..(20 + rng.below(40)))
            .map(|_| (b'0' + 1 + (rng.below(9) as u8)) as char)
            .collect();
        cases.push(digits);
    }
    for s in cases {
        assert_rejected(&format!("E6 driver({s:?})"), drv(&s));
    }
}

// -------------------------------------------------------------------------
// E7 — ERANGE below LONG_MIN
// -------------------------------------------------------------------------
#[test]
fn err_e7_erange_below_long_min() {
    let mut cases: Vec<String> = vec![
        "-9223372036854775809".into(), // LONG_MIN - 1
        "-9223372036854775810".into(),
        "-99999999999999999999".into(),
        format!("-{}", "9".repeat(400)),
        format!("  -{}", "7".repeat(25)),
    ];
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..32 {
        let digits: String = (0..(20 + rng.below(40)))
            .map(|_| (b'0' + 1 + (rng.below(9) as u8)) as char)
            .collect();
        cases.push(format!("-{digits}"));
    }
    for s in cases {
        assert_rejected(&format!("E7 driver({s:?})"), drv(&s));
    }
}

// -------------------------------------------------------------------------
// E8 — converts fine (errno == 0) but tmp > INT_MAX
// -------------------------------------------------------------------------
#[test]
fn err_e8_above_int_max() {
    let mut cases: Vec<String> = vec![
        "2147483648".into(), // INT_MAX + 1  <-- the off-by-one that matters
        "2147483649".into(),
        "+2147483648".into(),
        "4294967295".into(),
        "4294967296".into(),
        "9223372036854775806".into(),
        "9223372036854775807".into(), // LONG_MAX exactly: errno==0, still too big
        "2147483648abc".into(),
        " \t2147483648".into(),
        "0000000002147483648".into(),
    ];
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..48 {
        let v = 2_147_483_648i64 + (rng.next_u64() % (i64::MAX as u64 - 2_147_483_648)) as i64;
        cases.push(format!("{v}"));
    }
    for s in cases {
        assert_rejected(&format!("E8 driver({s:?})"), drv(&s));
    }
}

// -------------------------------------------------------------------------
// E9 — converts fine but tmp < INT_MIN
// -------------------------------------------------------------------------
#[test]
fn err_e9_below_int_min() {
    let mut cases: Vec<String> = vec![
        "-2147483649".into(), // INT_MIN - 1
        "-2147483650".into(),
        "-4294967296".into(),
        "-9223372036854775807".into(),
        "-9223372036854775808".into(), // LONG_MIN exactly
        "-2147483649xyz".into(),
        "  -2147483649".into(),
        "-0000000002147483649".into(),
    ];
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..48 {
        let mag = 2_147_483_649u64 + rng.next_u64() % (i64::MAX as u64 - 2_147_483_649);
        cases.push(format!("-{mag}"));
    }
    for s in cases {
        assert_rejected(&format!("E9 driver({s:?})"), drv(&s));
    }
}

// -------------------------------------------------------------------------
// E10 — one step *inside* the range must be accepted (no off-by-one)
// -------------------------------------------------------------------------
#[test]
fn err_e10_int_limits_accepted() {
    for (s, parsed) in [
        ("2147483647", i32::MAX),
        ("+2147483647", i32::MAX),
        ("-2147483648", i32::MIN),
        ("0000000002147483647", i32::MAX),
        (" \t-2147483648", i32::MIN),
        ("2147483647abc", i32::MAX),
        ("2147483646", 2147483646),
        ("-2147483647", -2147483647),
    ] {
        assert_parses_as(&format!("E10 driver({s:?})"), s, parsed);
    }
}

// -------------------------------------------------------------------------
// E11 — NULL pointer: no null check exists, so both must die identically.
// -------------------------------------------------------------------------
#[test]
fn err_e11_null_pointer_same_signal() {
    fn death(lib: &std::path::Path) -> (bool, c_int, c_int) {
        // (exited_normally, exit_code, term_signal)
        unsafe {
            libc::fflush(std::ptr::null_mut());
            let pid = libc::fork();
            assert!(pid >= 0, "fork failed");
            if pid == 0 {
                // child: silence stdio, then poke the library with NULL.
                let devnull = std::ffi::CString::new("/dev/null").unwrap();
                let fd = libc::open(devnull.as_ptr(), libc::O_WRONLY);
                if fd >= 0 {
                    libc::dup2(fd, 1);
                    libc::dup2(fd, 2);
                }
                let l = match libloading::Library::new(lib) {
                    Ok(l) => l,
                    Err(_) => libc::_exit(101),
                };
                let f: libloading::Symbol<unsafe extern "C" fn(*const std::ffi::c_char)> =
                    match l.get(b"driver\0") {
                        Ok(f) => f,
                        Err(_) => libc::_exit(102),
                    };
                f(std::ptr::null());
                // If it somehow survives, report that distinctly.
                libc::_exit(0);
            }
            let mut status: c_int = 0;
            libc::waitpid(pid, &mut status, 0);
            if libc::WIFEXITED(status) {
                (true, libc::WEXITSTATUS(status), 0)
            } else {
                (false, 0, libc::WTERMSIG(status))
            }
        }
    }

    let c = death(&c_lib());
    let r = death(&rust_lib());
    assert_eq!(
        c, r,
        "driver(NULL) must terminate identically: C={c:?} Rust={r:?}"
    );
    // Document what actually happens: glibc's strtol dereferences the NULL.
    assert!(
        !c.0 && c.2 == libc::SIGSEGV,
        "expected SIGSEGV from driver(NULL), observed {c:?}"
    );
}

// -------------------------------------------------------------------------
// E12 — raw zero-length buffer (first byte is NUL), not built via CString
// -------------------------------------------------------------------------
#[test]
fn err_e12_zero_length_raw_buffer() {
    // A 64-byte buffer whose first byte is NUL, rest is garbage: identical to E1
    // but reaches the library through a raw pointer.
    let mut buf = vec![0u8; 64];
    for (i, b) in buf.iter_mut().enumerate().skip(1) {
        *b = (b'A' as usize + (i % 26)) as u8;
    }
    let out = assert_same_one("E12 driver(raw \\0...)", Op::DriverRaw(buf));
    assert_eq!(String::from_utf8_lossy(&out), ERR);

    // And a buffer where the NUL truncates an otherwise-valid number.
    let mut buf2 = b"\0123456".to_vec();
    buf2.push(0);
    let out2 = assert_same_one("E12 driver(raw \\0123456)", Op::DriverRaw(buf2));
    assert_eq!(String::from_utf8_lossy(&out2), ERR);

    // NUL after a sign -> still a rejection (sign with no digit).
    let out3 = assert_same_one("E12 driver(raw -\\0)", Op::DriverRaw(b"-\0junk\0".to_vec()));
    assert_eq!(String::from_utf8_lossy(&out3), ERR);
}

// -------------------------------------------------------------------------
// E13 — `run` validates nothing: every 32-bit value, including "out of range
//       enum"-style values, must be accepted identically across the FFI edge.
// -------------------------------------------------------------------------
#[test]
fn err_e13_run_extreme_ints() {
    let fixed: [c_int; 14] = [
        0,
        1,
        -1,
        2,
        -2,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x7FFF_FFFF,
        -0x8000_0000,
        0x0000_00FF,
        0x0001_0000,
        -12345,
    ];
    for v in fixed {
        let out = assert_same_one(&format!("E13 run({v})"), run_op(v));
        assert!(
            !String::from_utf8_lossy(&out).contains("An error occurred"),
            "run() must never reject"
        );
        assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 4);
    }
    // Random full-range sweep, plus values that overflow `bedrooms`.
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..128 {
        assert_same_one("E13 run(random)", run_op(rng.next_i32()));
    }
    // Repeated INT_MAX additions: bedrooms wraps over and over.
    let ops: Vec<Op> = (0..16).map(|_| run_op(i32::MAX)).collect();
    assert_same("E13 run(INT_MAX) x16", &ops);
    let ops: Vec<Op> = (0..16).map(|_| run_op(i32::MIN)).collect();
    assert_same("E13 run(INT_MIN) x16", &ops);
}

// -------------------------------------------------------------------------
// E14 — trailing garbage is *accepted* (endp advanced): must not be rejected
// -------------------------------------------------------------------------
#[test]
fn err_e14_trailing_garbage_accepted() {
    for (s, parsed) in [
        ("12abc", 12),
        ("3 ", 3),
        ("5\n", 5),
        ("7;", 7),
        ("1,000", 1),
        ("42.7", 42),
        ("-8.9", -8),
        ("0zzz", 0),
        ("2147483647!", i32::MAX),
        ("-2147483648!", i32::MIN),
    ] {
        assert_parses_as(&format!("E14 driver({s:?})"), s, parsed);
    }
}

// -------------------------------------------------------------------------
// E15 — parse_val clears errno first, so a stale errno must not reject.
// -------------------------------------------------------------------------
#[test]
fn err_e15_stale_errno_cleared() {
    for e in [libc::ERANGE, libc::EINVAL, libc::ENOMEM, 1, 42, 4095] {
        let ops = [Op::SetErrno(e), drv("7")];
        let out = assert_same(&format!("E15 stale errno {e} then driver(\"7\")"), &ops);
        let text = String::from_utf8_lossy(&out);
        assert!(
            !text.contains("An error occurred"),
            "stale errno {e} must not cause rejection: {text:?}"
        );
        assert_eq!(text.lines().count(), 8);
    }
    // And a stale errno must not *rescue* an invalid input either.
    let ops = [Op::SetErrno(0), drv("abc")];
    let out = assert_same("E15 errno 0 then driver(\"abc\")", &ops);
    assert_eq!(String::from_utf8_lossy(&out), ERR);
}

// -------------------------------------------------------------------------
// E16 — why the `errno == 0` conjunct is unobservable through the public API.
//
// A mutant that DELETES `&& errno == 0` from parse_val is not detected by any
// differential test, which could look like a coverage gap. It is not: with
// base 10, glibc's strtol sets errno only to ERANGE, and only when it saturates
// the result to LONG_MAX / LONG_MIN -- both of which are then rejected by the
// `tmp >= INT_MIN && tmp <= INT_MAX` conjuncts anyway. This test establishes
// that invariant empirically on this platform, so the redundancy is documented
// rather than assumed. (The Rust translation still keeps the conjunct, matching
// the C source exactly.)
// -------------------------------------------------------------------------
#[test]
fn err_e16_errno_conjunct_is_redundant_for_base10() {
    extern "C" {
        fn strtol(
            nptr: *const std::ffi::c_char,
            endptr: *mut *mut std::ffi::c_char,
            base: c_int,
        ) -> std::ffi::c_long;
        fn __errno_location() -> *mut c_int;
    }

    let mut cases: Vec<String> = vec![
        String::new(),
        " ".into(),
        "abc".into(),
        "+".into(),
        "-".into(),
        "0".into(),
        "2147483647".into(),
        "-2147483648".into(),
        "2147483648".into(),
        "-2147483649".into(),
        "9223372036854775807".into(),
        "-9223372036854775808".into(),
        "9223372036854775808".into(),
        "-9223372036854775809".into(),
        "9".repeat(400),
        format!("-{}", "9".repeat(400)),
    ];
    let mut rng = Rng::new(SEED ^ 16);
    const ALPHABET: &[u8] = b"0123456789+- \tabcxX.";
    for _ in 0..20_000 {
        let len = rng.below(30);
        cases.push(
            String::from_utf8_lossy(&(0..len).map(|_| *rng.pick(ALPHABET)).collect::<Vec<u8>>())
                .into_owned(),
        );
    }
    // Deliberately generate many ERANGE-triggering magnitudes (19..80 digits,
    // no leading zero) so the invariant is probed where it actually applies.
    for _ in 0..4000 {
        let n = 19 + rng.below(62);
        let mut d = String::new();
        d.push((b'1' + rng.below(9) as u8) as char);
        for _ in 1..n {
            d.push((b'0' + rng.below(10) as u8) as char);
        }
        let sign = *rng.pick(&["", "+", "-"]);
        cases.push(format!("{sign}{d}"));
    }

    let mut saw_erange = 0usize;
    for s in &cases {
        let cs = std::ffi::CString::new(s.replace('\0', "")).unwrap();
        unsafe {
            *__errno_location() = 0;
            let mut endp: *mut std::ffi::c_char = cs.as_ptr() as *mut _;
            let tmp = strtol(cs.as_ptr(), &mut endp, 10);
            let e = *__errno_location();
            if e != 0 {
                saw_erange += 1;
                assert_eq!(e, libc::ERANGE, "unexpected errno {e} for {s:?}");
                assert!(
                    tmp > i32::MAX as std::ffi::c_long || tmp < i32::MIN as std::ffi::c_long,
                    "errno was set but value {tmp} is inside the int range for {s:?}: the \
                     `errno == 0` conjunct WOULD be observable, so a dedicated differential \
                     test is required"
                );
            }
        }
    }
    assert!(
        saw_erange > 1000,
        "expected many ERANGE cases in the corpus, saw {saw_erange}"
    );
}
