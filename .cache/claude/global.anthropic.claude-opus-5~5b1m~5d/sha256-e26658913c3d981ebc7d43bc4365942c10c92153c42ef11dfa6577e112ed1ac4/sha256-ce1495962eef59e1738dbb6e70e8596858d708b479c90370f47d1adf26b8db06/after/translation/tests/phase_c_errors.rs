// Phase C -- error/rejection-path differential tests.
//
// One test per row of ERRORS.md, plus the generic FFI boundary conditions.
//
// Every function in this library is `void`, so "the same error code or
// sentinel" takes its only available form here: the two implementations must
// take the SAME branch, observable as the same captured stdout bytes (for the
// null-pointer row: exactly zero bytes from both, and no crash), and must leave
// the library in the same state for the following call.

mod common;

use common::{
    assert_same, bad, call_void_with_extra_args, diff_print_line, driver, exports, good,
    print_line, print_line_raw, Impl, Rng, SEED,
};

use std::ffi::c_char;

// ---------------------------------------------------------------------------
// ERRORS.md row 1 -- printLine(NULL)
//
// driver.c:31  `if (line != NULL)` is false => puts() is NOT called.
// Expected C result: zero bytes on stdout, normal return, no crash.
// ---------------------------------------------------------------------------

#[test]
fn err_01_print_line_null() {
    let c_out = print_line_raw(Impl::C, std::ptr::null());
    let rust_out = print_line_raw(Impl::Rust, std::ptr::null());

    // Same rejection on both sides ...
    assert_same("printLine(NULL)", &c_out, &rust_out);
    // ... and specifically the C-defined one: the guard suppressed all output,
    // and the call returned normally rather than crashing.
    assert_eq!(
        c_out.ok_stdout(),
        b"",
        "C printLine(NULL) must write nothing (driver.c:31 guard)"
    );
    assert_eq!(
        rust_out.ok_stdout(),
        b"",
        "Rust printLine(NULL) must write nothing, matching the C null guard"
    );
}

#[test]
fn err_01b_print_line_null_repeated_and_interleaved() {
    // A rejected call must not corrupt state or output for the calls around it:
    // NULL, then a real string, then NULL again.
    for i in 0..32 {
        assert_same(
            &format!("printLine(NULL) repeat {i}"),
            &print_line_raw(Impl::C, std::ptr::null()),
            &print_line_raw(Impl::Rust, std::ptr::null()),
        );
        diff_print_line(&format!("printLine(\"after null\") repeat {i}"), b"after null");
        assert_same(
            &format!("bad() after NULL, repeat {i}"),
            &bad(Impl::C),
            &bad(Impl::Rust),
        );
    }
    // And the composed entry points still behave after many rejections.
    assert_same("good() after NULLs", &good(Impl::C), &good(Impl::Rust));
    assert_same("driver() after NULLs", &driver(Impl::C), &driver(Impl::Rust));
}

// ---------------------------------------------------------------------------
// Generic boundary: zero length (the empty string)
// ---------------------------------------------------------------------------

#[test]
fn err_02_print_line_empty() {
    let c_out = print_line(Impl::C, b"");
    let rust_out = print_line(Impl::Rust, b"");
    assert_same("printLine(\"\")", &c_out, &rust_out);

    // The empty string is a VALID (non-null) input, so unlike NULL it must
    // still emit the newline. Pinning this distinguishes the two branches --
    // an implementation that treated "" like NULL would pass a plain
    // C-vs-Rust diff only if BOTH were wrong.
    assert_eq!(
        c_out.ok_stdout(),
        b"\n",
        "C printLine(\"\") must print just the newline, not nothing"
    );
}

// ---------------------------------------------------------------------------
// Generic boundary: oversized input
//
// printLine takes no length argument, so "oversized" means a very long
// NUL-terminated string -- far past libc's stdout buffer, and past a page.
// ---------------------------------------------------------------------------

#[test]
fn err_03_print_line_oversized() {
    let mut rng = Rng::new(SEED ^ 0x03);
    for len in [
        4096usize, 8192, 65_536, 1_000_000, 4 * 1024 * 1024,
    ] {
        let s = vec![b'A'; len];
        diff_print_line(&format!("oversized all-'A' len {len}"), &s);

        let s = rng.bytes_nonzero(len.min(1 << 20));
        diff_print_line(&format!("oversized random len {}", s.len()), &s);
    }
}

// ---------------------------------------------------------------------------
// Generic boundary: non-UTF-8 bytes
//
// `puts` is byte-oriented. Rust strings are not. If the translation ever went
// through `str`/`String` it would either reject or replace these bytes; the C
// passes them straight through.
// ---------------------------------------------------------------------------

