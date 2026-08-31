//! Tests for the lowest-level exported function, `run`.
//!
//! `run` transitively exercises the static helpers `print_house`, `add_floor`
//! and `add_bedrooms`, which are not exported themselves.

mod common;

use common::{assert_run_matches, house_t};

fn h(floors: i32, bedrooms: i32, bathrooms: f64) -> house_t {
    house_t {
        floors,
        bedrooms,
        bathrooms,
    }
}

fn run_default_house() {
    assert_run_matches(h(2, 5, 2.5), 0);
    assert_run_matches(h(2, 5, 2.5), 1);
    assert_run_matches(h(2, 5, 2.5), -1);
    assert_run_matches(h(2, 5, 2.5), 7);
}

fn run_zeroed_and_small() {
    for extra in [-3, -1, 0, 1, 3] {
        assert_run_matches(h(0, 0, 0.0), extra);
        assert_run_matches(h(1, 1, 1.0), extra);
        assert_run_matches(h(-1, -1, -1.0), extra);
    }
}

fn run_negative_zero_bathrooms() {
    // -0.0 + 1.0 == 1.0, but the first three prints show "-0.0".
    assert_run_matches(h(3, 4, -0.0), 2);
}

fn run_rounding_boundaries() {
    // %.1f rounding: exercise ties and near-ties in both the pre- and
    // post-increment prints.
    let vals = [
        0.05, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85, 0.95, 1.05, 2.45, 2.55, -0.05,
        -0.15, -0.25, -2.45, -2.55, 0.049_999_999_999_999_99, 0.050_000_000_000_000_003,
        99.949_999_999_999_99, 99.95, 0.999_999_999_999_999_9,
    ];
    for v in vals {
        assert_run_matches(h(1, 2, v), 1);
    }
}

fn run_special_doubles() {
    let specials = [
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
        f64::from_bits(1), // smallest subnormal
        -f64::from_bits(1),
        f64::EPSILON,
        f64::MAX,
        f64::MIN,
        1e308,
        -1e308,
        1e-308,
        1e16,
        1e17,
        9_007_199_254_740_993.0, // 2^53 + 1 (not representable, rounds)
        4_503_599_627_370_497.0, // 2^52 + 1
        1.0 / 3.0,
        std::f64::consts::PI,
        std::f64::consts::E,
    ];
    for v in specials {
        assert_run_matches(h(2, 5, v), 3);
    }
}

fn run_signalling_nan_bit_pattern() {
    // A signalling NaN payload: printf must render it the same way on both
    // sides, and the `+= 1.0` result must have identical bits.
    for bits in [0x7ff0_0000_0000_0001u64, 0xfff0_0000_0000_0001, 0x7ff8_dead_beef_0000] {
        assert_run_matches(h(1, 1, f64::from_bits(bits)), 1);
    }
}

fn run_integer_extremes() {
    // The C code increments `floors` and adds to `bedrooms` without any
    // overflow checks; replicate whatever the compiled C does.
    assert_run_matches(h(i32::MAX, 0, 1.0), 0);
    assert_run_matches(h(i32::MAX - 1, 0, 1.0), 0);
    assert_run_matches(h(i32::MIN, 0, 1.0), 0);
    assert_run_matches(h(0, i32::MAX, 1.0), 1);
    assert_run_matches(h(0, i32::MAX, 1.0), i32::MAX);
    assert_run_matches(h(0, i32::MIN, 1.0), -1);
    assert_run_matches(h(0, i32::MIN, 1.0), i32::MIN);
    assert_run_matches(h(0, 0, 1.0), i32::MAX);
    assert_run_matches(h(0, 0, 1.0), i32::MIN);
    assert_run_matches(h(i32::MIN, i32::MIN, 1.0), i32::MIN);
    assert_run_matches(h(i32::MAX, i32::MAX, 1.0), i32::MAX);
}

fn run_pseudo_random_sweep() {
    // Deterministic xorshift sweep over the whole input space.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..300 {
        let floors = next() as u32 as i32;
        let bedrooms = next() as u32 as i32;
        // Mix of ordinary-magnitude and arbitrary bit-pattern doubles.
        let bathrooms = if next() & 1 == 0 {
            (next() as i64 as f64) / 8.0
        } else {
            f64::from_bits(next())
        };
        let extra = next() as u32 as i32;
        assert_run_matches(house_t { floors, bedrooms, bathrooms }, extra);
    }
}

/// Systematic sweep over the double exponent range and over mantissa patterns,
/// since `%.1f` and the `+= 1.0` addition must agree exactly.
fn run_double_exponent_sweep() {
    for exp in -40i32..=40 {
        for mant in [1.0f64, 1.5, 1.25, 1.9999999999, 3.0, 7.0, 9.999] {
            let v = mant * 10f64.powi(exp);
            assert_run_matches(h(1, 2, v), 1);
            assert_run_matches(h(1, 2, -v), 1);
        }
    }
    // Powers of two around the point where +1.0 stops changing the value.
    for p in 40i32..=60 {
        let v = 2f64.powi(p);
        assert_run_matches(h(1, 2, v), 1);
        assert_run_matches(h(1, 2, v + 0.5), 1);
        assert_run_matches(h(1, 2, -v), 1);
    }
    // Every exponent field value with a fixed mantissa, including 0 and 0x7ff.
    for e in 0u64..=0x7ff {
        let bits = (e << 52) | 0x000f_1234_5678_9abc;
        assert_run_matches(h(1, 2, f64::from_bits(bits)), 1);
        assert_run_matches(h(1, 2, f64::from_bits(bits | (1 << 63))), 1);
    }
}

fn run_extended_random_sweep() {
    let mut state: u64 = 0x1234_5678_9abc_def0;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..2000 {
        let floors = next() as u32 as i32;
        let bedrooms = next() as u32 as i32;
        let bathrooms = match next() % 4 {
            0 => (next() as i64 as f64) / 4.0,
            1 => f64::from_bits(next()),
            2 => (next() % 1000) as f64 / 8.0,
            _ => f64::from_bits((next() & 0x800f_ffff_ffff_ffff) | (((next() % 0x800) as u64) << 52)),
        };
        let extra = next() as u32 as i32;
        assert_run_matches(
            house_t {
                floors,
                bedrooms,
                bathrooms,
            },
            extra,
        );
    }
}

// Single entry point: fd 1 redirection during capture is process-global, so
// this binary must run exactly one libtest test.
#[test]
fn all_cases() {
    run_default_house();
    run_zeroed_and_small();
    run_negative_zero_bathrooms();
    run_rounding_boundaries();
    run_special_doubles();
    run_signalling_nan_bit_pattern();
    run_integer_extremes();
    run_double_exponent_sweep();
    run_pseudo_random_sweep();
    run_extended_random_sweep();
}
