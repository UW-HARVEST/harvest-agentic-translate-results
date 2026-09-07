//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are reached only through `dlopen`ed `extern "C"` exports.
//! Every row uses many randomized inputs from a fixed-seed PRNG.

mod common;

use common::*;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Rows 1–14: the low-level entry point `static_alias`, driven directly.
// ---------------------------------------------------------------------------

/// Row 1 — fresh `inner == 1`, distinct local, then-branch (`*outer >= 1`).
#[test]
fn cfg_row01_fresh_then_branch_positive() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 1);
    for k in 0..N {
        let p = Pair::fresh();
        let v = rng.i32_in(1, i32::MAX);
        p.both(&format!("row01[{k}] v={v}"), |l| vec![l.call_local(v)]);
    }
}

/// Row 2 — fresh `inner == 1`, distinct local, else-branch (`*outer <= 0`).
#[test]
fn cfg_row02_fresh_else_branch_nonpositive() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 2);
    for k in 0..N {
        let p = Pair::fresh();
        let v = rng.i32_in(i32::MIN, 0);
        p.both(&format!("row02[{k}] v={v}"), |l| vec![l.call_local(v)]);
    }
}

/// Row 3 — predicate boundary `*outer == inner` exactly (== 1) on a fresh library.
#[test]
fn cfg_row03_predicate_equal_boundary() {
    let _g = gate();
    // Deterministic value, but repeated after randomized *preceding* history so
    // the boundary is hit from many different `inner` values as well.
    let mut rng = Rng::new(SEED ^ 3);
    {
        let p = Pair::fresh();
        p.both("row03 fresh inner==1, v==1", |l| vec![l.call_local(1)]);
    }
    for k in 0..N {
        let p = Pair::fresh();
        // Build up some `inner`, learn it, then hit `*outer == inner`.
        let warm = rng.i32_in(1, 1 << 20);
        let obs_c = p.c.call_local(warm);
        let obs_r = p.r.call_local(warm);
        assert_eq!(obs_c, obs_r, "row03[{k}] warm-up diverged");
        let inner_now = obs_c.ret_val; // then-branch returned &inner
        p.both(&format!("row03[{k}] inner={inner_now} v==inner"), |l| {
            vec![l.call_local(inner_now)]
        });
    }
}

/// Row 4 — predicate boundary `*outer == inner - 1` (last else-branch value).
#[test]
fn cfg_row04_predicate_one_below_boundary() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 4);
    {
        let p = Pair::fresh();
        p.both("row04 fresh inner==1, v==0", |l| vec![l.call_local(0)]);
    }
    for k in 0..N {
        let p = Pair::fresh();
        let warm = rng.i32_in(1, 1 << 20);
        let oc = p.c.call_local(warm);
        let or = p.r.call_local(warm);
        assert_eq!(oc, or, "row04[{k}] warm-up diverged");
        let v = oc.ret_val.wrapping_sub(1);
        p.both(&format!("row04[{k}] v=inner-1={v}"), |l| {
            vec![l.call_local(v)]
        });
    }
}

/// Row 5 — accumulated positive `inner`, distinct local, then-branch.
#[test]
fn cfg_row05_accumulated_inner_then_branch() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 5);
    for k in 0..N {
        let p = Pair::fresh();
        let warm = rng.i32_in(1, 1 << 24);
        let oc = p.c.call_local(warm);
        let or = p.r.call_local(warm);
        assert_eq!(oc, or, "row05[{k}] warm-up diverged");
        let inner = oc.ret_val;
        let v = rng.i32_in(inner as i64 as i32, i32::MAX);
        p.both(&format!("row05[{k}] inner={inner} v={v}"), |l| {
            vec![l.call_local(v)]
        });
    }
}

