//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI boundary cases (null
//! pointers, zero/oversized lengths, values one step past every documented
//! range). Each test asserts that the C and the Rust library reject the input
//! in exactly the same way: same sentinel/return code **and** the same bytes on
//! `stdout`/`stderr`.

mod common;

use common::*;
use std::ffi::{CStr, CString, c_int};

const SEED: u64 = 0xE770_5EED_0000_0001;

/// Assert (against the C library, the ground truth) that a call really does hit
/// the error path we think it does — so the differential assertion above it is
/// not vacuous.
fn c_fge(x: c_int) -> (c_int, Cap) {
    capture(|| unsafe { (c_api().forward_goto_example)(x) })
}
fn c_driver(num: c_int, p: Option<&CStr>) -> (c_int, Cap) {
    let ptr = p.map(|f| f.as_ptr()).unwrap_or(std::ptr::null());
    capture(|| unsafe { (c_api().driver)(num, ptr) })
}
fn c_owc(p: Option<&CStr>) -> (StreamState, Cap) {
    let ptr = p.map(|f| f.as_ptr()).unwrap_or(std::ptr::null());
    capture(|| owc_and_close(c_api(), ptr))
}

// ===========================================================================
// Row 1 — forward_goto_example: x < 0 → goto error
// ===========================================================================
#[test]
fn err01_fge_negative() {
    diff_fge(-1);
    let (ret, cap) = c_fge(-1);
    assert_eq!(ret, -1, "C sentinel");
    assert_eq!(cap.err, b"Error: negative input\n", "C stderr");
    assert!(cap.out.is_empty(), "C stdout must stay empty");
}

// ===========================================================================
// Row 2 — forward_goto_example: x == INT_MIN
// ===========================================================================
#[test]
fn err02_fge_int_min() {
    diff_fge(i32::MIN);
    diff_fge(i32::MIN + 1);
    let (ret, _) = c_fge(i32::MIN);
    assert_eq!(ret, -1);
}

// ===========================================================================
// Row 3 — forward_goto_example: many randomized negatives
// ===========================================================================
#[test]
fn err03_fge_random_negatives() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..500 {
        let x = rng.range_i64(i32::MIN as i64, -1) as c_int;
        diff_fge(x);
    }
    // one step past the boundary in both directions
    diff_fge(-1);
    diff_fge(0);
}

// ===========================================================================
// Row 4 — open_with_cleanup: fopen fails, ENOENT
// ===========================================================================
#[test]
fn err04_owc_enoent() {
    let p = missing_path();
    diff_owc(Some(&p));
    diff_driver(1, Some(&p));

    let (st, cap) = c_owc(Some(&p));
    assert!(st.is_null, "C must return NULL");
    let expected = format!(
        "Error: opening or processing file {}\n",
        p.to_str().unwrap()
    );
    assert_eq!(cap.err, expected.as_bytes(), "C stderr");
    assert!(cap.out.is_empty());
}

// ===========================================================================
// Row 5 — open_with_cleanup: empty string path
// ===========================================================================
#[test]
fn err05_owc_empty_path() {
    let p = CString::new("").unwrap();
    diff_owc(Some(&p));
    diff_driver(0, Some(&p));
    diff_driver(-1, Some(&p));

    let (st, cap) = c_owc(Some(&p));
    assert!(st.is_null);
    assert_eq!(cap.err, b"Error: opening or processing file \n");
}

// ===========================================================================
// Row 6 — open_with_cleanup: NULL filename pointer
// ===========================================================================
#[test]
fn err06_owc_null_pointer() {
    diff_owc(None);

    let (st, cap) = c_owc(None);
    assert!(st.is_null, "C must return NULL for a NULL path");
    assert_eq!(
        cap.err, b"Error: opening or processing file (null)\n",
        "glibc renders a NULL %s as (null)"
    );
}

// ===========================================================================
// Row 7 — open_with_cleanup: EACCES (mode 000)
// ===========================================================================
#[test]
fn err07_owc_eacces() {
    let p = unreadable_path();
    diff_owc(Some(&p));
    diff_driver(2, Some(&p));

    let (st, cap) = c_owc(Some(&p));
    assert!(st.is_null, "C must return NULL for an unreadable file");
    assert!(
        cap.err.starts_with(b"Error: opening or processing file "),
        "C stderr: {:?}",
        Show(&cap.err)
    );
}

// ===========================================================================
// Row 8 — open_with_cleanup: ENAMETOOLONG (oversized path)
// ===========================================================================
#[test]
fn err08_owc_enametoolong() {
    for len in [255usize, 256, 4096, 5000] {
        let name: String = std::iter::repeat('n').take(len).collect();
        let p = CString::new(format!("{}/{}", tmp_dir().display(), name)).unwrap();
        diff_owc(Some(&p));
        diff_driver(1, Some(&p));
    }
    let name: String = std::iter::repeat('n').take(5000).collect();
    let p = CString::new(format!("{}/{}", tmp_dir().display(), name)).unwrap();
    let (st, _) = c_owc(Some(&p));
    assert!(st.is_null, "C must return NULL for an oversized path");
}

