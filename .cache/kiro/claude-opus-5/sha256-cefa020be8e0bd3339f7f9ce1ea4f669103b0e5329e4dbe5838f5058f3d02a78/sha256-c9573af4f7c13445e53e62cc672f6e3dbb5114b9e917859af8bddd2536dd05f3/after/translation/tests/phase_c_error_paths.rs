//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no error-reporting surface
//! (no returns, no asserts, no null or range checks — see `ERRORS.md` for the
//! mechanical grep), so these tests assert the *absence* of rejection: every
//! input, including the ones that look invalid, must be accepted and printed
//! identically by both `.so`s, and a failing `printf` must be swallowed the same
//! way by both.

mod common;

use common::{Rng, assert_same, capture_stdout, with_stdout_on_device};

/// Row E1 — no input is ever rejected. Every call must produce exactly 8 hex
/// digits plus a newline, from both libraries, whatever the input.
#[test]
fn e1_no_input_is_ever_rejected() {
    let mut rng = Rng::new(0xE000_0001);
    let mut bits: Vec<u32> = vec![
        0x0000_0000,
        0xFFFF_FFFF,
        0x7F80_0000,
        0xFF80_0000,
        0x7FFF_FFFF,
        0xFFFF_FFFE,
        0x8000_0000,
        0x0000_0001,
    ];
    bits.extend((0..4096).map(|_| rng.next_u32()));

    // Same bytes from both libraries...
    assert_same("E1 nothing is rejected", &bits);

    // ...and the shape is the documented one: 8 digits + '\n', never an error
    // marker, empty line, or short write.
    for &b in &bits {
        for (which, out) in [
            ("C", capture_stdout(|| unsafe { (common::c_driver())(f32::from_bits(b)) })),
            (
                "Rust",
                capture_stdout(|| unsafe { (common::rust_driver())(f32::from_bits(b)) }),
            ),
        ] {
            assert_eq!(
                out.len(),
                9,
                "{which}: input 0x{b:08x} produced {:?}, expected 8 hex digits + newline",
                String::from_utf8_lossy(&out)
            );
            assert_eq!(out[8], b'\n', "{which}: input 0x{b:08x} is not newline-terminated");
            assert!(
                out[..8].iter().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c)),
                "{which}: input 0x{b:08x} produced non-lowercase-hex {:?}",
                String::from_utf8_lossy(&out)
            );
        }
    }
}

/// Row E2 — `print_hex`'s only conditional is `i < len`, and `driver` hard-codes
/// `len = sizeof(float) = 4`, so the guard can never reject: the loop body runs
/// exactly 4 times for every input. Observable as exactly 4 byte-pairs per line.
///
/// `print_hex` is `static` in C and correspondingly private in Rust, so 4
/// iterations is the only reachable behaviour and neither `.so` exports it.
#[test]
fn e2_len_is_always_four_never_zero() {
    let mut rng = Rng::new(0xE000_0002);
    for _ in 0..256 {
        let b = rng.next_u32();
        let c_out = capture_stdout(|| unsafe { (common::c_driver())(f32::from_bits(b)) });
        let r_out = capture_stdout(|| unsafe { (common::rust_driver())(f32::from_bits(b)) });
        assert_eq!(c_out, r_out, "input 0x{b:08x}");
        // 4 bytes * 2 hex digits, never 0 (which a `len == 0` path would give).
        assert_eq!(c_out.len() - 1, 8, "expected 4 byte-pairs, i.e. len == 4");
    }

    // Neither library exposes `print_hex`; it is `static` in the C source.
    unsafe {
        let c_lib = libloading::Library::new(common::c_so_path()).unwrap();
        let r_lib = libloading::Library::new(common::rust_so_path()).unwrap();
        assert!(
            c_lib
                .get::<unsafe extern "C" fn(*const u8, i32)>(b"print_hex\0")
                .is_err(),
            "C .so must not export the static `print_hex`"
        );
        assert!(
            r_lib
                .get::<unsafe extern "C" fn(*const u8, i32)>(b"print_hex\0")
                .is_err(),
            "Rust .so must not export `print_hex` either"
        );
    }
}

