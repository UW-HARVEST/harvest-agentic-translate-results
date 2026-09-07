//! Phase B — valid-path differential tests. One test per row of `CONFIGS.md`.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` via `libloading`, calls
//! only their exported symbols (`run`, `driver`), and compares captured stdout
//! byte-for-byte. Randomised rows use a fixed-seed SplitMix64 PRNG.

#[path = "harness/mod.rs"]
mod harness;

use harness::{assert_each_driver, assert_each_run, assert_seq_matches, Op, Rng};

// ---------------------------------------------------------------------------
// Row 1-3: the low-level entry point `run` from pristine state, minimal shapes.
// ---------------------------------------------------------------------------

#[test]
fn c01_run_zero_pristine() {
    assert_seq_matches("c01", &[Op::Run(0)]);
}

#[test]
fn c02_run_one_pristine() {
    assert_seq_matches("c02", &[Op::Run(1)]);
}

#[test]
fn c03_run_neg_one_pristine() {
    assert_seq_matches("c03", &[Op::Run(-1)]);
}

// ---------------------------------------------------------------------------
// Rows 4-6: randomised argument shapes for `run`.
// ---------------------------------------------------------------------------

#[test]
fn c04_run_small_positive_random() {
    let mut rng = Rng::new(0x0000_0004_5EED);
    let args: Vec<i32> = (0..200).map(|_| rng.in_range(1, 1000)).collect();
    assert_each_run("c04", args);
}

#[test]
fn c05_run_small_negative_random() {
    let mut rng = Rng::new(0x0000_0005_5EED);
    // Includes values that take `bedrooms` (initially 5) negative.
    let args: Vec<i32> = (0..200).map(|_| rng.in_range(-1000, -1)).collect();
    assert_each_run("c05", args);
}

#[test]
fn c06_run_full_range_random() {
    let mut rng = Rng::new(0x0000_0006_5EED);
    let args: Vec<i32> = (0..500).map(|_| rng.next_i32()).collect();
    assert_each_run("c06", args);
}

// ---------------------------------------------------------------------------
// Rows 7-9: overflow / boundary landings of `bedrooms`.
// ---------------------------------------------------------------------------

#[test]
fn c07_run_int_max() {
    assert_seq_matches("c07", &[Op::Run(i32::MAX)]);
}

#[test]
fn c08_run_int_min() {
    assert_seq_matches("c08", &[Op::Run(i32::MIN)]);
}

#[test]
fn c09_run_boundary_landings() {
    // bedrooms starts at 5.
    let exact_max = i32::MAX - 5; // 5 + this == INT_MAX
    let args = [
        exact_max,             // lands exactly on INT_MAX
        exact_max + 1,         // one past INT_MAX -> wraps to INT_MIN
        exact_max - 1,         // one below INT_MAX
        i32::MIN + 5,          // 5 + this == INT_MIN
        i32::MIN + 4,          // one past INT_MIN -> wraps to INT_MAX
        i32::MIN + 6,          // one above INT_MIN
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        -5,                    // bedrooms -> exactly 0
        -6,                    // bedrooms -> -1
    ];
    assert_each_run("c09", args);
}

// ---------------------------------------------------------------------------
// Rows 10-13: the convenience wrapper `driver`.
// ---------------------------------------------------------------------------

#[test]
fn c10_driver_zero_pristine() {
    assert_seq_matches("c10", &[Op::Driver(0)]);
}

#[test]
fn c11_driver_small_positive_random() {
    let mut rng = Rng::new(0x0000_0011_5EED);
    let args: Vec<i32> = (0..200).map(|_| rng.in_range(1, 1000)).collect();
    assert_each_driver("c11", args);
}

#[test]
fn c12_driver_full_range_random() {
    let mut rng = Rng::new(0x0000_0012_5EED);
    let args: Vec<i32> = (0..500).map(|_| rng.next_i32()).collect();
    assert_each_driver("c12", args);
}

#[test]
fn c13_driver_extremes() {
    let args = [
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2 + 4,
        i32::MIN / 2 - 3,
        -1,
        1,
    ];
    assert_each_driver("c13", args);
}

// ---------------------------------------------------------------------------
// Rows 14-17: persistent global state across call sequences.
// ---------------------------------------------------------------------------