/// Row 6 — accumulated positive `inner`, distinct local, else-branch.
#[test]
fn cfg_row06_accumulated_inner_else_branch() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 6);
    for k in 0..N {
        let p = Pair::fresh();
        let warm = rng.i32_in(2, 1 << 24);
        let oc = p.c.call_local(warm);
        let or = p.r.call_local(warm);
        assert_eq!(oc, or, "row06[{k}] warm-up diverged");
        let inner = oc.ret_val;
        let v = rng.in_range(i32::MIN as i64, inner as i64 - 1) as i32;
        p.both(&format!("row06[{k}] inner={inner} v={v}"), |l| {
            vec![l.call_local(v)]
        });
    }
}

/// Drive `inner` negative: from `inner == 1`, a single then-branch call with a
/// sufficiently negative value is impossible (`v >= 1` required), so go through
/// the aliased doubling path into wraparound, or use `INT_MAX` overflow.
fn make_inner_negative(l: &Lib) -> Vec<Obs> {
    // inner = 1 -> then(INT_MAX): inner = INT_MIN (wrap) -> negative.
    vec![l.call_local(i32::MAX)]
}

/// Row 7 — negative `inner`, distinct local, then-branch (`*outer >= inner`).
#[test]
fn cfg_row07_negative_inner_then_branch() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 7);
    for k in 0..N {
        let p = Pair::fresh();
        let oc = make_inner_negative(&p.c);
        let or = make_inner_negative(&p.r);
        assert_eq!(oc, or, "row07[{k}] warm-up diverged");
        let inner = oc[0].ret_val;
        assert!(inner < 0, "row07: expected negative inner, got {inner}");
        // Any value >= inner takes the then-branch; sample across the whole range.
        let v = rng.in_range(inner as i64, i32::MAX as i64) as i32;
        p.both(&format!("row07[{k}] inner={inner} v={v}"), |l| {
            vec![l.call_local(v)]
        });
    }
}

/// Row 8 — negative `inner`, distinct local, else-branch (`*outer < inner`).
#[test]
fn cfg_row08_negative_inner_else_branch() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 8);
    for k in 0..N {
        let p = Pair::fresh();
        let oc = make_inner_negative(&p.c);
        let or = make_inner_negative(&p.r);
        assert_eq!(oc, or, "row08[{k}] warm-up diverged");
        let inner = oc[0].ret_val;
        if inner == i32::MIN {
            // Nothing is `< INT_MIN`; nudge `inner` up first, mirrored on both.
            let bump = rng.i32_in(1, 1 << 20);
            let a = p.c.call_local(i32::MIN + bump);
            let b = p.r.call_local(i32::MIN + bump);
            assert_eq!(a, b, "row08[{k}] bump diverged");
            let inner2 = Lib::inner_from_probe(p.c.probe_inner());
            let inner2b = Lib::inner_from_probe(p.r.probe_inner());
            assert_eq!(inner2, inner2b);
            if inner2 == i32::MIN {
                continue;
            }
            let v = rng.in_range(i32::MIN as i64, inner2 as i64 - 1) as i32;
            p.both(&format!("row08[{k}] inner={inner2} v={v}"), |l| {
                vec![l.call_local(v)]
            });
        } else {
            let v = rng.in_range(i32::MIN as i64, inner as i64 - 1) as i32;
            p.both(&format!("row08[{k}] inner={inner} v={v}"), |l| {
                vec![l.call_local(v)]
            });
        }
    }
}

/// Row 9 — `inner` near `INT_MAX`, else-branch, so `*outer += inner` overflows.
#[test]
fn cfg_row09_inner_near_intmax_else_overflow() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 9);
    for k in 0..N {
        let p = Pair::fresh();
        // inner = 1 + (INT_MAX - 1) = INT_MAX via a single then-branch call.
        let oc = p.c.call_local(i32::MAX - 1);
        let or = p.r.call_local(i32::MAX - 1);
        assert_eq!(oc, or, "row09[{k}] warm-up diverged");
        assert_eq!(oc.ret_val, i32::MAX, "row09: inner should be INT_MAX");
        let v = rng.i32_in(i32::MIN, i32::MAX - 1);
        p.both(&format!("row09[{k}] inner=INT_MAX v={v}"), |l| {
            vec![l.call_local(v)]
        });
    }
}

