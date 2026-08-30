//! Differential tests: the C `libStaticAlias.so` versus the Rust
//! `libStaticAlias.so`, both loaded via `libloading` and called only through
//! their exported C ABI symbols.
//!
//! Ordered lowest-level API first (`static_alias`), then the higher-level
//! `driver` that builds on it, then the exported symbol table.

mod common;

use std::ffi::c_int;

use common::{
    compare_alias_sequence, compare_driver, compare_driver_sequence, compare_script, Pair, Step,
    Target,
};

// ===========================================================================
// Level 1: static_alias
// ===========================================================================

#[test]
fn static_alias_first_call_taking_if_branch() {
    // inner starts at 1, so any outer >= 1 takes the `if` branch.
    for v in [1, 2, 3, 7, 100, 4096] {
        compare_alias_sequence(&[v]);
    }
}

#[test]
fn static_alias_first_call_taking_else_branch() {
    // outer < 1 takes the `else` branch: *outer += inner, return outer.
    for v in [0, -1, -2, -50, -100000] {
        compare_alias_sequence(&[v]);
    }
}

#[test]
fn static_alias_branch_classification_matches() {
    // Explicitly check the branch each implementation takes, not just values.
    let pair = Pair::new();

    let mut a: c_int = 5;
    let c_ret = pair.c.static_alias(&mut a);
    let mut b: c_int = 5;
    let rust_ret = pair.rust.static_alias(&mut b);

    // 5 >= 1 -> `if` branch: returns &inner, which is not the caller's object.
    assert_ne!(c_ret, (&mut a) as *mut c_int, "C should return &inner");
    assert_ne!(rust_ret, (&mut b) as *mut c_int, "Rust should return &inner");
    assert_eq!(unsafe { *c_ret }, unsafe { *rust_ret });
    assert_eq!(unsafe { *c_ret }, 6);
    // The caller's object is untouched by the `if` branch.
    assert_eq!(a, 5);
    assert_eq!(b, 5);

    // inner is now 6; 0 < 6 -> `else` branch: returns the caller's object.
    let mut a2: c_int = 0;
    let c_ret2 = pair.c.static_alias(&mut a2);
    let mut b2: c_int = 0;
    let rust_ret2 = pair.rust.static_alias(&mut b2);
    assert_eq!(c_ret2, (&mut a2) as *mut c_int, "C should return outer");
    assert_eq!(rust_ret2, (&mut b2) as *mut c_int, "Rust should return outer");
    assert_eq!(a2, 6);
    assert_eq!(b2, 6);
}

#[test]
fn static_alias_static_state_persists_across_calls() {
    // Repeated equal inputs: each `if`-branch call grows inner, so the branch
    // eventually flips. The sequence pins down the exact flip point.
    compare_alias_sequence(&[3, 3, 3, 3, 3, 3]);
    compare_alias_sequence(&[1, 1, 1, 1, 1, 1, 1, 1]);
    compare_alias_sequence(&[10, 10, 10, 10]);
}

#[test]
fn static_alias_returned_internal_pointer_is_stable() {
    // Two `if`-branch calls must hand back the same address (the static), and
    // both implementations must agree on that.
    let mut c_prev = None;
    let mut rust_prev = None;
    let pair = Pair::new();

    let c1 = common::observe_alias(&pair.c, 4, &mut c_prev);
    let r1 = common::observe_alias(&pair.rust, 4, &mut rust_prev);
    assert_eq!(c1, r1);
    assert_eq!(c1.target, Target::Internal);
    assert_eq!(c1.ret_ptr_stable, None); // no previous pointer yet

    // inner == 5 now; feed 5 to take the `if` branch again.
    let c2 = common::observe_alias(&pair.c, 5, &mut c_prev);
    let r2 = common::observe_alias(&pair.rust, 5, &mut rust_prev);
    assert_eq!(c2, r2);
    assert_eq!(c2.target, Target::Internal);
    assert_eq!(c2.ret_ptr_stable, Some(true), "&inner must be stable");
}

