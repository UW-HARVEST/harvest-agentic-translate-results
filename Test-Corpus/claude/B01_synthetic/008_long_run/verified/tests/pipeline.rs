//! Phase B — the composed pipeline (CONFIGS.md rows 20–24, 26).
//!
//! `main`'s body is
//!
//! ```c
//! srand(seed);
//! for (i...) array[i] = rand();
//! for (i < ITERATIONS) perform_expensive_operations();
//! for (i...) xor_result ^= array[i];
//! printf("%d\n", xor_result);
//! ```
//!
//! Running that verbatim takes ~5 minutes per side (2000 × 262 144 × 100
//! arithmetic steps), so the fast tests drive the *same* stages through the same
//! exported symbols with `ITERATIONS` reduced to 1 and 3: real glibc
//! `srand`/`rand` + the C `.so`'s `array`/`perform_expensive_operations` on one
//! side, `harness_srand`/`harness_rand` + the Rust `.so`'s
//! `array`/`perform_expensive_operations` on the other.
//!
//! `full_end_to_end` runs the unreduced program through `main` and is
//! `#[ignore]`d because of its runtime; `scripts/e2e_binaries.sh` does the same
//! for the real CMake / cargo artefacts.

mod common;

use common::{pairs, Impl, Pairs, ARRAY_SIZE};

/// `srand(seed); for (i...) array[i] = rand();` using **real glibc**, straight
/// into the C `.so`'s exported `array`.
fn glibc_fill(imp: &Impl, seed: u32) {
    let p = imp.array_ptr();
    unsafe {
        libc::srand(seed);
        for i in 0..ARRAY_SIZE {
            *p.add(i) = libc::rand();
        }
    }
}

/// The same, using the Rust port's exported RNG hooks.
fn rust_fill(imp: &Impl, seed: u32) {
    let p = imp.array_ptr();
    imp.harness_srand(seed);
    for i in 0..ARRAY_SIZE {
        unsafe { *p.add(i) = imp.harness_rand() };
    }
}

/// `main`'s body with `ITERATIONS` replaced by `iterations`; returns the final
/// array and the XOR reduction that the program would print.
fn reduced_pipeline(imp: &Impl, seed: u32, iterations: usize, glibc: bool) -> (Vec<i32>, i32) {
    if glibc {
        glibc_fill(imp, seed);
    } else {
        rust_fill(imp, seed);
    }
    for _ in 0..iterations {
        imp.perform();
    }
    let out = imp.get_array();
    (out.clone(), common::xor_reduce(&out))
}

fn assert_pipeline_matches(p: &Pairs, seed: u32, iterations: usize) {
    let _g = common::array_guard();
    let (rust_arr, rust_xor) = reduced_pipeline(&p.rust, seed, iterations, false);
    for c in &p.c {
        let (c_arr, c_xor) = reduced_pipeline(c, seed, iterations, true);
        common::assert_arrays_eq(
            &format!("pipeline seed={seed} iterations={iterations}"),
            &c.name,
            &c_arr,
            &rust_arr,
            &c_arr,
        );
        assert_eq!(
            c_xor, rust_xor,
            "pipeline seed={seed} iterations={iterations}: {} printed {c_xor}, rust printed {rust_xor}",
            c.name
        );
    }
}

/// CONFIGS.md row 20 — the full pipeline with `ITERATIONS = 1`, over the seed
/// classes the seeding code distinguishes.
#[test]
fn pipeline_one_iteration() {
    let p = pairs();
    let seeds: [u32; 24] = [
        0,
        1,
        2,
        3,
        7,
        42,
        1000,
        12_345,
        65_535,
        65_536,
        127_773,
        1_000_003,
        2_147_483_646,
        2_147_483_647,
        2_147_483_648,
        2_147_483_649,
        3_000_000_000,
        4_000_000_000,
        4_294_967_294,
        4_294_967_295,
        16_807,
        2_836,
        305_419_896,
        2_863_311_530,
    ];
    for seed in seeds {
        assert_pipeline_matches(&p, seed, 1);
    }
}

/// CONFIGS.md row 21 — three compounding iterations.
#[test]
fn pipeline_three_iterations() {
    let p = pairs();
    for seed in [0u32, 1, 42, 2_147_483_648, 4_294_967_295, 999_999_999] {
        assert_pipeline_matches(&p, seed, 3);
    }
}

