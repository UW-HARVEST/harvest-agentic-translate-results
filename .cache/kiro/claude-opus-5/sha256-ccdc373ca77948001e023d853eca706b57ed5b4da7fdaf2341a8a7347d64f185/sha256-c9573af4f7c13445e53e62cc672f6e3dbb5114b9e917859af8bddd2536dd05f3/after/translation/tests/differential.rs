//! Phase B — valid-path differential tests, rows 1..=20 of `CONFIGS.md`.
//!
//! These drive the **lowest-level** entry points directly: write the exported
//! `array` object, call the exported `perform_expensive_operations()` `k` times,
//! read the exported `array` back, and require the two 1 MiB images to be
//! byte-identical.  Every row feeds 262144 values at once, so each row is a
//! property-style test over many inputs with a fixed seed (`harness::Rng`,
//! splitmix32, identical to `tools/runner.c`).
//!
//! `long_exec` (rows 21..=34) lives in `long_exec_diff.rs` because a single C
//! `long_exec` is ~470 s of CPU.

mod harness;

use harness::{diff_pxo, rand_fill, rand_fill_nonneg, ARRAY_SIZE};

fn constant(v: i32) -> Vec<i32> {
    vec![v; ARRAY_SIZE]
}

#[test]
fn row01_bss_zeros_k1() {
    diff_pxo("row 1: all-zero array, k=1", &constant(0), 1);
}

#[test]
fn row02_zeros_k0_is_noop() {
    // k = 0 must leave the array untouched in both libraries.
    diff_pxo("row 2: all-zero array, k=0", &constant(0), 0);
    let input = rand_fill(0xC0FFEE);
    let _g = harness::lock();
    let (cl, rl) = (harness::c(), harness::rust());
    cl.array_mut().copy_from_slice(&input);
    rl.array_mut().copy_from_slice(&input);
    cl.pxo(0);
    rl.pxo(0);
    assert_eq!(cl.array(), &input[..], "row 2: C mutated the array with k=0");
    assert_eq!(rl.array(), &input[..], "row 2: Rust mutated the array with k=0");
}

#[test]
fn row03_all_ones_k1() {
    diff_pxo("row 3: all elements = 1, k=1", &constant(1), 1);
}

#[test]
fn row04_all_minus_ones_k1() {
    diff_pxo("row 4: all elements = -1, k=1", &constant(-1), 1);
}

#[test]
fn row05_all_int_max_k1() {
    diff_pxo("row 5: all elements = INT_MAX, k=1", &constant(i32::MAX), 1);
}

#[test]
fn row06_all_int_min_k1() {
    diff_pxo("row 6: all elements = INT_MIN, k=1", &constant(i32::MIN), 1);
}

/// The sentinel sweep from `CONFIGS.md` row 7 / `ERRORS.md` rows 6..=13.
pub const SENTINELS: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    7,
    -7,
    8,
    -8,
    6,
    -6,
    14,
    -14,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    1_073_741_824,
    -1_073_741_824,
    1_073_741_823,
    -1_073_741_825,
    2_147_483_646,
    -2_147_483_647,
    32767,
    -32768,
    65535,
    -65536,
    0x5555_5555,
    0x5555_5555u32 as i32,
    0xAAAA_AAAAu32 as i32,
    0x7FFF_FFFE,
    -715_827_883, // INT_MIN/3-ish: x*3 straddles the overflow boundary
    715_827_883,
    715_827_882,
    -715_827_882,
];

#[test]
fn row07_sentinel_sweep_k1() {
    let mut input = Vec::with_capacity(ARRAY_SIZE);
    for i in 0..ARRAY_SIZE {
        input.push(SENTINELS[i % SENTINELS.len()]);
    }
    diff_pxo("row 7: sentinel sweep, k=1", &input, 1);
}

#[test]
fn row07b_sentinel_sweep_k3() {
    let mut input = Vec::with_capacity(ARRAY_SIZE);
    for i in 0..ARRAY_SIZE {
        input.push(SENTINELS[i % SENTINELS.len()]);
    }
    diff_pxo("row 7b: sentinel sweep, k=3", &input, 3);
}

#[test]
fn row08_contiguous_window_k1() {
    // array[i] = i - 131072 : a contiguous -131072 ..= 131071 run, i.e. every
    // small magnitude of both signs, exhaustively.
    let input: Vec<i32> = (0..ARRAY_SIZE)
        .map(|i| (i as i64 - 131072) as i32)
        .collect();
    diff_pxo("row 8: contiguous window around 0, k=1", &input, 1);
}

#[test]
fn row08b_contiguous_window_at_int_min_k1() {
    let input: Vec<i32> = (0..ARRAY_SIZE)
        .map(|i| (i32::MIN as i64 + i as i64) as i32)
        .collect();
    diff_pxo("row 8b: contiguous window at INT_MIN, k=1", &input, 1);
}

