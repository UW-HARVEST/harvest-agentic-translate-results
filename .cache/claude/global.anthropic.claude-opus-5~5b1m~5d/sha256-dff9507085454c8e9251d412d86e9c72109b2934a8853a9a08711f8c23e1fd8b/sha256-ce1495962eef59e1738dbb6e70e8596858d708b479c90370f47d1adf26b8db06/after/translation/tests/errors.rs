//! Phase C — error-path / boundary differential tests, one test per row of
//! `ERRORS.md`.
//!
//! `driver` is `void`-returning and has no error channel, so "same error" means
//! "same observable rejection behaviour": the same byte stream on stdout, the
//! same global side effect, and the same non-crashing return.

mod common;

use common::*;
use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// ERRORS row 1 — `setlocale(LC_ALL, "C")` fails; the return value is
/// discarded and all 14 `printf`s still run.
#[test]
fn errors_row_01_setlocale_return_discarded() {
    let bogus = "definitely_not_a_locale.XYZ";
    assert!(!set_locale(bogus), "expected `{bogus}` to be unavailable");
    let p = libs();
    for &c in &[0i16, 65, 97, 127, -1, -128] {
        let c = c as c_char;
        set_locale(bogus);
        let a = capture_to_file(|| p.c.call(c));
        set_locale(bogus);
        let b = capture_to_file(|| p.rs.call(c));
        assert_same(&format!("setlocale-failed driver({c})"), &a, &b);
        // All 14 lines were still emitted.
        assert_eq!(
            a.iter().filter(|&&b| b == b'\n').count(),
            14,
            "expected 14 output lines"
        );
    }
    set_locale("C");
}

/// ERRORS row 2 — `c == 0`.
#[test]
fn errors_row_02_nul_char() {
    let p = libs();
    let a = capture_to_file(|| p.c.call(0));
    let b = capture_to_file(|| p.rs.call(0));
    assert_same("driver(0)", &a, &b);
    assert_eq!(
        a.iter().filter(|&&x| x == 0).count(),
        2,
        "expected two NUL bytes (to lower / to upper)"
    );
    assert!(
        a.starts_with(b"alphanumeric: 0\n"),
        "unexpected first line: {}",
        show(&a)
    );
}

/// ERRORS row 3 — `c == -1` (`0xFF`).
#[test]
fn errors_row_03_minus_one() {
    let p = libs();
    let a = capture_to_file(|| p.c.call(-1));
    let b = capture_to_file(|| p.rs.call(-1));
    assert_same("driver(-1)", &a, &b);
    assert!(
        a.windows(11).any(|w| w == b"to lower: \xff"),
        "expected `to lower: <0xff>`, got {}",
        show(&a)
    );
    assert!(
        a.windows(11).any(|w| w == b"to upper: \xff"),
        "expected `to upper: <0xff>`, got {}",
        show(&a)
    );
}

/// ERRORS row 4 — `c == -128` (`0x80`), the lowest legal table index.
#[test]
fn errors_row_04_minus_128() {
    let p = libs();
    let a = capture_to_file(|| p.c.call(-128));
    let b = capture_to_file(|| p.rs.call(-128));
    assert_same("driver(-128)", &a, &b);
    assert!(
        a.windows(11).any(|w| w == b"to lower: \x80"),
        "expected `to lower: <0x80>`, got {}",
        show(&a)
    );
}

/// ERRORS row 5 — `c == 127` (`DEL`).
#[test]
fn errors_row_05_del() {
    let p = libs();
    let a = capture_to_file(|| p.c.call(127));
    let b = capture_to_file(|| p.rs.call(127));
    assert_same("driver(127)", &a, &b);
    let txt = String::from_utf8_lossy(&a).into_owned();
    assert!(
        txt.contains("control: 2") && txt.contains("printing: 0") && txt.contains("graphical: 0"),
        "unexpected DEL classification:\n{txt}"
    );
}

/// ERRORS row 6 — every negative `char` (the whole high-bit-set byte range).
#[test]
fn errors_row_06_all_negative_chars() {
    let p = libs();
    for v in -128i16..=-1 {
        let c = v as c_char;
        let a = capture_to_file(|| p.c.call(c));
        let b = capture_to_file(|| p.rs.call(c));
        assert_same(&format!("driver({v})"), &a, &b);
        // Every predicate must be 0 and case conversion the identity byte.
        for line in a.split(|&x| x == b'\n') {
            if line.is_empty() || line.starts_with(b"to ") {
                continue;
            }
            assert!(
                line.ends_with(b" 0"),
                "expected a zero predicate for {v}: {}",
                show(line)
            );
        }
        let byte = v as u8;
        assert!(
            a.windows(11).any(|w| w[..10] == *b"to lower: " && w[10] == byte),
            "identity tolower expected for {v}: {}",
            show(&a)
        );
    }
}

