//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, asserting the two implementations return the
//! SAME error code / sentinel (not merely "both failed"), and emit the same
//! bytes on stdout and stderr.

mod common;

use common::*;
use std::ffi::{c_char, CString};

const SEED: u64 = 0x0000_C0DE_60_1_1;

// ---------------------------------------------------------------------------
// Row 1 — forward_goto_example: ordinary negative x -> `goto error`, returns -1
// ---------------------------------------------------------------------------
#[test]
fn e01_forward_negative_returns_minus_one() {
    for x in [-1i32, -2, -3, -42, -1000, -123_456_789] {
        diff_forward("E01", x);
        // Assert the concrete contract, not just parity.
        let (ret, out) = call_forward(Impl::C, x);
        assert_eq!(ret, -1, "C forward_goto_example({x}) should be -1");
        assert_eq!(out.stderr, b"Error: negative input\n");
        assert!(out.stdout.is_empty(), "no stdout on the error path");
        let (rret, rout) = call_forward(Impl::Rust, x);
        assert_eq!(rret, ret);
        assert_eq!(rout.stderr, out.stderr);
        assert_eq!(rout.stdout, out.stdout);
    }
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..500 {
        diff_forward("E01/rand", rng.range_i32(i32::MIN, -1));
    }
}

// ---------------------------------------------------------------------------
// Row 2 — forward_goto_example: x == INT_MIN (extreme negative)
// ---------------------------------------------------------------------------
#[test]
fn e02_forward_int_min() {
    for x in [i32::MIN, i32::MIN + 1, i32::MIN + 2, -i32::MAX] {
        diff_forward("E02", x);
        let (ret, _) = call_forward(Impl::C, x);
        assert_eq!(ret, -1);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — forward_goto_example: x == -1, sentinel/value collision
// ---------------------------------------------------------------------------
#[test]
fn e03_forward_minus_one_sentinel_collision() {
    diff_forward("E03", -1);
    let (c, _) = call_forward(Impl::C, -1);
    let (r, _) = call_forward(Impl::Rust, -1);
    assert_eq!(c, -1);
    assert_eq!(r, -1);
    // And the collision propagates identically through `driver`.
    let f = Fixture::file("e03", b"ok\n");
    diff_driver("E03/driver", -1, Some(f.path()));
    assert_eq!(call_driver(Impl::C, -1, Some(f.path())).0, -1);
    assert_eq!(call_driver(Impl::Rust, -1, Some(f.path())).0, -1);
}

// ---------------------------------------------------------------------------
// Row 4 — open_with_cleanup: nonexistent path (fopen -> NULL, ENOENT)
// ---------------------------------------------------------------------------
#[test]
fn e04_open_nonexistent_path() {
    let missing = Fixture::missing("e04");
    diff_open("E04", Some(&missing));

    let (c_null, c_out) = call_open(Impl::C, Some(&missing));
    let (r_null, r_out) = call_open(Impl::Rust, Some(&missing));
    assert!(c_null, "C should return NULL for a missing file");
    assert!(r_null, "Rust should return NULL for a missing file");
    let expected = format!(
        "Error: opening or processing file {}\n",
        missing.to_str().unwrap()
    );
    assert_eq!(c_out.stderr, expected.as_bytes());
    assert_eq!(r_out.stderr, expected.as_bytes());
    assert!(c_out.stdout.is_empty() && r_out.stdout.is_empty());

    // Also: a path deep under a missing directory, and ENAMETOOLONG.
    let nested = missing.join("a/b/c");
    diff_open("E04/nested", Some(&nested));
    let long = std::env::temp_dir().join("x".repeat(5000));
    diff_open("E04/ENAMETOOLONG", Some(&long));
}

// ---------------------------------------------------------------------------
// Row 5 — open_with_cleanup: filename == NULL
// ---------------------------------------------------------------------------
#[test]
fn e05_open_null_filename() {
    diff_open_raw("E05", std::ptr::null());

    let (c_null, c_out) = call_open_raw(Impl::C, std::ptr::null());
    let (r_null, r_out) = call_open_raw(Impl::Rust, std::ptr::null());
    assert!(c_null && r_null, "both must return NULL");
    assert_eq!(c_out.stderr, r_out.stderr);
    assert_eq!(
        c_out.stderr, b"Error: opening or processing file (null)\n",
        "glibc prints `(null)` for a NULL %s argument"
    );
}

// ---------------------------------------------------------------------------
// Row 6 — open_with_cleanup: directory -> fopen ok, fgets sets ferror
//         (the SECOND `goto cleanup`, where fclose(fp) IS executed)
// ---------------------------------------------------------------------------
#[test]
fn e06_open_directory_triggers_ferror() {
    let d = Fixture::dir("e06");
    diff_open("E06", Some(d.path()));

    let (c_null, c_out) = call_open(Impl::C, Some(d.path()));
    let (r_null, r_out) = call_open(Impl::Rust, Some(d.path()));
    assert!(c_null, "C must return NULL after ferror");
    assert!(r_null, "Rust must return NULL after ferror");
    let expected = format!(
        "Error: opening or processing file {}\n",
        d.path().to_str().unwrap()
    );
    assert_eq!(c_out.stderr, expected.as_bytes());
    assert_eq!(r_out.stderr, expected.as_bytes());
    assert!(c_out.stdout.is_empty() && r_out.stdout.is_empty());

    // Well-known directories too, to be independent of the fixture.
    for p in ["/", "/tmp", "/usr"] {
        diff_open(&format!("E06/{p}"), Some(std::path::Path::new(p)));
    }
}

// ---------------------------------------------------------------------------
// Row 7 — open_with_cleanup: unreadable file (fopen -> NULL, EACCES)
// ---------------------------------------------------------------------------
#[test]
fn e07_open_permission_denied() {
    let f = Fixture::file("e07", b"secret\n");
    f.chmod(0o000);
    // If the test runs as root the chmod is not enforced; the differential
    // assertion still holds either way because both impls see the same file.
    diff_open("E07", Some(f.path()));

    let (c_null, c_out) = call_open(Impl::C, Some(f.path()));
    let (r_null, r_out) = call_open(Impl::Rust, Some(f.path()));
    assert_eq!(c_null, r_null, "NULL-ness must match");
    assert_eq!(c_out.stderr, r_out.stderr);
    assert_eq!(c_out.stdout, r_out.stdout);
    if c_null {
        let expected = format!(
            "Error: opening or processing file {}\n",
            f.path().to_str().unwrap()
        );
        assert_eq!(c_out.stderr, expected.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 8 — open_with_cleanup: empty-string filename
// ---------------------------------------------------------------------------
#[test]
fn e08_open_empty_string_filename() {
    let empty = CString::new("").unwrap();
    diff_open_raw("E08", empty.as_ptr() as *const c_char);

    let (c_null, c_out) = call_open_raw(Impl::C, empty.as_ptr());
    let (r_null, r_out) = call_open_raw(Impl::Rust, empty.as_ptr());
    assert!(c_null && r_null);
    assert_eq!(c_out.stderr, b"Error: opening or processing file \n");
    assert_eq!(r_out.stderr, c_out.stderr);
}

// ---------------------------------------------------------------------------
// Row 9 — driver: num < 0 -> early `return -1`, file never touched
// ---------------------------------------------------------------------------
#[test]
fn e09_driver_negative_num_returns_minus_one() {
    let f = Fixture::file("e09", b"content that must not be printed\n");
    for num in [-1i32, -2, -100, i32::MIN] {
        diff_driver("E09", num, Some(f.path()));
        let (c, co) = call_driver(Impl::C, num, Some(f.path()));
        let (r, ro) = call_driver(Impl::Rust, num, Some(f.path()));
        assert_eq!(c, -1, "C driver({num}) must be -1");
        assert_eq!(r, -1, "Rust driver({num}) must be -1");
        assert!(co.stdout.is_empty(), "file must not be read");
        assert!(ro.stdout.is_empty());
        assert_eq!(co.stderr, b"Error: negative input\n");
        assert_eq!(ro.stderr, co.stderr);
    }
}

// ---------------------------------------------------------------------------
// Row 10 — driver: num >= 0 but open_with_cleanup returned NULL -> -2
// ---------------------------------------------------------------------------
#[test]
fn e10_driver_open_failure_returns_minus_two() {
    let missing = Fixture::missing("e10_missing");
    let dir = Fixture::dir("e10_dir");
    let denied = Fixture::file("e10_denied", b"nope\n");
    denied.chmod(0o000);

    let bad: Vec<std::path::PathBuf> =
        vec![missing.clone(), dir.path().to_path_buf(), denied.path().to_path_buf()];

    for p in &bad {
        for num in [0i32, 1, 7, i32::MAX] {
            diff_driver("E10", num, Some(p));
            let (c, co) = call_driver(Impl::C, num, Some(p));
            let (r, ro) = call_driver(Impl::Rust, num, Some(p));
            assert_eq!(c, r, "return differs for driver({num}, {p:?})");
            if c == -2 {
                let expect_stdout = format!(
                    "Processing: {}\nGoto output: {}\n",
                    num,
                    num.wrapping_mul(2)
                );
                assert_eq!(co.stdout, expect_stdout.as_bytes());
                assert_eq!(ro.stdout, expect_stdout.as_bytes());
                assert_eq!(
                    co.stderr,
                    format!("Error: opening or processing file {}\n", p.to_str().unwrap())
                        .as_bytes()
                );
                assert_eq!(ro.stderr, co.stderr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — driver: num < 0 AND bad filename -> the num check wins (-1, not -2)
// ---------------------------------------------------------------------------
#[test]
fn e11_driver_negative_num_wins_over_bad_file() {
    let missing = Fixture::missing("e11");
    let dir = Fixture::dir("e11_dir");
    for p in [missing.as_path(), dir.path()] {
        for num in [-1i32, -5, i32::MIN] {
            diff_driver("E11", num, Some(p));
            assert_eq!(call_driver(Impl::C, num, Some(p)).0, -1);
            assert_eq!(call_driver(Impl::Rust, num, Some(p)).0, -1);
        }
    }
    // ...and with a NULL filename as well.
    for num in [-1i32, i32::MIN] {
        diff_driver_raw("E11/null", num, std::ptr::null());
        assert_eq!(call_driver_raw(Impl::C, num, std::ptr::null()).0, -1);
        assert_eq!(call_driver_raw(Impl::Rust, num, std::ptr::null()).0, -1);
    }
}

// ---------------------------------------------------------------------------
// Row 12 — driver: NULL filename with num >= 0 -> -2
// ---------------------------------------------------------------------------
#[test]
fn e12_driver_null_filename() {
    for num in [0i32, 1, 99, i32::MAX] {
        diff_driver_raw("E12", num, std::ptr::null());
        let (c, co) = call_driver_raw(Impl::C, num, std::ptr::null());
        let (r, ro) = call_driver_raw(Impl::Rust, num, std::ptr::null());
        assert_eq!(c, -2, "C driver({num}, NULL) must be -2");
        assert_eq!(r, -2, "Rust driver({num}, NULL) must be -2");
        assert_eq!(co.stderr, b"Error: opening or processing file (null)\n");
        assert_eq!(ro.stderr, co.stderr);
        assert_eq!(co.stdout, ro.stdout);
    }
    // empty string, same shape
    let empty = CString::new("").unwrap();
    for num in [0i32, 5] {
        diff_driver_raw("E12/empty", num, empty.as_ptr());
        assert_eq!(call_driver_raw(Impl::C, num, empty.as_ptr()).0, -2);
        assert_eq!(call_driver_raw(Impl::Rust, num, empty.as_ptr()).0, -2);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — driver: num == INT_MIN
// ---------------------------------------------------------------------------
#[test]
fn e13_driver_int_min() {
    let f = Fixture::file("e13", b"z\n");
    for num in [i32::MIN, i32::MIN + 1] {
        diff_driver("E13", num, Some(f.path()));
        assert_eq!(call_driver(Impl::C, num, Some(f.path())).0, -1);
        assert_eq!(call_driver(Impl::Rust, num, Some(f.path())).0, -1);
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary sweep: every documented sentinel over random inputs.
// (No enum-typed parameter exists in this API — see ERRORS.md — so the full
// `int` range is swept instead.)
// ---------------------------------------------------------------------------
#[test]
fn e14_random_return_code_sweep() {
    let mut rng = Rng::new(SEED ^ 0x14);
    let good = Fixture::file("e14_good", b"payload\n");
    let missing = Fixture::missing("e14_missing");
    let dir = Fixture::dir("e14_dir");

    for iter in 0..600 {
        let num = match iter % 4 {
            0 => rng.i32_any(),
            1 => rng.range_i32(i32::MIN, 0),
            2 => rng.range_i32(0, i32::MAX),
            _ => rng.range_i32(-3, 3),
        };
        let p: &std::path::Path = match iter % 3 {
            0 => good.path(),
            1 => &missing,
            _ => dir.path(),
        };
        let (c, co) = call_driver(Impl::C, num, Some(p));
        let (r, ro) = call_driver(Impl::Rust, num, Some(p));
        assert_eq!(c, r, "driver({num}, {p:?}) return differs");
        assert!(
            c == 0 || c == -1 || c == -2,
            "unexpected driver return {c} for ({num}, {p:?})"
        );
        assert_eq!(co.stdout, ro.stdout, "stdout differs for ({num}, {p:?})");
        assert_eq!(co.stderr, ro.stderr, "stderr differs for ({num}, {p:?})");
    }
}

// ---------------------------------------------------------------------------
// Non-UTF-8 / odd byte sequences in the filename (raw const char* boundary).
// ---------------------------------------------------------------------------
#[test]
fn e15_non_utf8_filename_bytes() {
    let cases: &[&[u8]] = &[
        b"\xff\xfe",
        b"\x80bad",
        b"./",
        b"//",
        b"/dev/null",
        b"/proc/self/nonexistent-entry",
        b"\t",
        b" ",
    ];
    for c in cases {
        let cs = CString::new(*c).unwrap();
        diff_open_raw("E15", cs.as_ptr());
        diff_driver_raw("E15", 4, cs.as_ptr());
        diff_driver_raw("E15/neg", -4, cs.as_ptr());
    }
}
