//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Every function in this library returns
//! `void`, so "the same error/rejection" means the same observable rejection
//! behaviour: the identical byte stream on `stdout` (for E1: none at all) and a
//! normal return rather than a crash.

mod common;

use common::*;
use std::ffi::c_char;

// ── E1: the library's only rejection branch ──────────────────────────────────
/// `printLine(NULL)` — `driver.c:31` false arm. Must write zero bytes and
/// return normally in both implementations.
#[test]
fn err_e1_print_line_null() {
    let l = libs();
    let c_out = capture(|| unsafe { (l.c.print_line)(std::ptr::null()) });
    let r_out = capture(|| unsafe { (l.rust.print_line)(std::ptr::null()) });
    assert_eq!(
        c_out,
        Vec::<u8>::new(),
        "C printLine(NULL) unexpectedly wrote {:?}",
        String::from_utf8_lossy(&c_out)
    );
    assert_eq!(
        r_out,
        Vec::<u8>::new(),
        "Rust printLine(NULL) unexpectedly wrote {:?}",
        String::from_utf8_lossy(&r_out)
    );
    assert_eq!(c_out, r_out, "printLine(NULL) rejection differs");
}

// ── G1: NULL repeated and interleaved with valid calls ───────────────────────
#[test]
fn err_g1_null_interleaved() {
    let valid = b"still alive\0";
    assert_same("NULL x8", |api| unsafe {
        for _ in 0..8 {
            (api.print_line)(std::ptr::null())
        }
    });
    assert_same("NULL interleaved with valid calls", |api| unsafe {
        (api.print_line)(std::ptr::null());
        (api.print_line)(valid.as_ptr() as *const c_char);
        (api.print_line)(std::ptr::null());
        (api.print_int_line)(-7);
        (api.print_line)(std::ptr::null());
        (api.driver)();
        (api.print_line)(std::ptr::null());
        (api.print_line)(valid.as_ptr() as *const c_char);
    });
    // A NULL call must not corrupt state for the next valid call.
    let mut rng = Rng::seeded(SEED ^ 0xA1);
    for i in 0..128 {
        let len = rng.range(0, 32) as usize;
        let mut buf: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
        buf.push(0);
        assert_same(&format!("null-then-valid #{i}"), |api| unsafe {
            (api.print_line)(std::ptr::null());
            (api.print_line)(buf.as_ptr() as *const c_char);
        });
    }
}

// ── G2: zero-length but non-NULL ─────────────────────────────────────────────
#[test]
fn err_g2_empty_string() {
    let l = libs();
    let empty = b"\0";
    let c_out = capture(|| unsafe { (l.c.print_line)(empty.as_ptr() as *const c_char) });
    let r_out = capture(|| unsafe { (l.rust.print_line)(empty.as_ptr() as *const c_char) });
    assert_eq!(c_out, b"\n".to_vec(), "C printLine(\"\") should emit one \\n");
    assert_eq!(c_out, r_out, "printLine(\"\") differs");
}

// ── G3: oversized lengths ────────────────────────────────────────────────────
#[test]
fn err_g3_oversized_string() {
    for len in [4095usize, 4096, 4097, 65535, 65536, 1024 * 1024] {
        let s = vec![b'Z'; len];
        assert_same_print_line(&format!("oversized len={len}"), &s);
    }
}

// ── G4: format specifiers in content (never treated as a format string) ──────
#[test]
fn err_g4_format_specifiers_in_content() {
    for s in [
        &b"%s"[..],
        b"%n%n%n%n",
        b"%99999999d",
        b"%%%%",
        b"%1$s %2$s",
        b"AAAA%08x.%08x.%08x.%08x",
        b"%.2147483647f",
        b"%hhn",
    ] {
        assert_same_print_line(
            &format!("format specifier {:?}", String::from_utf8_lossy(s)),
            s,
        );
    }
}

