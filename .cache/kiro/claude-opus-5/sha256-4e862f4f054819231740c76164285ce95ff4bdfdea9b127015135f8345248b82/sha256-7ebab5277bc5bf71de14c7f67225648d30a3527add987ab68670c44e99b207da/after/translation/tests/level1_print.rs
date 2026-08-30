//! Level 1: the leaf output primitives `printLine` and `printIntLine`.

mod common;

use common::{FnVoidCharPtr, FnVoidInt, assert_same, capture_stdout, libs, show, sym};
use std::ffi::{CString, c_char};

#[test]
fn print_line_matches_for_assorted_strings() {
    let cases: &[&[u8]] = &[
        b"",
        b" ",
        b"a",
        b"Calling good()...",
        b"Finished good()",
        b"Calling bad()...",
        b"Finished bad()",
        b"ERROR: Array index is negative.",
        b"ERROR: Array index is out-of-bounds",
        b"%d %s %n %%",              // format specifiers must be data, not format
        b"tab\there",
        b"embedded\nnewline",
        b"trailing space ",
        b"\x01\x02\x7f",             // non-printable bytes
        b"\xc3\xa9\xe2\x82\xac",     // UTF-8 multibyte
        b"\xff\xfe",                 // invalid UTF-8
    ];

    for case in cases {
        let s = CString::new(*case).expect("no interior NUL");
        assert_same::<FnVoidCharPtr, _>(
            "printLine",
            |f| unsafe { f(s.as_ptr()) },
            &format!("{:?}", show(case)),
        );
    }
}

#[test]
fn print_line_matches_for_long_strings() {
    for len in [1usize, 63, 64, 255, 1023, 4096, 8192] {
        let buf = vec![b'x'; len];
        let s = CString::new(buf).unwrap();
        assert_same::<FnVoidCharPtr, _>(
            "printLine",
            |f| unsafe { f(s.as_ptr()) },
            &format!("{len} 'x' bytes"),
        );
    }
}

/// The C guards on `line != NULL` and prints nothing; the Rust wrapper must do
/// the same rather than dereferencing or printing "(null)".
#[test]
fn print_line_null_prints_nothing_in_both() {
    assert_same::<FnVoidCharPtr, _>(
        "printLine",
        |f| unsafe { f(std::ptr::null::<c_char>()) },
        "NULL",
    );

    let l = libs();
    let c: libloading::Symbol<'static, FnVoidCharPtr> = unsafe { sym(&l.c, "printLine") };
    let out = capture_stdout(|| unsafe { c(std::ptr::null()) });
    assert!(
        out.is_empty(),
        "C printLine(NULL) unexpectedly printed \"{}\"",
        show(&out)
    );
}

#[test]
fn print_int_line_matches_for_boundary_and_random_ints() {
    let mut cases: Vec<i32> = vec![
        0,
        1,
        -1,
        7,
        9,
        10,
        -10,
        99,
        100,
        -99,
        32767,
        -32768,
        65535,
        -65536,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];

    // Deterministic pseudo-random sweep (xorshift) over the full i32 range.
    let mut state: u32 = 0x2545_f491;
    for _ in 0..500 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        cases.push(state as i32);
    }

    for v in cases {
        assert_same::<FnVoidInt, _>("printIntLine", |f| unsafe { f(v) }, &format!("{v}"));
    }
}
