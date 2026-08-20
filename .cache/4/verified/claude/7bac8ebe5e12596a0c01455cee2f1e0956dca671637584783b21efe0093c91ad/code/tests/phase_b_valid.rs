//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every call goes through `dlopen`/`dlsym` on BOTH shared objects, so the
//! `#[no_mangle]` export wrappers are exercised exactly as an external C caller
//! would exercise them. For each row we compare the returned `int` per input
//! AND the complete stdout byte stream of the batch.

mod common;

use common::{diff_gotomach_batch, diff_op_batch, Args, Rng};

const INT_MIN: i32 = i32::MIN;
const INT_MAX: i32 = i32::MAX;

/// Model of the values `temp_buffer[i]` takes — used ONLY to pick interesting
/// `threshold` inputs (never to assert anything).
fn produced_seq(iterations: i32, seed: i32, mode: i32) -> Vec<i32> {
    let op = |v: i32| -> i32 {
        match mode {
            1 => v.wrapping_mul(2),
            2 => v.wrapping_mul(3),
            _ => v.wrapping_add(10),
        }
    };
    let mut out = Vec::new();
    let mut cur = seed;
    for _ in 0..iterations.max(0) {
        let p = op(cur);
        out.push(p);
        cur = p % 1000;
    }
    out
}

/// A `mode` value outside {0,1,2} (drives the `switch` `default:` branch).
fn invalid_mode(rng: &mut Rng) -> i32 {
    loop {
        let m = match rng.next_u64() % 3 {
            0 => rng.range_i32(-1000, 1000),
            1 => rng.next_i32(),
            _ => *[-1, 3, 4, 255, -255, INT_MIN, INT_MAX, 65536, -65536]
                .get((rng.next_u64() % 9) as usize)
                .unwrap(),
        };
        if !(0..=2).contains(&m) {
            return m;
        }
    }
}

/// Threshold regimes 0..=5 used by rows 1..24.
fn threshold_for_regime(rng: &mut Rng, regime: u32, mode: i32, iterations: i32, seed: i32) -> i32 {
    match regime {
        0 => INT_MIN,
        1 => rng.range_i32(INT_MIN + 1, -1),
        2 => 0,
        3 => match mode {
            1 => rng.range_i32(1, 2100),
            2 => rng.range_i32(1, 3100),
            _ => rng.range_i32(1, 1100),
        },
        4 => {
            // exactly equal to a value the sequence produces -> the strict `<`
            // must reject that element.
            let seq = produced_seq(iterations, seed, mode);
            if seq.is_empty() {
                0
            } else {
                seq[(rng.next_u64() as usize) % seq.len()]
            }
        }
        _ => INT_MAX,
    }
}

fn rows_1_24_row(row_no: usize, mode_class: u32, regime: u32) {
    let mut rng = Rng::new(0xC0FFEE_00 + row_no as u64);
    let mut inputs = Vec::with_capacity(200);
    for _ in 0..200 {
        let iterations = rng.range_i32(0, 300);
        let seed = rng.range_i32(0, 65535);
        let mode = match mode_class {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => invalid_mode(&mut rng),
        };
        let threshold = threshold_for_regime(&mut rng, regime, mode, iterations, seed);
        inputs.push(Args::new(iterations, seed, mode, threshold));
    }
    diff_gotomach_batch(&format!("CONFIGS row {row_no}"), &inputs);
}

