//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1 … E10, with E11 documented as N/A and
//! covered by E9). The C library has no error codes: `printLine`, `bad` and
//! `good` are `void` and `main` unconditionally returns 0, so a "rejection" is
//! observable only as *no bytes written, call returns normally*. Each test
//! therefore asserts the captured stdout bytes **and** (where one exists) the
//! return value are identical between the C and Rust `.so`.

mod common;

use common::{assert_output_is, assert_same_output, assert_same_output_and_ret, assert_same_sequence, Op, Rng, SEED};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

/// E1 — `printLine(NULL)`: the `if (line != NULL)` guard rejects the input.
/// Expected C result: nothing at all is written; the call returns normally.
#[test]
fn err_e1_print_line_null() {
    assert_output_is("E1 printLine(NULL)", b"", |im| {
        im.print_line(std::ptr::null())
    });

    // Repeated NULLs stay silent.
    assert_output_is("E1 printLine(NULL) x100", b"", |im| {
        for _ in 0..100 {
            im.print_line(std::ptr::null());
        }
    });
}

/// E2 — non-NULL pointer to an immediate NUL: the degenerate zero-length input
/// is *not* rejected, it prints exactly one newline.
#[test]
fn err_e2_print_line_empty() {
    assert_output_is("E2 printLine(\"\")", b"\n", |im| im.print_bytes(b""));

    // A pointer into the middle of a buffer, landing on its NUL terminator.
    let buf = b"abc\0";
    assert_output_is("E2 printLine(&\"abc\\0\"[3])", b"\n", |im| {
        im.print_line(unsafe { buf.as_ptr().add(3) } as *const c_char)
    });
}

/// E3 — NULL interleaved with valid strings: the guard must not latch, and the
/// rejected calls must not disturb the surrounding output.
#[test]
fn err_e3_null_interleaved_with_valid() {
    assert_output_is(
        "E3 NULL between valid strings",
        b"one\ntwo\nthree\n",
        |im| {
            im.print_line(std::ptr::null());
            im.print_bytes(b"one");
            im.print_line(std::ptr::null());
            im.print_line(std::ptr::null());
            im.print_bytes(b"two");
            im.print_line(std::ptr::null());
            im.print_bytes(b"three");
            im.print_line(std::ptr::null());
        },
    );

    // NULL interleaved with the composed entry points too.
    assert_output_is(
        "E3 NULL between bad()/good()",
        b"bad()\ngood()\nhelperGood()\n",
        |im| {
            im.print_line(std::ptr::null());
            im.bad();
            im.print_line(std::ptr::null());
            im.good();
            im.print_line(std::ptr::null());
        },
    );

    // Randomized NULL/valid mixes through the sequence driver.
    let mut rng = Rng::new(SEED ^ 3);
    for seq in 0..100 {
        let n = rng.range(1, 16);
        let ops: Vec<Op> = (0..n)
            .map(|_| {
                if rng.below(2) == 0 {
                    Op::PrintNull
                } else {
                    let len = rng.range(0, 32);
                    Op::PrintLine(rng.bytes_nonzero(len))
                }
            })
            .collect();
        assert_same_sequence(&format!("E3 randomized NULL mix {seq}"), &ops);
    }
}

/// E4 — oversized input: no length is validated, so nothing may be truncated,
/// including at the stdio buffer boundaries.
#[test]
fn err_e4_oversized_lengths() {
    let mut rng = Rng::new(SEED ^ 4241);
    for len in [
        4095usize, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 65537, 1024 * 1024,
    ] {
        let payload = rng.bytes_printable(len);
        let mut expected = payload.clone();
        expected.push(b'\n');
        assert_output_is(&format!("E4 len={len}"), &expected, |im| {
            im.print_bytes(&payload)
        });
    }
}

/// E5 — invalid UTF-8 must be copied verbatim (the C never validates encoding;
/// a Rust translation using `&str`/`to_str()` would reject or mangle these).
#[test]
fn err_e5_invalid_utf8_bytes() {
    const CASES: &[&[u8]] = &[
        &[0x80],                         // lone continuation byte
        &[0xbf],                         // lone continuation byte
        &[0xc0, 0x80],                   // overlong NUL encoding
        &[0xc0, 0xaf],                   // overlong '/'
        &[0xe0, 0x80, 0xaf],             // overlong 3-byte
        &[0xf0, 0x80, 0x80, 0xaf],       // overlong 4-byte
        &[0xed, 0xa0, 0x80],             // UTF-16 surrogate half
        &[0xf4, 0x90, 0x80, 0x80],       // > U+10FFFF
        &[0xfe],                         // invalid start byte
        &[0xff],                         // invalid start byte
        &[0xff, 0xfe, 0xfd, 0xfc],       // invalid run
        &[0xc3, 0x28],                   // bad continuation
        &[0x41, 0x80, 0x42, 0xff, 0x43], // valid ASCII with invalid bytes mixed in
        &[0xf8, 0xf9, 0xfa, 0xfb],       // 5/6-byte-style start bytes
    ];
    for case in CASES {
        let mut expected = case.to_vec();
        expected.push(b'\n');
        assert_output_is(&format!("E5 {case:02x?}"), &expected, |im| {
            im.print_bytes(case)
        });
    }

    // Randomized high-byte-only payloads.
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..200 {
        let len = rng.range(1, 64);
        let payload: Vec<u8> = (0..len).map(|_| (0x80 + rng.below(0x80)) as u8).collect();
        assert_same_output(&format!("E5 random high bytes {i}"), |im| {
            im.print_bytes(&payload)
        });
    }
}

