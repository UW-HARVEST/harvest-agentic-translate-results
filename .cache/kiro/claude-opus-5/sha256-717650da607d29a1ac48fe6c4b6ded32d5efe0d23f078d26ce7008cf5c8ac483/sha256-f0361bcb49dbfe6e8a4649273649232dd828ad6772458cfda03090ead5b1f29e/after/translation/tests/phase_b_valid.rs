//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through `dlopen`+`dlsym` on BOTH the C `.so` and the Rust
//! `.so`; the Rust implementation is never invoked directly.

mod harness;

use harness::{bits_from, Pair, Rng, MANTISSA_EDGES, RUNS};

/// Seed used by every randomized row (reproducible).
const SEED: u64 = 0x2545_F491_4F6C_DD1D;

/// Exercise one `CONFIGS.md` row: the run's endpoint `j`s, every mantissa edge,
/// and `n` randomized `(j, mantissa)` pairs drawn from inside the run.
fn check_run(p: &Pair, idx: usize, randoms: u32) {
    let (lo, hi, shift, label) = RUNS[idx];
    let what = format!("CONFIGS row for j={lo}..{hi} shift=0x{shift:02x} [{label}]");

    // Endpoints and (for wide runs) the interior neighbours of the endpoints.
    let mut js = vec![lo, hi];
    if hi > lo {
        js.push(lo + 1);
        js.push(hi - 1);
        js.push(lo + (hi - lo) / 2);
    }
    for &j in &js {
        for &m in MANTISSA_EDGES.iter() {
            p.assert_bits(bits_from(j, m), &what);
        }
    }

    // Every j in the run, with the mantissa edges (runs are <= 112 wide).
    for j in lo..=hi {
        for &m in MANTISSA_EDGES.iter() {
            p.assert_bits(bits_from(j, m), &what);
        }
    }

    // Randomized interior: random j in the run x random 23-bit mantissa.
    let mut rng = Rng::new(SEED ^ (idx as u64));
    let span = hi - lo + 1;
    for _ in 0..randoms {
        let j = lo + rng.below(span);
        let m = rng.next_u32() & 0x007f_ffff;
        p.assert_bits(bits_from(j, m), &what);
    }
}

macro_rules! row_test {
    ($name:ident, $idx:expr) => {
        #[test]
        fn $name() {
            let p = Pair::load();
            check_run(&p, $idx, 20_000);
        }
    };
}

// ---- CONFIGS.md rows 1..28 : one test per run the C tables distinguish ----
row_test!(row01_pos_flush_to_zero, 0);
row_test!(row02_pos_subnormal_step1, 1);
row_test!(row03_pos_subnormal_step2, 2);
row_test!(row04_pos_subnormal_step3, 3);
row_test!(row05_pos_subnormal_step4, 4);
row_test!(row06_pos_subnormal_step5, 5);
row_test!(row07_pos_subnormal_step6, 6);
row_test!(row08_pos_subnormal_step7, 7);
row_test!(row09_pos_subnormal_step8, 8);
row_test!(row10_pos_subnormal_step9, 9);
row_test!(row11_pos_subnormal_step10, 10);
row_test!(row12_pos_normal_halves, 11);
row_test!(row13_pos_overflow_saturate, 12);
row_test!(row14_pos_inf_and_nan, 13);
row_test!(row15_neg_flush_to_zero, 14);
row_test!(row16_neg_subnormal_step1, 15);
row_test!(row17_neg_subnormal_step2, 16);
row_test!(row18_neg_subnormal_step3, 17);
row_test!(row19_neg_subnormal_step4, 18);
row_test!(row20_neg_subnormal_step5, 19);
row_test!(row21_neg_subnormal_step6, 20);
row_test!(row22_neg_subnormal_step7, 21);
row_test!(row23_neg_subnormal_step8, 22);
row_test!(row24_neg_subnormal_step9, 23);
row_test!(row25_neg_subnormal_step10, 24);
row_test!(row26_neg_normal_halves, 25);
row_test!(row27_neg_overflow_saturate, 26);
row_test!(row28_neg_inf_and_nan, 27);