// ===========================================================================
// Row 9 — open_with_cleanup: ENOTDIR (regular file used as a directory)
// ===========================================================================
#[test]
fn err09_owc_enotdir() {
    let f = fixture("notdir", b"i am a file\n");
    let p = CString::new(format!("{}/child", f.to_str().unwrap())).unwrap();
    diff_owc(Some(&p));
    diff_driver(1, Some(&p));

    let (st, _) = c_owc(Some(&p));
    assert!(st.is_null, "C must return NULL for ENOTDIR");
}

// ===========================================================================
// Row 10 — open_with_cleanup: fopen succeeds, read sets ferror (directory)
//          → second `goto cleanup`, and fclose(fp) IS executed
// ===========================================================================
#[test]
fn err10_owc_ferror_directory() {
    let d = dir_path();
    diff_owc(Some(&d));
    diff_driver(1, Some(&d));
    diff_driver(-1, Some(&d));

    let (st, cap) = c_owc(Some(&d));
    assert!(st.is_null, "C must return NULL when ferror is set");
    let expected = format!(
        "Error: opening or processing file {}\n",
        d.to_str().unwrap()
    );
    assert_eq!(cap.err, expected.as_bytes());
    assert!(cap.out.is_empty(), "nothing may be echoed for a directory");

    // The directory really does open (so this row exercises the *second*
    // goto cleanup, not the first one).
    let fp = unsafe { libc::fopen(d.as_ptr(), c"r".as_ptr()) };
    assert!(
        !fp.is_null(),
        "precondition: glibc fopen() on a directory must succeed for this row \
         to exercise the ferror path"
    );
    unsafe { libc::fclose(fp) };

    // Repeat many times: if the Rust version leaked the FILE* instead of
    // closing it (the C code calls fclose in the cleanup label) the process
    // would run out of descriptors and the two libraries would diverge.
    for _ in 0..300 {
        diff_owc(Some(&d));
    }
}

// ===========================================================================
// Row 11 — driver: forward_goto_example returned -1 → early return -1
// ===========================================================================
#[test]
fn err11_driver_negative_num() {
    let good = fixture("ok", b"content\n");
    diff_driver(-1, Some(&good));

    let (ret, cap) = c_driver(-1, Some(&good));
    assert_eq!(ret, -1);
    assert_eq!(cap.err, b"Error: negative input\n");
    assert!(cap.out.is_empty(), "the file must not be echoed");
}

// ===========================================================================
// Row 12 — driver: num < 0 AND a bad file → -1 wins (error precedence)
// ===========================================================================
#[test]
fn err12_driver_error_precedence() {
    let missing = missing_path();
    let d = dir_path();
    let unreadable = unreadable_path();
    for p in [&missing, &d, &unreadable] {
        diff_driver(-1, Some(p));
        diff_driver(i32::MIN, Some(p));

        let (ret, cap) = c_driver(-1, Some(p));
        assert_eq!(ret, -1, "the first error must win");
        assert_eq!(cap.err, b"Error: negative input\n");
        assert!(
            !cap.err.windows(9).any(|w| w == b"opening o"),
            "the file must never be touched"
        );
    }
    diff_driver(-1, None);
    let (ret, cap) = c_driver(-1, None);
    assert_eq!(ret, -1);
    assert_eq!(cap.err, b"Error: negative input\n");
}

// ===========================================================================
// Row 13 — driver: open_with_cleanup returned NULL → -2
// ===========================================================================
#[test]
fn err13_driver_file_error_returns_minus2() {
    let missing = missing_path();
    let d = dir_path();
    let unreadable = unreadable_path();
    let empty_path = CString::new("").unwrap();
    let f = fixture("notdir2", b"x\n");
    let notdir = CString::new(format!("{}/child", f.to_str().unwrap())).unwrap();
    let long: String = std::iter::repeat('L').take(5000).collect();
    let toolong = CString::new(format!("{}/{}", tmp_dir().display(), long)).unwrap();

    for p in [
        &missing,
        &d,
        &unreadable,
        &empty_path,
        &notdir,
        &toolong,
    ] {
        for num in [0, 1, 2, 1000, 0x3FFF_FFFF, i32::MAX] {
            diff_driver(num, Some(p));
        }
        let (ret, cap) = c_driver(3, Some(p));
        assert_eq!(ret, -2, "C must return -2 for {:?}", p);
        assert_eq!(cap.out, b"Processing: 3\nGoto output: 6\n");
        assert!(cap.err.starts_with(b"Error: opening or processing file "));
    }
}

