//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row calls BOTH `.so`s through their
//! exported `driver` symbol and compares stdout byte-for-byte, using many
//! randomized inputs from a fixed seed.

mod common;

use common::{Rng, SEED, assert_same, assert_same_sequence, expected};

/// Row 1 — A:`x==1`, B:`y==2`, C:`z==3`. The one and only success path.
#[test]
fn row01_success_path() {
    let out = assert_same(1, 2, 3);
    assert_eq!(
        String::from_utf8_lossy(&out),
        format!("{}{}", expected::OK, expected::result_line(0)),
        "success path transcript"
    );
    // Repeated, to prove the success path is not state-dependent.
    for _ in 0..16 {
        assert_same(1, 2, 3);
    }
}

/// Row 2 — A:`x!=1`, B:`y==2`, C:`z==3`.
#[test]
fn row02_x_bad_only() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..256 {
        let x = rng.interesting_int_except(1);
        let out = assert_same(x, 2, 3);
        assert_eq!(String::from_utf8_lossy(&out), expected::transcript(x, 2, 3));
    }
}

/// Row 3 — A:`x==1`, B:`y!=2`, C:`z==3`.
#[test]
fn row03_y_bad_only() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..256 {
        let y = rng.interesting_int_except(2);
        let out = assert_same(1, y, 3);
        assert_eq!(String::from_utf8_lossy(&out), expected::transcript(1, y, 3));
    }
}

/// Row 4 — A:`x==1`, B:`y==2`, C:`z!=3`.
#[test]
fn row04_z_bad_only() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..256 {
        let z = rng.interesting_int_except(3);
        let out = assert_same(1, 2, z);
        assert_eq!(String::from_utf8_lossy(&out), expected::transcript(1, 2, z));
    }
}

/// Row 5 — A:`x!=1`, B:`y!=2`, C:`z==3`.
#[test]
fn row05_x_and_y_bad() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..256 {
        let x = rng.interesting_int_except(1);
        let y = rng.interesting_int_except(2);
        let out = assert_same(x, y, 3);
        assert_eq!(String::from_utf8_lossy(&out), expected::transcript(x, y, 3));
    }
}

/// Row 6 — A:`x!=1`, B:`y==2`, C:`z!=3`.
#[test]
fn row06_x_and_z_bad() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..256 {
        let x = rng.interesting_int_except(1);
        let z = rng.interesting_int_except(3);
        let out = assert_same(x, 2, z);
        assert_eq!(String::from_utf8_lossy(&out), expected::transcript(x, 2, z));
    }
}

/// Row 7 — A:`x==1`, B:`y!=2`, C:`z!=3`.
#[test]
fn row07_y_and_z_bad() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..256 {
        let y = rng.interesting_int_except(2);
        let z = rng.interesting_int_except(3);
        let out = assert_same(1, y, z);
        assert_eq!(String::from_utf8_lossy(&out), expected::transcript(1, y, z));
    }
}

/// Row 8 — A:`x!=1`, B:`y!=2`, C:`z!=3`: every guard would fail.
#[test]
fn row08_all_bad() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..256 {
        let x = rng.interesting_int_except(1);
        let y = rng.interesting_int_except(2);
        let z = rng.interesting_int_except(3);
        let out = assert_same(x, y, z);
        assert_eq!(String::from_utf8_lossy(&out), expected::transcript(x, y, z));
    }
}

/// Row 9 — unconstrained random triples over the whole `int` domain.
#[test]
fn row09_unconstrained_random() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..4096 {
        let (x, y, z) = (
            rng.interesting_int(),
            rng.interesting_int(),
            rng.interesting_int(),
        );
        let out = assert_same(x, y, z);
        assert_eq!(
            String::from_utf8_lossy(&out),
            expected::transcript(x, y, z),
            "driver({x}, {y}, {z})"
        );
    }
}

/// Row 10 — long randomized call sequence in one process, so each library's
/// residual file-scope `static int y` is compared as well.
#[test]
fn row10_stateful_call_sequence() {
    let mut rng = Rng::new(SEED ^ 10);
    let calls: Vec<(i32, i32, i32)> = (0..512)
        .map(|i| {
            if i % 7 == 0 {
                (1, 2, 3) // sprinkle in the success path
            } else {
                (
                    rng.interesting_int(),
                    rng.interesting_int(),
                    rng.interesting_int(),
                )
            }
        })
        .collect();
    assert_same_sequence(&calls);
}

/// Row 12 — C and Rust calls interleaved in one process: each library owns a
/// private `static int y`, so neither can perturb the other.
#[test]
fn row12_interleaved_libraries() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..512 {
        let (x, y, z) = (
            rng.interesting_int(),
            rng.interesting_int(),
            rng.interesting_int(),
        );
        // assert_same already alternates C then Rust; alternate the *order* of
        // the extra probing call too so a stale `y` in either library shows up.
        assert_same(x, y, z);
        assert_same(1, 2, 3);
        assert_same(x, y, z);
    }
}

/// Row 13 — idempotence: the same triple repeated must print the same bytes.
#[test]
fn row13_repeat_idempotence() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..64 {
        let (x, y, z) = (
            rng.interesting_int(),
            rng.interesting_int(),
            rng.interesting_int(),
        );
        let first = assert_same(x, y, z);
        for _ in 0..2 {
            let again = assert_same(x, y, z);
            assert_eq!(first, again, "driver({x}, {y}, {z}) not idempotent");
        }
    }
}

/// Row 14 — exhaustive boundary cross-product in all three slots.
#[test]
fn row14_boundary_cross_product() {
    const VALUES: [i32; 8] = [i32::MIN, -1, 0, 1, 2, 3, 4, i32::MAX];
    for &x in &VALUES {
        for &y in &VALUES {
            for &z in &VALUES {
                let out = assert_same(x, y, z);
                assert_eq!(
                    String::from_utf8_lossy(&out),
                    expected::transcript(x, y, z),
                    "driver({x}, {y}, {z})"
                );
            }
        }
    }
}