#[test]
fn static_alias_mixed_sequences() {
    let sequences: &[&[c_int]] = &[
        &[0],
        &[-1, -1, -1],
        &[1, 0, 1, 0, 1, 0],
        &[5, -5, 5, -5, 5],
        &[2, 4, 8, 16, 32, 64],
        &[100, 1, 2, 3, 200, -200, 1000],
        &[-7, 13, -13, 7, 0, 1, -1],
        &[1000000, -1000000, 1000000, 0],
        &[0, 0, 0, 0, 0, 0, 0, 0],
        &[123456789, 123456789],
    ];
    for s in sequences {
        compare_alias_sequence(s);
    }
}

#[test]
fn static_alias_extreme_values() {
    // Signed overflow is UB in C; the reference build is the ground truth and
    // these cases confirm the Rust translation reproduces it.
    compare_alias_sequence(&[c_int::MAX]);
    compare_alias_sequence(&[c_int::MIN]);
    compare_alias_sequence(&[c_int::MAX, c_int::MAX]);
    compare_alias_sequence(&[c_int::MIN, c_int::MIN]);
    compare_alias_sequence(&[c_int::MAX, c_int::MIN, c_int::MAX, c_int::MIN]);
    compare_alias_sequence(&[c_int::MAX, 1, 0, -1, c_int::MIN, 1]);
    compare_alias_sequence(&[1, c_int::MAX, c_int::MAX, 1, -1]);
    compare_alias_sequence(&[c_int::MIN, c_int::MAX, 0]);
}

#[test]
fn static_alias_long_pseudorandom_sequence() {
    // Deterministic xorshift so failures are reproducible.
    let mut state: u32 = 0x1234_5678;
    let mut inputs = Vec::with_capacity(2000);
    for _ in 0..2000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        // Mix of small and full-range values.
        let v = if state % 3 == 0 {
            (state % 21) as i32 - 10
        } else {
            state as i32
        };
        inputs.push(v as c_int);
    }
    compare_alias_sequence(&inputs);
}

#[test]
fn static_alias_null_outer_is_not_dereferenced_by_either() {
    // Both implementations dereference `outer` unconditionally, so NULL is not
    // a valid input for either. Nothing to compare; documented here so the gap
    // in coverage is intentional rather than overlooked.
}

// ===========================================================================
// Level 2: driver (calls static_alias in a loop and prints)
// ===========================================================================

#[test]
fn driver_zero_and_negative_iterations_print_nothing() {
    for it in [0, -1, -100, c_int::MIN] {
        compare_driver(0, it);
        compare_driver(42, it);
        compare_driver(-42, it);
    }
}

#[test]
fn driver_single_iteration() {
    for iv in [0, 1, 2, -1, -2, 100, -100, c_int::MAX, c_int::MIN] {
        compare_driver(iv, 1);
    }
}

#[test]
fn driver_small_iteration_counts() {
    for iv in [-3, -1, 0, 1, 2, 5, 17] {
        for it in 1..=10 {
            compare_driver(iv, it);
        }
    }
}

#[test]
fn driver_doubling_until_overflow() {
    // With initial_value >= 1 the running sum latches onto `inner` and doubles
    // each iteration, so ~31 iterations walks it through signed overflow.
    for iv in [1, 2, 3, 7, 12345] {
        compare_driver(iv, 40);
    }
}

#[test]
fn driver_extreme_initial_values() {
    for iv in [c_int::MAX, c_int::MIN, c_int::MAX - 1, c_int::MIN + 1] {
        for it in [1, 2, 3, 5, 20] {
            compare_driver(iv, it);
        }
    }
}

#[test]
fn driver_many_iterations() {
    // Longer runs, still bounded so the captured output stays small.
    compare_driver(0, 500);
    compare_driver(-1, 500);
    compare_driver(1, 500);
    compare_driver(-100000, 1000);
}

#[test]
fn driver_shares_static_state_across_calls() {
    // Consecutive `driver` calls on the same loaded library see the same
    // `inner`, so later calls behave differently from the first.
    compare_driver_sequence(&[(5, 3), (5, 3), (5, 3)]);
    compare_driver_sequence(&[(1, 4), (0, 4), (-1, 4), (1000, 4)]);
    compare_driver_sequence(&[(0, 1), (0, 1), (0, 1), (0, 1), (0, 1)]);
    compare_driver_sequence(&[(c_int::MAX, 2), (1, 2), (c_int::MIN, 2), (0, 3)]);
    compare_driver_sequence(&[(3, 40), (3, 5), (-3, 5)]);
}

