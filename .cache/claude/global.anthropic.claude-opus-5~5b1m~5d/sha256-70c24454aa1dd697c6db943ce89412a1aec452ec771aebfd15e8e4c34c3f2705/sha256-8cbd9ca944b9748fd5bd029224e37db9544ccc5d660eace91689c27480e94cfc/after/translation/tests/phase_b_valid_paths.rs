//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` through `libloading`
//! and calls only their exported C symbols. Randomised rows use a fixed seed.
//!
//! Rows start at the lowest-level entry point (`static_alias`, rows 1–12), then
//! move up to the composed wrapper (`driver`, rows 13–19, 23), then cover the
//! interaction of the two on shared static state (rows 20–22, 24).

mod common;

use common::{Obs, Pair, RetClass, Rng, EXTREMES};

const ITERS: usize = 500;

// ===========================================================================
// Row 1 — fresh state, single call, `*outer` over the whole i32 range.
// ===========================================================================
#[test]
fn row_01_single_call_full_i32_range() {
    let mut rng = Rng::new(0x5EED_0001);
    for i in 0..ITERS {
        // A brand-new mapping per iteration, so every call sees `inner == 1`.
        let p = Pair::fresh();
        let v = rng.next_i32();
        p.assert_static_alias(v, &format!("row1 #{i}"));
    }
}

// ===========================================================================
// Row 2 — `*outer == 1`, exactly equal to the initial `inner` (`>=` boundary).
// ===========================================================================
#[test]
fn row_02_equal_to_initial_inner() {
    let p = Pair::fresh();
    let obs = p.assert_static_alias(1, "row2");
    // `1 >= 1` takes the then-branch: inner becomes 2 and its address is
    // returned. Pin the exact C-derived expectation.
    assert_eq!(obs.class, RetClass::LibStatic, "row2 branch");
    assert_eq!(obs.ret_val, 2, "row2 inner after");
    assert_eq!(obs.outer_after, 1, "row2 caller cell untouched");
}

// ===========================================================================
// Row 3 — `*outer == 0`, one below `inner` (else-branch).
// ===========================================================================
#[test]
fn row_03_one_below_initial_inner() {
    let p = Pair::fresh();
    let obs = p.assert_static_alias(0, "row3");
    assert_eq!(obs.class, RetClass::Arg, "row3 branch");
    assert_eq!(obs.ret_val, 1, "row3 *outer after");
    assert_eq!(obs.outer_after, 1, "row3 caller cell mutated");
}

// ===========================================================================
// Row 4 — random negatives: always the else-branch, always returns the arg.
// ===========================================================================
#[test]
fn row_04_negative_values_else_branch() {
    let mut rng = Rng::new(0x5EED_0004);
    for i in 0..ITERS {
        let p = Pair::fresh();
        let v = rng.range_i32(i32::MIN, -1);
        let obs = p.assert_static_alias(v, &format!("row4 #{i}"));
        assert_eq!(
            obs.class,
            RetClass::Arg,
            "row4 #{i}: v = {v} is below inner = 1 so C returns `outer`"
        );
    }
}

// ===========================================================================
// Row 5 — random large positives: then-branch, returns the library static.
// ===========================================================================
#[test]
fn row_05_large_positive_then_branch() {
    let mut rng = Rng::new(0x5EED_0005);
    for i in 0..ITERS {
        let p = Pair::fresh();
        let v = rng.range_i32(1, i32::MAX);
        let obs = p.assert_static_alias(v, &format!("row5 #{i}"));
        assert_eq!(
            obs.class,
            RetClass::LibStatic,
            "row5 #{i}: v = {v} is >= inner = 1 so C returns `&inner`"
        );
    }
}

// ===========================================================================
// Row 6 — the i32 corner values, exhaustively.
// ===========================================================================
#[test]
fn row_06_extreme_values_exhaustive() {
    for v in EXTREMES {
        let p = Pair::fresh();
        p.assert_static_alias(v, &format!("row6 v={v}"));
    }
}

// ===========================================================================
// Row 7 — two calls, the second re-passing the pointer the first returned.
// ===========================================================================
#[test]
fn row_07_two_calls_refeeding_returned_pointer() {
    let mut rng = Rng::new(0x5EED_0007);
    for i in 0..ITERS {
        let p = Pair::fresh();
        let v = rng.next_i32();
        let ctx = format!("row7 #{i} v={v}");
        assert_eq!(
            chain(&p.c, v, 2),
            chain(&p.rust, v, 2),
            "divergence [{ctx}] over a 2-call re-feeding chain"
        );
    }
}