/// ERRORS row 7 — out-of-`char`-range ints across the FFI boundary must be
/// truncated identically (this is the "invalid enum value" class of bug).
#[test]
fn errors_row_07_out_of_range_int_arguments() {
    let p = libs();
    // For each wide value, the result must equal the result of the truncated
    // `char` on BOTH libraries.
    for v in [
        128i32, 200, 255, 256, 257, 300, 511, 512, -129, -200, -256, -257, 65535, 65536, i32::MIN,
        i32::MAX, 0x7FFF_FF80u32 as i32, i32::MIN + 65,
    ] {
        let a = capture_to_file(|| p.c.call_wide(v));
        let b = capture_to_file(|| p.rs.call_wide(v));
        assert_same(&format!("driver(wide {v})"), &a, &b);

        let truncated = (v as u8) as c_char;
        let ref_c = capture_to_file(|| p.c.call(truncated));
        assert_same(
            &format!("wide {v} must alias char {truncated}"),
            &ref_c,
            &a,
        );
    }
}

/// ERRORS row 8 — `' '` (32): print + space + blank, but NOT graph/punct.
#[test]
fn errors_row_08_space_boundary() {
    let p = libs();
    let a = capture_to_file(|| p.c.call(32));
    let b = capture_to_file(|| p.rs.call(32));
    assert_same("driver(' ')", &a, &b);
    let txt = String::from_utf8_lossy(&a).into_owned();
    for expect in [
        "space: 8192",
        "blank: 1",
        "printing: 16384",
        "graphical: 0",
        "punctuation: 0",
        "control: 0",
    ] {
        assert!(txt.contains(expect), "missing `{expect}` in:\n{txt}");
    }
}

/// ERRORS row 9 — `'\t'` (9): cntrl + space + blank.
#[test]
fn errors_row_09_tab_boundary() {
    let p = libs();
    let a = capture_to_file(|| p.c.call(9));
    let b = capture_to_file(|| p.rs.call(9));
    assert_same("driver('\\t')", &a, &b);
    let txt = String::from_utf8_lossy(&a).into_owned();
    for expect in ["control: 2", "space: 8192", "blank: 1", "printing: 0"] {
        assert!(txt.contains(expect), "missing `{expect}` in:\n{txt}");
    }
}

/// ERRORS row 10 — one step past the isprint / isgraph boundaries.
#[test]
fn errors_row_10_print_graph_boundaries() {
    for v in [31i16, 32, 33, 126, 127] {
        diff_char(v as c_char);
    }
    let p = libs();
    let t31 = String::from_utf8_lossy(&capture_to_file(|| p.rs.call(31))).into_owned();
    assert!(t31.contains("control: 2") && t31.contains("printing: 0"));
    let t33 = String::from_utf8_lossy(&capture_to_file(|| p.rs.call(33))).into_owned();
    assert!(t33.contains("graphical: 32768") && t33.contains("punctuation: 4"));
    let t126 = String::from_utf8_lossy(&capture_to_file(|| p.rs.call(126))).into_owned();
    assert!(t126.contains("graphical: 32768") && t126.contains("printing: 16384"));
}

/// ERRORS row 11 — one step below/above the isdigit range.
#[test]
fn errors_row_11_digit_boundaries() {
    for v in [b'/' as i16, b'0' as i16, b'9' as i16, b':' as i16] {
        diff_char(v as c_char);
    }
    let p = libs();
    for v in [b'/', b':'] {
        let t = String::from_utf8_lossy(&capture_to_file(|| p.rs.call(v as c_char))).into_owned();
        assert!(
            t.contains("digit: 0") && t.contains("hexadecimal: 0") && t.contains("punctuation: 4"),
            "unexpected classification for {}:\n{t}",
            v as char
        );
    }
}

/// ERRORS row 12 — one step outside each alpha range.
#[test]
fn errors_row_12_alpha_boundaries() {
    let p = libs();
    for v in [b'@', b'[', b'`', b'{'] {
        let c = v as c_char;
        let a = capture_to_file(|| p.c.call(c));
        let b = capture_to_file(|| p.rs.call(c));
        assert_same(&format!("driver({:?})", v as char), &a, &b);
        let t = String::from_utf8_lossy(&a).into_owned();
        assert!(
            t.contains("alphabetic: 0") && t.contains("punctuation: 4"),
            "unexpected classification for {:?}:\n{t}",
            v as char
        );
        // Case conversion is the identity here.
        assert!(a.windows(11).any(|w| w[..10] == *b"to lower: " && w[10] == v));
        assert!(a.windows(11).any(|w| w[..10] == *b"to upper: " && w[10] == v));
    }
    // And the in-range neighbours still classify as alpha.
    for v in [b'A', b'Z', b'a', b'z'] {
        diff_char(v as c_char);
    }
}

