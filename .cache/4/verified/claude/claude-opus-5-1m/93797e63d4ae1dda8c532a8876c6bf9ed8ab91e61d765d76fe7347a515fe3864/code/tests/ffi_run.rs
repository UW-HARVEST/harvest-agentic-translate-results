//! Phase B — differential tests for the lowest-level exported entry point,
//! `void run(house_t *, int)`, called through `dlsym` on both shared objects.
//!
//! Rows R01–R15 of CONFIGS.md.

mod common;
use common::*;

const NOMINAL: (i32, i32, f64) = (2, 5, 2.5);

fn nominal() -> House {
    House::new(NOMINAL.0, NOMINAL.1, NOMINAL.2)
}

// ---------------------------------------------------------------------------
// R01 — the state the program itself uses
// ---------------------------------------------------------------------------
#[test]
fn cfg_r01_nominal() {
    let mut cases = Vec::new();
    for extra in [7, 0, 1, -1, 42, -42] {
        cases.push((nominal(), extra));
    }
    assert_run_batch(&cases, "R01 nominal");
}

// ---------------------------------------------------------------------------
// R02 — floors across the ++ overflow edge
// ---------------------------------------------------------------------------
#[test]
fn cfg_r02_floors_edges() {
    let mut cases = Vec::new();
    for floors in [
        0,
        1,
        -1,
        2,
        i32::MAX - 1,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        -2147483647,
    ] {
        for extra in [0, 7, -7] {
            cases.push((House::new(floors, NOMINAL.1, NOMINAL.2), extra));
        }
    }
    assert_run_batch(&cases, "R02 floors edges");
}

// ---------------------------------------------------------------------------
// R03 — bedrooms += extra_bedrooms, full cross-product at the edges
// ---------------------------------------------------------------------------
#[test]
fn cfg_r03_bedrooms_extra_cross() {
    let bedrooms = [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 0, 5, -5];
    let extras = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let mut cases = Vec::new();
    for b in bedrooms {
        for e in extras {
            cases.push((House::new(NOMINAL.0, b, NOMINAL.2), e));
        }
    }
    assert_run_batch(&cases, "R03 bedrooms x extra");
}

// ---------------------------------------------------------------------------
// R04 — extra_bedrooms sweep + 512 random ints
// ---------------------------------------------------------------------------
#[test]
fn cfg_r04_extra_bedrooms() {
    let mut cases = Vec::new();
    for e in [0, 1, -1, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        cases.push((nominal(), e));
    }
    let mut rng = Rng::new(0xE471_A_5EED);
    for _ in 0..512 {
        cases.push((nominal(), rng.next_i32()));
    }
    assert_run_batch(&cases, "R04 extra sweep");
}

// ---------------------------------------------------------------------------
// R05 — bathrooms with an exact one-decimal value
// ---------------------------------------------------------------------------
#[test]
fn cfg_r05_exact_one_decimal() {
    let mut cases = Vec::new();
    let mut k: i64 = -2000;
    while k <= 2000 {
        cases.push((House::new(1, 1, k as f64 / 10.0), 3));
        k += 7;
    }
    for mag in [0.0, 1.0, 9.0, 99.0, 1234.0, 1e6, 1e9, 1e12, -1e6, -1234.0] {
        for d in 0..10 {
            let v = mag + d as f64 / 10.0;
            cases.push((House::new(1, 1, v), 3));
            cases.push((House::new(1, 1, -v), 3));
        }
    }
    assert_run_batch(&cases, "R05 exact one-decimal");
}

