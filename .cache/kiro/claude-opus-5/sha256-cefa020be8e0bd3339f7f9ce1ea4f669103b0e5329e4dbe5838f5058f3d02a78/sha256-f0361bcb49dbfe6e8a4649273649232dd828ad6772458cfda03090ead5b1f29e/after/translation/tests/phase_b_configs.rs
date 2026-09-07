//! Phase B — valid-path differential tests, GATED on `CONFIGS.md`.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares captured stdout byte-for-byte,
//! using many randomized inputs (fixed seed) rather than one hand-picked value.
//!
//! The only public entry point is `driver`; the lowest-level function
//! `print_hex` is `static` in C (absent from `nm -D` on both `.so`s) and is
//! therefore exercised transitively through `driver` in every row.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// IEEE-754 binary32 field helpers, used to build inputs per class.
// ---------------------------------------------------------------------------

fn bits(sign: u32, exp: u32, frac: u32) -> u32 {
    ((sign & 1) << 31) | ((exp & 0xff) << 23) | (frac & 0x007f_ffff)
}

// ---------------------------------------------------------------------------
// Row 1 — uniformly random 32-bit patterns reinterpreted as f32.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row01_random_bit_patterns() {
    let mut rng = Rng::new(1);
    let inputs: Vec<u32> = (0..20_000).map(|_| rng.next_u32()).collect();
    run_batch("cfg_row01_random_bit_patterns", &inputs);
}

// ---------------------------------------------------------------------------
// Row 2 / 3 — normal finite values, positive and negative.
// ---------------------------------------------------------------------------

fn random_normals(sign: u32, salt: u64, n: usize) -> Vec<u32> {
    let mut rng = Rng::new(salt);
    (0..n)
        .map(|_| {
            // exponent 1..=254 keeps it normal and finite
            let exp = 1 + rng.below(254);
            bits(sign, exp, rng.next_u32())
        })
        .collect()
}

#[test]
fn cfg_row02_positive_normals() {
    let inputs = random_normals(0, 2, 4000);
    for &b in &inputs {
        let f = f32::from_bits(b);
        assert!(f.is_normal() && f > 0.0, "0x{b:08x} is not a positive normal");
    }
    run_batch("cfg_row02_positive_normals", &inputs);
}

#[test]
fn cfg_row03_negative_normals() {
    let inputs = random_normals(1, 3, 4000);
    for &b in &inputs {
        let f = f32::from_bits(b);
        assert!(f.is_normal() && f < 0.0, "0x{b:08x} is not a negative normal");
    }
    run_batch("cfg_row03_negative_normals", &inputs);
}

// ---------------------------------------------------------------------------
// Row 4 — subnormals (exponent field 0, significand != 0), both signs.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row04_subnormals_both_signs() {
    let mut rng = Rng::new(4);
    let inputs: Vec<u32> = (0..4000)
        .map(|i| {
            let frac = 1 + (rng.next_u32() % 0x007f_ffff);
            bits((i & 1) as u32, 0, frac)
        })
        .collect();
    for &b in &inputs {
        assert!(
            f32::from_bits(b).is_subnormal(),
            "0x{b:08x} is not subnormal"
        );
    }
    run_batch("cfg_row04_subnormals_both_signs", &inputs);
}

// ---------------------------------------------------------------------------
// Row 5 / 6 — both zeros, both infinities.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row05_both_zeros() {
    run_batch("cfg_row05_both_zeros", &[0x0000_0000, 0x8000_0000]);
}

#[test]
fn cfg_row06_both_infinities() {
    run_batch("cfg_row06_both_infinities", &[0x7f80_0000, 0xff80_0000]);
}

// ---------------------------------------------------------------------------
// Row 7 — NaNs with random payloads, both signs (quiet and signalling).
// ---------------------------------------------------------------------------

#[test]
fn cfg_row07_nans_random_payload() {
    let mut rng = Rng::new(7);
    let inputs: Vec<u32> = (0..4000)
        .map(|i| {
            let frac = 1 + (rng.next_u32() % 0x007f_ffff);
            bits((i & 1) as u32, 0xff, frac)
        })
        .collect();
    for &b in &inputs {
        assert!(f32::from_bits(b).is_nan(), "0x{b:08x} is not NaN");
    }
    run_batch("cfg_row07_nans_random_payload", &inputs);
}

// ---------------------------------------------------------------------------
// Row 8 — small integral values, dense low-exponent region.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row08_small_integral_values() {
    let inputs: Vec<u32> = (-2048i32..=2048).map(|i| (i as f32).to_bits()).collect();
    run_batch("cfg_row08_small_integral_values", &inputs);
}