/// E6 — every representable single byte value (`0x01..=0xFF`).
/// `0x00` is unrepresentable: it terminates the C string (covered by E2).
#[test]
fn err_e6_all_single_byte_values() {
    for b in 1u8..=0xff {
        let payload = [b];
        assert_output_is(&format!("E6 byte {b:#04x}"), &[b, b'\n'], |im| {
            im.print_bytes(&payload)
        });
    }
}

/// E7 — `printf` directives in the *data*: the C format string is the fixed
/// `"%s\n"`, so `%n` must not write to memory and nothing may be interpreted.
#[test]
fn err_e7_format_specifiers_are_data() {
    const CASES: &[&[u8]] = &[
        b"%n",
        b"%n%n%n%n%n%n%n%n",
        b"%s",
        b"%99999999d",
        b"%*d",
        b"%.2147483647f",
        b"%%n",
        b"%p %p %p %p",
        b"%hn%hhn%ln%lln%zn",
        b"AAAA%08x.%08x.%08x.%08x",
    ];
    for case in CASES {
        let mut expected = case.to_vec();
        expected.push(b'\n');
        assert_output_is(&format!("E7 {case:?}"), &expected, |im| im.print_bytes(case));
    }
}

/// E8 — embedded control bytes, including newlines, must pass through and get
/// exactly one appended newline.
#[test]
fn err_e8_embedded_control_bytes() {
    let mut cases: Vec<Vec<u8>> = vec![
        b"\n".to_vec(),
        b"\r".to_vec(),
        b"\t".to_vec(),
        b"\x0b\x0c".to_vec(),
        b"\x1b".to_vec(),
        b"\x7f".to_vec(),
        b"a\nb\nc".to_vec(),
        b"\r\n\r\n".to_vec(),
        b"trailing\n".to_vec(),
        b"\nleading".to_vec(),
    ];
    // All C0 control bytes except NUL, in one string.
    cases.push((1u8..0x20).collect());
    for case in &cases {
        let mut expected = case.clone();
        expected.push(b'\n');
        assert_output_is(&format!("E8 {case:02x?}"), &expected, |im| {
            im.print_bytes(case)
        });
    }
}

/// E9 — degenerate/out-of-range `argc` and NULL `argv` across the FFI boundary.
/// The C body never touches them, so the full output must still be produced and
/// `0` returned. This also stands in for the "int with no valid variant"
/// (out-of-range enum) case: the C API has no enum, only this `int`.
#[test]
fn err_e9_main_degenerate_argc_argv() {
    const MAIN_OUTPUT: &[u8] = b"Calling good()...\ngood()\nhelperGood()\nFinished good()\nCalling bad()...\nbad()\nFinished bad()\n";

    // NULL argv, with a spread of argc values including the int extremes.
    for argc in [0i32, -1, 1, 2, 7, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1] {
        assert_output_is(&format!("E9 main({argc}, NULL)"), MAIN_OUTPUT, |im| {
            im.main(argc as c_int, std::ptr::null_mut());
        });
        assert_same_output_and_ret(&format!("E9 main({argc}, NULL) return"), |im| {
            im.main(argc as c_int, std::ptr::null_mut())
        });
    }

    // A real argv, but with an argc that disagrees with it in both directions.
    let owned: Vec<CString> = ["driver", "one", "two"]
        .iter()
        .map(|s| CString::new(*s).unwrap())
        .collect();
    for argc in [0i32, -5, 3, 1000, i32::MAX] {
        assert_output_is(&format!("E9 main({argc}, argv[3])"), MAIN_OUTPUT, |im| {
            let mut ptrs: Vec<*mut c_char> =
                owned.iter().map(|s| s.as_ptr() as *mut c_char).collect();
            ptrs.push(std::ptr::null_mut());
            im.main(argc as c_int, ptrs.as_mut_ptr());
        });
    }

    // Via the sequence driver, mixing NULL-argv main calls with other calls.
    let ops = vec![
        Op::MainNullArgv(0),
        Op::Bad,
        Op::MainNullArgv(i32::MIN as c_int),
        Op::PrintNull,
        Op::Good,
        Op::MainNullArgv(i32::MAX as c_int),
    ];
    assert_same_sequence("E9 mixed sequence with degenerate main", &ops);
}

/// E10 — the no-argument entry points have no invalid input; their boundary is
/// repeated invocation, and `bad()` must *not* call the `static helperBad()`.
#[test]
fn err_e10_no_arg_entry_points_repeated() {
    // `bad()` prints exactly one line: helperBad() is never called by the C.
    assert_output_is("E10 bad() once", b"bad()\n", |im| im.bad());
    assert_output_is("E10 good() once", b"good()\nhelperGood()\n", |im| im.good());

    let mut expected_bad = Vec::new();
    let mut expected_good = Vec::new();
    for _ in 0..64 {
        expected_bad.extend_from_slice(b"bad()\n");
        expected_good.extend_from_slice(b"good()\nhelperGood()\n");
    }
    assert_output_is("E10 bad() x64", &expected_bad, |im| {
        for _ in 0..64 {
            im.bad();
        }
    });
    assert_output_is("E10 good() x64", &expected_good, |im| {
        for _ in 0..64 {
            im.good();
        }
    });

    // "helperBad" must not be reachable from either .so (it is `static` in C).
    let (c, r) = common::impls();
    for im in [c, r] {
        for sym in [b"helperBad\0".as_ref(), b"helperGood\0".as_ref()] {
            let name = String::from_utf8_lossy(&sym[..sym.len() - 1]).to_string();
            let found = unsafe {
                libloading::Library::new(&im.path)
                    .unwrap()
                    .get::<common::VoidFn>(sym)
                    .is_ok()
            };
            assert!(
                !found,
                "{} unexpectedly exports the static helper {name}",
                im.name
            );
        }
    }
}