/// ERRORS row 13 — one step past the isxdigit letter ranges.
#[test]
fn errors_row_13_xdigit_boundaries() {
    let p = libs();
    for v in [b'F', b'G', b'f', b'g'] {
        let c = v as c_char;
        let a = capture_to_file(|| p.c.call(c));
        let b = capture_to_file(|| p.rs.call(c));
        assert_same(&format!("driver({:?})", v as char), &a, &b);
        let t = String::from_utf8_lossy(&a).into_owned();
        let expect_hex = v == b'F' || v == b'f';
        assert_eq!(
            t.contains("hexadecimal: 4096"),
            expect_hex,
            "wrong xdigit for {:?}:\n{t}",
            v as char
        );
        assert!(t.contains("alphabetic: 1024"));
    }
}

/// ERRORS row 14 — stdout is a failed fd: every `printf` fails, `driver` still
/// returns normally, and both libraries behave the same.
#[test]
fn errors_row_14_stdout_write_fails() {
    let p = libs();
    // Point fd 1 at O_RDONLY /dev/null, so writes fail with EBADF.
    const O_RDONLY: c_int = 0;
    let path = b"/dev/null\0";
    for &c in &[65i16, 9, -1] {
        let c = c as c_char;
        let mut results = Vec::new();
        for lib in [&p.c, &p.rs] {
            unsafe {
                fflush(std::ptr::null_mut());
                let saved = dup(1);
                let bad = open(path.as_ptr() as *const c_char, O_RDONLY);
                assert!(bad >= 0, "open /dev/null failed");
                assert!(dup2(bad, 1) >= 0);
                // The call must not crash even though writing is impossible.
                lib.call(c);
                let flush_rc = fflush(std::ptr::null_mut());
                assert!(dup2(saved, 1) >= 0);
                close(saved);
                close(bad);
                // Clear the sticky error state on stdout for the next round.
                clearerr_stdout();
                results.push(flush_rc != 0);
            }
        }
        assert_eq!(
            results[0], results[1],
            "C and Rust disagree on whether the flush failed for c={c}"
        );
    }
}

fn clearerr_stdout() {
    unsafe extern "C" {
        fn clearerr(stream: *mut c_void);
        static mut stdout: *mut c_void;
    }
    unsafe {
        let s = stdout;
        if !s.is_null() {
            clearerr(s);
        }
    }
}

/// ERRORS row 15 — the caller's locale is not `"C"` on entry.
#[test]
fn errors_row_15_caller_locale_overwritten() {
    let p = libs();
    for start in ["POSIX", "en_US.UTF-8", "C.UTF-8", "no_SUCH.locale-42", "C"] {
        set_locale(start);
        let a = capture_to_file(|| p.c.call(b'k' as c_char));
        let after_c = query_locale();
        set_locale(start);
        let b = capture_to_file(|| p.rs.call(b'k' as c_char));
        let after_rs = query_locale();
        assert_same(&format!("start-locale={start}"), &a, &b);
        assert_eq!(after_c, after_rs, "locale side effect differs ({start})");
        assert_eq!(after_c, "C");
    }
    set_locale("C");
}

/// Generic FFI boundary: a null pointer cannot be passed (the parameter is a
/// scalar `char`), but a caller can still mis-declare the symbol.  Verify that
/// calling through a *pointer-shaped* signature — the classic mismatch — is
/// handled identically: the low byte of the pointer value is what matters.
#[test]
fn errors_generic_pointer_shaped_argument() {
    let p = libs();
    type DriverPtrFn = unsafe extern "C" fn(*const c_void);
    let cd: DriverPtrFn = unsafe { std::mem::transmute(p.c.raw()) };
    let rd: DriverPtrFn = unsafe { std::mem::transmute(p.rs.raw()) };
    for raw in [0usize, 1, 0x41, 0xFF, 0x100, 0xDEAD_BEEF, usize::MAX] {
        let ptr = raw as *const c_void;
        let a = capture_to_file(|| unsafe { cd(ptr) });
        let b = capture_to_file(|| unsafe { rd(ptr) });
        assert_same(&format!("pointer-shaped arg {raw:#x}"), &a, &b);
        // It must alias the truncated `char`.
        let truncated = (raw as u8) as c_char;
        let refc = capture_to_file(|| p.c.call(truncated));
        assert_same(&format!("{raw:#x} aliases char {truncated}"), &refc, &a);
    }
}
