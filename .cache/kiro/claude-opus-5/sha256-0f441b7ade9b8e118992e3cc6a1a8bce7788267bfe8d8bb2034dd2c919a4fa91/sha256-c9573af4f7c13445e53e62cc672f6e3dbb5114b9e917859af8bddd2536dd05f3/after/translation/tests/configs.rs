//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both shared objects are driven through their exported symbols only.  The
//! observable compared is the full 1 MiB `array` object (element-wise *and* as a
//! raw byte image) plus, where `long_exec` is involved, the exact stdout bytes.

mod common;

use common::*;
use std::ffi::c_int;

/// Fixed generator seed, so every "random" row is reproducible.
const RNG_SEED: u64 = 0x5EED_1234_ABCD;

fn random_array(rng: &mut Rng) -> Vec<c_int> {
    let mut v = vec![0i32; ARRAY_LEN];
    rng.fill(&mut v);
    v
}

// ---------------------------------------------------------------------------
// Rows 1-2: randomised full-width and rand()-range inputs, single pass.
// ---------------------------------------------------------------------------

#[test]
fn row01_random_full_width_i32_k1() {
    let mut rng = Rng::new(RNG_SEED);
    for round in 0..32 {
        let input = random_array(&mut rng);
        diff_peo(&format!("row01 round {round}"), &input, 1);
    }
}

#[test]
fn row02_random_rand_range_k1() {
    let mut rng = Rng::new(RNG_SEED ^ 2);
    for round in 0..16 {
        let mut input = vec![0i32; ARRAY_LEN];
        for slot in input.iter_mut() {
            *slot = rng.next_rand_range();
        }
        diff_peo(&format!("row02 round {round}"), &input, 1);
    }
}

// ---------------------------------------------------------------------------
// Rows 3-11: the input shapes the kernel arithmetic special-cases.
// ---------------------------------------------------------------------------

#[test]
fn row03_all_zeros_k1() {
    diff_peo("row03 all zeros", &vec![0i32; ARRAY_LEN], 1);
}

#[test]
fn row04_all_int_max_k1() {
    diff_peo("row04 all INT_MAX", &vec![i32::MAX; ARRAY_LEN], 1);
}

#[test]
fn row05_all_int_min_k1() {
    diff_peo("row05 all INT_MIN", &vec![i32::MIN; ARRAY_LEN], 1);
}

#[test]
fn row06_all_minus_one_k1() {
    diff_peo("row06 all -1", &vec![-1i32; ARRAY_LEN], 1);
}

#[test]
fn row07_exhaustive_small_window_k1() {
    // -131072 ..= 131071 covers every sign, parity and mod-7 residue near zero.
    let half = (ARRAY_LEN / 2) as i32;
    let input: Vec<c_int> = (0..ARRAY_LEN as i32).map(|i| i - half).collect();
    diff_peo("row07 small window", &input, 1);
}

#[test]
fn row08_extreme_negative_band_k1() {
    let input: Vec<c_int> = (0..ARRAY_LEN as i32).map(|i| i32::MIN.wrapping_add(i)).collect();
    diff_peo("row08 INT_MIN band", &input, 1);
}

#[test]
fn row09_extreme_positive_band_k1() {
    let input: Vec<c_int> = (0..ARRAY_LEN as i32).map(|i| i32::MAX.wrapping_sub(i)).collect();
    diff_peo("row09 INT_MAX band", &input, 1);
}

#[test]
fn row10_multiples_of_seven_and_neighbours_k1() {
    // x % 7 is the only modulus in the kernel; straddle its zero residue.
    let input: Vec<c_int> = (0..ARRAY_LEN as i32)
        .map(|i| {
            let base = (i / 3).wrapping_mul(7);
            let base = if i % 2 == 0 { base } else { base.wrapping_neg() };
            match i % 3 {
                0 => base,
                1 => base.wrapping_add(1),
                _ => base.wrapping_sub(1),
            }
        })
        .collect();
    diff_peo("row10 multiples of 7", &input, 1);
}

#[test]
fn row11_powers_of_two_k1() {
    let input: Vec<c_int> = (0..ARRAY_LEN)
        .map(|i| {
            let b = (i % 32) as u32;
            let v = 1i32.wrapping_shl(b);
            if (i / 32) % 2 == 0 {
                v
            } else {
                v.wrapping_neg()
            }
        })
        .collect();
    diff_peo("row11 powers of two", &input, 1);
}

