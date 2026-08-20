// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md.  Every test drives BOTH shared objects
// through their exported `driver` symbol (never a direct Rust call) and
// compares the produced byte stream byte-for-byte.

mod common;

use common::{assert_same, assert_same_pipe, assert_same_sequence, Rng, SEED};

// --- C1 ---------------------------------------------------------------------
#[test]
fn c1_single_iteration() {
    assert_same(1);
    // exact expected bytes, so the test also pins the format down
    assert_eq!(common::c_output(1), b"0 0\n".to_vec());
    assert_eq!(common::rust_output(1), b"0 0\n".to_vec());
}

// --- C2 ---------------------------------------------------------------------
#[test]
fn c2_few_iterations_exhaustive() {
    for x in 2..=9 {
        assert_same(x);
    }
    assert_eq!(
        common::c_output(3),
        b"0 0\n1 2\n2 4\n".to_vec(),
        "C reference format changed?"
    );
}

// --- C3 --- j crosses 9 -> 10 while i is still one digit ---------------------
#[test]
fn c3_j_width_crosses_before_i() {
    for x in 5..=8 {
        assert_same(x);
    }
    assert_eq!(
        common::rust_output(6),
        common::c_output(6),
        "asymmetric field widths on one line"
    );
}

// --- C4 --- i crosses 9 -> 10 -----------------------------------------------
#[test]
fn c4_i_width_crosses_ten() {
    for x in 9..=12 {
        assert_same(x);
    }
}

// --- C5 --- 100-boundaries for j (i=50) and for i (i=100) --------------------
#[test]
fn c5_hundred_boundaries() {
    for x in 49..=52 {
        assert_same(x);
    }
    for x in 99..=102 {
        assert_same(x);
    }
}

// --- C6 --- 1000-boundaries for j (i=500) and for i (i=1000) -----------------
#[test]
fn c6_thousand_boundaries() {
    for x in 499..=502 {
        assert_same(x);
    }
    for x in 999..=1002 {
        assert_same(x);
    }
}

// --- C7 --- 10^4 / 10^5 boundaries for both fields --------------------------
#[test]
fn c7_ten_thousand_and_hundred_thousand_boundaries() {
    for x in 4999..=5001 {
        assert_same(x);
    }
    for x in 9999..=10001 {
        assert_same(x);
    }
    for x in 49999..=50001 {
        assert_same(x);
    }
    for x in 99999..=100001 {
        assert_same(x);
    }
}

// --- C8 --- randomized small ------------------------------------------------
#[test]
fn c8_random_small() {
    let mut rng = Rng::new(SEED ^ 0x08);
    for _ in 0..300 {
        assert_same(rng.range_i32(1, 200));
    }
}

// --- C9 --- randomized medium (crosses the 4 KiB stdio buffer) --------------
#[test]
fn c9_random_medium() {
    let mut rng = Rng::new(SEED ^ 0x09);
    for _ in 0..120 {
        assert_same(rng.range_i32(200, 5_000));
    }
}

// --- C10 --- randomized large (many buffer flushes, 6-digit fields) ---------
#[test]
fn c10_random_large() {
    let mut rng = Rng::new(SEED ^ 0x0A);
    for _ in 0..24 {
        assert_same(rng.range_i32(5_000, 120_000));
    }
}

// --- C11 --- very large volume, 7-digit fields ------------------------------
#[test]
fn c11_one_million() {
    assert_same(1_000_000);
}

// --- C12 --- fd 1 is a pipe rather than a regular file ----------------------
#[test]
fn c12_random_over_pipe() {
    let mut rng = Rng::new(SEED ^ 0x0C);
    for _ in 0..40 {
        assert_same_pipe(rng.range_i32(1, 3_000));
    }
}

// --- C13 --- repeated calls concatenate identically -------------------------
#[test]
fn c13_repeated_calls_concatenate() {
    let mut rng = Rng::new(SEED ^ 0x0D);
    for _ in 0..20 {
        let x = rng.range_i32(1, 500);
        assert_same_sequence(&[x, x, x, x, x]);
    }
}

// --- C14 --- mixed script of accepting and rejecting calls ------------------
#[test]
fn c14_mixed_call_script() {
    let mut rng = Rng::new(SEED ^ 0x0E);
    for _ in 0..25 {
        let n = rng.range_i32(1, 8) as usize;
        let script: Vec<i32> = (0..n)
            .map(|_| match rng.next_u32() % 4 {
                0 => rng.range_i32(i32::MIN, 0), // rejecting
                1 => 0,
                2 => rng.range_i32(1, 12),
                _ => rng.range_i32(1, 2_000),
            })
            .collect();
        assert_same_sequence(&script);
    }
}

// --- C15 --- C and Rust interleaved on the shared process-wide stdout -------
#[test]
fn c15_interleaved_c_and_rust() {
    let (cf, rf) = common::both_libraries_loaded();
    let mut rng = Rng::new(SEED ^ 0x0F);
    for _ in 0..20 {
        let xs: Vec<i32> = (0..4).map(|_| rng.range_i32(0, 300)).collect();

        // Reference: the pure-C sequence x0 x1 x2 x3.
        let reference = common::capture_file(|| {
            for &x in &xs {
                unsafe { cf(x) };
            }
        });
        // Interleaved: C, Rust, C, Rust on the same stream.
        let mixed = common::capture_file(|| {
            for (i, &x) in xs.iter().enumerate() {
                if i % 2 == 0 {
                    unsafe { cf(x) };
                } else {
                    unsafe { rf(x) };
                }
            }
        });
        // And the mirror image: Rust, C, Rust, C.
        let mixed2 = common::capture_file(|| {
            for (i, &x) in xs.iter().enumerate() {
                if i % 2 == 0 {
                    unsafe { rf(x) };
                } else {
                    unsafe { cf(x) };
                }
            }
        });
        assert_eq!(reference, mixed, "C/Rust interleaving diverged for {xs:?}");
        assert_eq!(reference, mixed2, "Rust/C interleaving diverged for {xs:?}");
    }
}

// --- C16 --- power-of-two-ish shapes ---------------------------------------
#[test]
fn c16_power_of_two_shapes() {
    const SHAPES: &[i32] = &[
        1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256, 511, 512, 1023, 1024, 2047,
        2048, 4095, 4096, 8191, 8192, 65535, 65536,
    ];
    for &x in SHAPES {
        assert_same(x);
    }
}

// --- harness self-check -----------------------------------------------------
#[test]
fn harness_loads_two_distinct_shared_objects() {
    let (c_path, rust_path) = common::so_paths();
    assert!(c_path.is_file(), "missing C .so at {c_path:?}");
    assert!(rust_path.is_file(), "missing Rust .so at {rust_path:?}");
    assert_ne!(c_path, rust_path);
    let (cf, rf) = common::both_libraries_loaded();
    assert_ne!(
        cf as usize, rf as usize,
        "both symbols resolved to the same address — the wrong library was loaded"
    );
    // The capture machinery itself must be trustworthy.
    assert_eq!(common::capture_file(|| {}), Vec::<u8>::new());
    assert_eq!(common::capture_pipe(|| {}), Vec::<u8>::new());
}
