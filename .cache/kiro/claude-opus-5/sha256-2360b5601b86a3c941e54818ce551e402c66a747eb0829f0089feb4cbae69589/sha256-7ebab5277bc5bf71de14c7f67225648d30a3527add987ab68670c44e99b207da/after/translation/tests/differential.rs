//! Differential tests: every call goes through `dlopen`/`dlsym` on the two
//! shared objects, so the `#[unsafe(no_mangle)]` export wrappers are exercised
//! exactly as an external C caller would exercise them.
//!
//! Tests are ordered from the lowest-level leaf functions upward:
//!   printIntLine / printLine  ->  bad / good  ->  driver

mod common;

use common::{Impl, assert_both_export, assert_same_bytes, sym};
use std::ffi::{CString, c_char, c_int};

type FnVoid = unsafe extern "C" fn();
type FnInt = unsafe extern "C" fn(c_int);
type FnStr = unsafe extern "C" fn(*const c_char);

// ---------------------------------------------------------------------------
// Symbol parity
// ---------------------------------------------------------------------------

fn all_c_symbols_are_exported_by_rust() {
    for name in ["printLine", "printIntLine", "bad", "good", "driver"] {
        assert_both_export(name);

        // Guard against a false pass: if both handles resolved to the same
        // object every comparison below would be trivially satisfied.
        let c: libloading::Symbol<FnVoid> = sym(Impl::C, name);
        let r: libloading::Symbol<FnVoid> = sym(Impl::Rust, name);
        let (cp, rp) = (*c as *const (), *r as *const ());
        assert_ne!(
            cp, rp,
            "`{name}` resolved to the same address in both libraries; the C and \
             Rust implementations are not actually distinct"
        );
    }
}

// ---------------------------------------------------------------------------
// Level 0: printIntLine
// ---------------------------------------------------------------------------

fn print_int_line_matches() {
    let c: libloading::Symbol<FnInt> = sym(Impl::C, "printIntLine");
    let r: libloading::Symbol<FnInt> = sym(Impl::Rust, "printIntLine");

    let cases: [c_int; 13] = [
        0,
        1,
        -1,
        7,
        -7,
        42,
        -42,
        1000,
        -1000,
        123456789,
        -123456789,
        c_int::MAX,
        c_int::MIN,
    ];

    for v in cases {
        assert_same_bytes(
            &format!("printIntLine({v})"),
            || unsafe { c(v) },
            || unsafe { r(v) },
        );
    }
}

fn print_int_line_repeated_calls_match() {
    let c: libloading::Symbol<FnInt> = sym(Impl::C, "printIntLine");
    let r: libloading::Symbol<FnInt> = sym(Impl::Rust, "printIntLine");

    // Many calls in one capture: verifies ordering and that no extra bytes
    // (padding, separators) are emitted per call.
    assert_same_bytes(
        "printIntLine sequence",
        || unsafe {
            for i in -50..50 {
                c(i);
            }
        },
        || unsafe {
            for i in -50..50 {
                r(i);
            }
        },
    );
}

// ---------------------------------------------------------------------------
// Level 0: printLine
// ---------------------------------------------------------------------------

fn print_line_null_matches() {
    let c: libloading::Symbol<FnStr> = sym(Impl::C, "printLine");
    let r: libloading::Symbol<FnStr> = sym(Impl::Rust, "printLine");

    // NULL must produce no output at all in both implementations.
    assert_same_bytes(
        "printLine(NULL)",
        || unsafe { c(std::ptr::null()) },
        || unsafe { r(std::ptr::null()) },
    );

    let out = common::capture_stdout(|| unsafe { r(std::ptr::null()) });
    assert!(out.is_empty(), "printLine(NULL) must print nothing, got {out:?}");
}

fn print_line_strings_match() {
    let c: libloading::Symbol<FnStr> = sym(Impl::C, "printLine");
    let r: libloading::Symbol<FnStr> = sym(Impl::Rust, "printLine");

    let cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"hello".to_vec(),
        b"hello world".to_vec(),
        b"  leading and trailing  ".to_vec(),
        b"tab\there".to_vec(),
        b"embedded\nnewline".to_vec(),
        b"percent %d %s %% literal".to_vec(),
        b"backslash \\n not a newline".to_vec(),
        b"\x01\x02\x7f high-ish control bytes".to_vec(),
        b"\xc3\xa9\xe2\x82\xac utf8 bytes".to_vec(),
        b"\xff\xfe invalid utf8 bytes".to_vec(),
        vec![b'x'; 1],
        vec![b'y'; 1024],
        vec![b'z'; 8192],
    ];

    for bytes in cases {
        let s = CString::new(bytes.clone()).expect("no interior NUL in test inputs");
        let label = format!("printLine({:?})", String::from_utf8_lossy(&bytes));
        let p = s.as_ptr();
        assert_same_bytes(&label, || unsafe { c(p) }, || unsafe { r(p) });
    }
}

fn print_line_sequence_matches() {
    let c: libloading::Symbol<FnStr> = sym(Impl::C, "printLine");
    let r: libloading::Symbol<FnStr> = sym(Impl::Rust, "printLine");

    let a = CString::new("first").unwrap();
    let b = CString::new("second").unwrap();
    let empty = CString::new("").unwrap();

    // Interleave NULL with real strings to confirm the NULL guard does not
    // disturb the surrounding output.
    let run = |f: &FnStr| unsafe {
        f(a.as_ptr());
        f(std::ptr::null());
        f(empty.as_ptr());
        f(b.as_ptr());
        f(std::ptr::null());
    };
    assert_same_bytes("printLine interleaved with NULL", || run(&c), || run(&r));
}