// ===========================================================================
// Row 8 — 64 calls, always re-feeding the returned pointer (driver by hand).
// ===========================================================================
#[test]
fn row_08_many_calls_refeeding_returned_pointer() {
    let mut rng = Rng::new(0x5EED_0008);
    for i in 0..ITERS {
        let p = Pair::fresh();
        let v = rng.next_i32();
        assert_eq!(
            chain(&p.c, v, 64),
            chain(&p.rust, v, 64),
            "divergence [row8 #{i} v={v}] over a 64-call re-feeding chain"
        );
    }
}

/// Reproduce `driver`'s pointer chain at the low level: seed a caller-owned
/// cell, then feed each call's *return value* into the next call.
fn chain(lib: &common::Lib, initial: i32, n: usize) -> Vec<(RetClass, i32)> {
    let mut cell: i32 = initial;
    let mut cur: *mut i32 = &mut cell as *mut i32;
    let own: *mut i32 = cur;
    let mut trace = Vec::with_capacity(n);
    for _ in 0..n {
        let (_, val, ret) = lib.call_raw(cur);
        // Classify against the caller's own cell, which is the only address the
        // two libraries can share a meaning for.
        let class = if ret == own {
            RetClass::Arg
        } else {
            RetClass::LibStatic
        };
        trace.push((class, val));
        cur = ret;
    }
    trace
}

// ===========================================================================
// Row 9 — 64 calls, each with a *fresh* caller-owned cell: `inner` climbs while
// `*outer` does not, so the branch condition keeps changing.
// ===========================================================================
#[test]
fn row_09_many_calls_fresh_caller_cell() {
    let mut rng = Rng::new(0x5EED_0009);
    for i in 0..ITERS {
        let p = Pair::fresh();
        let values: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();
        for (k, &v) in values.iter().enumerate() {
            p.assert_static_alias(v, &format!("row9 #{i} call {k}"));
        }
    }
}

// ===========================================================================
// Row 10 — alternate between the returned pointer and a fresh random cell.
// ===========================================================================
#[test]
fn row_10_alternating_alias_shapes() {
    let mut rng = Rng::new(0x5EED_000A);
    for i in 0..ITERS {
        let p = Pair::fresh();
        // Pre-draw the script so both libraries are driven identically.
        let script: Vec<(bool, i32)> = (0..48).map(|_| (rng.bool(), rng.next_i32())).collect();
        let c_trace = run_script(&p.c, &script);
        let r_trace = run_script(&p.rust, &script);
        assert_eq!(
            c_trace, r_trace,
            "divergence [row10 #{i}] on alternating alias shapes; script = {script:?}"
        );
    }
}

/// `use_returned == true` -> re-pass the previous return value;
/// `use_returned == false` -> pass a fresh caller-owned cell holding `value`.
fn run_script(lib: &common::Lib, script: &[(bool, i32)]) -> Vec<(RetClass, i32, i32)> {
    let mut own: i32 = 0;
    let own_ptr: *mut i32 = &mut own as *mut i32;
    let mut prev: *mut i32 = own_ptr;
    let mut trace = Vec::with_capacity(script.len());
    for &(use_returned, value) in script {
        let arg = if use_returned {
            prev
        } else {
            // SAFETY: `own_ptr` refers to the live local `own`.
            unsafe { *own_ptr = value };
            own_ptr
        };
        let (_, val, ret) = lib.call_raw(arg);
        let class = if ret == own_ptr {
            RetClass::Arg
        } else {
            RetClass::LibStatic
        };
        // SAFETY: `own_ptr` is live for the whole call.
        trace.push((class, val, unsafe { *own_ptr }));
        prev = ret;
    }
    trace
}

// ===========================================================================
// Row 11 — advance `inner` a long way first, then random inputs (else-branch
// dominates because `inner` is now huge).
// ===========================================================================
#[test]
fn row_11_advanced_state_then_random() {
    let mut rng = Rng::new(0x5EED_000B);
    for i in 0..200 {
        let p = Pair::fresh();
        // One big positive call pushes `inner` up.
        let seed_val = rng.range_i32(1 << 20, i32::MAX);
        p.assert_static_alias(seed_val, &format!("row11 #{i} seed"));
        for k in 0..16 {
            let v = rng.next_i32();
            p.assert_static_alias(v, &format!("row11 #{i} call {k} (inner advanced)"));
        }
    }
}

