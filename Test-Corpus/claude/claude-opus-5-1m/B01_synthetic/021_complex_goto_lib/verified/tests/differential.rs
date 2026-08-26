//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH shared objects via
//! `libloading` and compares the exact bytes each writes to `stdout`.

mod common;

use common::{Rng, SEED, assert_same, assert_same_labelled, bounded_pair, c_driver, rust_driver};
use std::ffi::c_int;

const INT_MIN: c_int = c_int::MIN;

/// Row 1 — loop guard false on entry: `x <= 0 && y <= 0`.
#[test]
fn row01_loop_never_entered() {
    let mut rng = Rng::new(SEED ^ 1);
    for x in [0, -1, -2, -7, -1000, INT_MIN, INT_MIN + 1] {
        for y in [0, -1, -3, -999, INT_MIN, INT_MIN + 1] {
            assert_same_labelled("row01/fixed", x, y);
        }
    }
    for _ in 0..300 {
        let x = rng.range(INT_MIN, 0);
        let y = rng.range(INT_MIN, 0);
        assert_same_labelled("row01/random", x, y);
    }
}

/// Row 2 — `x == 0`, `y > 0`: `label1` never fires.
#[test]
fn row02_x_zero_small_y() {
    for y in 1..=8 {
        assert_same_labelled("row02/exhaustive", 0, y);
    }
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..200 {
        let y = rng.range(1, 8);
        assert_same_labelled("row02/random", 0, y);
    }
}

/// Row 3 — `x < 0` (including `INT_MIN`) with `y > 0`.
#[test]
fn row03_x_negative_y_positive() {
    let mut rng = Rng::new(SEED ^ 3);
    for x in [-1, -2, -3, -17, -100000, INT_MIN + 1, INT_MIN] {
        for y in [1, 2, 3, 4, 5, 17, 64] {
            assert_same_labelled("row03/fixed", x, y);
        }
    }
    for _ in 0..300 {
        let x = rng.range(INT_MIN, -1);
        let y = rng.range(1, 64);
        assert_same_labelled("row03/random", x, y);
    }
}

/// Row 4 — `y == 0` on entry with `x < 3`.
#[test]
fn row04_y_zero_small_x() {
    for x in [1, 2] {
        assert_same_labelled("row04", x, 0);
    }
}

/// Row 5 — `y == 0` on entry with `x >= 3`.
#[test]
fn row05_y_zero_large_x() {
    for x in 3..=40 {
        assert_same_labelled("row05/exhaustive", x, 0);
    }
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..200 {
        let x = rng.range(3, 40);
        assert_same_labelled("row05/random", x, 0);
    }
}

/// Row 6 — the `x == 1 && y == 4` special case (`goto label2`).
#[test]
fn row06_special_case_x1_y4() {
    assert_same_labelled("row06", 1, 4);
    // Also reached mid-run? `x` only ever decreases and `y` only decreases, so
    // (1,4) can also be hit from (2,5) etc. — check the neighbourhood.
    for x in 1..=4 {
        for y in 3..=6 {
            assert_same_labelled("row06/neighbourhood", x, y);
        }
    }
}

/// Row 7 — `x == 1`, `y != 4`: right half of the `&&` short circuit.
#[test]
fn row07_x1_y_not_4() {
    for y in 1..=64 {
        if y != 4 {
            assert_same_labelled("row07/exhaustive", 1, y);
        }
    }
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..200 {
        let mut y = rng.range(1, 64);
        if y == 4 {
            y = 5;
        }
        assert_same_labelled("row07/random", 1, y);
    }
}

/// Row 8 — `x != 1`, `y == 4`: left half of the `&&` short circuit.
#[test]
fn row08_x_not_1_y4() {
    for x in 0..=64 {
        if x != 1 {
            assert_same_labelled("row08/exhaustive", x, 4);
        }
    }
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..200 {
        let mut x = rng.range(0, 64);
        if x == 1 {
            x = 2;
        }
        assert_same_labelled("row08/random", x, 4);
    }
}

/// Row 9 — `x == 2`: `x < 3` back-edge boundary (taken).
#[test]
fn row09_back_edge_boundary_x2() {
    for y in 1..=64 {
        assert_same_labelled("row09/exhaustive", 2, y);
    }
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..200 {
        let y = rng.range(1, 64);
        assert_same_labelled("row09/random", 2, y);
    }
}

/// Row 10 — `x == 3`: `x < 3` back-edge boundary (not taken after decrement).
#[test]
fn row10_back_edge_boundary_x3() {
    for y in 1..=64 {
        assert_same_labelled("row10/exhaustive", 3, y);
    }
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..200 {
        let y = rng.range(1, 64);
        assert_same_labelled("row10/random", 3, y);
    }
}

