//! Phase C — one differential test per row of ERRORS.md.
//!
//! Every test constructs the exact invalid input / condition, drives BOTH the C
//! `.so` (and executable) and the Rust `.so` (and executable), and asserts they
//! reject identically — same message, same exit status, same terminating signal.

mod common;
use common::*;

const ERR: &[u8] = b"An error occurred\n";

/// Asserts that C and Rust agree *and* that the C reference really did reject
/// the input (so the row is testing the rejection it claims to test).
fn assert_rejected(input: &[u8], ctx: &str) {
    assert_input_matches(input, ctx);
    let c = run_exe(&c_exe(), input);
    assert_eq!(
        c.stdout,
        ERR,
        "[{}] expected the C reference to REJECT {:?}, got {:?}",
        ctx,
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout)
    );
    assert_eq!(c.exit, Some(0), "[{}] C exit status", ctx);
    assert!(c.stderr.is_empty(), "[{}] C stderr", ctx);
}

/// Asserts C and Rust agree *and* that the C reference accepted the input.
fn assert_accepted(input: &[u8], ctx: &str) {
    assert_input_matches(input, ctx);
    let c = run_exe(&c_exe(), input);
    assert_eq!(
        c.stdout.iter().filter(|&&b| b == b'\n').count(),
        8,
        "[{}] expected the C reference to ACCEPT {:?}, got {:?}",
        ctx,
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout)
    );
    assert_eq!(c.exit, Some(0), "[{}] C exit status", ctx);
}

// ---------------------------------------------------------------------------
// E1 — endp == str: empty string (fgets returns NULL, buffer stays "")
// ---------------------------------------------------------------------------
#[test]
fn err_e1_empty_input() {
    assert_rejected(b"", "E1 empty stdin");
}

// ---------------------------------------------------------------------------
// E2 — endp == str: no digits anywhere
// ---------------------------------------------------------------------------
#[test]
fn err_e2_no_digits() {
    for s in [
        "abc\n",
        "hello world\n",
        "!\n",
        "?????\n",
        "NaN\n",
        "inf\n",
        "null\n",
        "the house\n",
    ] {
        assert_rejected(s.as_bytes(), &format!("E2 {:?}", s));
    }
    // arbitrary non-UTF-8 bytes with no digit
    assert_rejected(&[0xff, 0xfe, b'\n'], "E2 high bytes");
}

// ---------------------------------------------------------------------------
// E3 — endp == str: whitespace only
// ---------------------------------------------------------------------------
#[test]
fn err_e3_whitespace_only() {
    for s in ["\n", " \n", "  \n", "\t\n", "\t\x0b\x0c\r \n", " ", "\t", "\r\n"] {
        assert_rejected(s.as_bytes(), &format!("E3 {:?}", s));
    }
    assert_rejected(&vec![b' '; 99], "E3 99 spaces");
    assert_rejected(&vec![b' '; 200], "E3 200 spaces");
}

// ---------------------------------------------------------------------------
// E4 — endp == str: sign but no digits
// ---------------------------------------------------------------------------
#[test]
fn err_e4_sign_without_digits() {
    for s in [
        "+\n", "-\n", "+ 1\n", "- 1\n", "--1\n", "+-1\n", "-+1\n", "++1\n", "-abc\n", "+.5\n",
        " + 7\n", "-", "+",
    ] {
        assert_rejected(s.as_bytes(), &format!("E4 {:?}", s));
    }
}

// ---------------------------------------------------------------------------
// E5 — endp == str: non-digit prefix in front of the digits
// ---------------------------------------------------------------------------
#[test]
fn err_e5_garbage_prefix() {
    for s in [
        "x12\n", ".5\n", " .5\n", "#1\n", "e5\n", "$7\n", "(7)\n", "a-1\n", "/2\n", "'7'\n",
    ] {
        assert_rejected(s.as_bytes(), &format!("E5 {:?}", s));
    }
}

