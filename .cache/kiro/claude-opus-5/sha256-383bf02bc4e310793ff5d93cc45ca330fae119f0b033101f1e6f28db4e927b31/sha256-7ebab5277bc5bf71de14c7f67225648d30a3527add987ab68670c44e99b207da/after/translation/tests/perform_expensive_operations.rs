//! Differential tests for the lowest-level exported function,
//! `perform_expensive_operations`, which is a pure map over the `array` global.

mod common;

use common::{assert_arrays_equal, load_both, SplitMix64, ARRAY_SIZE};
use std::ffi::c_int;

/// Fill a full-size payload by cycling `seeds`.
fn tile(seeds: &[c_int]) -> Vec<c_int> {
    (0..ARRAY_SIZE).map(|i| seeds[i % seeds.len()]).collect()
}

/// Run one payload through both libraries and compare the resulting arrays
/// element-wise and byte-for-byte.
fn check_payload(label: &str, payload: &[c_int]) {
    let guard = load_both();
    let (c, rust) = &*guard;

    c.write_array(payload);
    rust.write_array(payload);

    c.perform_expensive_operations();
    rust.perform_expensive_operations();

    assert_arrays_equal(label, &c.read_array(), &rust.read_array());
    assert_eq!(
        c.read_array_bytes(),
        rust.read_array_bytes(),
        "{label}: raw bytes of `array` differ"
    );
}

#[test]
fn zeros() {
    // Zero is the `.bss` initial state of `array`, so it is the single most
    // important input to get right.
    let payload = vec![0 as c_int; ARRAY_SIZE];
    check_payload("all zeros", &payload);

    // Sanity: the call must actually mutate the array, otherwise a pair of
    // no-op stubs would pass every test in this file.
    let guard = load_both();
    let (c, rust) = &*guard;
    for imp in [c, rust] {
        imp.write_array(&payload);
        imp.perform_expensive_operations();
        assert_ne!(
            imp.read_array(),
            payload,
            "{} `perform_expensive_operations` left the array untouched",
            imp.name()
        );
    }
}

#[test]
fn boundary_values() {
    let seeds: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        6,
        -6,
        7,
        -7,
        8,
        -8,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MIN / 2,
        i32::MAX / 3,
        i32::MIN / 3,
        1 << 30,
        -(1 << 30),
        1 << 29,
        0x5555_5555u32 as c_int,
        0xAAAA_AAAAu32 as c_int,
        0x7FFF_FFFFu32 as c_int,
        0x8000_0000u32 as c_int,
        0xFFFF_FFF9u32 as c_int,
        0x0000_0007,
        715_827_882,  // (INT_MAX + 1) / 3
        -715_827_882,
        1_431_655_765, // 0x55555555
    ];
    check_payload("boundary values", &tile(&seeds));
}

/// Values that sit right at the edges of signed division/modulo behaviour:
/// every residue class mod 7 and mod 2, both signs, plus near-overflow
/// magnitudes for `x * 3 + 7` and `x - (x << 1)`.
#[test]
fn division_and_modulo_edges() {
    let mut seeds: Vec<c_int> = Vec::new();
    for r in -7i32..=7 {
        seeds.push(r);
        seeds.push(i32::MAX - r.unsigned_abs() as i32);
        seeds.push(i32::MIN + r.unsigned_abs() as i32);
    }
    // Multiples and near-multiples of 7 spread across the range.
    for k in 0..32 {
        let base = ((1u32 << k) as i32).wrapping_mul(7);
        seeds.push(base);
        seeds.push(base.wrapping_add(1));
        seeds.push(base.wrapping_sub(1));
        seeds.push(base.wrapping_neg());
    }
    check_payload("division / modulo edges", &tile(&seeds));
}

/// A stride sweep over the whole 32-bit domain: one distinct input per array
/// slot, spaced by a large odd step so the samples wrap around the full range.
#[test]
fn full_range_stride_sweep() {
    const STEP: u32 = 16_381 * 997; // odd, so it generates a long cycle
    let payload: Vec<c_int> = (0..ARRAY_SIZE)
        .map(|i| (i as u32).wrapping_mul(STEP).wrapping_add(0x1234_5678) as c_int)
        .collect();
    check_payload("full-range stride sweep", &payload);
}