// ---------------------------------------------------------------------------
// Row 9 — IEEE boundary constants.
// ---------------------------------------------------------------------------

const BOUNDARY: &[u32] = &[
    0x7f7f_ffff, // FLT_MAX
    0xff7f_ffff, // -FLT_MAX
    0x0080_0000, // FLT_MIN (smallest normal)
    0x8080_0000, // -FLT_MIN
    0x3400_0000, // FLT_EPSILON
    0x0000_0001, // smallest positive subnormal
    0x8000_0001, // smallest negative subnormal
    0x007f_ffff, // largest positive subnormal
    0x807f_ffff, // largest negative subnormal
    0x3f80_0000, // 1.0
    0xbf80_0000, // -1.0
    0x3f00_0000, // 0.5
    0x4000_0000, // 2.0
    0x4040_0000, // 3.0
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x7f80_0000, // +inf
    0xff80_0000, // -inf
    0x7fc0_0000, // quiet NaN
    0xffc0_0000, // -quiet NaN
    0x7fa0_0000, // signalling NaN
    0x7f80_0001, // NaN, minimal payload
    0x7fff_ffff, // NaN, all payload bits set
    0xffff_ffff, // -NaN, all payload bits set
];

#[test]
fn cfg_row09_boundary_constants() {
    run_batch("cfg_row09_boundary_constants", BOUNDARY);
}

// ---------------------------------------------------------------------------
// Row 10 — all four bytes < 0x10, forcing the `%02x` zero-pad on every byte.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row10_all_bytes_zero_padded() {
    let mut rng = Rng::new(10);
    let mut inputs = vec![0x0000_0000u32, 0x0101_0101, 0x0f0f_0f0f];
    for _ in 0..2000 {
        let r = rng.next_u32();
        // keep only the low nibble of each byte
        inputs.push(r & 0x0f0f_0f0f);
    }
    for &b in &inputs {
        for byte in b.to_le_bytes() {
            assert!(byte < 0x10, "0x{b:08x} has a byte >= 0x10");
        }
    }
    run_batch("cfg_row10_all_bytes_zero_padded", &inputs);
}

// ---------------------------------------------------------------------------
// Row 11 — all four bytes >= 0x80: the `unsigned char` -> `int` default
// argument promotion for the variadic `printf` must be ZERO-extending. A
// sign-extending translation would print `ffffffxx`.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row11_all_bytes_high_bit_set() {
    let mut rng = Rng::new(11);
    let mut inputs = vec![0x8080_8080u32, 0xffff_ffffu32];
    for _ in 0..2000 {
        inputs.push(rng.next_u32() | 0x8080_8080);
    }
    for &b in &inputs {
        for byte in b.to_le_bytes() {
            assert!(byte >= 0x80, "0x{b:08x} has a byte < 0x80");
        }
    }
    run_batch("cfg_row11_all_bytes_high_bit_set", &inputs);
}

// ---------------------------------------------------------------------------
// Row 12 — exhaustive per-byte coverage: every byte value 0..=255 at every one
// of the 4 positions. Verifies memory-order (little-endian) emission and every
// possible `%02x` rendering.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row12_each_byte_value_each_position() {
    let mut inputs = Vec::with_capacity(4 * 256);
    for pos in 0..4u32 {
        for b in 0..=255u32 {
            inputs.push(b << (8 * pos));
        }
    }
    assert_eq!(inputs.len(), 1024);
    run_batch("cfg_row12_each_byte_value_each_position", &inputs);
}

// ---------------------------------------------------------------------------
// Row 13 — exhaustive sweep of the exponent field x sign, random significand.
// Covers exponent 0 (zero/subnormal) and 255 (inf/NaN) alongside all normals.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row13_exponent_sweep() {
    let mut rng = Rng::new(13);
    let mut inputs = Vec::with_capacity(2 * 256 * 8);
    for sign in 0..2u32 {
        for exp in 0..=255u32 {
            for _ in 0..8 {
                inputs.push(bits(sign, exp, rng.next_u32()));
            }
        }
    }
    assert_eq!(inputs.len(), 4096);
    run_batch("cfg_row13_exponent_sweep", &inputs);
}