// ---------------------------------------------------------------------------
// Rows 12-19: composition count, including the boundary where the Rust crate
// switches from naive iteration to its cycle-accelerated path (n >= 8192).
// ---------------------------------------------------------------------------

#[test]
fn row12_zero_calls_leaves_array_untouched() {
    let mut rng = Rng::new(RNG_SEED ^ 12);
    let input = random_array(&mut rng);
    diff_peo("row12 k=0", &input, 0);
    // and the array must still equal the input on both sides
    let l = libs();
    assert_eq!(l.c.read_array(), input);
    assert_eq!(l.rs.read_array(), input);
}

#[test]
fn row13_random_k2() {
    let mut rng = Rng::new(RNG_SEED ^ 13);
    for round in 0..8 {
        let input = random_array(&mut rng);
        diff_peo(&format!("row13 round {round}"), &input, 2);
    }
}

#[test]
fn row14_random_k3() {
    let mut rng = Rng::new(RNG_SEED ^ 14);
    for round in 0..4 {
        let input = random_array(&mut rng);
        diff_peo(&format!("row14 round {round}"), &input, 3);
    }
}

#[test]
fn row15_random_k20() {
    let mut rng = Rng::new(RNG_SEED ^ 15);
    for round in 0..2 {
        let input = random_array(&mut rng);
        diff_peo(&format!("row15 round {round}"), &input, 20);
    }
}

#[test]
fn row16_random_k81_just_below_accelerator_threshold() {
    let mut rng = Rng::new(RNG_SEED ^ 16);
    let input = random_array(&mut rng);
    diff_peo("row16 k=81 (n=8100)", &input, 81);
}

#[test]
fn row17_random_k82_at_accelerator_threshold() {
    let mut rng = Rng::new(RNG_SEED ^ 17);
    let input = random_array(&mut rng);
    diff_peo("row17 k=82 (n=8200)", &input, 82);
}

#[test]
fn row18_random_k100_accelerated_regime() {
    let mut rng = Rng::new(RNG_SEED ^ 18);
    let input = random_array(&mut rng);
    diff_peo("row18 k=100 (n=10000)", &input, 100);
}

#[test]
fn row19_all_zeros_k100_single_shared_orbit() {
    diff_peo("row19 zeros k=100", &vec![0i32; ARRAY_LEN], 100);
}

// ---------------------------------------------------------------------------
// Rows 20-21: the exported `array` object itself.
// ---------------------------------------------------------------------------

#[test]
fn row20_boundary_indices_only() {
    let mut v = vec![0i32; ARRAY_LEN];
    v[0] = i32::MIN;
    v[ARRAY_LEN - 1] = i32::MAX;
    diff_peo("row20 boundary indices", &v, 1);
    let l = libs();
    let c = l.c.read_array();
    let rs = l.rs.read_array();
    assert_eq!(c[0], rs[0], "row20: array[0] diverged");
    assert_eq!(
        c[ARRAY_LEN - 1],
        rs[ARRAY_LEN - 1],
        "row20: array[262143] diverged"
    );
}

#[test]
fn row21_array_object_bytes_and_size() {
    let l = libs();
    // A caller-visible write of a full byte pattern must round-trip identically
    // through both exported objects, including alignment-sensitive tails.
    let pattern: Vec<c_int> = (0..ARRAY_LEN as i32).map(|i| i.wrapping_mul(-0x0101_0101)).collect();
    l.c.write_array(&pattern);
    l.rs.write_array(&pattern);
    assert_eq!(l.c.read_array_bytes(), l.rs.read_array_bytes());
    assert_eq!(l.c.read_array_bytes().len(), ARRAY_BYTES);
    // The two objects must be distinct memory (each `.so` has its own `.bss`).
    assert_ne!(l.c.array_ptr(), l.rs.array_ptr());
}

// ---------------------------------------------------------------------------
// Rows 22-33: the full `long_exec` pipeline (n = 200000) across seed classes.
//
// The C side costs ~470 s per call, so its reference output is captured out of
// band by `tests/ground_truth/capture.sh`, which dlopens the C `.so` and records
// the printed line (and, for three seeds, the final 1 MiB `array` image).  The
// Rust `.so` is driven live here and compared against those recorded C bytes.
// ---------------------------------------------------------------------------

