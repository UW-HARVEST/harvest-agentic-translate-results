//! Differential tests for `driver`.
//!
//! The public API in `c_src/include/driver.h` is a single function,
//! `void driver(int x)`. The only other function in the translation unit,
//! `static void print_hex(unsigned char *, int)`, has internal linkage and is
//! not exported by either library, so it is covered indirectly: `driver` is a
//! struct initialization followed by one `print_hex` call, so every byte
//! `print_hex` emits is compared here.
//!
//! Every call goes through `libloading`, including the Rust side, so the
//! `#[unsafe(no_mangle)] extern "C"` export wrapper is part of what is tested.
//!
//! Everything lives in a single `#[test]`: comparing output means temporarily
//! redirecting file descriptor 1, and libtest writes its own progress lines to
//! that same descriptor from the harness thread. One sequential test means
//! there is never a concurrent writer to fd 1 while a capture is active.

mod common;

use common::{assert_same, hex, run_both};

use std::ffi::c_int;

/// The byte sequence the C code is expected to produce for a given `floors`,
/// derived independently from the C source:
///
/// ```c
/// house_t house = {0};        // all bytes zero, padding included
/// house.floors = floors;      // int    at offset 0
/// house.bedrooms = 3;         // int    at offset 4
/// house.bathrooms = 2.;       // double at offset 8
/// ```
///
/// then all `sizeof(house_t)` bytes printed as `%02x`, plus a trailing newline.
/// On the little-endian x86-64 / aarch64 SysV ABI that is 16 bytes.
fn expected(floors: c_int) -> Vec<u8> {
    let mut raw = [0u8; 16];
    raw[0..4].copy_from_slice(&floors.to_le_bytes());
    raw[4..8].copy_from_slice(&3i32.to_le_bytes());
    raw[8..16].copy_from_slice(&2.0f64.to_le_bytes());

    let mut out = hex(&raw).into_bytes();
    out.push(b'\n');
    out
}

fn check(x: c_int) {
    let (c_out, rust_out) = run_both(x);
    assert_same(x, &c_out, &rust_out);
}

#[test]
fn driver_matches_c_for_all_tested_inputs() {
    // --- both libraries must export `driver` -----------------------------
    let libs = common::libs();
    let _c = common::driver_symbol(&libs.c);
    let _rust = common::driver_symbol(&libs.rust);

    // --- sanity-check against a model derived from driver.c --------------
    // Guards against both libraries agreeing on something wrong, and against
    // the capture machinery contaminating the compared bytes.
    for x in [c_int::MIN, -7, 0, 1, 3, 4096, c_int::MAX] {
        let (c_out, rust_out) = run_both(x);
        assert_same(x, &c_out, &rust_out);

        let model = expected(x);
        assert_eq!(
            c_out,
            model,
            "C output for driver({x}) disagrees with the model derived from driver.c: \
             got {}, model {}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&model)
        );

        // 16 struct bytes -> 32 hex chars + '\n'
        assert_eq!(c_out.len(), 33, "unexpected output length for driver({x})");
        assert_eq!(*c_out.last().unwrap(), b'\n');
        assert!(
            c_out[..32]
                .iter()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)),
            "output must be lowercase hex"
        );
    }

    // --- negative control: the capture must be input-sensitive -----------
    // If the redirection machinery ever returned empty or stale bytes, the
    // comparisons below would pass vacuously.
    {
        let (a_c, a_rust) = run_both(1);
        let (b_c, b_rust) = run_both(2);
        assert_ne!(a_c, b_c, "capture is not input-sensitive on the C side");
        assert_ne!(a_rust, b_rust, "capture is not input-sensitive on the Rust side");
        assert!(!a_c.is_empty() && !a_rust.is_empty(), "capture returned no bytes");
    }

    // --- zero, and small magnitudes both signs --------------------------
    for x in -64..=64 {
        check(x);
    }

    // --- extremes -------------------------------------------------------
    for x in [
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        c_int::MAX - 1,
        c_int::MAX,
    ] {
        check(x);
    }

    // --- powers of two and their neighbours -----------------------------
    for bit in 0..31 {
        let p = 1i32 << bit;
        check(p);
        check(p - 1);
        check(-p);
    }

    // --- byte patterns --------------------------------------------------
    // Chosen so individual bytes of `floors` hit interesting values:
    // 0x00, 0xff, 0x0a ('\n'), 0x25 ('%'), ASCII digits, high bit set.
    let patterns: [i32; 16] = [
        0x0000_0000,
        0x0000_00ff,
        0x0000_ff00,
        0x00ff_0000,
        0x7f00_0000,
        0x0a0a_0a0a,
        0x2525_2525,
        0x3031_3233,
        0x0102_0304,
        0x8000_0001u32 as i32,
        0xffff_ffffu32 as i32,
        0xdead_beefu32 as i32,
        0xcafe_babeu32 as i32,
        0xfeed_faceu32 as i32,
        0x8080_8080u32 as i32,
        0x7fff_fffe,
    ];
    for x in patterns {
        check(x);
    }

    // --- deterministic pseudorandom sweep -------------------------------
    let mut state: u32 = 0x1234_5678;
    for _ in 0..2000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        check(state as i32);
    }

    // --- repeated / interleaved calls ------------------------------------
    // `house` is a fresh local each call; nothing may carry over, and the two
    // libraries must stay in lockstep across interleaved calls.
    for _ in 0..8 {
        for x in [0, 1, -1, 12345, c_int::MIN] {
            check(x);
        }
    }

    // --- exhaustive over a contiguous 16-bit range ----------------------
    // Walks the two low bytes of `floors` through every value.
    for x in 0..=u16::MAX as i32 {
        let (c_out, rust_out) = run_both(x);
        if c_out != rust_out {
            assert_same(x, &c_out, &rust_out);
        }
    }
}
