// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH shared libraries
// through their exported `driver` symbol (loaded with `libloading`) and compares
// the captured stdout byte-for-byte. Rows marked "randomized" use a fixed-seed
// SplitMix64 PRNG so failures are reproducible.
//
// `driver` is simultaneously the lowest-level and the only public entry point,
// so there is no convenience wrapper being tested in place of the real API.

mod harness;

use harness::*;

// Each row gets its own seed so rows stay independent when run selectively.
const SEED_BASE: u64 = 0x0BAD_C0FF_EE0D_D00D;

fn rng_for(row: u64) -> Rng {
    Rng::new(SEED_BASE ^ row.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

// ---------------------------------------------------------------------------
// Row 1 — the two zero encodings
// ---------------------------------------------------------------------------
#[test]
fn row01_signed_zeros() {
    let xs = vec![0.0f64, -0.0f64, 0.0, -0.0];
    assert_same("row01 signed zeros", &xs);

    // Also assert the two zeros are actually distinguished (guards against the
    // test accidentally comparing identical output).
    let c = c_driver();
    let pos = run_one(c, 0.0);
    let neg = run_one(c, -0.0);
    assert_ne!(pos, neg, "C must distinguish +0.0 from -0.0");
}

// ---------------------------------------------------------------------------
// Row 2 / Row 3 — small exactly-representable integers, both signs
// ---------------------------------------------------------------------------
#[test]
fn row02_small_positive_integers() {
    let mut r = rng_for(2);
    let mut xs: Vec<f64> = (1..=1024).map(|i| i as f64).collect();
    for _ in 0..N {
        xs.push(r.range(1, 1024) as f64);
    }
    assert_same("row02 small positive integers", &xs);
}

#[test]
fn row03_small_negative_integers() {
    let mut r = rng_for(3);
    let mut xs: Vec<f64> = (1..=1024).map(|i| -(i as f64)).collect();
    for _ in 0..N {
        xs.push(-(r.range(1, 1024) as f64));
    }
    assert_same("row03 small negative integers", &xs);
}

// ---------------------------------------------------------------------------
// Row 4 / Row 5 — every power of two, 2^-1074 .. 2^1023, both signs
// ---------------------------------------------------------------------------
fn all_powers_of_two(sign: f64) -> Vec<f64> {
    let mut xs = Vec::new();
    // Subnormal powers: 2^-1074 .. 2^-1023 (exponent field 0, single mantissa bit).
    for bit in 0..52u64 {
        xs.push(sign * f64::from_bits(1u64 << bit));
    }
    // Normal powers: 2^-1022 .. 2^1023 (exponent field 1..2046, zero mantissa).
    for e in 1..=2046u64 {
        xs.push(sign * f64::from_bits(e << 52));
    }
    xs
}

#[test]
fn row04_positive_powers_of_two() {
    let mut xs = all_powers_of_two(1.0);
    // Shuffle deterministically so ordering cannot matter.
    let mut r = rng_for(4);
    for i in (1..xs.len()).rev() {
        xs.swap(i, r.below(i as u64 + 1) as usize);
    }
    assert_same("row04 positive powers of two", &xs);
}

#[test]
fn row05_negative_powers_of_two() {
    let mut xs = all_powers_of_two(-1.0);
    let mut r = rng_for(5);
    for i in (1..xs.len()).rev() {
        xs.swap(i, r.below(i as u64 + 1) as usize);
    }
    assert_same("row05 negative powers of two", &xs);
}

// ---------------------------------------------------------------------------
// Row 6 — general normals: random sign, random exponent 1..2046, random 52-bit
// mantissa
// ---------------------------------------------------------------------------
#[test]
fn row06_random_normals_full_mantissa() {
    let mut r = rng_for(6);
    let mut xs = Vec::with_capacity(N * 2);
    for _ in 0..N * 2 {
        let s = r.sign();
        let e = r.range(1, 2046);
        let m = r.next_u64() & ((1u64 << 52) - 1);
        xs.push(compose(s, e, m));
    }
    assert_same("row06 random normals, full mantissa", &xs);
}

// ---------------------------------------------------------------------------
// Row 7 — mantissas with random trailing-zero runs (drives glibc's `%a`
// trailing-hex-digit trimming at every truncation length)
// ---------------------------------------------------------------------------
#[test]
fn row07_mantissa_trailing_zero_runs() {
    let mut r = rng_for(7);
    let mut xs = Vec::with_capacity(N * 2);
    for _ in 0..N * 2 {
        let s = r.sign();
        let e = r.range(1, 2046);
        let k = r.below(53); // 0..=52 trailing zero bits
        let m = (r.next_u64() << k) & ((1u64 << 52) - 1);
        xs.push(compose(s, e, m));
    }
    // Plus a deterministic sweep: exactly k trailing zero bits, all k.
    for k in 0..53u64 {
        let m = if k >= 52 { 0 } else { ((1u64 << (52 - k)) - 1) << k };
        xs.push(compose(0, 1023, m));
        xs.push(compose(1 << 63, 1023, m));
    }
    assert_same("row07 mantissa trailing-zero runs", &xs);
}

// ---------------------------------------------------------------------------
// Row 8 — subnormals with random mantissas
// ---------------------------------------------------------------------------
#[test]
fn row08_random_subnormals() {
    let mut r = rng_for(8);
    let mut xs = Vec::with_capacity(N * 2);
    for _ in 0..N * 2 {
        let s = r.sign();
        // Non-zero mantissa keeps it a genuine subnormal rather than a zero.
        let mut m = r.next_u64() & ((1u64 << 52) - 1);
        if m == 0 {
            m = 1;
        }
        xs.push(compose(s, 0, m));
    }
    // Single-bit subnormals, every bit, both signs.
    for bit in 0..52u64 {
        xs.push(compose(0, 0, 1 << bit));
        xs.push(compose(1 << 63, 0, 1 << bit));
    }
    assert_same("row08 random subnormals", &xs);
}

// ---------------------------------------------------------------------------
// Row 9 — subnormal / normal boundary values and their neighbours
// ---------------------------------------------------------------------------
#[test]
fn row09_subnormal_boundaries() {
    let min_sub = f64::from_bits(1); // 2^-1074, 5e-324
    let max_sub = f64::from_bits((1u64 << 52) - 1);
    let min_norm = f64::MIN_POSITIVE; // 2^-1022
    let mut xs = Vec::new();
    for &v in &[min_sub, max_sub, min_norm] {
        for &sv in &[v, -v] {
            xs.push(sv);
            xs.push(next_up(sv));
            xs.push(next_down(sv));
        }
    }
    // Explicit crossings of the 0 / subnormal / normal frontiers.
    xs.push(next_up(0.0)); // == min subnormal
    xs.push(next_down(0.0)); // == -min subnormal
    xs.push(next_down(min_norm)); // == max subnormal
    xs.push(next_up(max_sub)); // == min normal
    assert_same("row09 subnormal boundaries", &xs);
}

// `nextafter(x, +inf)` / `nextafter(x, -inf)` implemented over raw bits, so the
// test does not depend on a libm import.
fn next_up(x: f64) -> f64 {
    if x.is_nan() || x == f64::INFINITY {
        return x;
    }
    let b = x.to_bits();
    if x == 0.0 {
        return f64::from_bits(1); // both +0.0 and -0.0 step to +min subnormal
    }
    f64::from_bits(if x > 0.0 { b + 1 } else { b - 1 })
}

fn next_down(x: f64) -> f64 {
    if x.is_nan() || x == f64::NEG_INFINITY {
        return x;
    }
    let b = x.to_bits();
    if x == 0.0 {
        return f64::from_bits(1 | (1u64 << 63)); // -min subnormal
    }
    f64::from_bits(if x > 0.0 { b - 1 } else { b + 1 })
}

// ---------------------------------------------------------------------------
// Row 10 — huge magnitudes: `%.4f` emits a ~310-character integer part
// ---------------------------------------------------------------------------
#[test]
fn row10_huge_magnitudes() {
    let mut r = rng_for(10);
    let mut xs = vec![
        f64::MAX,
        -f64::MAX,
        1e308,
        -1e308,
        f64::from_bits(0x7fef_ffff_ffff_ffff), // == DBL_MAX
        next_down(f64::MAX),
    ];
    for _ in 0..N {
        let s = r.sign();
        let e = r.range(2000, 2046);
        let m = r.next_u64() & ((1u64 << 52) - 1);
        xs.push(compose(s, e, m));
    }
    // Sanity: this row really does produce very long lines.
    let out = run_one(c_driver(), f64::MAX);
    assert!(out.len() > 300, "expected a long %.4f field, got {} bytes", out.len());
    assert_same("row10 huge magnitudes", &xs);
}

// ---------------------------------------------------------------------------
// Row 11 — tiny magnitudes: `%.4f` collapses to 0.0000 / -0.0000
// ---------------------------------------------------------------------------
#[test]
fn row11_tiny_magnitudes() {
    let mut r = rng_for(11);
    let mut xs = Vec::with_capacity(N + 8);
    for _ in 0..N {
        let s = r.sign();
        let e = r.range(1, 60);
        let m = r.next_u64() & ((1u64 << 52) - 1);
        xs.push(compose(s, e, m));
    }
    xs.extend_from_slice(&[1e-300, -1e-300, 1e-320, -1e-320, 5e-324, -5e-324]);
    assert_same("row11 tiny magnitudes", &xs);
}

// ---------------------------------------------------------------------------
// Row 12 — `%.4f` half-way / tie shapes (round-half-to-even vs half-away)
// ---------------------------------------------------------------------------
#[test]
fn row12_rounding_ties() {
    let mut xs = Vec::new();
    let bases: &[f64] = &[
        0.00005, 0.00015, 0.00025, 0.00035, 0.00045, 0.00055, 0.00065, 0.00075, 0.00085, 0.00095,
        0.10005, 0.10015, 1.00005, 1.00015, 2.00005, 2.00015, 3.00005, 12.34565, 12.34575,
        0.5 / 10000.0, 1.5 / 10000.0, 2.5 / 10000.0, 3.5 / 10000.0,
    ];
    for &b in bases {
        for &sv in &[b, -b] {
            xs.push(sv);
            xs.push(next_up(sv));
            xs.push(next_down(sv));
            xs.push(next_up(next_up(sv)));
            xs.push(next_down(next_down(sv)));
        }
    }
    // Systematic: k + 0.00005 for many k, plus neighbours.
    let mut r = rng_for(12);
    for _ in 0..N {
        let k = r.below(100_000) as f64;
        let half = r.range(0, 9) as f64;
        let v = k + (half * 10.0 + 5.0) / 100_000.0;
        xs.push(v);
        xs.push(-v);
        xs.push(next_up(v));
        xs.push(next_down(v));
    }
    assert_same("row12 rounding ties", &xs);
}

// ---------------------------------------------------------------------------
// Row 13 — carry propagation out of the fractional part
// ---------------------------------------------------------------------------
#[test]
fn row13_carry_propagation() {
    let mut xs: Vec<f64> = vec![
        0.99995, 0.99999, 9.99995, 9.99999, 99.99995, 999.99995, 999999.99995, 9999999.99999,
        0.999949999, 1.999951, 1e15 - 0.00005, 1e15 + 0.00005,
    ];
    let orig: Vec<f64> = xs.clone();
    for v in orig {
        xs.push(-v);
        xs.push(next_up(v));
        xs.push(next_down(v));
    }
    let mut r = rng_for(13);
    for _ in 0..N {
        let n = r.range(0, 15) as u32;
        let base = 10f64.powi(n as i32);
        let v = base - 0.00005;
        xs.push(v);
        xs.push(-v);
        xs.push(next_up(v));
        xs.push(next_down(v));
        // Random "all nines" style values.
        let k = r.below(1_000_000) as f64;
        xs.push(k + 0.99995);
        xs.push(-(k + 0.99995));
    }
    assert_same("row13 carry propagation", &xs);
}

// ---------------------------------------------------------------------------
// Row 14 — infinities
// ---------------------------------------------------------------------------
#[test]
fn row14_infinities() {
    assert_same(
        "row14 infinities",
        &[f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY],
    );
    assert_same_bits("row14 infinity bit patterns", &[0x7ff0_0000_0000_0000, 0xfff0_0000_0000_0000]);
}

// ---------------------------------------------------------------------------
// Row 15 — NaNs: quiet, signalling, random payloads, both signs
// ---------------------------------------------------------------------------
#[test]
fn row15_nans() {
    let mut bits = vec![
        0x7ff8_0000_0000_0000, // +quiet NaN
        0xfff8_0000_0000_0000, // -quiet NaN
        0x7ff0_0000_0000_0001, // +signalling NaN
        0xfff0_0000_0000_0001, // -signalling NaN
        0x7ff8_dead_beef_cafe, // +quiet NaN with payload
        0xfff8_dead_beef_cafe, // -quiet NaN with payload
        0x7fff_ffff_ffff_ffff, // max +NaN payload
        0xffff_ffff_ffff_ffff, // max -NaN payload
        f64::NAN.to_bits(),
        (-f64::NAN).to_bits(),
    ];
    let mut r = rng_for(15);
    for _ in 0..N {
        let s = r.sign();
        let mut m = r.next_u64() & ((1u64 << 52) - 1);
        if m == 0 {
            m = 1; // keep it a NaN, not an infinity
        }
        bits.push(s | (2047u64 << 52) | m);
    }
    assert_same_bits("row15 NaNs", &bits);
}

// ---------------------------------------------------------------------------
// Row 16 — uniformly random raw u64 bit patterns (the strongest single row)
// ---------------------------------------------------------------------------
#[test]
fn row16_uniform_random_bit_patterns() {
    let mut r = rng_for(16);
    let bits: Vec<u64> = (0..N * 5).map(|_| r.next_u64()).collect();
    assert_same_bits("row16 uniform random bit patterns", &bits);
}

// ---------------------------------------------------------------------------
// Row 17 — wide log-uniform magnitudes, exponent and mantissa decorrelated
// ---------------------------------------------------------------------------
#[test]
fn row17_log_uniform_magnitudes() {
    let mut r = rng_for(17);
    let mut xs = Vec::with_capacity(N * 3);
    for _ in 0..N * 3 {
        let s = r.sign();
        // Full exponent range including 0 (subnormal) and excluding 2047.
        let e = r.below(2047);
        let m = r.next_u64() & ((1u64 << 52) - 1);
        xs.push(compose(s, e, m));
    }
    assert_same("row17 log-uniform magnitudes", &xs);
}

// ---------------------------------------------------------------------------
// Row 18 — the dense unit interval
// ---------------------------------------------------------------------------
#[test]
fn row18_unit_interval() {
    let mut r = rng_for(18);
    let mut xs = Vec::with_capacity(N * 4);
    for _ in 0..N * 2 {
        let u = r.unit();
        xs.push(u);
        xs.push(-u);
    }
    xs.extend_from_slice(&[1.0, -1.0, next_down(1.0), next_up(-1.0)]);
    assert_same("row18 unit interval", &xs);
}

// ---------------------------------------------------------------------------
// Row 19 — zero high word / zero low word (the `%llx` leading-zero-suppression
// and union-pun byte-coverage path)
// ---------------------------------------------------------------------------
#[test]
fn row19_word_aligned_bit_patterns() {
    let mut r = rng_for(19);
    let mut bits = Vec::new();
    for _ in 0..N {
        let lo = r.next_u64() & 0xffff_ffff;
        let hi = r.next_u64() & 0xffff_ffff;
        bits.push(lo); // 0x00000000_xxxxxxxx
        bits.push(hi << 32); // 0xxxxxxxxx_00000000
        bits.push(lo & 0xffff); // very small patterns -> short %llx
        bits.push((hi & 0xff) << 56);
    }
    // Every single-bit pattern: exercises each nibble position of %llx.
    for b in 0..64u64 {
        bits.push(1u64 << b);
    }
    // Every "one nibble set to f" pattern.
    for n in 0..16u64 {
        bits.push(0xfu64 << (n * 4));
    }
    bits.push(0);
    assert_same_bits("row19 word-aligned bit patterns", &bits);
}

// ---------------------------------------------------------------------------
// Row 20 — doubles parsed from random short decimal strings
// ---------------------------------------------------------------------------
#[test]
fn row20_decimal_round_trip_shapes() {
    let mut r = rng_for(20);
    let mut xs = Vec::with_capacity(N * 2);
    for _ in 0..N * 2 {
        let sign = if r.next_u64() & 1 == 0 { "" } else { "-" };
        let d = r.below(10);
        let frac = r.below(10_000_000_000);
        let esign = if r.next_u64() & 1 == 0 { '+' } else { '-' };
        let e = r.below(309);
        let s = format!("{sign}{d}.{frac:010}e{esign}{e}");
        if let Ok(v) = s.parse::<f64>() {
            xs.push(v);
        }
    }
    // Familiar decimal constants a real consumer would pass.
    xs.extend_from_slice(&[
        0.1, 0.2, 0.3, 1.0 / 3.0, 2.0 / 3.0, 3.14159265358979, 2.718281828459045,
        1e-5, 1e-4, 1e-3, 123456.789, -123456.789, 1.0 / 7.0,
    ]);
    assert_same("row20 decimal round-trip shapes", &xs);
}

// ---------------------------------------------------------------------------
// Row 21 — sequential / stateful use and interleaving with the caller's stdout
// ---------------------------------------------------------------------------
#[test]
fn row21_sequential_and_interleaved() {
    // (a) Replay a mixed corpus back-to-back through one handle: proves there
    //     is no hidden per-call state and no buffering divergence.
    let mut r = rng_for(21);
    let mut xs = Vec::with_capacity(8000);
    for _ in 0..4000 {
        xs.push(f64::from_bits(r.next_u64()));
        let e = r.below(2048);
        let m = r.next_u64() & ((1u64 << 52) - 1);
        xs.push(compose(r.sign(), e, m));
    }
    assert_same("row21a sequential replay", &xs);

    // (b) Interleave the library's writes with the test's own libc `printf`
    //     and compare the whole combined stream.
    let cf = c_driver();
    let rf = rust_driver();
    let sample: Vec<f64> = xs.iter().copied().take(200).collect();

    let interleave = |f: DriverFn| {
        capture_stdout(|| {
            for (i, &x) in sample.iter().enumerate() {
                caller_printf_marker(i);
                unsafe { f(x) };
                if i % 3 == 0 {
                    caller_printf_marker(1000 + i);
                }
            }
        })
    };

    let c_out = interleave(cf);
    let r_out = interleave(rf);
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "row21b interleaved stdout stream differs"
    );
    assert!(c_out.contains(&b'\n') && c_out.len() > 200, "interleaved capture looks empty");
}

// ---------------------------------------------------------------------------
// Row 22 — exhaustive exponent-field sweep: all 2048 exponents x representative
// mantissas x both signs
// ---------------------------------------------------------------------------
#[test]
fn row22_exhaustive_exponent_sweep() {
    const MANTISSAS: &[u64] = &[
        0,
        1,
        2,
        0x8_0000_0000_0000,          // top mantissa bit
        0xf_ffff_ffff_ffff,          // all mantissa bits
        0x5_5555_5555_5555,
        0xa_aaaa_aaaa_aaaa,
        0x0_0000_0000_00ff,
        0xf_f000_0000_0000,
        0x1_2345_6789_abcd,
    ];
    let mut bits = Vec::with_capacity(2048 * MANTISSAS.len() * 2);
    for e in 0..2048u64 {
        for &m in MANTISSAS {
            bits.push((e << 52) | m);
            bits.push((1u64 << 63) | (e << 52) | m);
        }
    }
    assert_same_bits("row22 exhaustive exponent sweep", &bits);
}
