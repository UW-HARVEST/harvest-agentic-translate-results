//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every assertion compares the C `.so` against the Rust `.so`, both loaded via
//! `libloading` and invoked only through their exported `float2half` symbol.

mod common;

use common::{bits_for, observed_base, observed_shift, pair, Rng, OBSERVABLE_SHIFT_CAP, SHIFT_RUNS};

/// Sanity: the harness really did load two *different* shared objects and both
/// export the symbol. Guards against the whole suite silently comparing a
/// library against itself.
#[test]
fn harness_loads_two_distinct_libraries() {
    let p = pair();
    assert_ne!(
        p.c_path, p.rust_path,
        "C and Rust .so paths must differ, got {:?}",
        p.c_path
    );
    assert!(p.c_path.is_file(), "missing C .so: {}", p.c_path.display());
    assert!(
        p.rust_path.is_file(),
        "missing Rust .so: {}",
        p.rust_path.display()
    );
    assert!(
        p.c_path.to_string_lossy().contains("c_src"),
        "C library should come from c_src/, got {}",
        p.c_path.display()
    );
    // Both must actually answer.
    let a = unsafe { (p.c)(1.0f32) };
    let b = unsafe { (p.rust)(1.0f32) };
    assert_eq!(a, b);
    assert_eq!(a, 0x3c00, "float2half(1.0) should be half 1.0 = 0x3c00");
}

/// The two lookup tables are `static` in C, so they are only observable through
/// `float2half`. Recover all 512 entries of each table from *both* libraries and
/// require them to agree — this is the behavioural equivalent of diffing the
/// tables element-by-element.
#[test]
fn tables_match_c_source() {
    let p = pair();
    for j in 0..512u32 {
        let cb = observed_base(p.c, j);
        let rb = observed_base(p.rust, j);
        assert_eq!(cb, rb, "m__base[{j}] differs: C=0x{cb:04x} Rust=0x{rb:04x}");

        let cs = observed_shift(p.c, j);
        let rs = observed_shift(p.rust, j);
        assert_eq!(cs, rs, "m__shift[{j}] differs: C={cs} Rust={rs}");

        // Also cross-check against the run layout recorded in CONFIGS.md so a
        // silently-identical-but-wrong pair of tables would still be caught.
        let expected = SHIFT_RUNS
            .iter()
            .find(|(lo, hi, _)| j >= *lo && j <= *hi)
            .map(|(_, _, s)| *s)
            .unwrap_or_else(|| panic!("j={j} not covered by SHIFT_RUNS"));
        // Shifts >= 23 discard the whole 23-bit mantissa and so are not
        // distinguishable from each other through the public API.
        assert_eq!(
            cs,
            expected.min(OBSERVABLE_SHIFT_CAP),
            "m__shift[{j}] observed {cs} but CONFIGS.md run layout says {expected}"
        );
    }
}

/// C1 — behaviourally exhaustive over all 2^32 inputs.
///
/// `float2half(n)` is exactly `base[j] + ((n & 0x7fffff) >> shift[j])` with
/// `j = (n >> 23) & 0x1ff`, so the result depends on the input only through the
/// pair `(j, q)` where `q = mantissa >> shift[j]`. Enumerating every `j`
/// against every attainable `q`, with both the all-zero and the all-one
/// discarded low bits, therefore covers every one of the 2^32 possible float
/// bit patterns' behaviours.
#[test]
fn configs_exhaustive_by_reduction() {
    let p = pair();
    let mut cases = 0u64;

    for j in 0..512u32 {
        let shift = observed_shift(p.c, j);
        assert!(
            (13..=24).contains(&shift),
            "shift for j={j} out of expected 13..=24 range: {shift}"
        );

        if shift >= 23 {
            // At shift 23 only q in {0,1} exist; at 24 only q == 0. Enumerate
            // the mantissa quotients directly.
            let q_count = 1u32 << (23u32.saturating_sub(shift));
            for q in 0..q_count.max(1) {
                let lo_mask = (1u32 << shift.min(23)) - 1;
                let hi = (q << shift.min(23)) & 0x007f_ffff;
                p.check_bits(bits_for(j, hi));
                p.check_bits(bits_for(j, (hi | lo_mask) & 0x007f_ffff));
                cases += 2;
            }
            // Extra: the maximal mantissa, which for shift 24 is the only way
            // to see that all bits really are discarded.
            p.check_bits(bits_for(j, 0x007f_ffff));
            cases += 1;
            continue;
        }

        let lo_mask = (1u32 << shift) - 1;
        let q_count = 1u32 << (23 - shift);
        for q in 0..q_count {
            let hi = q << shift;
            // Minimum discarded low bits.
            p.check_bits(bits_for(j, hi));
            // Maximum discarded low bits (same q, must give the same answer).
            p.check_bits(bits_for(j, hi | lo_mask));
            cases += 2;
        }
    }

    // 62 buckets with shift 13 dominate: 62 * 1024 * 2 ~= 127k.
    assert!(
        cases > 100_000,
        "exhaustive-by-reduction sweep only covered {cases} cases, expected >100k"
    );
}

