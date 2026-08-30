//! Differential tests for `bad`, `good` and the top level `driver` entry point.
//!
//! `bad` and the `goodB2G` branch inside `good` both evaluate
//! `(int)(100.0 / data)`. For `data == 0` that division yields an infinity whose
//! conversion to `int` is undefined in C, so the interesting cases here are the
//! zeros, the infinities, the NaNs and the magnitudes that push the quotient
//! past `INT_MAX`.

mod common;

use common::{c_api, capture, rust_api, show, Rng};

/// Float inputs that stress every branch and every rounding edge.
fn interesting_floats() -> Vec<f32> {
    let mut values: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        3.0,
        -3.0,
        4.0,
        7.0,
        100.0,
        -100.0,
        0.5,
        -0.5,
        0.1,
        1.0 / 3.0,
        // Straddles the `fabs(data) > 0.000001` guard in `goodB2G`.
        0.000_001,
        -0.000_001,
        9.999_999e-7,
        1.000_000_1e-6,
        1.0000001e-6,
        9.99e-7,
        1.01e-6,
        2e-6,
        -2e-6,
        // Quotients that land on or beyond `INT_MAX`.
        100.0 / 2_147_483_647.0,
        100.0 / 2_147_483_648.0,
        4.656_613e-8,
        4.6566126e-8,
        1e-8,
        -1e-8,
        1e-30,
        -1e-30,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        // Subnormals.
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x0080_0000 - 1),
        // Large magnitudes drive the quotient towards zero.
        f32::MAX,
        f32::MIN,
        1e30,
        -1e30,
        // Non finite inputs.
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001), // signalling NaN
        f32::from_bits(0xffc0_0000),
    ];
    // Exact and near-exact divisors of 100 exercise the truncation direction.
    for n in 1..=32u32 {
        values.push(n as f32);
        values.push(-(n as f32));
        values.push(100.0 / n as f32);
    }
    values
}

fn compare_bad(data: f32) {
    let from_c = capture(|| unsafe { (c_api().bad)(data) });
    let from_rust = capture(|| unsafe { (rust_api().bad)(data) });
    assert_eq!(
        from_c,
        from_rust,
        "bad({data:?} / bits {:#010x}): C {} != Rust {}",
        data.to_bits(),
        show(&from_c),
        show(&from_rust)
    );
}

fn compare_good(data: f32) {
    let from_c = capture(|| unsafe { (c_api().good)(data) });
    let from_rust = capture(|| unsafe { (rust_api().good)(data) });
    assert_eq!(
        from_c,
        from_rust,
        "good({data:?} / bits {:#010x}): C {} != Rust {}",
        data.to_bits(),
        show(&from_c),
        show(&from_rust)
    );
}

fn compare_driver(good_data: f32, bad_data: f32) {
    let from_c = capture(|| unsafe { (c_api().driver)(good_data, bad_data) });
    let from_rust = capture(|| unsafe { (rust_api().driver)(good_data, bad_data) });
    assert_eq!(
        from_c,
        from_rust,
        "driver({good_data:?}, {bad_data:?}): C {} != Rust {}",
        show(&from_c),
        show(&from_rust)
    );
}

fn bad_matches_on_interesting_floats() {
    for data in interesting_floats() {
        compare_bad(data);
    }
}

fn bad_matches_on_random_bit_patterns() {
    let mut rng = Rng::new(0x0BAD_0BAD);
    for _ in 0..4096 {
        compare_bad(rng.next_f32_bits());
    }
}

fn good_matches_on_interesting_floats() {
    for data in interesting_floats() {
        compare_good(data);
    }
}

fn good_matches_on_random_bit_patterns() {
    let mut rng = Rng::new(0x600D_600D);
    for _ in 0..4096 {
        compare_good(rng.next_f32_bits());
    }
}

/// The `goodG2B` half of `good` is input independent, so its constant output
/// must appear for every argument. This pins the fixed prefix rather than only
/// checking C against Rust.
fn good_always_emits_the_g2b_result() {
    for data in interesting_floats() {
        let from_c = capture(|| unsafe { (c_api().good)(data) });
        let from_rust = capture(|| unsafe { (rust_api().good)(data) });
        assert!(
            from_c.starts_with(b"50\n"),
            "unexpected C goodG2B output for {data:?}: {}",
            show(&from_c)
        );
        assert_eq!(from_c, from_rust);
    }
}

/// Values at or below the `fabs` threshold must take the message branch.
fn good_below_threshold_takes_the_message_branch() {
    let expected = b"50\nThis would result in a divide by zero\n";
    for data in [
        0.0f32,
        -0.0,
        0.000_001,
        -0.000_001,
        f32::from_bits(1),
        f32::NAN,
        9.0e-7,
    ] {
        let from_c = capture(|| unsafe { (c_api().good)(data) });
        let from_rust = capture(|| unsafe { (rust_api().good)(data) });
        assert_eq!(from_c, from_rust, "good({data:?})");
        assert_eq!(
            from_c,
            expected,
            "good({data:?}) should hit the guard: {}",
            show(&from_c)
        );
    }
}

fn driver_matches_on_interesting_pairs() {
    let values = interesting_floats();
    // Full cross product would be quadratic; pair each value with itself, with
    // the zero cases, and with a rotated partner for coverage.
    for (index, &good_data) in values.iter().enumerate() {
        compare_driver(good_data, good_data);
        compare_driver(good_data, 0.0);
        compare_driver(0.0, good_data);
        let partner = values[(index * 7 + 3) % values.len()];
        compare_driver(good_data, partner);
    }
}

fn driver_matches_on_random_bit_patterns() {
    let mut rng = Rng::new(0xD817_E200);
    for _ in 0..2048 {
        compare_driver(rng.next_f32_bits(), rng.next_f32_bits());
    }
}

fn main() {
    common::run_suite(
        "driver_api",
        &[
        ("bad_matches_on_interesting_floats", bad_matches_on_interesting_floats),
        ("bad_matches_on_random_bit_patterns", bad_matches_on_random_bit_patterns),
        ("good_matches_on_interesting_floats", good_matches_on_interesting_floats),
        ("good_matches_on_random_bit_patterns", good_matches_on_random_bit_patterns),
        ("good_always_emits_the_g2b_result", good_always_emits_the_g2b_result),
        ("good_below_threshold_takes_the_message_branch", good_below_threshold_takes_the_message_branch),
        ("driver_matches_on_interesting_pairs", driver_matches_on_interesting_pairs),
        ("driver_matches_on_random_bit_patterns", driver_matches_on_random_bit_patterns),
        ],
    )
}