#[test]
fn pseudo_random_payloads() {
    for seed in [0u64, 1, 0xDEAD_BEEF, u64::MAX] {
        let mut rng = SplitMix64(seed);
        let payload: Vec<c_int> = (0..ARRAY_SIZE).map(|_| rng.next_i32()).collect();
        check_payload(&format!("random payload seed={seed:#x}"), &payload);
    }
}

/// The C code applies the transform 2000 times in a row inside `long_exec`.
/// Applying it repeatedly here confirms the state carried between calls (the
/// contents of the global array) stays in lockstep, which is what makes the
/// full `long_exec` run reproducible.
#[test]
fn repeated_application_stays_in_lockstep() {
    let guard = load_both();
    let (c, rust) = &*guard;

    let mut rng = SplitMix64(0xA5A5_5A5A);
    let payload: Vec<c_int> = (0..ARRAY_SIZE).map(|_| rng.next_i32()).collect();
    c.write_array(&payload);
    rust.write_array(&payload);

    for round in 1..=12 {
        c.perform_expensive_operations();
        rust.perform_expensive_operations();
        assert_arrays_equal(
            &format!("after {round} applications"),
            &c.read_array(),
            &rust.read_array(),
        );
    }
}

/// Broad coverage of the 32-bit input domain: many batches, each a different
/// strided slice of the range, so roughly 8.4 million distinct inputs are run
/// through all 100 rounds of the transform on both sides.
#[test]
fn wide_domain_sweep() {
    let guard = load_both();
    let (c, rust) = &*guard;

    const BATCHES: u32 = 32;
    // Odd stride, coprime with 2^32, so each batch walks the whole range rather
    // than clustering in one region.
    const STEP: u32 = 0x9E37_79B9;

    for batch in 0..BATCHES {
        let base = batch.wrapping_mul(0x0100_0001);
        let payload: Vec<c_int> = (0..ARRAY_SIZE)
            .map(|i| base.wrapping_add((i as u32).wrapping_mul(STEP)) as c_int)
            .collect();

        c.write_array(&payload);
        rust.write_array(&payload);
        c.perform_expensive_operations();
        rust.perform_expensive_operations();

        assert_arrays_equal(
            &format!("wide domain sweep batch {batch} (base {base:#010x})"),
            &c.read_array(),
            &rust.read_array(),
        );
    }
}

/// Exhaustive coverage of three contiguous windows of the domain: the 262144
/// values at `INT_MIN`, the 262144 values straddling zero, and the 262144
/// values at `INT_MAX` — where the wrapping multiply, the negate and the
/// truncating division are most extreme.
#[test]
fn exhaustive_extreme_windows() {
    let guard = load_both();
    let (c, rust) = &*guard;

    let windows: [(&str, i32); 3] = [
        ("at INT_MIN", i32::MIN),
        ("straddling zero", -(ARRAY_SIZE as i32) / 2),
        ("at INT_MAX", i32::MAX - ARRAY_SIZE as i32 + 1),
    ];

    for (label, start) in windows {
        let payload: Vec<c_int> = (0..ARRAY_SIZE)
            .map(|i| start.wrapping_add(i as i32))
            .collect();
        c.write_array(&payload);
        rust.write_array(&payload);
        c.perform_expensive_operations();
        rust.perform_expensive_operations();
        assert_arrays_equal(
            &format!("exhaustive window: {label}"),
            &c.read_array(),
            &rust.read_array(),
        );
    }
}

/// The XOR reduction at the tail of `long_exec` is only correct if the arrays
/// agree; check the reduction itself over an already-transformed array so a
/// failure here points at the accumulator rather than the transform.
#[test]
fn xor_reduction_matches() {
    let guard = load_both();
    let (c, rust) = &*guard;
    let mut rng = SplitMix64(7);
    let payload: Vec<c_int> = (0..ARRAY_SIZE).map(|_| rng.next_i32()).collect();
    c.write_array(&payload);
    rust.write_array(&payload);
    c.perform_expensive_operations();
    rust.perform_expensive_operations();

    let fold = |v: &[c_int]| v.iter().fold(0 as c_int, |acc, &x| acc ^ x);
    assert_eq!(
        fold(&c.read_array()),
        fold(&rust.read_array()),
        "XOR reduction over the transformed array differs"
    );
}