/// C2 — 2,000,000 uniform random 32-bit patterns, fixed seed.
#[test]
fn configs_random_bit_patterns() {
    let p = pair();
    let mut rng = Rng::new();
    for _ in 0..2_000_000u32 {
        p.check_bits(rng.next_u32());
    }
}

/// C3 — 500,000 structured random floats: independent random sign, exponent
/// drawn from a set weighted toward the interesting boundaries, and mantissa.
#[test]
fn configs_random_structured_floats() {
    let p = pair();
    let mut rng = Rng::with_seed(0xDEAD_BEEF_1234_5678);

    // Exponents clustered around half-precision underflow (~0x66..0x71),
    // the normal range (0x71..0x8f), overflow (0x8f..0xfe), and the specials.
    let interesting: [u32; 20] = [
        0x00, 0x01, 0x02, 0x64, 0x65, 0x66, 0x67, 0x68, 0x6f, 0x70, 0x71, 0x7f, 0x8d, 0x8e, 0x8f,
        0x90, 0xfd, 0xfe, 0xff, 0x7e,
    ];

    for _ in 0..500_000u32 {
        let sign = rng.next_u32() & 1;
        let exp = if rng.next_u32() % 3 == 0 {
            // Fully random exponent.
            rng.next_u32() & 0xff
        } else {
            interesting[(rng.below(interesting.len() as u32)) as usize]
        };
        let mantissa = rng.next_u32() & 0x007f_ffff;
        p.check_bits((sign << 31) | (exp << 23) | mantissa);
    }
}

/// C4..C31 — one sub-case per contiguous `m__shift` run, with randomized
/// mantissas (and every `j` in the run visited at least once).
#[test]
fn configs_per_shift_run() {
    let p = pair();
    let mut rng = Rng::with_seed(0x0BAD_C0DE_0BAD_C0DE);

    for (idx, (lo, hi, shift)) in SHIFT_RUNS.iter().enumerate() {
        let row = idx + 4; // CONFIGS.md rows C4..C31
        let mut checked = 0u32;

        for j in *lo..=*hi {
            // Confirm this bucket really has the run's shift, in both libs.
            let cs = observed_shift(p.c, j);
            let rs = observed_shift(p.rust, j);
            assert_eq!(cs, rs, "row C{row}: shift mismatch at j={j}");
            assert_eq!(
                cs,
                (*shift).min(OBSERVABLE_SHIFT_CAP),
                "row C{row}: expected shift {shift} at j={j}, got {cs}"
            );

            // Deterministic edge mantissas plus randomized ones.
            let s = (*shift).min(22);
            let edges = [
                0u32,
                1,
                (1u32 << s) - 1,
                1u32 << s,
                (1u32 << s) | 1,
                0x0040_0000,
                0x003f_ffff,
                0x007f_ffff,
                0x007f_fffe,
            ];
            for m in edges {
                p.check_bits(bits_for(j, m & 0x007f_ffff));
                checked += 1;
            }
            for _ in 0..64 {
                p.check_bits(bits_for(j, rng.next_u32() & 0x007f_ffff));
                checked += 1;
            }
        }

        assert!(checked > 0, "row C{row} exercised no inputs");
    }
}

/// C32..C36 — the five mantissa sub-shapes, each crossed with all 512 buckets.
#[test]
fn configs_mantissa_subshapes() {
    let p = pair();

    for j in 0..512u32 {
        let shift = observed_shift(p.c, j).min(23);

        // C32: mantissa 0.
        p.check_bits(bits_for(j, 0));
        // C33: quotient 0 with every discarded bit set.
        p.check_bits(bits_for(j, (1u32 << shift) - 1));
        // C34: quotient exactly 1.
        p.check_bits(bits_for(j, (1u32 << shift) & 0x007f_ffff));
        // C35: all mantissa bits set (maximum quotient).
        p.check_bits(bits_for(j, 0x007f_ffff));
        // C36: only the quiet-NaN bit set.
        p.check_bits(bits_for(j, 0x0040_0000));
    }
}