#[test]
fn err_04_print_line_non_utf8() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0xff],
        vec![0xfe, 0xff],
        vec![0x80],
        vec![0xc0, 0x80],       // overlong encoding of NUL
        vec![0xed, 0xa0, 0x80], // UTF-16 surrogate half
        vec![0xf5, 0x80, 0x80, 0x80], // > U+10FFFF
        vec![0xc3],             // truncated 2-byte sequence
        vec![0xe2, 0x82],       // truncated 3-byte sequence
        (1u8..=255).collect(),  // every non-NUL byte at once
        (1u8..=255).rev().collect(),
    ];

    for (i, s) in cases.iter().enumerate() {
        diff_print_line(&format!("non-utf8 case {i} ({} bytes)", s.len()), s);
    }
}

// ---------------------------------------------------------------------------
// Generic boundary: buffer whose FIRST byte is the terminator, reached through
// an interior pointer -- i.e. a legitimately empty string that is not `b""`.
// ---------------------------------------------------------------------------

#[test]
fn err_05_print_line_interior_nul() {
    // "abc\0def\0": pointing at index 0 must print "abc", index 3 must print
    // the empty line, index 4 must print "def". The bytes past the first NUL
    // must never be read.
    let buf: &[u8] = b"abc\0def\0";
    for offset in [0usize, 1, 2, 3, 4, 7] {
        let ptr = unsafe { buf.as_ptr().add(offset) } as *const c_char;
        let c_out = print_line_raw(Impl::C, ptr);
        let rust_out = print_line_raw(Impl::Rust, ptr);
        assert_same(&format!("interior NUL offset {offset}"), &c_out, &rust_out);
    }
}

// ---------------------------------------------------------------------------
// Generic boundary: control and whitespace bytes, including embedded newlines
// (which must NOT change how many bytes puts appends).
// ---------------------------------------------------------------------------

#[test]
fn err_06_print_line_control_bytes() {
    for s in [
        &b"\n"[..],
        b"\r",
        b"\r\n",
        b"\n\n\n",
        b"\t",
        b"\x1b[31mred\x1b[0m",
        b"\x07\x08\x0b\x0c",
        b"line1\nline2\nline3",
        b" ",
        b"   trailing   ",
    ] {
        diff_print_line(&format!("control bytes {s:?}"), s);
    }
}

// ---------------------------------------------------------------------------
// Out-of-range enum values / extra arguments across the FFI boundary.
//
// ERRORS.md records that this class is NOT INSTANTIABLE for this library: it
// declares no enum and no function takes an integer, so there is no int without
// a valid variant to pass. This test pins that fact structurally, so that if a
// future change adds such a parameter the omission is caught rather than
// silently inherited.
// ---------------------------------------------------------------------------

#[test]
fn err_07_no_enum_or_integer_parameters_to_abuse() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/driver.h"),
    )
    .expect("read driver.h");
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/driver.c"),
    )
    .expect("read driver.c");

    assert!(
        !header.contains("enum") && !source.contains("enum"),
        "the C source has grown an enum -- ERRORS.md must gain out-of-range \
         enum rows and Phase C must test them"
    );

    // The only parameter anywhere in the library is `const char *line`.
    assert!(
        source.contains("void printLine(const char *line)"),
        "printLine's signature changed; re-derive ERRORS.md"
    );

    // The three no-argument entry points really do take no arguments, so an
    // extra-argument call is the only "out of range" abuse available at this
    // boundary. The C ABI ignores extra register arguments; verify both sides
    // tolerate junk values identically rather than assuming it.
    for name in [&b"bad"[..], b"good", b"driver"] {
        let label = String::from_utf8_lossy(name).to_string();
        assert_same(
            &format!("{label}() called with 3 junk extra args"),
            &call_void_with_extra_args(Impl::C, name),
            &call_void_with_extra_args(Impl::Rust, name),
        );
    }
}

// ---------------------------------------------------------------------------
// Negative symbol parity: the `static` C helpers must NOT be exported by
// either .so. An over-eager translation that exported them would be a
// divergence in the opposite direction from a missing symbol.
// ---------------------------------------------------------------------------

#[test]
fn err_08_static_helpers_not_exported_by_either_so() {
    for name in [&b"helperGood"[..], b"helperBad"] {
        let n = String::from_utf8_lossy(name).to_string();
        assert!(
            !exports(Impl::C, name),
            "C .so unexpectedly exports static helper {n}"
        );
        assert!(
            !exports(Impl::Rust, name),
            "Rust .so exports {n}, but it is `static` (internal linkage) in the C \
             source and absent from the C .so"
        );
    }
}
