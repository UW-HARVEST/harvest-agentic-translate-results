//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//! Both libraries are driven exclusively through their `.so` exports.

mod common;
use common::{Libs, Rng, SEED};

// ---------------------------------------------------------------- rows 1..12
// Each in-range selector value 0..=11 dispatches to its own `_PfnN` and the
// self-comparison in `call_predict` must report a match.

macro_rules! row_in_range {
    ($name:ident, $pfcn:expr, $row:expr) => {
        #[test]
        fn $name() {
            let l = Libs::load();
            let v = l.assert_same($pfcn);
            assert_eq!(v, 1, "CONFIGS row {}: pfcn={} must match its Pfn", $row, $pfcn);
        }
    };
}

row_in_range!(row01_pfcn0, 0, 1);
row_in_range!(row02_pfcn1, 1, 2);
row_in_range!(row03_pfcn2, 2, 3);
row_in_range!(row04_pfcn3, 3, 4);
row_in_range!(row05_pfcn4, 4, 5);
row_in_range!(row06_pfcn5, 5, 6);
row_in_range!(row07_pfcn6, 6, 7);
row_in_range!(row08_pfcn7, 7, 8);
row_in_range!(row09_pfcn8, 8, 9);
row_in_range!(row10_pfcn9, 9, 10);
row_in_range!(row11_pfcn10, 10, 11);
row_in_range!(row12_pfcn11, 11, 12);

// ------------------------------------------------------------------- row 13
// pfcn 12..=15: the shared `12|13|14|15` FIR band of BTAC1C2_PredictSample,
// reached through GetPredictFunc's `default` arm.
#[test]
fn row13_fir_band_12_to_15() {
    let l = Libs::load();
    for pfcn in 12..=15 {
        let v = l.assert_same(pfcn);
        assert_eq!(v, 0, "row 13: pfcn={pfcn} is outside GetPredictFunc's cases");
    }
}

// ------------------------------------------------------------------- row 14
// Exhaustive sweep across every boundary crossing.
#[test]
fn row14_exhaustive_sweep_minus1000_to_1000() {
    let l = Libs::load();
    for pfcn in -1000..=1000 {
        let v = l.assert_same(pfcn);
        let expect = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        assert_eq!(v, expect, "row 14: unexpected shared value for pfcn={pfcn}");
    }
}

// ------------------------------------------------------------------- row 15
// Boundary set including the i32 extremes.
#[test]
fn row15_boundary_values() {
    let l = Libs::load();
    let cases = [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        10,
        11,
        12,
        15,
        16,
        17,
        i32::MAX - 1,
        i32::MAX,
    ];
    for pfcn in cases {
        l.assert_same(pfcn);
    }
}

// ------------------------------------------------------------------- row 16
// 20k randomized values over the full i32 range, fixed seed.
#[test]
fn row16_random_full_i32_range() {
    let l = Libs::load();
    let mut rng = Rng::new(SEED);
    for _ in 0..20_000 {
        let pfcn = rng.next_i32();
        l.assert_same(pfcn);
    }
}

// ------------------------------------------------------------------- row 17
// 20k randomized values biased into -32..=47 so in-range hits are dense.
#[test]
fn row17_random_near_range() {
    let l = Libs::load();
    let mut rng = Rng::new(SEED);
    let mut in_range_hits = 0;
    for _ in 0..20_000 {
        let pfcn = rng.range(-32, 47);
        let v = l.assert_same(pfcn);
        if (0..=11).contains(&pfcn) {
            assert_eq!(v, 1);
            in_range_hits += 1;
        } else {
            assert_eq!(v, 0);
        }
    }
    assert!(in_range_hits > 1000, "expected dense in-range coverage, got {in_range_hits}");
}

// ------------------------------------------------------------------- row 18
// Repeated / interleaved / reversed invocation: no hidden per-call state.
#[test]
fn row18_repeated_and_reversed_order() {
    let l = Libs::load();
    let mut forward = Vec::new();
    for pfcn in -40..=60 {
        let a = l.assert_same(pfcn);
        let b = l.assert_same(pfcn); // called twice
        assert_eq!(a, b, "row 18: second call differs for pfcn={pfcn}");
        forward.push((pfcn, a));
    }
    for (pfcn, expect) in forward.iter().rev() {
        let v = l.assert_same(*pfcn);
        assert_eq!(v, *expect, "row 18: reversed order differs for pfcn={pfcn}");
    }
}

// ------------------------------------------------------------------- row 19
// dlopen / drop / dlopen again: no initialisation-order dependence.
#[test]
fn row19_reload_libraries() {
    let first: Vec<i32> = {
        let l = Libs::load();
        (-20..=30).map(|p| l.assert_same(p)).collect()
    }; // both libraries dropped (dlclose)

    let l = Libs::load();
    for (i, pfcn) in (-20..=30).enumerate() {
        let v = l.assert_same(pfcn);
        assert_eq!(v, first[i], "row 19: value changed after reload for pfcn={pfcn}");
    }
}

// ------------------------------------------------------------------- row 20
// Build-configuration row: the crate declares no [features], so the cargo
// profile is the only remaining build axis. Both the `debug` cdylib and the
// `release` cdylib (the shipped artifact, `panic = "abort"`) are loaded and
// compared against the same C library. This matters because `release` enables
// optimisations that could legally merge or fold the identity comparisons in
// `call_predict`.
#[test]
fn row20_both_build_profiles_full_contract() {
    for profile in ["debug", "release"] {
        let l = Libs::load_profile(profile);
        for pfcn in -100..=100 {
            let v = l.assert_same(pfcn);
            assert_eq!(
                v,
                i32::from((0..=11).contains(&pfcn)),
                "row 20 [{profile}]: pfcn={pfcn}"
            );
        }
        // Randomised sweep in this profile too.
        let mut rng = Rng::new(SEED);
        for _ in 0..5_000 {
            l.assert_same(rng.next_i32());
        }
        for _ in 0..5_000 {
            l.assert_same(rng.range(-40, 60));
        }
        eprintln!("row20: verified rust profile = {profile}");
    }
}

// Both Rust profiles must also agree with EACH OTHER (and hence with C).
#[test]
fn row20b_debug_and_release_agree() {
    let d = Libs::load_profile("debug");
    let r = Libs::load_profile("release");
    for pfcn in -2000..=2000 {
        let (c1, rd) = (d.c(pfcn), d.rust(pfcn));
        let (c2, rr) = (r.c(pfcn), r.rust(pfcn));
        assert_eq!(c1, c2, "C is non-deterministic at pfcn={pfcn}");
        assert_eq!(rd, rr, "debug/release Rust disagree at pfcn={pfcn}");
        assert_eq!(c1, rd, "C vs Rust disagree at pfcn={pfcn}");
    }
}

// ------------------------------------------------- structural: exactly 12 hits
// The set of pfcn values for which call_predict reports a match must be
// identical between C and Rust, and must be exactly {0..=11}.
#[test]
fn matching_set_is_identical() {
    let l = Libs::load();
    let mut c_set = Vec::new();
    let mut r_set = Vec::new();
    for pfcn in -5000..=5000 {
        if l.c(pfcn) != 0 {
            c_set.push(pfcn);
        }
        if l.rust(pfcn) != 0 {
            r_set.push(pfcn);
        }
    }
    assert_eq!(c_set, r_set, "matching sets differ between C and Rust");
    assert_eq!(c_set, (0..=11).collect::<Vec<i32>>());
}