// ---------------------------------------------------------------------------
// Row 14 — call multiplicity on the shared stdout stream. Output for N calls
// must be exactly N x 9 bytes with nothing carried across calls, and each call
// captured in isolation must produce exactly its own 9 bytes.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row14_call_multiplicity() {
    let api = api();
    let mut rng = Rng::new(14);

    // Zero calls: an empty capture window must yield zero bytes for both.
    let c_empty = capture(|| {});
    let r_empty = capture(|| {});
    assert_eq!(c_empty, r_empty);
    assert!(c_empty.is_empty(), "empty window produced {c_empty:?}");

    for n in [1usize, 2, 5, 64, 500] {
        let inputs: Vec<u32> = (0..n).map(|_| rng.next_u32()).collect();
        run_batch(&format!("cfg_row14_call_multiplicity(n={n})"), &inputs);
    }

    // Each individual call, captured in its own window, is exactly 9 bytes and
    // identical between C and Rust.
    for &b in BOUNDARY.iter().take(12) {
        let x = f32::from_bits(b);
        let c = capture(|| unsafe { (api.c_driver)(x) });
        let r = capture(|| unsafe { (api.r_driver)(x) });
        assert_eq!(
            c,
            r,
            "single-call divergence for 0x{b:08x}: C {:?} Rust {:?}",
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
        assert_eq!(c.len(), 9, "single call emitted {} bytes", c.len());
        assert_eq!(c, expected_line(b));
    }
}

// ---------------------------------------------------------------------------
// Row 15 — interleaved C / Rust calls into the SAME capture window: neither
// library may perturb the other's buffered stdout, and alternating output must
// pair up line by line.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row15_interleaved_c_and_rust() {
    let api = api();
    let mut rng = Rng::new(15);
    let inputs: Vec<u32> = (0..500).map(|_| rng.next_u32()).collect();

    let out = capture(|| {
        for &b in &inputs {
            let x = f32::from_bits(b);
            unsafe { (api.c_driver)(x) };
            unsafe { (api.r_driver)(x) };
        }
    });

    assert_eq!(
        out.len(),
        inputs.len() * 2 * 9,
        "interleaved stream has unexpected length"
    );
    for (i, &b) in inputs.iter().enumerate() {
        let c_line = &out[i * 18..i * 18 + 9];
        let r_line = &out[i * 18 + 9..i * 18 + 18];
        assert_line_shape(c_line, "cfg_row15 C", i);
        assert_line_shape(r_line, "cfg_row15 Rust", i);
        assert_eq!(
            c_line,
            r_line,
            "interleaved divergence at pair {i}, input 0x{b:08x}: C {:?} Rust {:?}",
            String::from_utf8_lossy(c_line),
            String::from_utf8_lossy(r_line)
        );
        assert_eq!(c_line, &expected_line(b)[..]);
    }
}

// ---------------------------------------------------------------------------
// Row 16 — strided scan of the entire 32-bit input space: bit pattern
// k * 0x00010001 for k = 0..=65535 varies all four bytes together.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row16_strided_full_range_scan() {
    let inputs: Vec<u32> = (0..=65_535u32)
        .map(|k| k.wrapping_mul(0x0001_0001))
        .collect();
    assert_eq!(inputs.len(), 65_536);
    run_batch("cfg_row16_strided_full_range_scan", &inputs);
}

// ---------------------------------------------------------------------------
// Row 17 — the output-shape invariant. Asserted inside `run_batch` for every
// row above (via `assert_line_shape`); this test states it explicitly and adds
// the negative checks: no uppercase hex, no `0x` prefix, no separator bytes.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row17_output_shape_invariant() {
    let api = api();
    let mut rng = Rng::new(17);
    let inputs: Vec<u32> = (0..3000).map(|_| rng.next_u32()).collect();

    for (name, f) in [
        ("C", api.c_driver as DriverFn),
        ("Rust", api.r_driver as DriverFn),
    ] {
        let out = capture(|| {
            for &b in &inputs {
                unsafe { f(f32::from_bits(b)) };
            }
        });
        assert_eq!(out.len(), inputs.len() * 9, "{name}: bad total length");
        for (i, &b) in inputs.iter().enumerate() {
            assert_line_shape(&out[i * 9..i * 9 + 9], name, i);
            assert_eq!(&out[i * 9..i * 9 + 9], &expected_line(b)[..]);
        }
        // Negative checks over the whole stream.
        assert!(
            !out.iter().any(|c| c.is_ascii_uppercase()),
            "{name}: uppercase hex in output"
        );
        assert!(
            !out.windows(2).any(|w| w == b"0x"),
            "{name}: `0x` prefix in output"
        );
        assert!(
            !out.iter().any(|c| *c == b' ' || *c == b',' || *c == b'\r'),
            "{name}: separator byte in output"
        );
        assert_eq!(
            out.iter().filter(|c| **c == b'\n').count(),
            inputs.len(),
            "{name}: newline count != call count"
        );
    }
}