// ===========================================================================
// Row 14 — driver: extreme ints with a bad file
// ===========================================================================
#[test]
fn err14_driver_extreme_ints() {
    let missing = missing_path();
    for num in [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        diff_driver(num, Some(&missing));
    }
    assert_eq!(c_driver(i32::MIN, Some(&missing)).0, -1);
    assert_eq!(c_driver(i32::MAX, Some(&missing)).0, -2);
    // INT_MAX doubles to -2, which must NOT be mistaken for the -1 sentinel.
    let (ret, cap) = c_fge(i32::MAX);
    assert_eq!(ret, -2, "INT_MAX*2 wraps to -2 in C");
    assert_eq!(cap.out, b"Processing: 2147483647\n");
}

// ===========================================================================
// Row 15 — driver: NULL filename with num >= 0
// ===========================================================================
#[test]
fn err15_driver_null_filename() {
    for num in [0, 1, 7, 0x4000_0000u32 as i32, i32::MAX] {
        diff_driver(num, None);
    }
    let (ret, cap) = c_driver(0, None);
    assert_eq!(ret, -2);
    assert_eq!(cap.out, b"Processing: 0\nGoto output: 0\n");
    assert_eq!(cap.err, b"Error: opening or processing file (null)\n");
}

// ===========================================================================
// Row 16 — negative control: an empty-but-openable stream is NOT an error
// ===========================================================================
#[test]
fn err16_empty_stream_is_not_an_error() {
    let empty = fixture("ctrl-empty", b"");
    let devnull = CString::new("/dev/null").unwrap();
    for p in [&empty, &devnull] {
        diff_owc(Some(p));
        diff_driver(1, Some(p));

        let (st, cap) = c_owc(Some(p));
        assert!(!st.is_null, "{:?} must open successfully", p);
        assert!(cap.err.is_empty(), "no error message expected");
        assert!(cap.out.is_empty(), "nothing to echo");
        assert_eq!(c_driver(1, Some(p)).0, 0);
    }
}

// ===========================================================================
// Generic FFI boundary cases (beyond the table)
// ===========================================================================

/// Every `int` bit-pattern is a legal argument for a C `int` parameter (the API
/// has no enums, so this is the analogous "value with no valid variant" case).
#[test]
fn ffi_all_int_bit_patterns_are_accepted_identically() {
    let mut rng = Rng::new(SEED ^ 0xFF);
    let mut xs: Vec<i32> = vec![
        0,
        -1,
        1,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        0x4000_0000u32 as i32,
        0x8000_0000u32 as i32,
        0xFFFF_FFFFu32 as i32,
        0x7FFF_FFFF,
        -0x4000_0000,
    ];
    for _ in 0..256 {
        xs.push(rng.i32());
    }
    // Also every power of two and its neighbours.
    for b in 0..32 {
        let v = 1i32.wrapping_shl(b);
        xs.push(v);
        xs.push(v.wrapping_sub(1));
        xs.push(v.wrapping_neg());
    }
    for x in xs {
        diff_fge(x);
    }
}

/// A NULL pointer for every pointer parameter of every entry point.
#[test]
fn ffi_null_pointers() {
    diff_owc(None);
    for num in [i32::MIN, -1, 0, 1, i32::MAX] {
        diff_driver(num, None);
    }
}

/// Zero-length and oversized "lengths": the only length-ish input is the path
/// string, so exercise the empty path and progressively oversized ones.
#[test]
fn ffi_zero_and_oversized_path_lengths() {
    let empty = CString::new("").unwrap();
    diff_owc(Some(&empty));
    diff_driver(1, Some(&empty));

    for len in [1usize, 2, 254, 255, 256, 257, 1023, 4095, 4096, 4097, 8192] {
        let name: String = std::iter::repeat('p').take(len).collect();
        let p = CString::new(name).unwrap();
        diff_owc(Some(&p));
        diff_driver(1, Some(&p));
    }
}

/// A path whose *bytes* are not valid UTF-8 — a Rust translation that went
/// through `String`/`str` instead of the raw pointer would diverge here.
#[test]
fn ffi_non_utf8_paths() {
    for raw in [
        &b"\xff"[..],
        &b"\x80\x80"[..],
        &b"/tmp/\xc3"[..],
        &b"\xed\xa0\x80"[..], // UTF-16 surrogate encoded as UTF-8
    ] {
        let p = CString::new(raw.to_vec()).unwrap();
        diff_owc(Some(&p));
        diff_driver(1, Some(&p));
    }
    // A valid file with a non-UTF-8 name, so the success path is covered too.
    let f = fixture_raw_name(b"\xff\xfe-name", b"ok\n");
    diff_owc(Some(&f));
    diff_driver(1, Some(&f));
}