// ---------------------------------------------------------------------------
// R06 — exact ties for %.1f (round-half-to-even in glibc)
// ---------------------------------------------------------------------------
fn tie_values() -> Vec<f64> {
    let mut v = Vec::new();
    for n in -20i64..=20 {
        v.push(n as f64 + 0.25);
        v.push(n as f64 + 0.75);
        v.push(n as f64 - 0.25);
        v.push(n as f64 - 0.75);
        v.push(n as f64 / 4.0);
        v.push(n as f64 / 8.0);
        v.push(n as f64 / 16.0);
        v.push(n as f64 / 32.0);
        v.push(n as f64 + 0.5);
    }
    for scale in [1.0f64, 10.0, 100.0, 1e5, 1e10] {
        for n in [1i64, 3, 5, 7, 9, 11, 13, 15] {
            v.push(scale * n as f64 + 0.25);
            v.push(scale * n as f64 + 0.75);
            v.push(-(scale * n as f64 + 0.25));
            v.push(-(scale * n as f64 + 0.75));
        }
    }
    v
}

#[test]
fn cfg_r06_exact_ties() {
    let cases: Vec<(House, i32)> = tie_values()
        .into_iter()
        .map(|b| (House::new(3, -4, b), 6))
        .collect();
    assert_run_batch(&cases, "R06 exact ties");
}

// ---------------------------------------------------------------------------
// R07 — decimals whose nearest double sits just below/above a .x5 tie
// ---------------------------------------------------------------------------
fn near_tie_values() -> Vec<f64> {
    let mut v: Vec<f64> = vec![
        0.05, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85, 0.95, 1.05, 1.15, 1.25, 1.35, 1.45,
        2.05, 2.15, 2.25, 2.35, 2.45, 3.05, 3.15, 4.35, 5.55, 6.65, 7.75, 8.3, 8.85, 9.95, 0.145,
        0.1449999999999999, 0.15000000000000002, 12.05, 123.05, 1234.05, 99999.95, 1e15 + 0.5,
        0.049999999999999996, 0.050000000000000003,
    ];
    let n = v.len();
    for i in 0..n {
        v.push(-v[i]);
        // one ulp either side of each of these
        v.push(f64::from_bits(v[i].to_bits() + 1));
        v.push(f64::from_bits(v[i].to_bits() - 1));
    }
    v
}

#[test]
fn cfg_r07_near_ties() {
    let cases: Vec<(House, i32)> = near_tie_values()
        .into_iter()
        .map(|b| (House::new(-1, 7, b), -3))
        .collect();
    assert_run_batch(&cases, "R07 near ties");
}

// ---------------------------------------------------------------------------
// R08 — zero and negative zero
// ---------------------------------------------------------------------------
#[test]
fn cfg_r08_zeros() {
    let mut cases = Vec::new();
    for b in [0.0f64, -0.0f64, 0.04, -0.04, -0.0000001, 0.0000001] {
        for extra in [0, 1] {
            cases.push((House::new(0, 0, b), extra));
        }
    }
    assert_run_batch(&cases, "R08 zeros");
}

// ---------------------------------------------------------------------------
// R09 — non-finite bathrooms
// ---------------------------------------------------------------------------
fn non_finite_values() -> Vec<f64> {
    vec![
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001), // quiet NaN, payload 1
        f64::from_bits(0xfff8_0000_dead_beef), // negative quiet NaN, payload
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xfff0_0000_0000_0001), // negative signalling NaN
        f64::from_bits(0x7ff_f_ffff_ffff_ffff),
    ]
}

#[test]
fn cfg_r09_non_finite() {
    let cases: Vec<(House, i32)> = non_finite_values()
        .into_iter()
        .flat_map(|b| [(House::new(2, 2, b), 1), (House::new(-9, -9, b), -1)])
        .collect();
    assert_run_batch(&cases, "R09 non-finite");
}

// ---------------------------------------------------------------------------
// R10 — subnormal / tiny magnitudes
// ---------------------------------------------------------------------------
fn tiny_values() -> Vec<f64> {
    let mut v = vec![
        5e-324,
        f64::from_bits(1),
        f64::from_bits(2),
        f64::from_bits(0x000f_ffff_ffff_ffff), // largest subnormal
        f64::MIN_POSITIVE,
        2.2250738585072014e-308,
        1e-300,
        1e-16,
        1e-10,
        1e-5,
        0.0499999,
    ];
    let n = v.len();
    for i in 0..n {
        v.push(-v[i]);
    }
    v
}

