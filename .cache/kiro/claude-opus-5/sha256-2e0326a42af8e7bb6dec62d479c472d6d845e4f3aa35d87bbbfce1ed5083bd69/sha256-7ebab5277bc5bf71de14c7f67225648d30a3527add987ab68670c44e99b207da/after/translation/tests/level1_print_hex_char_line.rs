//! Level 1 (leaf): `void printHexCharLine(char charHex)`.
//!
//! The C body is `printf("%02x\n", charHex)`. `charHex` is promoted to `int`
//! before reaching the variadic call, and `%02x` then reinterprets that `int`
//! as `unsigned int`. With a signed `char` (the x86_64 Linux default) every
//! negative input therefore prints eight hex digits, e.g. `0x80` -> `ffffff80`.
//! Exhaustively covering all 256 bit patterns pins that behaviour down.
//!
//! Note on structure: comparing output requires temporarily redirecting file
//! descriptor 1, which is process-global. libtest runs the tests of one binary
//! on concurrent threads and prints its own progress lines to stdout, so a
//! second `#[test]` here would risk having harness text land inside a capture
//! window. Everything therefore runs from a single `#[test]`.

mod common;

use std::ffi::c_char;

use common::{assert_char_fn_matches, capture_stdout, char_fns};

/// Every possible `char` bit pattern, expressed the way the ABI sees it.
fn all_char_values() -> impl Iterator<Item = c_char> {
    (0u8..=255u8).map(|b| b as c_char)
}

fn matches_for_every_byte() {
    for value in all_char_values() {
        assert_char_fn_matches("printHexCharLine", value);
    }
}

fn boundary_values() {
    // Signedness boundaries and the two-digit / eight-digit crossover.
    for byte in [0x00u8, 0x01, 0x0f, 0x10, 0x7e, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
        assert_char_fn_matches("printHexCharLine", byte as c_char);
    }
}

/// Guards against a translation that is self-consistent but produces the wrong
/// text: the exact expected bytes are asserted against the C library's output.
fn c_output_shape_is_as_expected() {
    let (c_fn, _) = char_fns("printHexCharLine");

    for byte in 0u8..=255u8 {
        // SAFETY: `void (char)`.
        let out = capture_stdout(|| unsafe { c_fn(byte as c_char) });
        let text = String::from_utf8(out).expect("printf output is ASCII");
        let expected = format!("{:02x}\n", (byte as i8) as i32 as u32);
        assert_eq!(text, expected, "unexpected C output for byte 0x{byte:02x}");
    }
}

/// Repeated invocations must not accumulate state (no hidden buffering or
/// static variables introduced by the translation).
fn stateless_across_calls() {
    let (c_fn, rust_fn) = char_fns("printHexCharLine");

    let sequence: Vec<c_char> = all_char_values().collect();

    // SAFETY: `void (char)`.
    let c_out = capture_stdout(|| {
        for &v in &sequence {
            unsafe { c_fn(v) }
        }
    });
    let rust_out = capture_stdout(|| {
        for &v in &sequence {
            unsafe { rust_fn(v) }
        }
    });

    assert_eq!(c_out, rust_out, "batched printHexCharLine output differs");
}

#[test]
fn print_hex_char_line_matches_c() {
    c_output_shape_is_as_expected();
    boundary_values();
    matches_for_every_byte();
    stateless_across_calls();
}
