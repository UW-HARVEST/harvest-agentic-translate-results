//! Differential tests for the higher-level exported entry point, `void siphash(int)`.
//!
//! `siphash` has no return value: its entire observable behaviour is the 64 lines it
//! `printf`s. Both libraries are run with fd 1 redirected to a temp file and the raw
//! bytes are compared, so formatting, spacing and newlines must match exactly.

mod common;

use common::{capture_stdout, libs};
use std::ffi::c_int;

fn check(init: c_int) {
    let (c_fn, rust_fn) = libs().siphash();

    let c_out = capture_stdout("c", || unsafe { c_fn(init) });
    let rust_out = capture_stdout("rust", || unsafe { rust_fn(init) });

    if c_out != rust_out {
        // Report the first differing line to keep the failure readable.
        let c_str = String::from_utf8_lossy(&c_out);
        let r_str = String::from_utf8_lossy(&rust_out);
        let mut detail = String::new();
        for (n, (cl, rl)) in c_str.lines().zip(r_str.lines()).enumerate() {
            if cl != rl {
                detail = format!("\n  first diff at line {n}:\n    C    = {cl:?}\n    Rust = {rl:?}");
                break;
            }
        }
        panic!(
            "siphash({init}) stdout mismatch ({} vs {} bytes){detail}",
            c_out.len(),
            rust_out.len()
        );
    }

    // Sanity: the C prints 64 lines of 8 hex bytes; make sure we really captured them.
    assert_eq!(
        c_out.lines().count(),
        64,
        "expected 64 captured lines for siphash({init}), got {:?}",
        String::from_utf8_lossy(&c_out)
    );
}

trait Lines {
    fn lines(&self) -> std::str::Lines<'_>;
}
impl Lines for Vec<u8> {
    fn lines(&self) -> std::str::Lines<'_> {
        std::str::from_utf8(self).expect("captured output is ASCII").lines()
    }
}

#[test]
fn init_zero() {
    check(0);
}

#[test]
fn small_inits() {
    for init in 0..40 {
        check(init);
    }
}

#[test]
fn negative_inits() {
    // `int z = init; mem[i] = z;` truncates to the low 8 bits, so negative seeds are
    // stored as their two's-complement low byte.
    for init in -40..0 {
        check(init);
    }
}

#[test]
fn byte_boundary_inits() {
    // Values where `z` crosses a 0xff -> 0x00 byte boundary partway through the 64
    // stores, plus the extremes where `z` overflows `int` (UB in C, wraps in practice).
    for init in [
        1, 127, 128, 200, 250, 255, 256, 257, 511, 512, 65535, 65536, 1 << 24, i32::MAX - 64,
        i32::MAX - 1, i32::MAX, i32::MIN, i32::MIN + 1, i32::MIN + 64, -128, -255, -256, -257,
    ] {
        check(init);
    }
}

#[test]
fn randomised_inits() {
    let mut rng = common::Rng(0xFEED_FACE_1234_5678);
    for _ in 0..60 {
        check(rng.next_u64() as c_int);
    }
}

/// Interleaving check: calling C then Rust then C again through the same stdio must
/// still produce identical, self-consistent output (no state leaks between them).
#[test]
fn repeated_and_interleaved_calls() {
    let (c_fn, rust_fn) = libs().siphash();
    let a = capture_stdout("i1", || unsafe { c_fn(7) });
    let b = capture_stdout("i2", || unsafe { rust_fn(7) });
    let c = capture_stdout("i3", || unsafe { c_fn(7) });
    let d = capture_stdout("i4", || unsafe { rust_fn(7) });
    assert_eq!(a, b);
    assert_eq!(a, c);
    assert_eq!(a, d);
}