#[test]
fn driver_and_static_alias_share_the_same_static() {
    // Interleave the two entry points on one library instance; whatever `driver`
    // did to the shared static must be visible to the later `static_alias`
    // calls, and the printed bytes plus the observation lines must match.
    compare_script(&[
        Step::Driver(9, 3),
        Step::Alias(0),
        Step::Alias(1),
        Step::Alias(100),
        Step::Alias(-100),
        Step::Alias(72),
        Step::Alias(72),
        Step::Driver(72, 2),
        Step::Alias(0),
    ]);

    compare_script(&[
        Step::Alias(1),
        Step::Driver(1, 1),
        Step::Alias(2),
        Step::Driver(0, 3),
        Step::Alias(-1),
        Step::Driver(c_int::MAX, 2),
        Step::Alias(c_int::MIN),
        Step::Alias(c_int::MAX),
        Step::Driver(0, 1),
    ]);

    compare_script(&[
        Step::Driver(3, 40),
        Step::Alias(0),
        Step::Alias(0),
        Step::Driver(-1, 4),
        Step::Alias(c_int::MIN),
        Step::Driver(c_int::MIN, 4),
    ]);
}

// ===========================================================================
// Exported symbol table
// ===========================================================================

#[test]
fn rust_so_exports_every_symbol_the_c_so_exports() {
    let c_path = common::c_so_path();
    let rust_path = common::rust_so_path();

    let c_syms = common::nm_defined(&c_path);
    let rust_syms = common::nm_defined(&rust_path);

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C exports: {c_syms:?}"
    );

    // The documented public API must be present in both.
    for required in ["static_alias", "driver"] {
        assert!(
            c_syms.iter().any(|s| s == required),
            "C .so unexpectedly lacks `{required}`"
        );
        assert!(
            rust_syms.iter().any(|s| s == required),
            "Rust .so lacks `{required}`"
        );
    }
}

// ===========================================================================
// Randomised whole-API scripts
// ===========================================================================

/// Deterministic xorshift32 so any failure is reproducible.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// A value biased towards the interesting region (near zero and near the
    /// signed limits) rather than uniform over the whole range.
    fn value(&mut self) -> c_int {
        match self.next() % 8 {
            0..=2 => (self.next() % 41) as i32 - 20,
            3 => 0,
            4 => c_int::MAX - (self.next() % 4) as i32,
            5 => c_int::MIN + (self.next() % 4) as i32,
            _ => self.next() as i32,
        }
    }
}

#[test]
fn randomised_mixed_scripts() {
    for seed in [
        1u32,
        0xDEAD_BEEF,
        0x0BAD_F00D,
        7,
        0x5EED_1234,
        0xFFFF_FFFF,
        42,
        999_983,
    ] {
        let mut rng = Rng(seed);
        let steps: Vec<Step> = (0..60)
            .map(|_| {
                if rng.next() % 2 == 0 {
                    // Keep iteration counts small so scripts stay fast, but let
                    // negative and zero counts through.
                    let it = (rng.next() % 13) as i32 - 2;
                    Step::Driver(rng.value(), it)
                } else {
                    Step::Alias(rng.value())
                }
            })
            .collect();
        compare_script(&steps);
    }
}

#[test]
fn randomised_alias_only_scripts() {
    for seed in [3u32, 0xABCD_1234, 0x1357_9BDF, 0x2468_ACE0] {
        let mut rng = Rng(seed);
        let inputs: Vec<c_int> = (0..500).map(|_| rng.value()).collect();
        compare_alias_sequence(&inputs);
    }
}

#[test]
fn exhaustive_small_driver_grid() {
    // Every (initial_value, iterations) pair in a small window, each in a fresh
    // library so the first call always sees `inner == 1`.
    for iv in -6..=6 {
        for it in -2..=8 {
            compare_driver(iv, it);
        }
    }
}

#[test]
fn boundary_values_around_the_initial_static() {
    // `inner` starts at 1, so the comparison `*outer >= inner` flips between
    // 0 and 1 on the very first call.
    for v in [-1, 0, 1, 2] {
        compare_alias_sequence(&[v]);
        compare_driver(v, 3);
    }
}