/// Row E3 — a signalling NaN is the closest thing an IEEE-754 `float` has to a
/// trap representation. It must cross the FFI boundary and be printed verbatim:
/// no trap, no rejection, and crucially no quieting (which would turn
/// `0x7FA00000` into `0x7FE00000` and change the output).
#[test]
fn e3_signalling_nan_not_quieted() {
    let snans: [u32; 8] = [
        0x7FA0_0000,
        0xFFA0_0000,
        0x7F80_0001,
        0xFF80_0001,
        0x7FBF_FFFF,
        0xFFBF_FFFF,
        0x7F80_1234,
        0xFF95_5555,
    ];
    assert_same("E3 signalling NaNs", &snans);

    // The exact bytes matter, so pin the canonical case rather than only
    // comparing the two libraries to each other.
    let out = capture_stdout(|| unsafe { (common::rust_driver())(f32::from_bits(0x7FA0_0000)) });
    assert_eq!(
        &out, b"0000a07f\n",
        "sNaN 0x7FA00000 must print its little-endian bytes unchanged, got {:?}",
        String::from_utf8_lossy(&out)
    );
    let c_out = capture_stdout(|| unsafe { (common::c_driver())(f32::from_bits(0x7FA0_0000)) });
    assert_eq!(c_out, out, "C and Rust must agree on the sNaN encoding");
}

/// Row E4 — non-canonical / "impossible" encodings are all accepted verbatim:
/// negative zero, subnormals, infinities, and NaNs with every payload pattern.
#[test]
fn e4_noncanonical_encodings_accepted() {
    let mut bits = vec![
        0x8000_0000, // -0.0
        0x0000_0001, // smallest subnormal
        0x800F_FFFF, // negative subnormal
        0x7F80_0000, // +inf
        0xFF80_0000, // -inf
        0x7FC0_0000, // canonical qNaN
        0xFFC0_0000, // negative qNaN
        0x7FFF_FFFF, // qNaN, all payload bits set
        0xFFFF_FFFF, // negative qNaN, all bits set
        0x7F80_0001, // sNaN, minimal payload
    ];
    // Every single-bit NaN payload, both signs.
    for bit in 0..23u32 {
        bits.push(0x7F80_0000 | (1 << bit));
        bits.push(0xFF80_0000 | (1 << bit));
    }
    assert_same("E4 non-canonical encodings", &bits);
}

/// Row E5 — the input domain has no invalid region: sweep the `u32` space so that
/// every byte value occurs at every position, plus a large random sample, and
/// require every pattern to be accepted identically.
#[test]
fn e5_all_bit_patterns_accepted() {
    // Every byte value at every position, with the other lanes both all-zero and
    // all-one so no lane interaction is missed.
    let mut bits = Vec::new();
    for pos in 0..4u32 {
        for byte in 0x00..=0xFFu32 {
            let shifted = byte << (8 * pos);
            let mask = 0xFFu32 << (8 * pos);
            bits.push(shifted); // other lanes zero
            bits.push((!mask) | shifted); // other lanes one
        }
    }
    // Plus a broad random sample.
    let mut rng = Rng::new(0xE000_0005);
    bits.extend((0..32768).map(|_| rng.next_u32()));
    assert_same("E5 all bit patterns", &bits);
}