/// Row 11 — `x == 4`: first decrement lands exactly on the boundary value 3.
#[test]
fn row11_back_edge_flip_x4() {
    for y in 1..=64 {
        assert_same_labelled("row11/exhaustive", 4, y);
    }
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..200 {
        let y = rng.range(1, 64);
        assert_same_labelled("row11/random", 4, y);
    }
}

/// Row 12 — `x > 4`, `y > 0`: back-edge state flips part-way through the run.
#[test]
fn row12_mixed_back_edge() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..400 {
        let x = rng.range(5, 40);
        let y = rng.range(1, 40);
        assert_same_labelled("row12/random", x, y);
    }
}

/// Row 13 — exhaustive pruned cross product over the whole branch-relevant
/// neighbourhood: `x, y ∈ [-4, 24]`.
#[test]
fn row13_exhaustive_small_grid() {
    for x in -4..=24 {
        for y in -4..=24 {
            if x > 0 && y < 0 {
                continue; // unbounded in C — see ERRORS.md rows 14/15
            }
            assert_same_labelled("row13", x, y);
        }
    }
}

/// Row 14 — randomized `x, y ∈ [-64, 64]`.
#[test]
fn row14_random_small() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..2000 {
        let (x, y) = bounded_pair(rng.range(-64, 64), rng.range(-64, 64));
        assert_same_labelled("row14", x, y);
    }
}

/// Row 15 — randomized medium magnitudes `x, y ∈ [0, 1000]`.
#[test]
fn row15_random_medium() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..200 {
        let x = rng.range(0, 1000);
        let y = rng.range(0, 1000);
        assert_same_labelled("row15", x, y);
    }
}

/// Row 16 — strongly asymmetric shapes.
#[test]
fn row16_asymmetric_shapes() {
    for x in [0, 1, 2] {
        for y in [500, 1000, 4096] {
            assert_same_labelled("row16/y-heavy", x, y);
        }
    }
    for x in [500, 1000, 5000] {
        for y in [0, 1, 2] {
            assert_same_labelled("row16/x-heavy", x, y);
        }
    }
}

/// Row 17 — large scale.
#[test]
fn row17_large_scale() {
    for x in [5_000, 50_000, 200_000] {
        for y in [0, 7, 200_000] {
            assert_same_labelled("row17", x, y);
        }
    }
}

/// Row 18 — sequencing: repeated and interleaved invocations must not leak
/// state between calls in either implementation.
#[test]
fn row18_sequencing_and_interleaving() {
    let mut rng = Rng::new(SEED ^ 18);

    // 100 sequential invocations, each compared on its own.
    for _ in 0..100 {
        let (x, y) = bounded_pair(rng.range(-8, 40), rng.range(-8, 40));
        assert_same_labelled("row18/sequential", x, y);
    }

    // Interleave C and Rust calls inside a single captured region and compare
    // the concatenated stream against the reverse interleaving.
    let cf = c_driver();
    let rf = rust_driver();
    let cases: Vec<(c_int, c_int)> = (0..25)
        .map(|_| bounded_pair(rng.range(-4, 20), rng.range(-4, 20)))
        .collect();

    let all_c = common::capture(|| {
        for &(x, y) in &cases {
            unsafe { cf(x, y) };
        }
    });
    let all_rust = common::capture(|| {
        for &(x, y) in &cases {
            unsafe { rf(x, y) };
        }
    });
    assert_eq!(
        all_c, all_rust,
        "row18: concatenated output of 25 chained calls diverged"
    );

    // C-then-Rust for each case, in one capture, must equal Rust-then-C.
    let cr = common::capture(|| {
        for &(x, y) in &cases {
            unsafe { cf(x, y) };
            unsafe { rf(x, y) };
        }
    });
    let rc = common::capture(|| {
        for &(x, y) in &cases {
            unsafe { rf(x, y) };
            unsafe { cf(x, y) };
        }
    });
    assert_eq!(cr, rc, "row18: interleaved C/Rust streams diverged");
}

/// Row 19 — extreme `int` values with bounded runtime.
#[test]
fn row19_extremes_bounded() {
    let cases: [(c_int, c_int); 14] = [
        (INT_MIN, 1),
        (INT_MIN, 2),
        (INT_MIN, 4),
        (INT_MIN, 5),
        (INT_MIN, 64),
        (INT_MIN, 0),
        (INT_MIN, INT_MIN),
        (INT_MIN, -1),
        (-1, 0),
        (0, -1),
        (0, 0),
        (1, 0),
        (0, 1),
        (-1, INT_MIN),
    ];
    for (x, y) in cases {
        assert_same_labelled("row19", x, y);
    }
}

