// Differential tests: C `.so` vs Rust `.so`, both loaded via `libloading`.
//
// Row numbers refer to CONFIGS.md (valid paths, Phase B) and ERRORS.md
// (rejections / generic boundaries, Phase C).

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Phase B -- CONFIGS.md rows
// ---------------------------------------------------------------------------

/// CONFIGS rows 1-4, ERRORS rows G1-G4: the extremes of the `int` range.
#[test]
fn boundary_extremes() {
    assert_same_many([0, -1, i32::MAX, i32::MIN]);
}

/// CONFIGS rows 5-6: values whose bytes are all distinct / mostly zero, which
/// pin down byte order exactly. A big-endian-vs-little-endian mistake in the
/// `memcpy` translation shows up here and nowhere else.
#[test]
fn endianness_discriminators() {
    assert_same_many([
        1,
        0x0100_0000,
        0x0000_0100,
        0x0001_0000,
        0x0102_0304,
        0x0403_0201,
        0x1234_5678,
        0x7856_3412,
    ]);
}

/// CONFIGS row 7, ERRORS row G8: bytes >= 0x80 must print as two hex digits.
/// `raw` is `char` (signed on x86-64) and is cast to `unsigned char *`; if the
/// translation promoted through a signed type it would emit `ffffff80`.
#[test]
fn high_bytes_no_sign_extension() {
    let mut cases = Vec::new();
    // one high byte at a time, in each of the 4 positions
    for pos in 0..4u32 {
        for &b in &[0x80u32, 0x81, 0xA5, 0xFE, 0xFF] {
            cases.push((b << (8 * pos)) as i32);
            // high byte plus small bytes elsewhere
            cases.push(((b << (8 * pos)) | 0x0000_0001) as i32);
        }
    }
    // every byte high simultaneously
    cases.push(0x8080_8080u32 as i32);
    cases.push(0xFFFF_FFFFu32 as i32);
    cases.push(0x80FF_80FFu32 as i32);
    assert_same_many(cases);
}

/// CONFIGS row 8: every byte < 0x10, so `%02x` must zero-pad all four.
/// A `{:x}` instead of `{:02x}` bug produces short output only here.
#[test]
fn low_nibble_zero_padding() {
    assert_same_many([
        0x0102_0304,
        0x0f0e_0d0c,
        0x0000_0001,
        0x0100_0000,
        0x0000_0000,
        0x0f0f_0f0f,
        0x0001_0203,
    ]);
}

/// CONFIGS row 9: a single set bit swept across all 32 positions.
#[test]
fn single_bit_sweep() {
    let cases: Vec<i32> = (0..32).map(|i| (1u32 << i) as i32).collect();
    assert_eq!(cases.len(), 32);
    assert_same_many(cases);
    // and the complement of each single bit
    let cases: Vec<i32> = (0..32).map(|i| !(1u32 << i) as i32).collect();
    assert_same_many(cases);
}

/// CONFIGS row 10: all 256 byte values in each of the 4 byte positions.
/// Exhaustive per-position byte coverage (1024 differential comparisons).
#[test]
fn all_byte_values_each_position() {
    let mut cases = Vec::with_capacity(1024);
    for pos in 0..4u32 {
        for b in 0..256u32 {
            cases.push((b << (8 * pos)) as i32);
        }
    }
    assert_eq!(cases.len(), 1024);
    assert_same_many(cases);
}

/// CONFIGS row 11: property-style randomized sweep over the full `i32` range
/// with a fixed seed for reproducibility.
#[test]
fn randomized_full_range() {
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    let cases: Vec<i32> = (0..4000).map(|_| rng.next_i32()).collect();
    assert_same_many(cases);
}

/// CONFIGS row 12: consecutive calls in one process. Verifies the per-call
/// output framing (exactly one `\n`-terminated 8-digit line per call) and that
/// no state leaks between calls in either implementation.
#[test]
fn repeated_calls_framing() {
    let mut rng = Rng::new(0xABCD_0001);
    let xs: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();

    let c_out = capture_stdout(|| {
        let f = c_driver();
        for &x in &xs {
            unsafe { f(x) };
        }
    });
    let rust_out = capture_stdout(|| {
        let f = rust_driver();
        for &x in &xs {
            unsafe { f(x) };
        }
    });

    assert_wellformed_c_output(&c_out, xs.len(), "repeated calls");
    assert_eq!(
        c_out,
        rust_out,
        "DIVERGENCE across {} consecutive calls:\n  C   = {:?}\n  Rust= {:?}",
        xs.len(),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out)
    );
}