// ---------------------------------------------------------------------------
// E6 — errno == ERANGE, positive overflow of long
// ---------------------------------------------------------------------------
#[test]
fn err_e6_erange_positive() {
    let mut cases: Vec<Vec<u8>> = vec![
        b"9223372036854775808\n".to_vec(),
        b"9223372036854775809\n".to_vec(),
        b"18446744073709551615\n".to_vec(),
        b"18446744073709551616\n".to_vec(),
        b"99999999999999999999\n".to_vec(),
    ];
    for n in [20usize, 25, 40, 60, 98, 99] {
        cases.push(format!("{}\n", "9".repeat(n)).into_bytes());
        cases.push(format!("1{}\n", "0".repeat(n)).into_bytes());
    }
    for (i, c) in cases.iter().enumerate() {
        assert_rejected(c, &format!("E6 #{}", i));
    }
}

// ---------------------------------------------------------------------------
// E7 — errno == ERANGE, negative overflow of long
// ---------------------------------------------------------------------------
#[test]
fn err_e7_erange_negative() {
    let mut cases: Vec<Vec<u8>> = vec![
        b"-9223372036854775809\n".to_vec(),
        b"-9223372036854775810\n".to_vec(),
        b"-18446744073709551616\n".to_vec(),
        b"-99999999999999999999\n".to_vec(),
    ];
    for n in [20usize, 25, 40, 60, 98] {
        cases.push(format!("-{}\n", "9".repeat(n)).into_bytes());
    }
    for (i, c) in cases.iter().enumerate() {
        assert_rejected(c, &format!("E7 #{}", i));
    }
}

// ---------------------------------------------------------------------------
// E8 — tmp > INT_MAX (but representable as long)
// ---------------------------------------------------------------------------
#[test]
fn err_e8_above_int_max() {
    for s in [
        "2147483648\n",
        "2147483649\n",
        "2147483650\n",
        "4294967295\n",
        "4294967296\n",
        "10000000000\n",
        "9223372036854775806\n",
        "9223372036854775807\n",
        "+2147483648\n",
        "0002147483648\n",
    ] {
        assert_rejected(s.as_bytes(), &format!("E8 {:?}", s));
    }
}

// ---------------------------------------------------------------------------
// E9 — tmp < INT_MIN (but representable as long)
// ---------------------------------------------------------------------------
#[test]
fn err_e9_below_int_min() {
    for s in [
        "-2147483649\n",
        "-2147483650\n",
        "-4294967296\n",
        "-10000000000\n",
        "-9223372036854775807\n",
        "-9223372036854775808\n",
        "-0002147483649\n",
    ] {
        assert_rejected(s.as_bytes(), &format!("E9 {:?}", s));
    }
}

// ---------------------------------------------------------------------------
// E10 — the 99-byte fgets truncation changes the parse result
// ---------------------------------------------------------------------------
#[test]
fn err_e10_truncation_changes_result() {
    // 120 digits: the stored prefix is 99 digits -> ERANGE -> rejection
    assert_rejected(
        format!("{}\n", "1".repeat(120)).as_bytes(),
        "E10 120 digits",
    );
    assert_rejected(
        format!("-{}\n", "1".repeat(120)).as_bytes(),
        "E10 -120 digits",
    );
    // 99 digits stored exactly, still ERANGE
    assert_rejected(format!("{}\n", "1".repeat(99)).as_bytes(), "E10 99 digits");
    // padding eats the whole buffer: only spaces are stored -> endp == str
    assert_rejected(
        format!("{}7\n", " ".repeat(120)).as_bytes(),
        "E10 padding then digit",
    );
    assert_rejected(
        format!("{}7\n", " ".repeat(99)).as_bytes(),
        "E10 99 spaces then digit",
    );
    // truncation keeps a *valid* prefix: accepted, but with a different value
    assert_accepted(
        format!("{}1234567890123\n", " ".repeat(90)).as_bytes(),
        "E10 truncated valid prefix",
    );
    // zeros then a digit that gets cut off -> value 0 (accepted)
    assert_accepted(
        format!("{}7\n", "0".repeat(120)).as_bytes(),
        "E10 zeros truncated",
    );
}

// ---------------------------------------------------------------------------
// E11 — embedded NUL terminates the C string early
// ---------------------------------------------------------------------------
#[test]
fn err_e11_embedded_nul() {
    assert_rejected(b"\0", "E11 lone NUL");
    assert_rejected(b"\0\n", "E11 NUL then newline");
    assert_rejected(b"\x007\n", "E11 NUL before digits");
    assert_rejected(b" \0 7\n", "E11 space NUL digit");
    assert_rejected(b"-\0 7\n", "E11 sign NUL digit");
    assert_accepted(b"7\0 9\n", "E11 digits then NUL");
    assert_accepted(b"12\0", "E11 digits then NUL, no newline");
}