/// The failure path is taken thousands of times: any descriptor or memory leak
/// that the C code does not have would show up as a divergence.
#[test]
fn ffi_repeated_failures_do_not_drift() {
    let missing = missing_path();
    let d = dir_path();
    for i in 0..400 {
        if i % 2 == 0 {
            diff_owc(Some(&missing));
        } else {
            diff_owc(Some(&d));
        }
    }
    for _ in 0..200 {
        diff_driver(1, Some(&d));
        diff_driver(-1, Some(&missing));
    }
}

// ===========================================================================
// Resource parity — the C cleanup label executes `fclose(fp)` and `driver`
// closes the stream it got back. Neither is visible in stdout/stderr/return
// value, so it is checked directly by counting open file descriptors: the
// Rust library must consume descriptors exactly like the C one.
// ===========================================================================

#[cfg(target_os = "linux")]
fn open_fd_count() -> usize {
    std::fs::read_dir("/proc/self/fd")
        .expect("/proc/self/fd")
        .count()
}

/// Number of descriptors the library retains after `iters` calls.
#[cfg(target_os = "linux")]
fn fd_delta(api: &'static Api, iters: usize, mut call: impl FnMut(&Api)) -> isize {
    // Warm up: the first call may lazily allocate stdio buffers etc.
    capture(|| call(api));
    let before = open_fd_count();
    for _ in 0..iters {
        capture(|| call(api));
    }
    let after = open_fd_count();
    after as isize - before as isize
}

#[cfg(target_os = "linux")]
#[test]
fn leak01_owc_error_paths_have_identical_fd_accounting() {
    let missing = missing_path();
    let d = dir_path();
    let unreadable = unreadable_path();

    // `dir_path()` is the interesting one: fopen SUCCEEDS, the read sets
    // ferror, and the cleanup label must fclose(fp).
    let cases: Vec<(&str, Option<&CStr>)> = vec![
        ("missing", Some(&missing)),
        ("directory", Some(&d)),
        ("unreadable", Some(&unreadable)),
        ("null", None),
    ];
    for (tag, p) in cases {
        let ptr = p.map(|f| f.as_ptr()).unwrap_or(std::ptr::null());
        let cd = fd_delta(c_api(), 64, |api| {
            let s = unsafe { (api.open_with_cleanup)(ptr) };
            assert!(s.is_null(), "{tag}: expected the error path");
        });
        let rd = fd_delta(rust_api(), 64, |api| {
            let s = unsafe { (api.open_with_cleanup)(ptr) };
            assert!(s.is_null(), "{tag}: expected the error path");
        });
        assert_eq!(
            cd, rd,
            "{tag}: open_with_cleanup leaks descriptors differently \
             (C kept {cd}, Rust kept {rd} after 64 calls)"
        );
        assert_eq!(cd, 0, "{tag}: the C library must not leak (ground truth)");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn leak02_owc_success_path_fd_accounting() {
    // The caller owns the returned stream; `owc_and_close` closes it, so the
    // net descriptor delta must be zero for both libraries.
    let good = fixture("leak-ok", b"one\ntwo\nthree\n");
    let empty = fixture("leak-empty", b"");
    for p in [&good, &empty] {
        let ptr = p.as_ptr();
        let cd = fd_delta(c_api(), 64, |api| {
            let s = owc_and_close(api, ptr);
            assert!(!s.is_null, "expected the success path");
        });
        let rd = fd_delta(rust_api(), 64, |api| {
            let s = owc_and_close(api, ptr);
            assert!(!s.is_null, "expected the success path");
        });
        assert_eq!(cd, rd, "success path fd accounting differs (C {cd}, Rust {rd})");
        assert_eq!(cd, 0);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn leak03_driver_closes_the_stream_it_opened() {
    let good = fixture("leak-driver", b"alpha\nbeta\n");
    let missing = missing_path();
    let d = dir_path();
    for (tag, p, want) in [
        ("ok", &good, 0i32),
        ("missing", &missing, -2),
        ("directory", &d, -2),
    ] {
        let ptr = p.as_ptr();
        let cd = fd_delta(c_api(), 64, |api| {
            let r = unsafe { (api.driver)(1, ptr) };
            assert_eq!(r, want, "{tag}");
        });
        let rd = fd_delta(rust_api(), 64, |api| {
            let r = unsafe { (api.driver)(1, ptr) };
            assert_eq!(r, want, "{tag}");
        });
        assert_eq!(
            cd, rd,
            "{tag}: driver's descriptor accounting differs (C {cd}, Rust {rd})"
        );
        assert_eq!(cd, 0, "{tag}: the C driver must not leak (ground truth)");
    }
}
