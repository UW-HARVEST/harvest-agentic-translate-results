//! Differential tests: C .so vs Rust .so, both loaded via `libloading`.
//!
//! Both sides are exercised only through their exported `next_double` symbol,
//! exactly as an external caller would, so the `#[no_mangle]` wrapper is under
//! test too.

mod common;

use common::{CnRnd, Libs};

/// Run `next_double` `n` times against both libraries, comparing the returned
/// double bit-for-bit *and* the mutated state after every single call.
fn compare_sequence(libs: &Libs, seed: [u64; 2], n: usize) {
    let mut c_state = CnRnd { state: seed };
    let mut r_state = CnRnd { state: seed };

    for i in 0..n {
        let c = unsafe { (libs.c_next_double)(&mut c_state) };
        let r = unsafe { (libs.rust_next_double)(&mut r_state) };

        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "value mismatch at step {i} for seed {seed:?}: C={c:?} Rust={r:?}"
        );
        assert_eq!(
            c_state, r_state,
            "state mismatch at step {i} for seed {seed:?}"
        );
    }
}

#[test]
fn both_libs_export_next_double() {
    // Loading itself asserts the symbol exists in both .so files.
    let _libs = Libs::load();
}

#[test]
fn zero_state() {
    let libs = Libs::load();
    // Degenerate seed: the C generator is stuck at 0 here. Replicate exactly.
    compare_sequence(&libs, [0, 0], 64);
}

#[test]
fn simple_seeds() {
    let libs = Libs::load();
    for seed in [
        [1, 0],
        [0, 1],
        [1, 1],
        [2, 3],
        [u64::MAX, 0],
        [0, u64::MAX],
        [u64::MAX, u64::MAX],
        [0x0000_0000_0000_0001, 0x8000_0000_0000_0000],
        [0x8000_0000_0000_0000, 0x0000_0000_0000_0001],
        [0xdead_beef_dead_beef, 0xcafe_babe_cafe_babe],
        [0x5555_5555_5555_5555, 0xaaaa_aaaa_aaaa_aaaa],
        [0x0123_4567_89ab_cdef, 0xfedc_ba98_7654_3210],
    ] {
        compare_sequence(&libs, seed, 128);
    }
}

/// Single-bit seeds: exercises every shift/xor lane of the xorshift step.
#[test]
fn single_bit_seeds() {
    let libs = Libs::load();
    for bit in 0..64 {
        compare_sequence(&libs, [1u64 << bit, 0], 8);
        compare_sequence(&libs, [0, 1u64 << bit], 8);
        compare_sequence(&libs, [1u64 << bit, 1u64 << bit], 8);
    }
}

/// Seeds that make `x + y` wrap around 2^64 (C `uint64_t` is modular).
#[test]
fn addition_wraparound() {
    let libs = Libs::load();
    for seed in [
        [u64::MAX, 1],
        [1, u64::MAX],
        [u64::MAX - 1, 2],
        [0x8000_0000_0000_0000, 0x8000_0000_0000_0000],
        [0xffff_ffff_ffff_fffe, 0xffff_ffff_ffff_ffff],
    ] {
        compare_sequence(&libs, seed, 256);
    }
}

/// Long run from one seed: catches divergence that only shows up after the
/// state has been stirred many times.
#[test]
fn long_run() {
    let libs = Libs::load();
    compare_sequence(
        &libs,
        [0x243f_6a88_85a3_08d3, 0x1319_8a2e_0370_7344],
        100_000,
    );
}

/// Many pseudo-random seeds, driven by an independent splitmix64 so the seed
/// distribution is not correlated with the generator under test.
#[test]
fn randomized_seeds() {
    let libs = Libs::load();
    let mut s: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut splitmix = || {
        s = s.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };
    for _ in 0..2000 {
        let seed = [splitmix(), splitmix()];
        compare_sequence(&libs, seed, 32);
    }
}

/// Result must land in `[0.0, 1.0)`; verify C and Rust agree on that too.
#[test]
fn range_property_matches() {
    let libs = Libs::load();
    let mut c_state = CnRnd {
        state: [0xa5a5_a5a5_a5a5_a5a5, 0x5a5a_5a5a_5a5a_5a5a],
    };
    let mut r_state = c_state;
    for i in 0..50_000 {
        let c = unsafe { (libs.c_next_double)(&mut c_state) };
        let r = unsafe { (libs.rust_next_double)(&mut r_state) };
        assert_eq!(c.to_bits(), r.to_bits(), "mismatch at {i}");
        assert!((0.0..1.0).contains(&c), "C out of range at {i}: {c}");
        assert_eq!(c_state, r_state);
    }
}

/// Interleave calls on independent states to make sure neither implementation
/// keeps hidden global state.
#[test]
fn no_hidden_global_state() {
    let libs = Libs::load();
    let mut a_c = CnRnd { state: [7, 11] };
    let mut a_r = a_c;
    let mut b_c = CnRnd { state: [13, 17] };
    let mut b_r = b_c;

    for i in 0..1000 {
        let c = unsafe { (libs.c_next_double)(&mut a_c) };
        let r = unsafe { (libs.rust_next_double)(&mut a_r) };
        assert_eq!(c.to_bits(), r.to_bits(), "A mismatch at {i}");

        let c = unsafe { (libs.c_next_double)(&mut b_c) };
        let r = unsafe { (libs.rust_next_double)(&mut b_r) };
        assert_eq!(c.to_bits(), r.to_bits(), "B mismatch at {i}");

        assert_eq!(a_c, a_r);
        assert_eq!(b_c, b_r);
    }
}
