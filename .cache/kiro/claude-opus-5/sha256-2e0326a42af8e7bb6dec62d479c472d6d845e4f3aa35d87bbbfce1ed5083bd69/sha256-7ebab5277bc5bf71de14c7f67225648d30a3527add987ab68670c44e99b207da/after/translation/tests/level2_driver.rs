//! Level 2 (public API from `include/driver.h`): `void driver(char data)`.
//!
//! ```c
//! void driver(char data) {
//!     char result = data + 1;
//!     printHexCharLine(result);
//! }
//! ```
//!
//! `data + 1` is evaluated in `int` and truncated back to `char` on assignment,
//! so `0x7f` becomes `-128` (printed `ffffff80`) and `0xff` wraps to `0`
//! (printed `00`). All 256 inputs are compared against the C library.
//!
//! As in the level 1 file, all checks share one `#[test]` because capturing
//! stdout redirects a process-global file descriptor.

mod common;

use std::ffi::c_char;

use common::{assert_char_fn_matches, capture_stdout, char_fns};

fn matches_for_every_byte() {
    for byte in 0u8..=255u8 {
        assert_char_fn_matches("driver", byte as c_char);
    }
}

fn wraparound_boundaries() {
    // 0x7f -> overflows the signed char range; 0xff -> wraps to 0x00.
    for byte in [0x7eu8, 0x7f, 0x80, 0xfe, 0xff, 0x00] {
        assert_char_fn_matches("driver", byte as c_char);
    }
}

/// `driver` must produce exactly what `printHexCharLine(data + 1)` produces,
/// confirming the composition of the two exported symbols is wired up the same
/// way in both libraries.
fn is_print_hex_char_line_of_successor() {
    let (c_driver, rust_driver) = char_fns("driver");
    let (c_print, rust_print) = char_fns("printHexCharLine");

    for byte in 0u8..=255u8 {
        let arg = byte as c_char;
        let successor = arg.wrapping_add(1);

        // SAFETY: `void (char)` in both libraries.
        let c_via_driver = capture_stdout(|| unsafe { c_driver(arg) });
        let c_via_print = capture_stdout(|| unsafe { c_print(successor) });
        let rust_via_driver = capture_stdout(|| unsafe { rust_driver(arg) });
        let rust_via_print = capture_stdout(|| unsafe { rust_print(successor) });

        assert_eq!(
            c_via_driver, c_via_print,
            "C: driver(0x{byte:02x}) != printHexCharLine(successor)"
        );
        assert_eq!(
            rust_via_driver, rust_via_print,
            "Rust: driver(0x{byte:02x}) != printHexCharLine(successor)"
        );
        assert_eq!(
            c_via_driver, rust_via_driver,
            "driver(0x{byte:02x}) differs between C and Rust"
        );
    }
}

fn stateless_across_calls() {
    let (c_fn, rust_fn) = char_fns("driver");
    let sequence: Vec<c_char> = (0u8..=255u8).map(|b| b as c_char).collect();

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

    assert_eq!(c_out, rust_out, "batched driver output differs");
}

#[test]
fn driver_matches_c() {
    wraparound_boundaries();
    matches_for_every_byte();
    is_print_hex_char_line_of_successor();
    stateless_across_calls();
}