/// Row 10 — aliased argument (`outer == &inner`) on a fresh library: `inner` doubles.
#[test]
fn cfg_row10_aliased_single_call() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 10);
    for k in 0..N {
        let p = Pair::fresh();
        // Reveal &inner with a randomized then-branch call, then re-enter aliased.
        let warm = rng.i32_in(1, i32::MAX);
        let oc = p.c.call_local(warm);
        let or = p.r.call_local(warm);
        assert_eq!(oc, or, "row10[{k}] warm-up diverged");
        p.both(&format!("row10[{k}] warm={warm} aliased"), |l| {
            vec![l.call_aliased()]
        });
    }
}

/// Row 11 — aliased argument re-entered `2..=40` times: repeated doubling into wrap.
#[test]
fn cfg_row11_aliased_repeated_doubling() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 11);
    for k in 0..N {
        let p = Pair::fresh();
        let warm = rng.i32_in(1, i32::MAX);
        let oc = p.c.call_local(warm);
        let or = p.r.call_local(warm);
        assert_eq!(oc, or, "row11[{k}] warm-up diverged");
        let reps = rng.in_range(2, 40) as usize;
        p.both(&format!("row11[{k}] warm={warm} reps={reps}"), |l| {
            (0..reps).map(|_| l.call_aliased()).collect()
        });
    }
}

/// Row 12 — fully random `i32` sequences on fresh locals; the data picks the branch.
#[test]
fn cfg_row12_random_sequence_fresh_locals() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 12);
    for k in 0..N {
        let p = Pair::fresh();
        let len = rng.in_range(1, 64) as usize;
        let vals: Vec<i32> = (0..len).map(|_| rng.i32_any()).collect();
        let v2 = vals.clone();
        p.both(&format!("row12[{k}] len={len} vals={vals:?}"), move |l| {
            l.seq_fresh_locals(&v2)
        });
    }
}

/// Row 13 — the real consumer pattern at the low level: feed the returned pointer
/// straight back in (this is `driver` without the `printf`), random initial value
/// and length.
#[test]
fn cfg_row13_random_feedback_sequence() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 13);
    for k in 0..N {
        let p = Pair::fresh();
        let initial = rng.i32_any();
        let len = rng.in_range(1, 64) as usize;
        p.both(&format!("row13[{k}] initial={initial} len={len}"), |l| {
            l.feedback(initial, len)
        });
    }
}

/// Row 14 — the five extreme inputs, each on a freshly loaded pair.
#[test]
fn cfg_row14_extreme_values_fresh() {
    let _g = gate();
    for v in [i32::MIN, -1, 0, 1, i32::MAX] {
        let p = Pair::fresh();
        p.both(&format!("row14 v={v}"), |l| vec![l.call_local(v)]);
    }
    // Also every ordered pair of extremes on one library (state interaction).
    for a in [i32::MIN, -1, 0, 1, i32::MAX] {
        for b in [i32::MIN, -1, 0, 1, i32::MAX] {
            let p = Pair::fresh();
            p.both(&format!("row14 pair ({a},{b})"), |l| {
                vec![l.call_local(a), l.call_local(b)]
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 15–23: the `driver` wrapper, stdout compared byte-for-byte.
// ---------------------------------------------------------------------------

/// Row 15 — `iterations == 0`, random `initial_value`: no output at all.
#[test]
fn cfg_row15_driver_zero_iterations() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 15);
    for k in 0..N {
        let p = Pair::fresh();
        let v = rng.i32_any();
        p.both_driver(&format!("row15[{k}]"), v, 0);
        let out = capture_stdout(|| p.c.driver(v, 0));
        assert!(out.is_empty(), "row15[{k}]: expected no output, got {out:?}");
    }
}

/// Row 16 — random negative `iterations`: no output.
#[test]
fn cfg_row16_driver_negative_iterations() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 16);
    for k in 0..N {
        let p = Pair::fresh();
        let v = rng.i32_any();
        let n = rng.i32_in(i32::MIN, -1);
        p.both_driver(&format!("row16[{k}]"), v, n);
    }
}

/// Row 17 — `iterations == 1`, random positive `initial_value` (then-branch first).
#[test]
fn cfg_row17_driver_one_iteration_positive() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 17);
    for k in 0..N {
        let p = Pair::fresh();
        let v = rng.i32_in(1, i32::MAX);
        p.both_driver(&format!("row17[{k}]"), v, 1);
    }
}