/// Row 20 — multi-MiB output, forcing many `stdout` buffer flushes.
#[test]
fn row20_multi_mib_output() {
    assert_same(0, 400_000);
    assert_same(400_000, 0);
    assert_same(150_000, 150_000);
}

/// Row 21 — caller-selected `stdout` buffering mode (`_IOFBF` / `_IOLBF` /
/// `_IONBF` / inherited): the byte stream must be identical in every mode.
#[test]
fn row21_buffering_modes_byte_equality() {
    let cf = c_driver();
    let rf = rust_driver();
    let mut rng = Rng::new(SEED ^ 21);
    let mut cases: Vec<(c_int, c_int)> = vec![
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 4),
        (2, 2),
        (3, 0),
        (4, 4),
        (2, 5),
        (INT_MIN, 5),
        (40, 40),
        (0, 4096),
        (4096, 0),
    ];
    for _ in 0..40 {
        cases.push(bounded_pair(rng.range(-8, 60), rng.range(-8, 60)));
    }
    for mode in common::BufMode::ALL {
        for &(x, y) in &cases {
            let c = common::capture_outcome_mode(mode, || unsafe { cf(x, y) });
            let r = common::capture_outcome_mode(mode, || unsafe { rf(x, y) });
            assert_eq!(
                c.status, r.status,
                "row21 {mode:?}: driver({x}, {y}) status differs"
            );
            assert_eq!(
                c.out.len(),
                r.out.len(),
                "row21 {mode:?}: driver({x}, {y}) length differs"
            );
            assert!(
                c.out == r.out,
                "row21 {mode:?}: driver({x}, {y}) bytes differ"
            );
        }
    }
}

/// Row 22 — `write(2)` framing per buffering mode. fd 1 is a `SOCK_SEQPACKET`
/// socket, so every element compared is exactly one write the library issued.
/// This is what distinguishes the C library's compiler-generated
/// `puts("loop")` from a `printf("%s", "loop\n")` translation.
#[test]
fn row22_write_framing_matches() {
    let cf = c_driver();
    let rf = rust_driver();
    let cases: [(c_int, c_int); 9] = [
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 4),
        (2, 2),
        (3, 0),
        (4, 4),
        (2, 5),
        (INT_MIN, 5),
    ];
    for mode in common::BufMode::ALL {
        for (x, y) in cases {
            let c = common::capture_frames(cf, x, y, mode);
            let r = common::capture_frames(rf, x, y, mode);
            assert_eq!(
                c,
                r,
                "row22 {mode:?}: driver({x}, {y}) write framing differs\n  C   frames: {:?}\n  Rust frames: {:?}",
                c.iter().map(|f| f.len()).collect::<Vec<_>>(),
                r.iter().map(|f| f.len()).collect::<Vec<_>>()
            );
        }
    }

    // Sanity: the modes really do produce different framing, i.e. `setvbuf`
    // took effect and the comparison above is not vacuous.
    let unbuffered = common::capture_frames(cf, 2, 2, common::BufMode::Unbuffered);
    let full = common::capture_frames(cf, 2, 2, common::BufMode::Full);
    let line = common::capture_frames(cf, 2, 2, common::BufMode::Line);
    assert_eq!(full.len(), 1, "expected one flush for a full buffer");
    assert_eq!(line.len(), 5, "expected one write per line for _IOLBF");
    assert_eq!(
        unbuffered.len(),
        10,
        "expected payload+newline writes for _IONBF"
    );
}

/// Row 23 — concurrent callers sharing `stdout`: the *set* of lines produced by
/// N threads must match between the two libraries (the interleaving itself is
/// not deterministic, but line integrity and totals are).
#[test]
fn row23_concurrent_callers() {
    fn run(f: common::DriverFn, cases: &'static [(c_int, c_int)]) -> Vec<Vec<u8>> {
        let mut out = common::capture(|| {
            let mut handles = Vec::new();
            for chunk in cases.chunks(2) {
                let f = f;
                handles.push(std::thread::spawn(move || {
                    for &(x, y) in chunk {
                        unsafe { f(x, y) };
                    }
                }));
            }
            for h in handles {
                h.join().unwrap();
            }
        })
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| l.to_vec())
        .collect::<Vec<_>>();
        out.sort();
        out
    }

    static CASES: [(c_int, c_int); 8] = [
        (5, 5),
        (2, 7),
        (1, 4),
        (0, 9),
        (3, 0),
        (12, 3),
        (INT_MIN, 4),
        (0, 0),
    ];
    let c = run(c_driver(), &CASES);
    let r = run(rust_driver(), &CASES);
    assert_eq!(
        c, r,
        "row23: multiset of lines produced by concurrent callers differs"
    );
    assert!(!c.is_empty());
}