fn ground_truth_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/ground_truth")
}

/// Compare the Rust `.so`'s live `long_exec` output against recorded C output.
fn diff_long_exec_vs_recorded(seed: u32) {
    let dir = ground_truth_dir();
    let out_file = dir.join(format!("c_{seed}.out"));
    let expected = std::fs::read(&out_file).unwrap_or_else(|e| {
        panic!(
            "missing C ground truth {}: {e}\nrun tests/ground_truth/capture.sh first",
            out_file.display()
        )
    });
    assert!(
        !expected.is_empty(),
        "{}: recorded C output is empty",
        out_file.display()
    );

    let l = libs();
    let actual = capture_stdout(|| l.rs.long_exec(seed));
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "seed={seed}: Rust long_exec stdout differs from recorded C stdout"
    );

    // If the C run also recorded the final array image, compare all 1 MiB.
    let arr_file = dir.join(format!("arr_{seed}.bin"));
    if let Ok(expected_arr) = std::fs::read(&arr_file) {
        assert_eq!(
            expected_arr.len(),
            ARRAY_BYTES,
            "{}: wrong size",
            arr_file.display()
        );
        let actual_arr = l.rs.read_array_bytes();
        if actual_arr != expected_arr {
            let ne = actual_arr
                .chunks(4)
                .zip(expected_arr.chunks(4))
                .filter(|(a, b)| a != b)
                .count();
            panic!("seed={seed}: final `array` differs from C in {ne} of {ARRAY_LEN} elements");
        }
    }
}

#[test]
fn row22_long_exec_seed_0() {
    diff_long_exec_vs_recorded(0);
}
#[test]
fn row23_long_exec_seed_1() {
    diff_long_exec_vs_recorded(1);
}
#[test]
fn row24_long_exec_seed_7() {
    diff_long_exec_vs_recorded(7);
}
#[test]
fn row25_long_exec_seed_255() {
    diff_long_exec_vs_recorded(255);
}
#[test]
fn row26_long_exec_seed_65535() {
    diff_long_exec_vs_recorded(65535);
}
#[test]
fn row27_long_exec_seed_42() {
    diff_long_exec_vs_recorded(42);
}
#[test]
fn row28_long_exec_seed_12345() {
    diff_long_exec_vs_recorded(12345);
}
#[test]
fn row29_long_exec_seed_3() {
    diff_long_exec_vs_recorded(3);
}
#[test]
fn row30_long_exec_seed_100() {
    diff_long_exec_vs_recorded(100);
}
#[test]
fn row31_long_exec_seed_999983() {
    diff_long_exec_vs_recorded(999983);
}
#[test]
fn row32_long_exec_seed_2_pow_31() {
    diff_long_exec_vs_recorded(2147483648);
}
#[test]
fn row33_long_exec_seed_uint_max() {
    diff_long_exec_vs_recorded(4294967295);
}

// ---------------------------------------------------------------------------
// Rows 34-37: state interaction across calls.
// ---------------------------------------------------------------------------

/// Row 34: `long_exec(seed)` then one extra low-level pass (n = 200100).
/// The recorded C `array` image is the state right after `long_exec`, so the
/// extra pass is applied to *both* the recorded C image (via the C `.so`, one
/// cheap call) and the live Rust state.
#[test]
fn row34_long_exec_then_extra_pass() {
    let dir = ground_truth_dir();
    for seed in [42u32, 0, 4294967295] {
        let arr = std::fs::read(dir.join(format!("arr_{seed}.bin")))
            .unwrap_or_else(|e| panic!("missing arr_{seed}.bin: {e}"));
        assert_eq!(arr.len(), ARRAY_BYTES);
        let recorded: Vec<c_int> = arr
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let l = libs();
        // Rust: run the real pipeline, then the extra pass, live.
        let _ = capture_stdout(|| l.rs.long_exec(seed));
        l.rs.peo();
        let rs_after = l.rs.read_array();
        // C: continue from its own recorded post-pipeline state.
        let c_after = run_peo(l.c, &recorded, 1);
        assert_arrays_eq(
            &format!("row34 seed={seed}"),
            &recorded,
            &c_after,
            &rs_after,
        );
    }
}

