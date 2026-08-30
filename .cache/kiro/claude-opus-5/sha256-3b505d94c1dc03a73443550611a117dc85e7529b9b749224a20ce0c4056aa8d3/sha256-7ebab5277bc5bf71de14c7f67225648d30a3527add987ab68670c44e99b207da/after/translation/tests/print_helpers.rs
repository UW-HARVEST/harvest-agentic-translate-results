//! Differential tests for the two leaf output helpers, `printLine` and
//! `printIntLine`.

mod common;

use common::{c_api, capture, rust_api, show, Rng};
use std::ffi::{c_char, CString};

fn compare_print_line(bytes_with_nul: &[u8], label: &str) {
    let ptr = bytes_with_nul.as_ptr() as *const c_char;
    let from_c = capture(|| unsafe { (c_api().print_line)(ptr) });
    let from_rust = capture(|| unsafe { (rust_api().print_line)(ptr) });
    assert_eq!(
        from_c,
        from_rust,
        "printLine({label}): C {} != Rust {}",
        show(&from_c),
        show(&from_rust)
    );
}

fn compare_print_int_line(value: i32) {
    let from_c = capture(|| unsafe { (c_api().print_int_line)(value) });
    let from_rust = capture(|| unsafe { (rust_api().print_int_line)(value) });
    assert_eq!(
        from_c,
        from_rust,
        "printIntLine({value}): C {} != Rust {}",
        show(&from_c),
        show(&from_rust)
    );
}

fn print_line_null_pointer_prints_nothing() {
    let from_c = capture(|| unsafe { (c_api().print_line)(std::ptr::null()) });
    let from_rust = capture(|| unsafe { (rust_api().print_line)(std::ptr::null()) });
    assert_eq!(from_c, b"", "C printLine(NULL) should print nothing");
    assert_eq!(from_c, from_rust);
}

fn print_line_fixed_strings() {
    let cases: &[&[u8]] = &[
        b"\0",
        b" \0",
        b"a\0",
        b"Calling good()...\0",
        b"Finished good()\0",
        b"Calling bad()...\0",
        b"Finished bad()\0",
        b"This would result in a divide by zero\0",
        // Contents that look like format directives must be printed verbatim,
        // since they arrive as an argument to `%s`.
        b"%d %s %n %%\0",
        b"100%\0",
        // Embedded control characters and newlines.
        b"line1\nline2\0",
        b"tab\there\0",
        b"\x01\x02\x7f\0",
        // Non ASCII bytes pass through untouched.
        "unicode: \u{2764} \u{1F600}\0".as_bytes(),
        b"\xff\xfe\xfd\0",
    ];

    for case in cases {
        compare_print_line(case, &show(&case[..case.len() - 1]));
    }
}

fn print_line_long_strings() {
    for len in [1usize, 63, 64, 255, 1024, 4095, 4096, 8192] {
        let mut buf = vec![b'x'; len];
        buf.push(0);
        compare_print_line(&buf, &format!("{len} x's"));
    }
}

fn print_line_random_strings() {
    let mut rng = Rng::new(0xA11CE);
    for _ in 0..256 {
        let len = (rng.next_u32() % 48) as usize;
        let mut bytes = Vec::with_capacity(len + 1);
        for _ in 0..len {
            // Any non NUL byte is a legal `char *` payload.
            bytes.push(1 + (rng.next_u32() % 255) as u8);
        }
        bytes.push(0);
        let label = show(&bytes[..bytes.len() - 1]);
        compare_print_line(&bytes, &label);
    }
}

fn print_line_accepts_cstring_allocations() {
    // Exercises a heap allocated, non static pointer as well.
    for text in ["", "cstring", "with spaces and 42"] {
        let owned = CString::new(text).unwrap();
        let ptr = owned.as_ptr();
        let from_c = capture(|| unsafe { (c_api().print_line)(ptr) });
        let from_rust = capture(|| unsafe { (rust_api().print_line)(ptr) });
        assert_eq!(from_c, from_rust, "printLine({text:?})");
    }
}

fn print_int_line_boundary_values() {
    let cases = [
        0i32,
        1,
        -1,
        7,
        -7,
        9,
        10,
        99,
        100,
        -100,
        32767,
        -32768,
        65535,
        1_000_000_000,
        -1_000_000_000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    for value in cases {
        compare_print_int_line(value);
    }
}

fn print_int_line_random_values() {
    let mut rng = Rng::new(0xBEEF);
    for _ in 0..2048 {
        compare_print_int_line(rng.next_i32());
    }
}

fn main() {
    common::run_suite(
        "print_helpers",
        &[
        ("print_line_null_pointer_prints_nothing", print_line_null_pointer_prints_nothing),
        ("print_line_fixed_strings", print_line_fixed_strings),
        ("print_line_long_strings", print_line_long_strings),
        ("print_line_random_strings", print_line_random_strings),
        ("print_line_accepts_cstring_allocations", print_line_accepts_cstring_allocations),
        ("print_int_line_boundary_values", print_int_line_boundary_values),
        ("print_int_line_random_values", print_int_line_random_values),
        ],
    )
}