macro_rules! rows_1_24 {
    ($($name:ident => ($no:expr, $mc:expr, $rg:expr)),+ $(,)?) => {
        $(#[test] fn $name() { rows_1_24_row($no, $mc, $rg); })+
    };
}

rows_1_24! {
    row_01_mode0_threshold_int_min      => (1, 0, 0),
    row_02_mode0_threshold_negative     => (2, 0, 1),
    row_03_mode0_threshold_zero         => (3, 0, 2),
    row_04_mode0_threshold_partial      => (4, 0, 3),
    row_05_mode0_threshold_exact        => (5, 0, 4),
    row_06_mode0_threshold_int_max      => (6, 0, 5),
    row_07_mode1_threshold_int_min      => (7, 1, 0),
    row_08_mode1_threshold_negative     => (8, 1, 1),
    row_09_mode1_threshold_zero         => (9, 1, 2),
    row_10_mode1_threshold_partial      => (10, 1, 3),
    row_11_mode1_threshold_exact        => (11, 1, 4),
    row_12_mode1_threshold_int_max      => (12, 1, 5),
    row_13_mode2_threshold_int_min      => (13, 2, 0),
    row_14_mode2_threshold_negative     => (14, 2, 1),
    row_15_mode2_threshold_zero         => (15, 2, 2),
    row_16_mode2_threshold_partial      => (16, 2, 3),
    row_17_mode2_threshold_exact        => (17, 2, 4),
    row_18_mode2_threshold_int_max      => (18, 2, 5),
    row_19_modedef_threshold_int_min    => (19, 3, 0),
    row_20_modedef_threshold_negative   => (20, 3, 1),
    row_21_modedef_threshold_zero       => (21, 3, 2),
    row_22_modedef_threshold_partial    => (22, 3, 3),
    row_23_modedef_threshold_exact      => (23, 3, 4),
    row_24_modedef_threshold_int_max    => (24, 3, 5),
}

// ---------------------------------------------------------------------------
// Rows 25..32 — `iterations` shape boundaries, all four mode classes.
// ---------------------------------------------------------------------------
fn iterations_shape_row(row_no: usize, draws: usize, pick_iterations: impl Fn(&mut Rng) -> i32) {
    let mut rng = Rng::new(0xBEEF_0000 + row_no as u64);
    let mut inputs = Vec::new();
    for i in 0..draws {
        let iterations = pick_iterations(&mut rng);
        let seed = rng.range_i32(0, 65535);
        let mode = match i % 4 {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => invalid_mode(&mut rng),
        };
        for regime in 0..6u32 {
            let threshold = threshold_for_regime(&mut rng, regime, mode, iterations, seed);
            inputs.push(Args::new(iterations, seed, mode, threshold));
        }
    }
    diff_gotomach_batch(&format!("CONFIGS row {row_no}"), &inputs);
}

#[test]
fn row_25_iterations_zero() {
    iterations_shape_row(25, 64, |_| 0);
}
#[test]
fn row_26_iterations_one() {
    iterations_shape_row(26, 64, |_| 1);
}
#[test]
fn row_27_iterations_two() {
    iterations_shape_row(27, 64, |_| 2);
}
#[test]
fn row_28_iterations_three() {
    iterations_shape_row(28, 64, |_| 3);
}
#[test]
fn row_29_iterations_small_random() {
    iterations_shape_row(29, 64, |r| r.range_i32(4, 64));
}
#[test]
fn row_30_iterations_large_random() {
    iterations_shape_row(30, 64, |r| r.range_i32(1000, 4096));
}
#[test]
fn row_31_iterations_65534() {
    iterations_shape_row(31, 4, |_| 65534);
}
#[test]
fn row_32_iterations_65535() {
    iterations_shape_row(32, 4, |_| 65535);
}

// ---------------------------------------------------------------------------
// Rows 33..37 — `seed` shape boundaries.
// ---------------------------------------------------------------------------
fn seed_shape_row(row_no: usize, seeds: &[i32]) {
    let mut rng = Rng::new(0x5EED_0000 + row_no as u64);
    let mut inputs = Vec::new();
    for &seed in seeds {
        for _ in 0..64 {
            let iterations = rng.range_i32(0, 400);
            for mode_class in 0..4u32 {
                let mode = match mode_class {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    _ => invalid_mode(&mut rng),
                };
                for regime in 0..6u32 {
                    let threshold = threshold_for_regime(&mut rng, regime, mode, iterations, seed);
                    inputs.push(Args::new(iterations, seed, mode, threshold));
                }
            }
        }
    }
    diff_gotomach_batch(&format!("CONFIGS row {row_no}"), &inputs);
}

#[test]
fn row_33_seed_zero() {
    seed_shape_row(33, &[0]);
}
#[test]
fn row_34_seed_one() {
    seed_shape_row(34, &[1]);
}
#[test]
fn row_35_seed_mod1000_boundary() {
    seed_shape_row(35, &[999, 1000, 1001]);
}
#[test]
fn row_36_seed_65534() {
    seed_shape_row(36, &[65534]);
}
#[test]
fn row_37_seed_65535() {
    seed_shape_row(37, &[65535]);
}

// ---------------------------------------------------------------------------
// Rows 38..42 — `count` saturation interaction (iterations == UINT16_MAX).
// ---------------------------------------------------------------------------
fn saturation_row(row_no: usize, mode: i32, threshold: i32) {
    let mut inputs = Vec::new();
    for &seed in &[0i32, 1, 7, 499, 500, 999, 1000, 65534, 65535] {
        inputs.push(Args::new(65535, seed, mode, threshold));
    }
    diff_gotomach_batch(&format!("CONFIGS row {row_no}"), &inputs);
}

#[test]
fn row_38_saturation_mode0() {
    saturation_row(38, 0, INT_MAX);
}
#[test]
fn row_39_saturation_mode1() {
    saturation_row(39, 1, INT_MAX);
}
#[test]
fn row_40_saturation_mode2() {
    saturation_row(40, 2, INT_MAX);
}
#[test]
fn row_41_saturation_mode_default() {
    saturation_row(41, 7, INT_MAX);
}
#[test]
fn row_42_no_saturation_partial_threshold() {
    // count stays below UINT16_MAX -> no "[WARNING] Reached maximum count".
    let mut inputs = Vec::new();
    for mode in [0i32, 1, 2, -5] {
        for threshold in [1i32, 11, 500, 512, 1000, 1010, 1500, 2000, 2998, 3000] {
            for seed in [0i32, 3, 501, 999, 65535] {
                inputs.push(Args::new(65535, seed, mode, threshold));
            }
        }
    }
    diff_gotomach_batch("CONFIGS row 42", &inputs);
}

// ---------------------------------------------------------------------------
// Rows 43..48 — the low-level exported `operation_fn`s, called directly.
// ---------------------------------------------------------------------------
fn random_op_inputs(seed: u64, n: usize) -> Vec<(i32, i32, usize)> {
    let mut rng = Rng::new(seed);
    let mut ctx_storage = 0u64;
    let ctx_addr = &mut ctx_storage as *mut u64 as usize;
    (0..n)
        .map(|i| {
            (
                rng.next_i32(),
                rng.next_i32(),
                if i % 2 == 0 { 0 } else { ctx_addr },
            )
        })
        .collect()
}

fn boundary_op_inputs(values: &[i32]) -> Vec<(i32, i32, usize)> {
    let mut rng = Rng::new(0xA11CE);
    let mut ctx_storage = 0u64;
    let ctx_addr = &mut ctx_storage as *mut u64 as usize;
    let mut out = Vec::new();
    for &v in values {
        for p in [0i32, 1, -1, INT_MIN, INT_MAX, rng.next_i32()] {
            out.push((v, p, 0));
            out.push((v, p, ctx_addr));
        }
    }
    out
}

#[test]
fn row_43_process_value_random() {
    diff_op_batch(
        "CONFIGS row 43",
        "process_value",
        &random_op_inputs(0x4343, 5000),
    );
}

#[test]
fn row_44_process_value_boundaries() {
    diff_op_batch(
        "CONFIGS row 44",
        "process_value",
        &boundary_op_inputs(&[
            INT_MIN,
            INT_MIN + 1,
            INT_MIN + 9,
            INT_MIN + 10,
            -11,
            -10,
            -9,
            -1,
            0,
            1,
            9,
            10,
            INT_MAX - 10,
            INT_MAX - 9,
            INT_MAX - 1,
            INT_MAX,
        ]),
    );
}

#[test]
fn row_45_double_value_random() {
    diff_op_batch(
        "CONFIGS row 45",
        "double_value",
        &random_op_inputs(0x4545, 5000),
    );
}

#[test]
fn row_46_double_value_boundaries() {
    diff_op_batch(
        "CONFIGS row 46",
        "double_value",
        &boundary_op_inputs(&[
            INT_MIN,
            INT_MIN + 1,
            INT_MIN / 2,
            INT_MIN / 2 - 1,
            -2,
            -1,
            0,
            1,
            2,
            INT_MAX / 2,
            INT_MAX / 2 + 1,
            INT_MAX - 1,
            INT_MAX,
        ]),
    );
}

#[test]
fn row_47_triple_value_random() {
    diff_op_batch(
        "CONFIGS row 47",
        "triple_value",
        &random_op_inputs(0x4747, 5000),
    );
}

#[test]
fn row_48_triple_value_boundaries() {
    diff_op_batch(
        "CONFIGS row 48",
        "triple_value",
        &boundary_op_inputs(&[
            INT_MIN,
            INT_MIN + 1,
            INT_MIN / 3,
            INT_MIN / 3 - 1,
            -3,
            -1,
            0,
            1,
            3,
            INT_MAX / 3,
            INT_MAX / 3 + 1,
            INT_MAX - 1,
            INT_MAX,
        ]),
    );
}

/// Row 49 — drive the ops the way `gotomach` composes them (feedback through
/// `% 1000`), comparing every single step of the pipeline in both `.so`s.
#[test]
fn row_49_op_pipeline_composition() {
    let c = common::c_impl();
    let r = common::rust_impl();
    let mut rng = Rng::new(0x4949);
    for trial in 0..64 {
        let start = if trial < 8 {
            [0i32, 1, 2, 499, 500, 999, 65534, 65535][trial]
        } else {
            rng.range_i32(0, 65535)
        };
        for which in 0..3usize {
            let (cf, rf) = match which {
                0 => (c.process_value, r.process_value),
                1 => (c.double_value, r.double_value),
                _ => (c.triple_value, r.triple_value),
            };
            let mut c_cur = start;
            let mut r_cur = start;
            for step in 0..4096 {
                let cv = unsafe { cf(c_cur, 0, std::ptr::null_mut()) };
                let rv = unsafe { rf(r_cur, 0, std::ptr::null_mut()) };
                assert_eq!(
                    cv, rv,
                    "[CONFIGS row 49] op#{which} diverged at step {step} (start={start}): C={cv} Rust={rv}"
                );
                c_cur = cv % 1000;
                r_cur = rv % 1000;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 50..51 — saturating / massive random sweeps.
// ---------------------------------------------------------------------------
#[test]
fn row_50_exhaustive_small_domain() {
    let mut inputs = Vec::new();
    for mode in -4i32..=6 {
        for iterations in 0i32..=40 {
            for &seed in &[0i32, 1, 7, 999, 1000, 65535] {
                for &threshold in &[INT_MIN, -1, 0, 1, 15, 1000, 1010, 2000, 3000, INT_MAX] {
                    inputs.push(Args::new(iterations, seed, mode, threshold));
                }
            }
        }
    }
    assert_eq!(inputs.len(), 11 * 41 * 6 * 10);
    diff_gotomach_batch("CONFIGS row 50", &inputs);
}

#[test]
fn row_51_massive_random_whole_domain() {
    let ladder = [
        INT_MIN,
        INT_MIN + 1,
        -65536,
        -65535,
        -2,
        -1,
        0,
        1,
        2,
        999,
        1000,
        1001,
        65534,
        65535,
        65536,
        65537,
        INT_MAX - 1,
        INT_MAX,
    ];
    let mut rng = Rng::new(0x5151);
    let mut inputs = Vec::with_capacity(20_000);
    for _ in 0..20_000 {
        let (iterations, seed, mode, threshold) = match rng.next_u64() % 4 {
            // fully random over the whole i32 domain (mostly rejected early)
            0 => (
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            ),
            // straddles the validity edges with real work in between
            1 => (
                rng.range_i32(-2, 2050),
                rng.range_i32(-2, 65537),
                rng.range_i32(-3, 5),
                rng.next_i32(),
            ),
            // pure boundary ladder cross product
            2 => (
                ladder[(rng.next_u64() as usize) % ladder.len()],
                ladder[(rng.next_u64() as usize) % ladder.len()],
                ladder[(rng.next_u64() as usize) % ladder.len()],
                ladder[(rng.next_u64() as usize) % ladder.len()],
            ),
            // short valid runs with wild mode/threshold
            _ => (
                rng.range_i32(0, 64),
                rng.range_i32(0, 65535),
                rng.next_i32(),
                if rng.next_u64() % 2 == 0 {
                    rng.next_i32()
                } else {
                    rng.range_i32(-5, 3200)
                },
            ),
        };
        inputs.push(Args::new(iterations, seed, mode, threshold));
    }
    diff_gotomach_batch("CONFIGS row 51", &inputs);
}