/// C37 — FFI representation: non-canonical bit patterns (sNaN payloads,
/// pseudo-denormals, all-ones) must cross the `extern "C"` boundary unchanged in
/// both directions for both libraries.
#[test]
fn configs_ffi_representation() {
    let p = pair();

    let patterns: [u32; 24] = [
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x8000_0001,
        0x007f_ffff,
        0x807f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7f80_0001, // sNaN, minimal payload
        0xff80_0001,
        0x7fbf_ffff, // sNaN, maximal payload
        0xffbf_ffff,
        0x7fc0_0000, // qNaN
        0xffc0_0000,
        0x7fff_ffff, // qNaN, all payload bits
        0xffff_ffff,
        0x7f7f_ffff, // f32::MAX
        0xff7f_ffff,
        0x0080_0000, // f32::MIN_POSITIVE
        0x8080_0000,
        0x3f80_0000, // 1.0
        0xbf80_0000,
        0x4780_0000, // 65536.0
        0x477f_e000, // 65504.0, largest finite half
    ];

    for bits in patterns {
        let x = f32::from_bits(bits);
        // The argument must survive being materialised as an f32 at all.
        assert_eq!(
            x.to_bits(),
            bits,
            "f32::from_bits/to_bits round trip lost bits for 0x{bits:08x}"
        );

        let got_c = unsafe { (p.c)(x) };
        let got_rust = unsafe { (p.rust)(x) };
        assert_eq!(
            got_c, got_rust,
            "FFI representation divergence for 0x{bits:08x}: C=0x{got_c:04x} Rust=0x{got_rust:04x}"
        );
    }

    // Widening the return: the ABI puts a uint16_t in `ax` and leaves the upper
    // bits of `eax` unspecified. Re-reading the same symbol through a
    // `u32`-returning signature must still agree on the low 16 bits, which is
    // all the declared return type promises.
    unsafe {
        type Wide = unsafe extern "C" fn(f32) -> u32;
        let c_wide: Wide = std::mem::transmute::<common::Float2Half, Wide>(p.c);
        let rust_wide: Wide = std::mem::transmute::<common::Float2Half, Wide>(p.rust);
        for bits in patterns {
            let x = f32::from_bits(bits);
            let cw = c_wide(x) & 0xffff;
            let rw = rust_wide(x) & 0xffff;
            assert_eq!(cw, rw, "low-16 return divergence for 0x{bits:08x}");
            assert_eq!(cw as u16, (p.c)(x));
        }
    }
}

/// C38 — representative human-meaningful values a real consumer would pass.
#[test]
fn configs_representative_values() {
    let p = pair();

    let mut values: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        -2.0,
        3.0,
        10.0,
        100.0,
        1000.0,
        65504.0,   // largest finite half
        -65504.0,
        65505.0,
        65519.0,
        65520.0,   // first value rounding to half infinity
        65536.0,
        -65536.0,
        6.103_515_6e-5, // smallest normal half
        6.0e-5,          // half subnormal
        5.960_464_5e-8,  // smallest positive half subnormal
        2.980_232_2e-8,  // below the smallest half subnormal
        std::f32::consts::PI,
        std::f32::consts::E,
        std::f32::consts::LN_2,
        f32::MAX,
        f32::MIN,
        f32::EPSILON,
        f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
    ];

    // Powers of two across the whole exponent range, both signs.
    for e in -30i32..=30 {
        let v = 2.0f32.powi(e);
        values.push(v);
        values.push(-v);
        // And a value just off each power of two.
        values.push(v * 1.000_001);
        values.push(v * 0.999_999);
    }

    // f32 subnormals near zero.
    for k in 0..24u32 {
        values.push(f32::from_bits(1 << k));
        values.push(f32::from_bits(0x8000_0000 | (1 << k)));
    }

    for v in values {
        p.check_f32(v);
    }
}

/// C39 — no hidden mutable state: interleaving thousands of other calls between
/// two identical calls must not change the answer, in either library.
#[test]
fn configs_statelessness() {
    let p = pair();
    let mut rng = Rng::with_seed(0x5EED_5EED_5EED_5EED);

    let probes: [u32; 6] = [
        0x3f80_0000,
        0xbf80_0000,
        0x7f80_0000,
        0x0000_0001,
        0x477f_e000,
        0x7fff_ffff,
    ];

    let before: Vec<u16> = probes.iter().map(|&b| p.agreed(b)).collect();

    for _ in 0..10_000u32 {
        p.check_bits(rng.next_u32());
    }

    let after: Vec<u16> = probes.iter().map(|&b| p.agreed(b)).collect();
    assert_eq!(before, after, "float2half is not stateless");

    // Also assert the two libraries agree call-for-call under repetition.
    for &b in &probes {
        for _ in 0..1000 {
            p.check_bits(b);
        }
    }
}