/// CONFIGS row 13: interleave C and Rust calls on the shared libc `stdout` in
/// both orders. Catches any dependence on stream state or call ordering.
#[test]
fn interleaved_call_order() {
    let mut rng = Rng::new(0x5EED_9999);
    for _ in 0..200 {
        let x = rng.next_i32();
        let ctx = format!("interleaved x={x} (0x{:08x})", x as u32);

        // C first, then Rust, inside a single capture.
        let cr = capture_stdout(|| unsafe {
            c_driver()(x);
            rust_driver()(x);
        });
        assert_wellformed_c_output(&cr, 2, &ctx);
        let mut lines = cr.split(|&b| b == b'\n');
        let a = lines.next().unwrap().to_vec();
        let b = lines.next().unwrap().to_vec();
        assert_eq!(a, b, "DIVERGENCE (C then Rust) for {ctx}");

        // Rust first, then C.
        let rc = capture_stdout(|| unsafe {
            rust_driver()(x);
            c_driver()(x);
        });
        assert_wellformed_c_output(&rc, 2, &ctx);
        let mut lines = rc.split(|&b| b == b'\n');
        let a2 = lines.next().unwrap().to_vec();
        let b2 = lines.next().unwrap().to_vec();
        assert_eq!(a2, b2, "DIVERGENCE (Rust then C) for {ctx}");

        // Order must not matter at all.
        assert_eq!(cr, rc, "output depends on call order for {ctx}");
    }
}

// ---------------------------------------------------------------------------
// Phase C -- ERRORS.md rows
//
// The C library has no rejection path at all (no `return`, no assert, no null
// check, no range check -- see ERRORS.md for the grep evidence), so there is no
// error code to compare. What remains testable is that the two implementations
// agree on the generic boundaries, which the tests above cover, plus the
// structural claims below.
// ---------------------------------------------------------------------------

/// ERRORS G3/G7: `len` is not caller-controlled -- `driver` always passes
/// `sizeof(int)`. Assert both implementations emit exactly 4 bytes' worth of hex
/// for every input, so neither can be over- or under-reading the buffer.
#[test]
fn output_width_is_always_sizeof_int() {
    let mut rng = Rng::new(0x1111_2222);
    for _ in 0..300 {
        let x = rng.next_i32();
        let c_out = capture_stdout(|| unsafe { c_driver()(x) });
        let rust_out = capture_stdout(|| unsafe { rust_driver()(x) });
        assert_eq!(c_out.len(), 9, "C emitted {} bytes for x={x}", c_out.len());
        assert_eq!(
            rust_out.len(),
            9,
            "Rust emitted {} bytes for x={x} (expected 8 hex digits + newline)",
            rust_out.len()
        );
        assert_eq!(c_out, rust_out, "DIVERGENCE for x={x}");
    }
}

/// The output must be newline-terminated by both, matching `printf("\n")`.
#[test]
fn output_is_newline_terminated() {
    for x in [0, -1, i32::MIN, i32::MAX, 12345] {
        let c_out = capture_stdout(|| unsafe { c_driver()(x) });
        let rust_out = capture_stdout(|| unsafe { rust_driver()(x) });
        assert_eq!(c_out.last(), Some(&b'\n'), "C output not newline-terminated");
        assert_eq!(
            rust_out.last(),
            Some(&b'\n'),
            "Rust output not newline-terminated for x={x}"
        );
        assert_eq!(c_out, rust_out);
    }
}

/// Guard against a harness that lies: prove the capture mechanism really does
/// observe distinct output for distinct input, so a passing differential run
/// cannot be the result of capturing nothing.
#[test]
fn harness_actually_observes_output() {
    let a = capture_stdout(|| unsafe { c_driver()(0) });
    let b = capture_stdout(|| unsafe { c_driver()(1) });
    assert_eq!(a, b"00000000\n", "unexpected C output for 0: {a:?}");
    assert_ne!(a, b, "capture harness cannot distinguish different inputs");

    let ra = capture_stdout(|| unsafe { rust_driver()(0) });
    let rb = capture_stdout(|| unsafe { rust_driver()(1) });
    assert_eq!(ra, a);
    assert_eq!(rb, b);
}

/// Extended sweep, run explicitly with `--ignored`, kept out of the default run
/// so the normal suite stays fast. 200k seeded random values plus a dense walk
/// through the low and high ends of the range.
#[test]
#[ignore = "long-running; run with `cargo test -- --ignored`"]
fn extended_randomized_sweep() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);
    for _ in 0..200_000 {
        assert_same(rng.next_i32());
    }
    for x in -5000..5000 {
        assert_same(x);
    }
    for i in 0..5000 {
        assert_same(i32::MAX - i);
        assert_same(i32::MIN + i);
    }
}