// ---------------------------------------------------------------------------
// E12 — run(NULL, x): unchecked dereference
// ---------------------------------------------------------------------------
#[test]
fn err_e12_null_house_segv() {
    assert_run_null_matches();
}

// ---------------------------------------------------------------------------
// E13 — one step inside vs one step outside the accepted range
// ---------------------------------------------------------------------------
#[test]
fn err_e13_off_by_one_range() {
    assert_accepted(b"2147483647\n", "E13 INT_MAX");
    assert_rejected(b"2147483648\n", "E13 INT_MAX+1");
    assert_accepted(b"-2147483648\n", "E13 INT_MIN");
    assert_rejected(b"-2147483649\n", "E13 INT_MIN-1");
    assert_accepted(b"2147483646\n", "E13 INT_MAX-1");
    assert_accepted(b"-2147483647\n", "E13 INT_MIN+1");
    assert_accepted(b"0\n", "E13 zero");
}

// ---------------------------------------------------------------------------
// E14 — LONG_MAX / LONG_MIN exactly: rejected by the INT range test, not errno
// ---------------------------------------------------------------------------
#[test]
fn err_e14_long_boundaries() {
    assert_rejected(b"9223372036854775807\n", "E14 LONG_MAX");
    assert_rejected(b"-9223372036854775808\n", "E14 LONG_MIN");
    assert_rejected(b"9223372036854775806\n", "E14 LONG_MAX-1");
    assert_rejected(b"-9223372036854775807\n", "E14 LONG_MIN+1");
    // ... and one step past, which is ERANGE instead
    assert_rejected(b"9223372036854775808\n", "E14 LONG_MAX+1");
    assert_rejected(b"-9223372036854775809\n", "E14 LONG_MIN-1");
}

// ---------------------------------------------------------------------------
// E15 — oversized / unterminated stdin
// ---------------------------------------------------------------------------
#[test]
fn err_e15_oversized_and_unterminated() {
    assert_rejected(&vec![b'9'; 100 * 1024], "E15 100KiB of digits, no newline");
    assert_rejected(
        format!("{}\n", "9".repeat(100 * 1024)).as_bytes(),
        "E15 100KiB line",
    );
    assert_rejected(&vec![b' '; 100 * 1024], "E15 100KiB of spaces");
    assert_accepted(b"7", "E15 unterminated valid");
    assert_accepted(b" 42", "E15 unterminated with padding");
    assert_rejected(b"a", "E15 unterminated invalid");
}

// ---------------------------------------------------------------------------
// E16 — extra_bedrooms at the int extremes (signed += overflow)
// ---------------------------------------------------------------------------
#[test]
fn err_e16_extra_bedrooms_extremes() {
    let mut cases = Vec::new();
    for bedrooms in [i32::MIN, i32::MIN + 1, -1, 0, 1, 5, i32::MAX - 1, i32::MAX] {
        for extra in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            cases.push((House::new(2, bedrooms, 2.5), extra));
        }
    }
    assert_run_batch(&cases, "E16 extra extremes");
    assert_run_twice_batch(&cases, "E16 extra extremes x2");
}

// ---------------------------------------------------------------------------
// E17 — floors at INT_MAX (++ overflow)
// ---------------------------------------------------------------------------
#[test]
fn err_e17_floors_overflow() {
    let cases: Vec<(House, i32)> = [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1]
        .into_iter()
        .flat_map(|f| {
            [
                (House::new(f, 5, 2.5), 1),
                (House::new(f, i32::MAX, 2.5), i32::MAX),
                (House::new(f, i32::MIN, 2.5), i32::MIN),
            ]
        })
        .collect();
    assert_run_batch(&cases, "E17 floors overflow");
    assert_run_twice_batch(&cases, "E17 floors overflow x2");
}