/// Row 35: a poisoned `array` must be completely overwritten by the seeded fill,
/// so the printed line is unchanged from the clean-state run.
#[test]
fn row35_poisoned_array_then_long_exec() {
    let mut rng = Rng::new(RNG_SEED ^ 35);
    let poison = random_array(&mut rng);
    let l = libs();
    l.rs.write_array(&poison);
    l.rs.peo(); // churn the poison further
    let out = capture_stdout(|| l.rs.long_exec(42));
    let expected = std::fs::read(ground_truth_dir().join("c_42.out")).expect("c_42.out");
    assert_eq!(
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(&expected),
        "row35: pre-existing array contents leaked into long_exec's result"
    );
}

/// Row 36: two `long_exec` calls with the same seed must print the same line.
#[test]
fn row36_long_exec_twice_same_seed() {
    let l = libs();
    let first = capture_stdout(|| l.rs.long_exec(7));
    let second = capture_stdout(|| l.rs.long_exec(7));
    assert_eq!(first, second, "row36: long_exec is not reseeding");
    let expected = std::fs::read(ground_truth_dir().join("c_7.out")).expect("c_7.out");
    assert_eq!(
        String::from_utf8_lossy(&first),
        String::from_utf8_lossy(&expected)
    );
}

/// Row 37: different seeds, back to back, with no carry-over.
#[test]
fn row37_long_exec_two_different_seeds() {
    let l = libs();
    let a = capture_stdout(|| l.rs.long_exec(1));
    let b = capture_stdout(|| l.rs.long_exec(12345));
    assert_ne!(a, b, "row37: different seeds produced the same output");
    let dir = ground_truth_dir();
    assert_eq!(
        String::from_utf8_lossy(&a),
        String::from_utf8_lossy(&std::fs::read(dir.join("c_1.out")).unwrap())
    );
    assert_eq!(
        String::from_utf8_lossy(&b),
        String::from_utf8_lossy(&std::fs::read(dir.join("c_12345.out")).unwrap())
    );
}

// ---------------------------------------------------------------------------
// Rows 38-40.
// ---------------------------------------------------------------------------

/// Row 38: alternate the two libraries in one process.  They share glibc's
/// `rand` state and `stdout`, so this catches any hidden global coupling.
#[test]
fn row38_interleaved_calls() {
    let mut rng = Rng::new(RNG_SEED ^ 38);
    let l = libs();
    for round in 0..4 {
        let input = random_array(&mut rng);
        l.c.write_array(&input);
        l.rs.write_array(&input);
        for _ in 0..3 {
            l.c.peo();
            l.rs.peo();
        }
        assert_arrays_eq(
            &format!("row38 round {round}"),
            &input,
            &l.c.read_array(),
            &l.rs.read_array(),
        );
    }
}

/// Row 39: inputs that already sit on the kernel's cycles.  They are obtained
/// from the *C* library (100 passes of the real C kernel), so the input set is
/// C-derived rather than guessed, then re-run for k = 1 and k = 100.
#[test]
fn row39_values_on_kernel_cycles() {
    let mut rng = Rng::new(RNG_SEED ^ 39);
    let seedv = random_array(&mut rng);
    let deep = {
        let l = libs();
        run_peo(l.c, &seedv, 100)
    };
    diff_peo("row39 cycle members", &deep, 1);
    diff_peo("row39 cycle members", &deep, 100);
}

/// Row 40: the `debug-stats` feature is diagnostics-only.  Under either feature
/// setting the observable behaviour must be identical, which this test asserts
/// by running the standard differential; the feature switch itself is driven by
/// the test runner script.
#[test]
fn row40_feature_does_not_perturb_observables() {
    let mut rng = Rng::new(RNG_SEED ^ 40);
    let input = random_array(&mut rng);
    diff_peo("row40 feature-invariance", &input, 1);
    // stdout must stay empty for the low-level entry point regardless of feature
    let l = libs();
    let out = capture_stdout(|| l.rs.peo());
    assert!(
        out.is_empty(),
        "row40: perform_expensive_operations wrote to stdout: {:?}",
        String::from_utf8_lossy(&out)
    );
}