#[test]
fn row08c_contiguous_window_at_int_max_k1() {
    let input: Vec<i32> = (0..ARRAY_SIZE)
        .map(|i| (i32::MAX as i64 - i as i64) as i32)
        .collect();
    diff_pxo("row 8c: contiguous window at INT_MAX, k=1", &input, 1);
}

#[test]
fn row09_random_k1() {
    diff_pxo("row 9: random full-range, k=1", &rand_fill(1), 1);
}

#[test]
fn row10_random_k2() {
    diff_pxo("row 10: random full-range, k=2", &rand_fill(2), 2);
}

#[test]
fn row11_random_k3() {
    diff_pxo("row 11: random full-range, k=3", &rand_fill(3), 3);
}

#[test]
fn row12_random_k5() {
    diff_pxo("row 12: random full-range, k=5", &rand_fill(4), 5);
}

#[test]
fn row13_random_nonneg_k7() {
    diff_pxo(
        "row 13: random non-negative (rand() shape), k=7",
        &rand_fill_nonneg(5),
        7,
    );
}

#[test]
fn row14_random_k20() {
    diff_pxo("row 14: random full-range, k=20", &rand_fill(6), 20);
}

#[test]
fn row15_random_k81_below_fast_path_boundary() {
    // n = 8100 < fast::LEARN_MIN_N (8192) -> naive strategy in the Rust crate.
    diff_pxo("row 15: random full-range, k=81 (n=8100)", &rand_fill(7), 81);
}

#[test]
fn row16_random_k82_at_fast_path_boundary() {
    // n = 8200 >= fast::LEARN_MIN_N -> cycle/memo strategy in the Rust crate.
    diff_pxo("row 16: random full-range, k=82 (n=8200)", &rand_fill(8), 82);
}

#[test]
fn row17_random_k83_above_fast_path_boundary() {
    diff_pxo("row 17: random full-range, k=83 (n=8300)", &rand_fill(9), 83);
}

#[test]
fn row18_sparse_k3() {
    let mut input = constant(0);
    input[0] = i32::MIN;
    input[1] = i32::MAX;
    input[ARRAY_SIZE / 2] = -1;
    input[ARRAY_SIZE - 1] = 123_456_789;
    diff_pxo("row 18: sparse non-zero elements, k=3", &input, 3);
}

#[test]
fn row19_values_already_on_cycles_k1() {
    // Take the post-`long_exec` image (f^200000 of a rand() fill): those values
    // sit on / very near the cycles of f, a shape random ints never reach.
    let input = {
        let _g = harness::lock();
        let rl = harness::rust();
        rl.long_exec_capture(4242);
        rl.array().to_vec()
    };
    diff_pxo("row 19: values already on cycles of f, k=1", &input, 1);
    diff_pxo("row 19b: values already on cycles of f, k=7", &input, 7);
}

#[test]
fn row20_state_carries_through_the_global() {
    // Two separate calls with a read-back in between must agree at both points,
    // proving the exported global really is the shared state.
    let input = rand_fill(20);
    let _g = harness::lock();
    let (cl, rl) = (harness::c(), harness::rust());
    cl.array_mut().copy_from_slice(&input);
    rl.array_mut().copy_from_slice(&input);

    cl.pxo(1);
    rl.pxo(1);
    let after1_c = cl.array().to_vec();
    let after1_r = rl.array().to_vec();
    harness::assert_arrays_eq("row 20 (after 1st call)", 1, &input, &after1_c, &after1_r);

    cl.pxo(1);
    rl.pxo(1);
    harness::assert_arrays_eq(
        "row 20 (after 2nd call)",
        2,
        &after1_c,
        cl.array(),
        rl.array(),
    );

    // ... and equal to a single k=2 run from the same input.
    cl.array_mut().copy_from_slice(&input);
    cl.pxo(2);
    harness::assert_arrays_eq("row 20 (1+1 == 2)", 2, &input, cl.array(), rl.array());
}

/// Extra property-style matrix: many random seeds crossed with many `k`,
/// including both sides of the Rust naive/fast strategy boundary.
///
/// Kept deliberately modest in total `k`, because a single C
/// `perform_expensive_operations` is ~0.24 s and `tools/sweep.sh` already
/// verifies `k=1` exhaustively over all 2^32 inputs.
#[test]
fn randomized_matrix() {
    for seed in [11u64, 12] {
        for k in [1usize, 2, 4, 9, 17] {
            diff_pxo(
                &format!("matrix: rand_fill({seed}), k={k}"),
                &rand_fill(seed),
                k,
            );
        }
    }
    for k in [81usize, 82] {
        diff_pxo(&format!("matrix: rand_fill(21), k={k}"), &rand_fill(21), k);
    }
}