/// Row 18 — `iterations == 1`, random non-positive `initial_value` (else-branch first).
#[test]
fn cfg_row18_driver_one_iteration_nonpositive() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 18);
    for k in 0..N {
        let p = Pair::fresh();
        let v = rng.i32_in(i32::MIN, 0);
        p.both_driver(&format!("row18[{k}]"), v, 1);
    }
}

/// Row 19 — `iterations == 2`, random `initial_value`: first state carry-over.
#[test]
fn cfg_row19_driver_two_iterations() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 19);
    for k in 0..N {
        let p = Pair::fresh();
        let v = rng.i32_any();
        p.both_driver(&format!("row19[{k}]"), v, 2);
    }
}

/// Row 20 — random `iterations` in `3..=64` with random `initial_value`.
#[test]
fn cfg_row20_driver_many_iterations_random() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 20);
    for k in 0..N {
        let p = Pair::fresh();
        let v = rng.i32_any();
        let n = rng.i32_in(3, 64);
        p.both_driver(&format!("row20[{k}]"), v, n);
    }
}

/// Row 21 — long run (4096 iterations) from a large positive initial value.
#[test]
fn cfg_row21_driver_long_run_positive() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 21);
    for k in 0..16 {
        let p = Pair::fresh();
        let v = rng.i32_in(1 << 20, i32::MAX);
        p.both_driver(&format!("row21[{k}]"), v, 4096);
    }
}

/// Row 22 — long run (4096 iterations) from `INT_MIN`.
#[test]
fn cfg_row22_driver_long_run_intmin() {
    let _g = gate();
    let p = Pair::fresh();
    p.both_driver("row22 INT_MIN x4096", i32::MIN, 4096);
}

/// Row 23 — full cross-product of extreme `initial_value` × extreme `iterations`.
#[test]
fn cfg_row23_driver_extremes_cross_product() {
    let _g = gate();
    for v in [i32::MIN, -1, 0, 1, i32::MAX] {
        for n in [0, -1, 1, 2, 5] {
            let p = Pair::fresh();
            p.both_driver(&format!("row23 v={v} n={n}"), v, n);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 24–26: interaction of the two entry points through the shared static.
// ---------------------------------------------------------------------------

/// Row 24 — `driver` first, then direct `static_alias` calls on the same library.
#[test]
fn cfg_row24_driver_then_static_alias() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 24);
    for k in 0..N {
        let p = Pair::fresh();
        let v = rng.i32_any();
        let n = rng.i32_in(1, 20);
        p.both_driver(&format!("row24[{k}] driver"), v, n);
        let len = rng.in_range(1, 8) as usize;
        let vals: Vec<i32> = (0..len).map(|_| rng.i32_any()).collect();
        let v2 = vals.clone();
        p.both(&format!("row24[{k}] then sa {vals:?}"), move |l| {
            l.seq_fresh_locals(&v2)
        });
    }
}

/// Row 25 — direct `static_alias` calls first, then `driver` (stdout compared).
#[test]
fn cfg_row25_static_alias_then_driver() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 25);
    for k in 0..N {
        let p = Pair::fresh();
        let len = rng.in_range(1, 8) as usize;
        let vals: Vec<i32> = (0..len).map(|_| rng.i32_any()).collect();
        let v2 = vals.clone();
        p.both(&format!("row25[{k}] sa {vals:?}"), move |l| {
            l.seq_fresh_locals(&v2)
        });
        let v = rng.i32_any();
        let n = rng.i32_in(1, 20);
        p.both_driver(&format!("row25[{k}] then driver({v},{n})"), v, n);
    }
}