#[test]
fn c14_run_repeated_same_arg() {
    for n in [2usize, 3, 10] {
        for arg in [0i32, 1, -1, 7, i32::MAX, i32::MIN] {
            let ops: Vec<Op> = (0..n).map(|_| Op::Run(arg)).collect();
            assert_seq_matches(&format!("c14_n{n}_a{arg}"), &ops);
        }
    }
}

#[test]
fn c15_run_repeated_random_args() {
    let mut rng = Rng::new(0x0000_0015_5EED);
    for trial in 0..20 {
        let ops: Vec<Op> = (0..50).map(|_| Op::Run(rng.next_i32())).collect();
        assert_seq_matches(&format!("c15_t{trial}"), &ops);
    }
}

#[test]
fn c16_interleaved_random() {
    let mut rng = Rng::new(0x0000_0016_5EED);
    for trial in 0..10 {
        let ops: Vec<Op> = (0..300)
            .map(|_| {
                // Mix wide-range and small args so both wrapping and plain
                // arithmetic paths are hit within one sequence.
                let arg = if rng.next_u64() % 3 == 0 {
                    rng.next_i32()
                } else {
                    rng.in_range(-50, 50)
                };
                if rng.next_u64() % 2 == 0 {
                    Op::Run(arg)
                } else {
                    Op::Driver(arg)
                }
            })
            .collect();
        assert_seq_matches(&format!("c16_t{trial}"), &ops);
    }
}

#[test]
fn c17_driver_repeated() {
    let mut rng = Rng::new(0x0000_0017_5EED);
    let ops: Vec<Op> = (0..20).map(|_| Op::Driver(rng.in_range(-500, 500))).collect();
    assert_seq_matches("c17", &ops);
    let ops2: Vec<Op> = (0..20).map(|_| Op::Driver(i32::MAX)).collect();
    assert_seq_matches("c17_max", &ops2);
}

// ---------------------------------------------------------------------------
// Row 18: long accumulation — large `floors` and large `bathrooms` double.
// ---------------------------------------------------------------------------

#[test]
fn c18_long_accumulation() {
    let ops: Vec<Op> = (0..20_000).map(|_| Op::Run(0)).collect();
    assert_seq_matches("c18", &ops);
    // Same length, but also growing `bedrooms` through repeated wrapping.
    let mut rng = Rng::new(0x0000_0018_5EED);
    let ops2: Vec<Op> = (0..5_000).map(|_| Op::Run(rng.next_i32())).collect();
    assert_seq_matches("c18b", &ops2);
}

// ---------------------------------------------------------------------------
// Row 19: the empty sequence — loading must not print anything by itself.
// ---------------------------------------------------------------------------

#[test]
fn c19_no_calls_no_output() {
    let p = harness::fresh_pair("c19");
    // Resolve the symbols (proving they exist) but never call them.
    let out = harness::capture(|| {
        let _c_run = p.c_fn("run");
        let _c_drv = p.c_fn("driver");
        let _r_run = p.r_fn("run");
        let _r_drv = p.r_fn("driver");
    });
    assert!(
        out.is_empty(),
        "loading the libraries produced output: {:?}",
        String::from_utf8_lossy(&out)
    );
    assert_seq_matches("c19_empty", &[]);
}

// ---------------------------------------------------------------------------
// Row 20: the *initial* global state must match, not merely the deltas.
// ---------------------------------------------------------------------------

#[test]
fn c20_fresh_load_initial_state() {
    // Three independent fresh loads with the same argument must all yield the
    // same output on both sides -- i.e. `the_house` really is reset and the
    // C `.data` initialiser equals the Rust `static mut` initialiser.
    let mut baseline: Option<Vec<u8>> = None;
    for i in 0..3 {
        let p = harness::fresh_pair(&format!("c20_{i}"));
        let c_out = harness::capture(|| unsafe { (p.c_fn("run"))(3) });
        let r_out = harness::capture(|| unsafe { (p.r_fn("run"))(3) });
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
            "fresh load #{i}: C vs Rust divergence"
        );
        // The very first line reflects the untouched initialiser {2, 5, 2.5}.
        let first = c_out.split(|b| *b == b'\n').next().unwrap().to_vec();
        assert_eq!(
            String::from_utf8_lossy(&first),
            "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms",
            "unexpected initial state from C"
        );
        match &baseline {
            None => baseline = Some(c_out),
            Some(b) => assert_eq!(
                String::from_utf8_lossy(b),
                String::from_utf8_lossy(&c_out),
                "fresh load #{i} was not pristine"
            ),
        }
    }
}