// ---------------------------------------------------------------------------
// Level 1: bad / good
// ---------------------------------------------------------------------------

fn bad_matches() {
    let c: libloading::Symbol<FnVoid> = sym(Impl::C, "bad");
    let r: libloading::Symbol<FnVoid> = sym(Impl::Rust, "bad");
    assert_same_bytes("bad()", || unsafe { c() }, || unsafe { r() });
}

fn good_matches() {
    let c: libloading::Symbol<FnVoid> = sym(Impl::C, "good");
    let r: libloading::Symbol<FnVoid> = sym(Impl::Rust, "good");
    assert_same_bytes("good()", || unsafe { c() }, || unsafe { r() });
}

fn bad_and_good_are_idempotent_across_calls() {
    let cb: libloading::Symbol<FnVoid> = sym(Impl::C, "bad");
    let rb: libloading::Symbol<FnVoid> = sym(Impl::Rust, "bad");
    let cg: libloading::Symbol<FnVoid> = sym(Impl::C, "good");
    let rg: libloading::Symbol<FnVoid> = sym(Impl::Rust, "good");

    assert_same_bytes(
        "bad()/good() repeated and interleaved",
        || unsafe {
            for _ in 0..16 {
                cb();
                cg();
            }
        },
        || unsafe {
            for _ in 0..16 {
                rb();
                rg();
            }
        },
    );
}

// ---------------------------------------------------------------------------
// Level 2: driver (public API from driver.h)
// ---------------------------------------------------------------------------

fn driver_matches_for_all_truthiness_classes() {
    let c: libloading::Symbol<FnInt> = sym(Impl::C, "driver");
    let r: libloading::Symbol<FnInt> = sym(Impl::Rust, "driver");

    // 0 selects bad(); every non-zero value, including negatives and the
    // extremes, selects good().
    let cases: [c_int; 10] = [0, 1, -1, 2, -2, 255, 256, 65536, c_int::MAX, c_int::MIN];

    for v in cases {
        assert_same_bytes(
            &format!("driver({v})"),
            || unsafe { c(v) },
            || unsafe { r(v) },
        );
    }
}

fn driver_sequence_matches() {
    let c: libloading::Symbol<FnInt> = sym(Impl::C, "driver");
    let r: libloading::Symbol<FnInt> = sym(Impl::Rust, "driver");

    let pattern: [c_int; 12] = [0, 1, 1, 0, 0, 0, 1, -5, 0, i32::MAX, 0, 1];
    assert_same_bytes(
        "driver() sequence",
        || unsafe {
            for v in pattern {
                c(v);
            }
        },
        || unsafe {
            for v in pattern {
                r(v);
            }
        },
    );
}

fn driver_output_shape_is_a_single_decimal_line() {
    // Pin the observable contract rather than only comparing the two sides, so
    // a shared regression (e.g. both printing nothing) would still be caught.
    let c: libloading::Symbol<FnInt> = sym(Impl::C, "driver");
    for v in [0, 1] {
        let out = common::capture_stdout(|| unsafe { c(v) });
        assert_eq!(out, b"0\n", "driver({v}) C output");
    }
}

// ---------------------------------------------------------------------------
// Sequential runner
//
// A custom harness is used instead of libtest because the checks above capture
// file descriptor 1; libtest's concurrent progress output would land inside
// those captures and be mistaken for library output.
// ---------------------------------------------------------------------------

fn main() {
    // Supports the usual `cargo test -- <substring>` filtering.
    let filters: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();

    let cases: &[(&str, fn())] = &[
        (
            "all_c_symbols_are_exported_by_rust",
            all_c_symbols_are_exported_by_rust,
        ),
        ("print_int_line_matches", print_int_line_matches),
        (
            "print_int_line_repeated_calls_match",
            print_int_line_repeated_calls_match,
        ),
        ("print_line_null_matches", print_line_null_matches),
        ("print_line_strings_match", print_line_strings_match),
        ("print_line_sequence_matches", print_line_sequence_matches),
        ("bad_matches", bad_matches),
        ("good_matches", good_matches),
        (
            "bad_and_good_are_idempotent_across_calls",
            bad_and_good_are_idempotent_across_calls,
        ),
        (
            "driver_matches_for_all_truthiness_classes",
            driver_matches_for_all_truthiness_classes,
        ),
        ("driver_sequence_matches", driver_sequence_matches),
        (
            "driver_output_shape_is_a_single_decimal_line",
            driver_output_shape_is_a_single_decimal_line,
        ),
    ];

    let selected: Vec<_> = cases
        .iter()
        .filter(|(name, _)| filters.is_empty() || filters.iter().any(|f| name.contains(f.as_str())))
        .collect();

    eprintln!("\nrunning {} differential tests", selected.len());
    let mut failed = Vec::new();

    for (name, f) in selected {
        // Results go to stderr so they can never be confused with, or captured
        // alongside, the library output being compared on fd 1.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match outcome {
            Ok(()) => eprintln!("test {name} ... ok"),
            Err(_) => {
                eprintln!("test {name} ... FAILED");
                failed.push(*name);
            }
        }
    }

    if failed.is_empty() {
        eprintln!("\ntest result: ok. all differential tests passed\n");
    } else {
        eprintln!("\ntest result: FAILED. failing tests: {failed:?}\n");
        std::process::exit(1);
    }
}