/// CONFIGS.md row 29 — every `j` in 0..512 x every mantissa boundary shape.
#[test]
fn row29_all_j_with_mantissa_edges() {
    let p = Pair::load();
    for j in 0u32..512 {
        for &m in MANTISSA_EDGES.iter() {
            p.assert_bits(bits_from(j, m), "CONFIGS row 29 (all j x mantissa edges)");
        }
    }
}

/// CONFIGS.md row 30 — every run transition `(j-1, j)` with mantissa edges.
/// Catches a lookup table transcribed off by one element.
#[test]
fn row30_run_boundaries() {
    let p = Pair::load();
    for &(lo, hi, _, label) in RUNS.iter() {
        let what = format!("CONFIGS row 30 (boundary of [{label}])");
        for j in [lo.saturating_sub(1), lo, hi, (hi + 1).min(511)] {
            for &m in MANTISSA_EDGES.iter() {
                p.assert_bits(bits_from(j, m), &what);
            }
        }
    }
}

/// CONFIGS.md row 31 — uniformly random 32-bit argument patterns.
#[test]
fn row31_random_uniform_bit_patterns() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED);
    for _ in 0..4_000_000u32 {
        p.assert_bits(rng.next_u32(), "CONFIGS row 31 (uniform random bits)");
    }
}

/// CONFIGS.md row 32 — random sign x exponent drawn from run endpoints x
/// random mantissa, concentrating on the value classes the C distinguishes.
#[test]
fn row32_random_interesting_shapes() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xA5A5_A5A5_A5A5_A5A5);

    // Pool of "interesting" j values: every run endpoint and its neighbours.
    let mut pool: Vec<u32> = Vec::new();
    for &(lo, hi, _, _) in RUNS.iter() {
        for j in [lo.saturating_sub(1), lo, lo + 1, hi.saturating_sub(1), hi, (hi + 1).min(511)] {
            if j < 512 {
                pool.push(j);
            }
        }
    }
    pool.sort_unstable();
    pool.dedup();

    let n = pool.len() as u32;
    for _ in 0..1_000_000u32 {
        let j = pool[rng.below(n) as usize];
        // Mix of tiny, huge, and fully random mantissas.
        let m = match rng.below(4) {
            0 => rng.below(8),
            1 => 0x007f_ffff - rng.below(8),
            2 => 1u32 << rng.below(23),
            _ => rng.next_u32() & 0x007f_ffff,
        };
        p.assert_bits(bits_from(j, m), "CONFIGS row 32 (random interesting shapes)");
    }
}

/// CONFIGS.md row 33 — ordinary `f32` values a real consumer would pass:
/// ratios, powers, and integer-valued floats, rather than raw bit patterns.
#[test]
fn row33_ordinary_consumer_values() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x1234_5678_9ABC_DEF0);

    for i in 0..50_000u32 {
        let num = (rng.next_u32() % 100_000) as f32;
        let den = ((rng.next_u32() % 99_999) + 1) as f32;
        p.assert_val(num / den, "CONFIGS row 33 (ratio)");
        p.assert_val(-num / den, "CONFIGS row 33 (negative ratio)");

        let e = (rng.below(90) as i32) - 45;
        p.assert_val(2.0f32.powi(e), "CONFIGS row 33 (power of two)");
        p.assert_val(10.0f32.powi(e / 3), "CONFIGS row 33 (power of ten)");

        let _ = i;
    }

    // Named constants and the classic float32 landmarks.
    for &v in &[
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        65504.0,          // largest finite half
        65519.0,          // just under the half overflow threshold
        65520.0,          // first value that saturates
        6.103515625e-5,   // smallest normal half
        5.960464477539063e-8, // smallest subnormal half
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::EPSILON,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        core::f32::consts::PI,
        core::f32::consts::E,
    ] {
        p.assert_val(v, "CONFIGS row 33 (landmark constant)");
    }
}

/// Sanity: both libraries were actually loaded from distinct files, so the
/// comparison is not accidentally a library against itself.
#[test]
fn harness_loads_two_distinct_shared_objects() {
    let p = Pair::load();
    assert_ne!(p.c_path, p.rust_path);
    assert!(p.c_path.is_file(), "{}", p.c_path.display());
    assert!(p.rust_path.is_file(), "{}", p.rust_path.display());
    assert!(p.c_path.starts_with(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src")
    ));
    // And the two function pointers are genuinely different code.
    assert_ne!(p.c as usize, p.rust as usize);
}
