//! Lowest level of the call hierarchy: `void printLine(const char *)`.
//!
//! Both implementations are reached only through `dlsym` on their respective
//! shared objects, so the `#[unsafe(no_mangle)]` export wrapper is part of what
//! is under test.
//!
//! The whole file is driven from a single `#[test]`. Capturing output means
//! redirecting the process-wide file descriptor 1, and libtest writes its own
//! per-test progress lines to that descriptor from other threads; one test
//! entry point keeps those writes out of the captured bytes.

mod common;

use common::{capture_stdout, print_line_fns, show};
use std::ffi::{CString, c_char};

/// Feeds one NUL-terminated byte string to both `printLine`s and returns an
/// error describing any difference in the captured stdout.
fn check(input: &[u8]) -> Result<(), String> {
    let cstr = CString::new(input).expect("test input must not contain NUL");
    let ptr = cstr.as_ptr();
    let (c_fn, rust_fn) = print_line_fns();

    let c_out = capture_stdout(|| unsafe { c_fn(ptr) });
    let rust_out = capture_stdout(|| unsafe { rust_fn(ptr) });

    if c_out == rust_out {
        Ok(())
    } else {
        Err(format!(
            "printLine({:?}) mismatch\n    C:    {}\n    Rust: {}",
            String::from_utf8_lossy(input),
            show(&c_out),
            show(&rust_out)
        ))
    }
}

/// NULL is the one input `printLine` special-cases: nothing is printed.
fn case_null() -> Result<(), String> {
    let (c_fn, rust_fn) = print_line_fns();

    let c_out = capture_stdout(|| unsafe { c_fn(std::ptr::null()) });
    let rust_out = capture_stdout(|| unsafe { rust_fn(std::ptr::null()) });

    if !c_out.is_empty() {
        return Err(format!("C printLine(NULL) wrote {}", show(&c_out)));
    }
    if c_out != rust_out {
        return Err(format!(
            "printLine(NULL) mismatch\n    C:    {}\n    Rust: {}",
            show(&c_out),
            show(&rust_out)
        ));
    }
    Ok(())
}

/// The buffer `driver` hands to `printLine` is a 100 byte array whose content
/// stops at the first NUL, so a pointer into a larger buffer must read only up
/// to that NUL on both sides.
fn case_stops_at_first_nul() -> Result<(), String> {
    let mut buf = [b'Z'; 100];
    buf[10] = 0;
    let ptr = buf.as_ptr() as *const c_char;
    let (c_fn, rust_fn) = print_line_fns();

    let c_out = capture_stdout(|| unsafe { c_fn(ptr) });
    let rust_out = capture_stdout(|| unsafe { rust_fn(ptr) });

    if c_out != b"ZZZZZZZZZZ\n" {
        return Err(format!("unexpected C output {}", show(&c_out)));
    }
    if c_out != rust_out {
        return Err(format!(
            "embedded-NUL mismatch\n    C:    {}\n    Rust: {}",
            show(&c_out),
            show(&rust_out)
        ));
    }
    Ok(())
}

#[test]
fn print_line_matches_c() {
    let mut failures: Vec<String> = Vec::new();
    let mut record = |name: &str, r: Result<(), String>| {
        if let Err(e) = r {
            failures.push(format!("[{name}] {e}"));
        }
    };

    record("null", case_null());
    record("stops_at_first_nul", case_stops_at_first_nul());

    // Empty string and plain text.
    for s in [
        &b""[..],
        b"A",
        b"AB",
        b"hello world",
        b"trailing space ",
        b" leading space",
        b"tab\there",
        b"embedded\nnewline",
    ] {
        record("plain", check(s));
    }

    // `printf("%s\n", line)` treats `line` as data, so conversion specifiers in
    // the payload must come out verbatim.
    for s in [
        &b"%s"[..],
        b"%d",
        b"%%",
        b"%n",
        b"100%",
        b"%s %d %x %p %n %%",
        b"{}",
    ] {
        record("format_specifiers", check(s));
    }

    // Non-ASCII and non-UTF-8 bytes are copied through unchanged.
    record("utf8", check("héllo wörld".as_bytes()));
    record("high_bytes", check(&[0x80, 0xfe, 0xff, 0x41]));
    record("all_nonzero_bytes", check(&(1u8..=255).collect::<Vec<u8>>()));

    // Lengths around the libc buffer sizes and around `driver`'s 100 bytes.
    for len in [1usize, 63, 64, 99, 100, 1000, 4095, 4096, 8192] {
        record("length", check(&vec![b'A'; len]));
    }

    assert!(
        failures.is_empty(),
        "{} printLine case(s) differ:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