/// Row 26 — randomized interleaving of both entry points on one library instance.
#[test]
fn cfg_row26_random_interleaving() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 26);
    for k in 0..N {
        let p = Pair::fresh();
        let ops = rng.in_range(1, 32) as usize;
        for op in 0..ops {
            match rng.next_u64() % 4 {
                0 => {
                    let v = rng.i32_any();
                    p.both(&format!("row26[{k}].{op} sa_local({v})"), |l| {
                        vec![l.call_local(v)]
                    });
                }
                1 => {
                    let initial = rng.i32_any();
                    let len = rng.in_range(1, 6) as usize;
                    p.both(
                        &format!("row26[{k}].{op} feedback({initial},{len})"),
                        |l| l.feedback(initial, len),
                    );
                }
                2 => {
                    let v = rng.i32_any();
                    let n = rng.i32_in(-2, 12);
                    p.both_driver(&format!("row26[{k}].{op} driver({v},{n})"), v, n);
                }
                _ => {
                    // Aliased re-entry, only once &inner has been revealed.
                    let known_c = p.c.call_local(i32::MAX).ret_is_internal;
                    let known_r = p.r.call_local(i32::MAX).ret_is_internal;
                    assert_eq!(known_c, known_r, "row26[{k}].{op} reveal diverged");
                    if known_c {
                        let reps = rng.in_range(1, 5) as usize;
                        p.both(&format!("row26[{k}].{op} aliased x{reps}"), |l| {
                            (0..reps).map(|_| l.call_aliased()).collect()
                        });
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Binary-executable comparison
// ---------------------------------------------------------------------------

/// The project builds **no** binary/driver executable: `c_src/CMakeLists.txt`
/// contains only `add_library(StaticAlias SHARED …)` and
/// `translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]`.
///
/// The nearest equivalent — the `driver` entry point, which is what a driver
/// executable would call, and whose entire observable output is stdout — is
/// compared byte-for-byte in rows 15–26 above. This test asserts the "no binary
/// target" premise so it cannot silently become stale.
#[test]
fn no_binary_target_premise_holds() {
    let root = env!("CARGO_MANIFEST_DIR");
    let cmake = std::fs::read_to_string(format!("{root}/../c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "c_src/CMakeLists.txt now builds an executable; add a stdout diff for it"
    );
    let cargo = std::fs::read_to_string(format!("{root}/Cargo.toml")).unwrap();
    assert!(
        !cargo.contains("[[bin]]"),
        "translation/Cargo.toml now builds a binary; add a stdout diff for it"
    );
    assert!(!std::path::Path::new(&format!("{root}/src/main.rs")).exists());
    assert!(!std::path::Path::new(&format!("{root}/src/bin")).exists());
}

/// Sanity: the two `.so`s really are two distinct objects with distinct `&inner`,
/// so a "pass" can never be an artefact of accidentally testing one library twice.
#[test]
fn libraries_are_distinct_objects() {
    let _g = gate();
    let p = Pair::fresh();
    let a = p.c.call_local(7);
    let b = p.r.call_local(7);
    assert_eq!(a, b);
    let mut lc: c_int = 5;
    let mut lr: c_int = 5;
    let rc = unsafe { (p.c.sa)(&mut lc) };
    let rr = unsafe { (p.r.sa)(&mut lr) };
    assert_ne!(rc, rr, "C and Rust returned the same &inner address");
}