#[test]
fn cfg_r10_tiny() {
    let cases: Vec<(House, i32)> = tiny_values()
        .into_iter()
        .map(|b| (House::new(1, 2, b), 4))
        .collect();
    assert_run_batch(&cases, "R10 tiny");
}

// ---------------------------------------------------------------------------
// R11 — large magnitudes, incl. the 2^53/10 neighbourhood
// ---------------------------------------------------------------------------
fn large_values() -> Vec<f64> {
    let mut v = Vec::new();
    let boundary: f64 = 9.007_199_254_740_992e15 / 10.0; // 2^53 / 10
    for d in -4i64..=4 {
        let mut b = boundary;
        if d >= 0 {
            for _ in 0..d {
                b = f64::from_bits(b.to_bits() + 1);
            }
        } else {
            for _ in 0..(-d) {
                b = f64::from_bits(b.to_bits() - 1);
            }
        }
        v.push(b);
    }
    v.extend_from_slice(&[
        9.007_199_254_740_992e15,       // 2^53
        9.007_199_254_740_991e15,
        4.503_599_627_370_496e15,       // 2^52
        4.503_599_627_370_496e15 + 0.5,
        1e15,
        1e16,
        1e17,
        1e300,
        1.7976931348623157e308, // DBL_MAX
        f64::from_bits(0x7fef_ffff_ffff_fffe),
        123456789012345.6,
        1e22,
        1e23,
    ]);
    let n = v.len();
    for i in 0..n {
        v.push(-v[i]);
    }
    v
}

#[test]
fn cfg_r11_large() {
    let cases: Vec<(House, i32)> = large_values()
        .into_iter()
        .map(|b| (House::new(5, 6, b), 7))
        .collect();
    assert_run_batch(&cases, "R11 large");
}

// ---------------------------------------------------------------------------
// R12 — values where `bathrooms += 1.0` is lossy / saturating
// ---------------------------------------------------------------------------
#[test]
fn cfg_r12_lossy_increment() {
    let vals = [
        9.007_199_254_740_992e15, // 2^53: +1.0 is a no-op
        9.007_199_254_740_991e15,
        1.8014398509481984e16, // 2^54
        1e16,
        1e300,
        f64::MAX,
        -f64::MAX,
        -1.0,
        -0.5,
        -1.5,
        -2.0,
        -0.9999999999999999,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
    ];
    let cases: Vec<(House, i32)> = vals
        .into_iter()
        .map(|b| (House::new(8, 9, b), 2))
        .collect();
    assert_run_batch(&cases, "R12 lossy increment");
    assert_run_twice_batch(&cases, "R12 lossy increment x2");
}

// ---------------------------------------------------------------------------
// R13 — uniformly random bit patterns
// ---------------------------------------------------------------------------
#[test]
fn cfg_r13_random_bit_patterns() {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    let mut cases = Vec::with_capacity(20000);
    for _ in 0..20000 {
        cases.push((
            House::new(rng.next_i32(), rng.next_i32(), rng.next_f64_bits()),
            rng.next_i32(),
        ));
    }
    assert_run_batch(&cases, "R13 random bits");
}

// ---------------------------------------------------------------------------
// R14 — random "mixed" doubles (quarters, tenths, scaled)
// ---------------------------------------------------------------------------
#[test]
fn cfg_r14_random_mixed() {
    let mut rng = Rng::new(0x0FED_CBA9_8765_4321);
    let mut cases = Vec::with_capacity(20000);
    for _ in 0..20000 {
        cases.push((
            House::new(rng.next_i32(), rng.next_i32(), rng.next_f64_mixed()),
            rng.next_i32(),
        ));
    }
    assert_run_batch(&cases, "R14 random mixed");
}