// ===========================================================================
// Row 12 — the returned `&inner` is the *same* address every time, and is never
// the caller's pointer.
// ===========================================================================
#[test]
fn row_12_returned_static_pointer_is_stable() {
    let p = Pair::fresh();
    for (name, lib) in [("C", &p.c), ("Rust", &p.rust)] {
        let mut cell: i32 = 5; // >= inner, so the then-branch returns &inner
        let own: *mut i32 = &mut cell as *mut i32;
        let (class1, _, ret1) = lib.call_raw(own);
        assert_eq!(class1, RetClass::LibStatic, "{name}: first call branch");
        assert_ne!(ret1, own, "{name}: &inner must differ from the argument");

        // Feed the static's own address back in repeatedly; the address must
        // never move.
        let mut ret = ret1;
        for k in 0..10 {
            let (_, _, next) = lib.call_raw(ret);
            assert_eq!(next, ret1, "{name}: &inner moved on call {k}");
            ret = next;
        }
    }
}

// ===========================================================================
// Row 13 — driver with iterations == 0 (empty output).
// ===========================================================================
#[test]
fn row_13_driver_zero_iterations() {
    let mut rng = Rng::new(0x5EED_000D);
    for i in 0..100 {
        let p = Pair::fresh();
        let v = rng.next_i32();
        let out = p.assert_driver(v, 0, &format!("row13 #{i}"));
        assert!(out.is_empty(), "row13 #{i}: expected no output, got {out:?}");
    }
}

// ===========================================================================
// Row 14 — driver with iterations == 1.
// ===========================================================================
#[test]
fn row_14_driver_one_iteration() {
    let mut rng = Rng::new(0x5EED_000E);
    for i in 0..200 {
        let p = Pair::fresh();
        let v = rng.next_i32();
        let out = p.assert_driver(v, 1, &format!("row14 #{i}"));
        assert_eq!(
            out.iter().filter(|&&b| b == b'\n').count(),
            1,
            "row14 #{i}: exactly one line expected"
        );
    }
}

// ===========================================================================
// Row 15 — driver with iterations == 2 (the branch can flip between calls).
// ===========================================================================
#[test]
fn row_15_driver_two_iterations() {
    let mut rng = Rng::new(0x5EED_000F);
    for i in 0..200 {
        let p = Pair::fresh();
        let v = rng.next_i32();
        p.assert_driver(v, 2, &format!("row15 #{i}"));
    }
}

// ===========================================================================
// Row 16 — initial_value >= 1: then-branch first, running_sum latches onto
// `&inner` and doubles (overflowing well before 24 iterations).
// ===========================================================================
#[test]
fn row_16_driver_positive_initial_doubling() {
    let mut rng = Rng::new(0x5EED_0010);
    for i in 0..200 {
        let p = Pair::fresh();
        let v = rng.range_i32(1, i32::MAX);
        let n = rng.range_i32(1, 24);
        p.assert_driver(v, n, &format!("row16 #{i}"));
    }
}

// ===========================================================================
// Row 17 — initial_value < 1: else-branch first, the stack slot climbs by
// `inner` until it catches up and the branch flips.
// ===========================================================================
#[test]
fn row_17_driver_negative_initial_flip() {
    let mut rng = Rng::new(0x5EED_0011);
    for i in 0..200 {
        let p = Pair::fresh();
        let v = rng.range_i32(i32::MIN, 0);
        let n = rng.range_i32(1, 24);
        p.assert_driver(v, n, &format!("row17 #{i}"));
    }
}

// ===========================================================================
// Row 18 — small negative start, enough iterations to cross the flip point.
// ===========================================================================
#[test]
fn row_18_driver_small_negative_many_iterations() {
    let mut rng = Rng::new(0x5EED_0012);
    for i in 0..100 {
        let p = Pair::fresh();
        let v = rng.range_i32(-64, 0);
        let n = rng.range_i32(25, 200);
        p.assert_driver(v, n, &format!("row18 #{i}"));
    }
}

// ===========================================================================
// Row 19 — cross-product of extreme initial_value and iteration counts.
// ===========================================================================
#[test]
fn row_19_driver_extremes_cross_product() {
    for v in [i32::MIN, i32::MIN + 1, -1, 0, 1, 2, i32::MAX - 1, i32::MAX] {
        for n in [1, 2, 3, 8, 40] {
            let p = Pair::fresh();
            p.assert_driver(v, n, &format!("row19 v={v} n={n}"));
        }
    }
}