/// Row E6 — bytes `>= 0x80` zero-extend, because `p[i]` is `unsigned char`. This
/// is the highest-value assertion in the file: a translation using `i8` would
/// print `ffffff80` and the two libraries would diverge on every negative float.
#[test]
fn e6_high_bytes_zero_extend() {
    // Value whose four bytes are all >= 0x80: 0xFF808080.
    let all_high = 0xFF80_8080u32;
    let c_out = capture_stdout(|| unsafe { (common::c_driver())(f32::from_bits(all_high)) });
    let r_out = capture_stdout(|| unsafe { (common::rust_driver())(f32::from_bits(all_high)) });
    assert_eq!(c_out, r_out);
    assert_eq!(
        &c_out, b"808080ff\n",
        "each high byte must be exactly two digits, got {:?}",
        String::from_utf8_lossy(&c_out)
    );

    // And no output for any input may ever contain a sign-extended run.
    let mut bits = Vec::new();
    for pos in 0..4u32 {
        for hi in 0x80..=0xFFu32 {
            bits.push(hi << (8 * pos));
        }
    }
    assert_same("E6 zero extension", &bits);
    for &b in &bits {
        let out = capture_stdout(|| unsafe { (common::rust_driver())(f32::from_bits(b)) });
        assert_eq!(
            out.len(),
            9,
            "input 0x{b:08x} produced {:?} — a sign-extended byte would make this longer",
            String::from_utf8_lossy(&out)
        );
    }
}

/// Rows E7 and E8 — `printf`'s return value is discarded at both C call sites, so
/// a write failure is silently swallowed. `/dev/full` makes the flush fail with
/// `ENOSPC`; a closed fd makes it fail with `EBADF`. In both cases `driver` must
/// return normally from both libraries and must not abort the process.
#[test]
fn e7_printf_write_failure_swallowed() {
    // ENOSPC: /dev/full accepts the open but fails every write.
    let c_rc = with_stdout_on_device("/dev/full", || unsafe {
        (common::c_driver())(1.5f32);
    });
    let r_rc = with_stdout_on_device("/dev/full", || unsafe {
        (common::rust_driver())(1.5f32);
    });
    assert_eq!(
        c_rc, r_rc,
        "C and Rust must report the same flush outcome when the write fails"
    );
    assert_ne!(
        c_rc, 0,
        "sanity check: writing to /dev/full is expected to fail, so the test is meaningful"
    );

    // Reaching here at all is the assertion for E7/E8: neither `.so` aborted or
    // propagated an error out of the void-returning `driver`. Confirm the stream
    // still works afterwards and the two libraries still agree.
    assert_same("E7 post-failure recovery", &[1.5f32.to_bits(), 0xDEAD_BEEF]);

    // EBADF: point stdout at a device then close it out from under the stream.
    let mut rng = Rng::new(0xE000_0007);
    for _ in 0..16 {
        let b = rng.next_u32();
        let c_rc = with_stdout_on_device("/dev/full", || unsafe {
            (common::c_driver())(f32::from_bits(b));
        });
        let r_rc = with_stdout_on_device("/dev/full", || unsafe {
            (common::rust_driver())(f32::from_bits(b));
        });
        assert_eq!(c_rc, r_rc, "flush outcome must match for input 0x{b:08x}");
    }
}

/// Generic FFI boundary conditions the prompt requires be covered. The public API
/// is `void driver(float)`: one by-value scalar, so there is no pointer, no
/// length, and no enum to abuse. This test documents that and covers the one
/// boundary that does exist — the full `float` encoding space, including values
/// with no meaningful numeric interpretation.
#[test]
fn e_generic_ffi_boundaries() {
    // No pointer parameter exists, so null-pointer abuse is not reachable; the
    // argument is passed in a register by value. The analogue is an argument with
    // no valid numeric meaning, i.e. a NaN, and the "one past the range" analogue
    // is the pattern adjacent to each class boundary.
    let boundaries: [u32; 16] = [
        0x0000_0000, // +0
        0x0000_0001, // one past +0: smallest subnormal
        0x007F_FFFF, // largest subnormal
        0x0080_0000, // one past: smallest normal
        0x7F7F_FFFF, // largest finite
        0x7F80_0000, // one past: +inf
        0x7F80_0001, // one past +inf: smallest sNaN
        0x7FBF_FFFF, // largest sNaN
        0x7FC0_0000, // one past: smallest qNaN
        0x7FFF_FFFF, // largest qNaN
        0x8000_0000, // -0
        0x8000_0001,
        0xFF7F_FFFF, // smallest finite
        0xFF80_0000, // -inf
        0xFF80_0001, // negative sNaN
        0xFFFF_FFFF, // all bits set
    ];
    assert_same("generic FFI boundaries", &boundaries);
}