// ── G5: arbitrary non-UTF-8 bytes ────────────────────────────────────────────
#[test]
fn err_g5_non_utf8_bytes() {
    // Every non-NUL byte value, as a whole run.
    let all: Vec<u8> = (1u8..=255).collect();
    assert_same_print_line("all bytes 0x01..0xFF", &all);
    // Lone continuation bytes / truncated sequences / overlong forms.
    for s in [
        &b"\x80"[..],
        b"\xff\xfe",
        b"\xc3",          // truncated 2-byte sequence
        b"\xe2\x82",      // truncated 3-byte sequence
        b"\xf0\x9f\x92",  // truncated 4-byte sequence
        b"\xc0\xaf",      // overlong encoding of '/'
        b"\xed\xa0\x80",  // UTF-16 surrogate half
        b"ok\xffmixed\x80",
    ] {
        assert_same_print_line(&format!("invalid utf8 {s:?}"), s);
    }
}

// ── G6: int extremes ─────────────────────────────────────────────────────────
#[test]
fn err_g6_int_extremes() {
    let l = libs();
    for v in [i32::MIN, -1, 0, 1, i32::MAX] {
        assert_same_print_int_line(&format!("extreme {v}"), v);
    }
    // Also pin the expected rendering of INT_MIN, the classic negation trap.
    for api in [&l.c, &l.rust] {
        let out = capture(|| unsafe { (api.print_int_line)(i32::MIN) });
        assert_eq!(
            String::from_utf8_lossy(&out),
            "-2147483648\n",
            "{} printIntLine(INT_MIN)",
            api.name
        );
    }
}

// ── G7: one step past every decimal-width boundary ───────────────────────────
#[test]
fn err_g7_int_width_boundaries() {
    let mut p: i64 = 1;
    for _ in 1..=9 {
        p *= 10;
        for v in [p - 1, p, p + 1, -(p - 1), -p, -(p + 1)] {
            assert_same_print_int_line(&format!("boundary {v}"), v as i32);
        }
    }
    // Just past INT_MAX / INT_MIN, wrapped the way a C caller's int would wrap.
    for v in [
        (i32::MAX as i64 + 1) as i32,
        (i32::MIN as i64 - 1) as i32,
        i32::MAX - 1,
        i32::MIN + 1,
    ] {
        assert_same_print_int_line(&format!("past-range {v}"), v);
    }
}

// ── G8: arbitrary bit patterns where an enum-typed parameter would sit ───────
/// The C API declares no `enum` parameter, so the out-of-range-enum class maps
/// onto `int`: every 32-bit pattern is a legal input and must render the same.
/// Includes patterns a naive enum-style `match` in Rust would fall through on.
#[test]
fn err_g8_int_arbitrary_bit_patterns() {
    for v in [
        0i32,
        -1,
        i32::MIN,
        i32::MAX,
        0x5555_5555u32 as i32,
        0xAAAA_AAAAu32 as i32,
        0xDEAD_BEEFu32 as i32,
        0xFFFF_FFFFu32 as i32,
        0x8000_0001u32 as i32,
        0x7FFF_FFFE,
        99999,
        -99999,
    ] {
        assert_same_print_int_line(&format!("bit pattern {v:#010x}", v = v as u32), v);
    }
    let mut rng = Rng::seeded(SEED ^ 8);
    for i in 0..1024 {
        let v = rng.next_i32();
        assert_same_print_int_line(&format!("random bit pattern #{i}"), v);
    }
}

// ── The void-return functions have no invalid input to reject ────────────────
/// `bad`, `good` and `driver` take no parameters, so their only "error surface"
/// is being called at all — under any repetition and ordering they must agree.
#[test]
fn err_no_arg_functions_have_no_rejection_path() {
    assert_same("no-arg ordering permutation", |api| unsafe {
        (api.bad)();
        (api.good)();
        (api.driver)();
        (api.good)();
        (api.bad)();
    });
}