// ===========================================================================
// Row 20 — static_alias calls first, then driver on the advanced state.
// ===========================================================================
#[test]
fn row_20_static_alias_then_driver() {
    let mut rng = Rng::new(0x5EED_0014);
    for i in 0..150 {
        let p = Pair::fresh();
        let n_pre = rng.below(6) as usize;
        for k in 0..n_pre {
            let v = rng.next_i32();
            p.assert_static_alias(v, &format!("row20 #{i} pre {k}"));
        }
        let v = rng.next_i32();
        let n = rng.range_i32(0, 20);
        p.assert_driver(v, n, &format!("row20 #{i} driver after {n_pre} pre-calls"));
    }
}

// ===========================================================================
// Row 21 — driver first, then static_alias observes the carried-over `inner`.
// ===========================================================================
#[test]
fn row_21_driver_then_static_alias() {
    let mut rng = Rng::new(0x5EED_0015);
    for i in 0..150 {
        let p = Pair::fresh();
        let v = rng.next_i32();
        let n = rng.range_i32(0, 20);
        p.assert_driver(v, n, &format!("row21 #{i} driver"));
        for k in 0..8 {
            let w = rng.next_i32();
            p.assert_static_alias(w, &format!("row21 #{i} post {k}"));
        }
    }
}

// ===========================================================================
// Row 22 — randomised interleaving of both entry points on shared state.
// ===========================================================================
#[test]
fn row_22_interleaved_entry_points() {
    let mut rng = Rng::new(0x5EED_0016);
    for i in 0..150 {
        let p = Pair::fresh();
        for step in 0..12 {
            if rng.bool() {
                let v = rng.next_i32();
                p.assert_static_alias(v, &format!("row22 #{i} step {step} static_alias"));
            } else {
                let v = rng.next_i32();
                let n = rng.range_i32(0, 6);
                p.assert_driver(v, n, &format!("row22 #{i} step {step} driver"));
            }
        }
    }
}

// ===========================================================================
// Row 23 — long output stream: buffering/flush parity of the printf path.
// ===========================================================================
#[test]
fn row_23_driver_large_iteration_count() {
    for v in [-5000i32, 0, 1, 7] {
        let p = Pair::fresh();
        let out = p.assert_driver(v, 4096, &format!("row23 v={v}"));
        assert_eq!(
            out.iter().filter(|&&b| b == b'\n').count(),
            4096,
            "row23 v={v}: one line per iteration"
        );
    }
}

// ===========================================================================
// Row 24 — state persistence within one load: back-to-back driver calls.
// ===========================================================================
#[test]
fn row_24_state_persists_across_driver_calls() {
    let mut rng = Rng::new(0x5EED_0018);
    for i in 0..150 {
        let p = Pair::fresh();
        let (v1, n1) = (rng.next_i32(), rng.range_i32(1, 12));
        let (v2, n2) = (rng.next_i32(), rng.range_i32(1, 12));
        let _first = p.assert_driver(v1, n1, &format!("row24 #{i} first"));
        let second = p.assert_driver(v2, n2, &format!("row24 #{i} second"));

        // Prove the second call really observed carried-over state rather than a
        // virgin `inner`: replay exactly the second call on a fresh pair. Both
        // libraries must agree there too, AND -- whenever the two differ -- the
        // difference must be identical for C and Rust, i.e. the state carries
        // over the same way in both.
        let virgin = Pair::fresh();
        let replayed = virgin.assert_driver(v2, n2, &format!("row24 #{i} virgin replay"));
        // `assert_driver` already proved C == Rust for both the carried-over run
        // and the virgin replay. If the two byte streams differ, the static
        // state demonstrably influenced the result in *both* libraries the same
        // way; tally those cases so the row cannot pass vacuously.
        if second != replayed {
            CARRIED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    assert!(
        CARRIED.load(std::sync::atomic::Ordering::Relaxed) > 0,
        "row24: no case showed state carrying over between driver calls -- the \
         test would be vacuous"
    );
}

static CARRIED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// ===========================================================================
// Extra: the `inner` static really is per-mapping, so `Pair::fresh()` gives a
// virgin state. If this ever failed, every "fresh state" row above would be
// silently testing something else.
// ===========================================================================
#[test]
fn harness_fresh_pair_has_virgin_inner() {
    for i in 0..5 {
        let p = Pair::fresh();
        let obs = p.assert_static_alias(1, &format!("virgin check #{i}"));
        assert_eq!(
            obs,
            Obs {
                class: RetClass::LibStatic,
                ret_val: 2,
                outer_after: 1
            },
            "Pair::fresh() #{i} did not start from inner == 1"
        );
    }
}