// ---------------------------------------------------------------------------
// E18 — non-finite bathrooms through %.1f
// ---------------------------------------------------------------------------
#[test]
fn err_e18_non_finite_bathrooms() {
    let vals = [
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0x7ff8_0000_0000_0001),
        f64::from_bits(0xfff8_0000_0000_0001),
        f64::from_bits(0x7ff0_0000_0000_0001),
        f64::from_bits(0xfff0_0000_dead_beef),
        f64::from_bits(0x7fff_ffff_ffff_ffff),
        f64::from_bits(0xffff_ffff_ffff_ffff),
    ];
    let cases: Vec<(House, i32)> = vals
        .into_iter()
        .flat_map(|b| {
            [
                (House::new(0, 0, b), 0),
                (House::new(i32::MAX, i32::MAX, b), i32::MAX),
            ]
        })
        .collect();
    assert_run_batch(&cases, "E18 non-finite");
    assert_run_twice_batch(&cases, "E18 non-finite x2");
}

// ---------------------------------------------------------------------------
// Generic FFI boundary: main() must return the same int, always 0
// ---------------------------------------------------------------------------
#[test]
fn err_generic_main_return_value() {
    for inp in [b"7\n".to_vec(), b"".to_vec(), b"abc\n".to_vec()] {
        let (c, r) = diff_ffi_main(&inp);
        assert_eq!(c.exit, Some(0), "C main() return value");
        assert_eq!(r.exit, Some(0), "Rust main() return value");
        assert_eq!(c, r, "main() outcome for {:?}", inp);
    }
}

// ---------------------------------------------------------------------------
// E19 — misaligned house_t* (UB in C, works on x86-64; must not trip a
// Rust-side alignment check either)
// ---------------------------------------------------------------------------
#[test]
fn err_e19_misaligned_house() {
    let mut backing = vec![0u8; 128];
    let base = backing.as_mut_ptr();
    for off in 1usize..8 {
        let start = House::new(3, 4, 2.5);
        unsafe {
            let p = base.add(off) as *mut House;
            std::ptr::write_unaligned(p, start);
            let cf = c_run();
            let c_out = capture_stdout(|| cf(p, 5));
            let after_c = std::ptr::read_unaligned(p);

            std::ptr::write_unaligned(p, start);
            let rf = rust_run();
            let r_out = capture_stdout(|| rf(p, 5));
            let after_r = std::ptr::read_unaligned(p);

            assert_eq!(
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out),
                "E19 misaligned (+{}) stdout",
                off
            );
            assert!(
                after_c.bit_eq(&after_r),
                "E19 misaligned (+{}) struct: C {} vs Rust {}",
                off,
                after_c.show(),
                after_r.show()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E20 — stdout is a pipe with no reader: printf fails.  The C program never
// touches signal dispositions, so it dies with SIGPIPE when the inherited
// disposition is the default and survives when SIGPIPE is inherited ignored.
// ---------------------------------------------------------------------------
#[test]
fn err_e20_stdout_without_reader() {
    for input in [b"7\n".to_vec(), b"abc\n".to_vec(), b"".to_vec()] {
        for ignore in [false, true] {
            let c = run_exe_without_stdout_reader(&c_exe(), &input, ignore);
            let r = run_exe_without_stdout_reader(&rust_exe(), &input, ignore);
            assert_eq!(
                c, r,
                "E20 (input {:?}, SIGPIPE ignored = {}): C (exit, signal) = {:?}, Rust = {:?}",
                String::from_utf8_lossy(&input),
                ignore,
                c,
                r
            );
        }
    }
    // and the C reference really is killed by SIGPIPE with the default
    // disposition, i.e. the row is not vacuous
    let c = run_exe_without_stdout_reader(&c_exe(), b"7\n", false);
    assert_eq!(c, (None, Some(13)), "E20 C reference should die of SIGPIPE");
}

// ---------------------------------------------------------------------------
// E21 — file descriptor 0 closed: fgets() fails with EBADF and returns NULL
// ---------------------------------------------------------------------------
#[test]
fn err_e21_closed_stdin() {
    let c = run_exe_with_closed_stdin(&c_exe());
    let r = run_exe_with_closed_stdin(&rust_exe());
    assert_eq!(
        c,
        r,
        "E21 closed stdin: C {} vs Rust {}",
        c.show(),
        r.show()
    );
    assert_eq!(c.stdout, ERR, "E21 C reference output");
}