// ---------------------------------------------------------------------------
// R15 — two consecutive calls on the carried-over struct (what main does)
// ---------------------------------------------------------------------------
#[test]
fn cfg_r15_two_consecutive_calls() {
    let mut vals = tie_values();
    vals.extend(near_tie_values());
    vals.extend(non_finite_values());
    vals.extend(tiny_values());
    vals.extend(large_values());
    let cases: Vec<(House, i32)> = vals
        .into_iter()
        .map(|b| (House::new(i32::MAX - 1, i32::MAX - 3, b), 5))
        .collect();
    assert_run_twice_batch(&cases, "R15 run twice");

    // ... and randomised
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
    let mut cases = Vec::with_capacity(1024);
    for _ in 0..1024 {
        cases.push((
            House::new(rng.next_i32(), rng.next_i32(), rng.next_f64_mixed()),
            rng.next_i32(),
        ));
    }
    assert_run_twice_batch(&cases, "R15 run twice random");
}

// ---------------------------------------------------------------------------
// R16 — random decimal-looking doubles: the densest source of %.1f rounding
// decisions (m / 10^d)
// ---------------------------------------------------------------------------
#[test]
fn cfg_r16_random_decimalish() {
    let mut rng = Rng::new(0xABCD_1234_5678_9EF0);
    let mut cases = Vec::with_capacity(20000);
    for _ in 0..20000 {
        cases.push((
            House::new(rng.next_i32(), rng.next_i32(), rng.next_f64_decimalish()),
            rng.next_i32(),
        ));
    }
    assert_run_batch(&cases, "R16 random decimalish");

    // the same values, but reached after `+= 1.0` twice (two run() calls)
    let mut rng = Rng::new(0x0F0F_0F0F_1234_5678);
    let mut cases = Vec::with_capacity(4000);
    for _ in 0..4000 {
        cases.push((
            House::new(rng.next_i32(), rng.next_i32(), rng.next_f64_decimalish()),
            rng.next_i32(),
        ));
    }
    assert_run_twice_batch(&cases, "R16 random decimalish x2");
}

// ---------------------------------------------------------------------------
// R17 — structured sweep of the values `%.1f` finds hardest:
//   * every k/10 for k in [-5000, 5000] and its two ulp neighbours (the fast
//     path's `v*10.0 == trunc` test is decided by one ulp here)
//   * every dyadic rational n/2^k for n in [-1500, 1500], k in 1..=12 (exact
//     ties: .5, .25, .125, .0625, ... at many magnitudes)
// ---------------------------------------------------------------------------
#[test]
fn cfg_r17_ulp_neighbourhood_and_dyadics() {
    let mut cases: Vec<(House, i32)> = Vec::with_capacity(80_000);
    for k in -5000i64..=5000 {
        let v = k as f64 / 10.0;
        for w in [v, f64::from_bits(v.to_bits() + 1), f64::from_bits(v.to_bits().wrapping_sub(1))] {
            cases.push((House::new(1, 2, w), 3));
        }
    }
    assert_run_batch(&cases, "R17 k/10 +- 1ulp");

    let mut cases: Vec<(House, i32)> = Vec::with_capacity(40_000);
    for k in 1u32..=12 {
        let den = (1u64 << k) as f64;
        for n in -1500i64..=1500 {
            cases.push((House::new(4, 5, n as f64 / den), 6));
        }
    }
    assert_run_batch(&cases, "R17 dyadics");

    // ... and the same values scaled by powers of ten, so the rounding decision
    // happens at different exponents
    let mut cases: Vec<(House, i32)> = Vec::with_capacity(40_000);
    for e in [-3i32, -1, 1, 3, 6, 12, 15] {
        let scale = 10f64.powi(e);
        for n in -700i64..=700 {
            for k in [1u32, 2, 3] {
                cases.push((House::new(7, 8, (n as f64 / (1u64 << k) as f64) * scale), 9));
            }
        }
    }
    assert_run_batch(&cases, "R17 scaled dyadics");
}