/// CONFIGS.md row 22 — every accepted textual form of the seed argument decodes
/// to the same `unsigned int` on both sides, and drives the same pipeline.
#[test]
fn pipeline_matches_for_seed_arguments() {
    let p = pairs();
    let forms: &[(&[u8], u32)] = &[
        (b"", 0),
        (b"0", 0),
        (b"-0", 0),
        (b"+0", 0),
        (b"1", 1),
        (b"-18446744073709551615", 1), // unsigned negation wraps to 1
        (b"42", 42),
        (b"+42", 42),
        (b"0042", 42),
        (b" 42", 42),
        (b"\t\n\x0b\x0c\r 42", 42),
        (b"4294967295", u32::MAX),
        (b"-18446744069414584321", u32::MAX),
        (b"2147483648", 2_147_483_648),
    ];

    // 1. the exported validation must decode every form to the documented seed
    let mut seeds: Vec<u32> = Vec::new();
    for (arg, expected) in forms {
        let got = p.rust.harness_parse_seed(arg).unwrap_or_else(|_| {
            panic!(
                "rust rejected the valid seed argument {:?}",
                String::from_utf8_lossy(arg)
            )
        });
        assert_eq!(
            got,
            *expected,
            "argv[1]={:?} decoded to {got}, expected {expected}",
            String::from_utf8_lossy(arg)
        );
        if !seeds.contains(expected) {
            seeds.push(*expected);
        }
    }

    // 2. and the pipeline for each distinct decoded seed must match the C
    for seed in seeds {
        assert_pipeline_matches(&p, seed, 1);
    }
}

/// CONFIGS.md row 23 / ERRORS.md row 38 — the global `array` keeps state between
/// runs, but the seeding loop overwrites all of it, so a second run of the same
/// seed must reproduce the first result on both sides.
#[test]
fn pipeline_is_repeatable() {
    let p = pairs();
    let _g = common::array_guard();
    for seed in [7u32, 4_294_967_295] {
        let first_rust = reduced_pipeline(&p.rust, seed, 2, false);
        let second_rust = reduced_pipeline(&p.rust, seed, 2, false);
        assert_eq!(first_rust.1, second_rust.1, "rust is not repeatable");
        assert!(first_rust.0 == second_rust.0, "rust array is not repeatable");

        for c in &p.c {
            let first_c = reduced_pipeline(c, seed, 2, true);
            let second_c = reduced_pipeline(c, seed, 2, true);
            assert_eq!(first_c.1, second_c.1, "{} is not repeatable", c.name);
            assert_eq!(
                first_c.1, first_rust.1,
                "{} vs rust differ on the repeated pipeline",
                c.name
            );
            assert!(first_c.0 == first_rust.0);
            assert!(second_c.0 == second_rust.0);
        }
    }
}

/// CONFIGS.md rows 24 & 26 — the real thing: `main` with `ITERATIONS = 2000`,
/// compared byte-for-byte (stdout, stderr, exit code).
///
/// ~4.5 min per implementation, so it is `#[ignore]`d. Run with:
/// `cargo test --release --test pipeline -- --ignored --nocapture`
/// (`E2E_SEEDS=1,42` selects the seeds, `E2E_C_IMPLS=c-O2` the C builds).
#[test]
#[ignore]
fn full_end_to_end() {
    let p = pairs();
    let seeds = std::env::var("E2E_SEEDS").unwrap_or_else(|_| "42".to_string());
    let want_impls = std::env::var("E2E_C_IMPLS").unwrap_or_else(|_| "c-O2".to_string());

    for seed in seeds.split(',') {
        let arg = seed.as_bytes().to_vec();
        let argv = common::Argv::from_strs(&[b"driver", &arg]);

        let t0 = std::time::Instant::now();
        let rust = common::run_main(&p.rust, 2, &argv);
        eprintln!(
            "rust  seed {seed}: {} ({:?})",
            rust.describe(),
            t0.elapsed()
        );
        assert_eq!(rust.status, 0, "rust rejected seed {seed}");
        assert!(rust.stderr.is_empty());

        for c in p.c.iter().filter(|c| want_impls.split(',').any(|w| w == c.name)) {
            let t0 = std::time::Instant::now();
            let got = common::run_main(c, 2, &argv);
            eprintln!(
                "{:5} seed {seed}: {} ({:?})",
                c.name,
                got.describe(),
                t0.elapsed()
            );
            assert_eq!(
                (got.status, &got.stdout, &got.stderr),
                (rust.status, &rust.stdout, &rust.stderr),
                "full end-to-end mismatch for seed {seed}: {} gave [{}], rust gave [{}]",
                c.name,
                got.describe(),
                rust.describe()
            );
        }
    }
}
