//! Phase B — valid-path differential tests, one per row of `CONFIGS.md`.
//!
//! Every test calls the exported `driver` symbol of the C `.so` and of every
//! Rust `.so` present, captures stdout, and compares byte-for-byte.

mod common;

use common::{assert_same, assert_same_all, impls, model_output, Rng};

// ---------------------------------------------------------------- C1 .. C11
// Hand-picked shapes: decimal-width crossings of `i` and of `j` (which happen
// at different values of `i`, since `j == 2*i`) and stdio buffer boundaries.

#[test]
fn c1_single_iteration() {
    assert_same(1);
    // Independent check of the expected bytes for the smallest case.
    assert_eq!(impls().c.run_out(1), b"0 0\n".to_vec());
}

#[test]
fn c2_two_iterations() {
    assert_same(2);
    assert_eq!(impls().c.run_out(2), b"0 0\n1 2\n".to_vec());
}

#[test]
fn c3_all_single_digit() {
    assert_same_all([3, 4]);
}

#[test]
fn c4_j_crosses_one_to_two_digits() {
    // j == 10 first appears at i == 5, while i is still one digit.
    assert_same_all([5, 6, 7]);
}

#[test]
fn c5_i_crosses_one_to_two_digits() {
    assert_same_all([9, 10, 11]);
}

#[test]
fn c6_j_crosses_two_to_three_digits() {
    assert_same_all([49, 50, 51]);
}

#[test]
fn c7_i_crosses_two_to_three_digits() {
    assert_same_all([99, 100, 101]);
}

#[test]
fn c8_j_crosses_three_to_four_digits() {
    assert_same_all([499, 500, 501]);
}

#[test]
fn c9_i_crosses_three_to_four_digits_and_stdio_buffer() {
    // ~7 KB of output: the first case that forces mid-loop write(2) flushes.
    assert_same_all([999, 1000, 1001]);
}

#[test]
fn c10_j_crosses_four_to_five_digits() {
    assert_same_all([4999, 5000, 5001]);
}

#[test]
fn c11_i_crosses_four_to_five_digits() {
    assert_same_all([9999, 10000, 10001]);
}

// --------------------------------------------------------------- C12 .. C15
// Randomised, seeded draws per row rather than one hand-picked value.

#[test]
fn c12_random_single_digit() {
    let mut rng = Rng::new(0xC012_5EED);
    for _ in 0..200 {
        assert_same(rng.range_i32(1, 9));
    }
}

#[test]
fn c13_random_up_to_100() {
    let mut rng = Rng::new(0xC013_5EED);
    for _ in 0..200 {
        assert_same(rng.range_i32(1, 100));
    }
}

#[test]
fn c14_random_up_to_2000() {
    let mut rng = Rng::new(0xC014_5EED);
    for _ in 0..120 {
        assert_same(rng.range_i32(1, 2000));
    }
}

#[test]
fn c15_random_multi_flush() {
    let mut rng = Rng::new(0xC015_5EED);
    for _ in 0..25 {
        assert_same(rng.range_i32(2000, 20000));
    }
}

// --------------------------------------------------------------- C16 .. C17
// Large single calls.

#[test]
fn c16_large_fixed() {
    assert_same_all([65536, 100000]);
}

#[test]
fn c17_random_large() {
    let mut rng = Rng::new(0xC017_5EED);
    for _ in 0..3 {
        assert_same(rng.range_i32(100000, 200000));
    }
}

// --------------------------------------------------------------- C18 .. C21
// Call multiplicity, interleaving, threading, mixed validity.

#[test]
fn c18_repeated_identical_calls_have_no_residual_state() {
    let impls = impls();
    let first = impls.c.run(37);
    for _ in 0..10 {
        assert_eq!(impls.c.run(37), first, "C output changed across repeats");
        for r in &impls.rust {
            assert_eq!(r.run(37), first, "{} diverged on a repeated call", r.name);
        }
    }
}

#[test]
fn c19_interleaved_c_and_rust_calls_with_varying_x() {
    let impls = impls();
    let mut rng = Rng::new(0xC019_5EED);
    for _ in 0..150 {
        let x = rng.range_i32(1, 1500);
        // Alternate C and Rust so any cross-library state leakage would show up.
        let c_out = impls.c.run(x);
        for r in &impls.rust {
            assert_eq!(r.run(x), c_out, "{} diverged for driver({x}) when interleaved", r.name);
        }
        let c_again = impls.c.run(x);
        assert_eq!(c_again, c_out, "C output for driver({x}) changed after Rust ran");
    }
}

#[test]
fn c20_call_from_non_main_thread() {
    // Each measurement is isolated in its own forked child, so this is safe;
    // the point is that neither library depends on thread-affine state.
    let handle = std::thread::spawn(|| {
        assert_same_all([1, 7, 64, 1234]);
    });
    handle.join().expect("worker thread panicked");
}

#[test]
fn c21_mixed_valid_and_invalid_sequence() {
    let mut rng = Rng::new(0xC021_5EED);
    for _ in 0..150 {
        // Deliberately mix the accepting and rejecting halves of `i < x`.
        let x = if rng.next_u64() % 2 == 0 {
            rng.range_i32(-1000, 0)
        } else {
            rng.range_i32(1, 1000)
        };
        assert_same(x);
    }
}

// --------------------------------------------------------------- C22 .. C23

#[test]
fn c22_powers_of_two_and_neighbours() {
    let mut xs = Vec::new();
    for k in 1..=14u32 {
        let p = 1i32 << k;
        xs.extend_from_slice(&[p - 1, p, p + 1]);
    }
    assert_same_all(xs);
}

#[test]
fn c23_exhaustive_small_domain() {
    // Not sampled: every value in 1..=300.
    assert_same_all(1..=300);
}

// ------------------------------------------------------------------ oracle
// Guard against "both implementations are broken identically": compare the C
// output against an independent model of what driver.c specifies.

#[test]
fn oracle_c_matches_independent_model() {
    let impls = impls();
    let mut rng = Rng::new(0x0AC1E_5EED);
    let mut xs: Vec<i32> = (0..=40).collect();
    for _ in 0..40 {
        xs.push(rng.range_i32(-500, 5000));
    }
    for x in xs {
        assert_eq!(impls.c.run_out(x), model_output(x), "C output for driver({x}) is not what driver.c specifies");
        for r in &impls.rust {
            assert_eq!(r.run_out(x), model_output(x), "{} output for driver({x}) is not what driver.c specifies", r.name);
        }
    }
}
