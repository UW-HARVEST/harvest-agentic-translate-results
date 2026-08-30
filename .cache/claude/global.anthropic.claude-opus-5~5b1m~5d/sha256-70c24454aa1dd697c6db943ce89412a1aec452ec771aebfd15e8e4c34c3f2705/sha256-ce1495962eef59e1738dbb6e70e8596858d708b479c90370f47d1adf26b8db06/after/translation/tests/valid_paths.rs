//! Phase B -- valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` in the same configuration and compares every
//! observable effect: the returned value, the *identity* of the returned
//! pointer, the caller's variable afterwards, the library's private `inner`
//! afterwards, and -- for `driver` -- the exact stdout bytes.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Row 28 -- the as-loaded initial state: `static int inner = 1;`
//
// Every other row presets `inner` explicitly, so the initialiser baked into the
// `.so`'s data segment would otherwise never be compared. This row reads it out
// of both libraries exactly as they were dlopen'ed.
// ---------------------------------------------------------------------------

#[test]
fn cfg_28_as_loaded_initial_state() {
    let _g = lock();
    let l = libs();
    assert_eq!(
        l.c.inner_at_load, l.rust.inner_at_load,
        "the `static int inner` initialiser differs: C={} Rust={}",
        l.c.inner_at_load, l.rust.inner_at_load
    );
    assert_eq!(
        l.c.inner_at_load, INNER_INITIAL,
        "the C library is expected to start from `inner == 1`"
    );
    // Behavioural corollary, checked without ever presetting the state: on a
    // freshly loaded library `driver(1, 3)` must double from 1.
    for lib in [&l.c, &l.rust] {
        set_inner(lib, lib.inner_at_load);
        let out = capture_stdout(lib.name, || unsafe { (lib.driver)(1, 3) });
        assert_eq!(
            out, b"2\n4\n8\n",
            "{}: fresh-state driver(1,3) output",
            lib.name
        );
    }
}

// ---------------------------------------------------------------------------
// Row 1 -- lowest-level entry point, fresh state, randomized input
// ---------------------------------------------------------------------------

#[test]
fn cfg_01_static_alias_fresh_state_random() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 1);
    // Fresh state is `inner == 1`, exactly as the library is loaded.
    let mut saw_if = false;
    let mut saw_else = false;
    for _ in 0..2000 {
        let outer = rng.int_biased();
        let obs = assert_alias_eq("cfg-01", INNER_INITIAL, outer);
        if obs.ret_is_inner {
            saw_if = true;
        } else {
            saw_else = true;
        }
    }
    assert!(saw_if && saw_else, "row 1 must exercise both arms");
}

// ---------------------------------------------------------------------------
// Row 2 -- `*outer > inner` (strict) => `if` arm
// ---------------------------------------------------------------------------

#[test]
fn cfg_02_strictly_greater_takes_if_arm() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..2000 {
        let inner = rng.int_in(c_int::MIN, c_int::MAX - 1);
        let outer = rng.int_in(inner + 1, c_int::MAX);
        let obs = assert_alias_eq("cfg-02", inner, outer);
        assert!(obs.ret_is_inner && !obs.ret_is_outer, "expected &inner");
        assert_eq!(obs.outer_after, outer, "`if` arm must not touch *outer");
        assert_eq!(obs.inner_after, inner.wrapping_add(outer));
    }
}

// ---------------------------------------------------------------------------
// Row 3 -- `*outer == inner` (the `>=` equality boundary) => `if` arm
// ---------------------------------------------------------------------------

#[test]
fn cfg_03_equality_boundary_takes_if_arm() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..1000 {
        let v = rng.int_biased();
        let obs = assert_alias_eq("cfg-03", v, v);
        assert!(obs.ret_is_inner, "`>=` must include equality");
        assert_eq!(obs.inner_after, v.wrapping_add(v));
    }
    for &v in BOUNDARIES.iter() {
        assert_alias_eq("cfg-03", v, v);
    }
}

// ---------------------------------------------------------------------------
// Row 4 -- `*outer < inner` => `else` arm, caller's pointer returned
// ---------------------------------------------------------------------------

#[test]
fn cfg_04_strictly_less_takes_else_arm() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..2000 {
        let inner = rng.int_in(c_int::MIN + 1, c_int::MAX);
        let outer = rng.int_in(c_int::MIN, inner - 1);
        let obs = assert_alias_eq("cfg-04", inner, outer);
        assert!(obs.ret_is_outer && !obs.ret_is_inner, "expected the caller's pointer");
        assert_eq!(obs.inner_after, inner, "`else` arm must not touch inner");
        assert_eq!(obs.outer_after, outer.wrapping_add(inner));
    }
}

// ---------------------------------------------------------------------------
// Row 5 -- one step either side of the branch boundary
// ---------------------------------------------------------------------------

#[test]
fn cfg_05_one_step_past_branch_boundary() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..2000 {
        let inner = rng.int_biased();
        assert_alias_eq("cfg-05", inner, inner.wrapping_sub(1));
        assert_alias_eq("cfg-05", inner, inner.wrapping_add(1));
    }
}

// ---------------------------------------------------------------------------
// Row 6 -- `inner == 0`
// ---------------------------------------------------------------------------

#[test]
fn cfg_06_inner_zero() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..2000 {
        assert_alias_eq("cfg-06", 0, rng.int_biased());
    }
    for &v in BOUNDARIES.iter() {
        assert_alias_eq("cfg-06", 0, v);
    }
}

// ---------------------------------------------------------------------------
// Rows 7-9 -- sign combinations of `inner` and `*outer`
// ---------------------------------------------------------------------------

#[test]
fn cfg_07_inner_negative_outer_negative() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..2000 {
        let inner = rng.int_in(c_int::MIN, -1);
        let outer = rng.int_in(c_int::MIN, -1);
        assert_alias_eq("cfg-07", inner, outer);
    }
}

#[test]
fn cfg_08_inner_positive_outer_negative() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..2000 {
        let inner = rng.int_in(1, c_int::MAX);
        let outer = rng.int_in(c_int::MIN, -1);
        let obs = assert_alias_eq("cfg-08", inner, outer);
        assert!(obs.ret_is_outer, "negative < positive must take the else arm");
    }
}

#[test]
fn cfg_09_inner_negative_outer_positive() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..2000 {
        let inner = rng.int_in(c_int::MIN, -1);
        let outer = rng.int_in(0, c_int::MAX);
        let obs = assert_alias_eq("cfg-09", inner, outer);
        assert!(obs.ret_is_inner, "positive >= negative must take the if arm");
    }
}

// ---------------------------------------------------------------------------
// Row 10 -- `inner` at INT_MAX / INT_MIN (wrap-around in both arms)
// ---------------------------------------------------------------------------

#[test]
fn cfg_10_inner_at_extremes() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..2000 {
        let outer = rng.int_biased();
        assert_alias_eq("cfg-10", c_int::MAX, outer);
        assert_alias_eq("cfg-10", c_int::MIN, outer);
    }
}

// ---------------------------------------------------------------------------
// Row 11 -- `*outer` at the interesting fixed values, `inner` random
// ---------------------------------------------------------------------------

#[test]
fn cfg_11_outer_at_extremes() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..1000 {
        let inner = rng.int_biased();
        for &outer in &[c_int::MAX, c_int::MIN, 0, 1, -1] {
            assert_alias_eq("cfg-11", inner, outer);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 -- exhaustive 8x8 boundary cross-product
// ---------------------------------------------------------------------------

#[test]
fn cfg_12_boundary_cross_product() {
    let _g = lock();
    for &inner in BOUNDARIES.iter() {
        for &outer in BOUNDARIES.iter() {
            assert_alias_eq("cfg-12", inner, outer);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 -- self-aliasing: feed `&inner` back in (what `driver` does)
// ---------------------------------------------------------------------------

#[test]
fn cfg_13_self_aliasing_chain() {
    let _g = lock();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..300 {
        let inner = rng.int_biased();
        let steps = 5 + (rng.next_u64() % 36) as usize;
        // Start the chain directly on `&inner` (strict self-alias).
        let run = |lib: &Lib| -> Vec<AliasObs> {
            set_inner(lib, inner);
            let mut cur: *mut c_int = lib.inner_addr;
            let mut out = Vec::new();
            for _ in 0..steps {
                let ret = unsafe { (lib.static_alias)(cur) };
                out.push(AliasObs {
                    ret_is_inner: ret == lib.inner_addr,
                    ret_is_outer: ret == cur,
                    ret_val: unsafe { *ret },
                    outer_after: get_inner(lib),
                    inner_after: get_inner(lib),
                });
                cur = ret;
            }
            out
        };
        let got_c = run(&l.c);
        let got_rust = run(&l.rust);
        assert_eq!(
            got_c, got_rust,
            "[cfg-13] self-alias divergence for inner={inner}, steps={steps}"
        );
        for (i, o) in got_c.iter().enumerate() {
            assert!(
                o.ret_is_inner && o.ret_is_outer,
                "[cfg-13] step {i}: a self-alias must stay pinned in the if arm"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 -- state persistence across a long randomized call sequence
// ---------------------------------------------------------------------------

#[test]
fn cfg_14_state_persists_across_calls() {
    let _g = lock();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..50 {
        let inner0 = rng.int_biased();
        let inputs: Vec<c_int> = (0..256).map(|_| rng.int_biased()).collect();
        // No reset between the calls: `inner` carries over, so each step's
        // behaviour depends on all previous steps.
        let run = |lib: &Lib| -> Vec<AliasObs> {
            set_inner(lib, inner0);
            inputs
                .iter()
                .map(|&v| {
                    let mut outer = v;
                    let p: *mut c_int = &mut outer;
                    let ret = unsafe { (lib.static_alias)(p) };
                    AliasObs {
                        ret_is_inner: ret == lib.inner_addr,
                        ret_is_outer: ret == p,
                        ret_val: unsafe { *ret },
                        outer_after: outer,
                        inner_after: get_inner(lib),
                    }
                })
                .collect()
        };
        let got_c = run(&l.c);
        let got_rust = run(&l.rust);
        assert_eq!(got_c, got_rust, "[cfg-14] divergence with inner0={inner0}");
    }
}

// ---------------------------------------------------------------------------
// Row 15 -- pointer identity and stability
// ---------------------------------------------------------------------------

#[test]
fn cfg_15_pointer_identity_and_stability() {
    let _g = lock();
    let l = libs();
    for lib in [&l.c, &l.rust] {
        set_inner(lib, 0);
        let mut a: c_int = 7;
        let mut b: c_int = 9;
        let pa: *mut c_int = &mut a;
        let pb: *mut c_int = &mut b;
        // `if` arm twice from different caller variables => same &inner.
        set_inner(lib, 0);
        let r1 = unsafe { (lib.static_alias)(pa) };
        set_inner(lib, 0);
        let r2 = unsafe { (lib.static_alias)(pb) };
        assert_eq!(r1, r2, "{}: &inner must be stable", lib.name);
        assert_eq!(r1, lib.inner_addr, "{}: must return &inner", lib.name);
        // `else` arm returns exactly the caller's pointer.
        set_inner(lib, 1000);
        unsafe { *pa = 1 };
        let r3 = unsafe { (lib.static_alias)(pa) };
        assert_eq!(r3, pa, "{}: else arm must return `outer`", lib.name);
        assert_ne!(r3, lib.inner_addr, "{}: else arm must not return &inner", lib.name);
    }
    // And differentially: the *classification* of the returned pointer must be
    // identical for the two libraries across randomized inputs.
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..2000 {
        assert_alias_eq("cfg-15", rng.int_biased(), rng.int_biased());
    }
}

// ---------------------------------------------------------------------------
// Row 16 -- two distinct caller variables used alternately
// ---------------------------------------------------------------------------

#[test]
fn cfg_16_two_caller_variables_alternating() {
    let _g = lock();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..200 {
        let inner0 = rng.int_biased();
        let a0 = rng.int_biased();
        let b0 = rng.int_biased();
        let n = 32;
        let run = |lib: &Lib| -> Vec<(bool, bool, c_int, c_int, c_int, c_int)> {
            set_inner(lib, inner0);
            let mut a = a0;
            let mut b = b0;
            let mut out = Vec::new();
            for i in 0..n {
                let p: *mut c_int = if i % 2 == 0 { &mut a } else { &mut b };
                let ret = unsafe { (lib.static_alias)(p) };
                out.push((
                    ret == lib.inner_addr,
                    ret == p,
                    unsafe { *ret },
                    a,
                    b,
                    get_inner(lib),
                ));
            }
            out
        };
        let got_c = run(&l.c);
        let got_rust = run(&l.rust);
        assert_eq!(
            got_c, got_rust,
            "[cfg-16] divergence inner0={inner0} a0={a0} b0={b0}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 17 -- `driver` with zero iterations
// ---------------------------------------------------------------------------

#[test]
fn cfg_17_driver_zero_iterations() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 17);
    let obs = assert_driver_eq("cfg-17", INNER_INITIAL, 42, 0);
    assert!(obs.stdout.is_empty(), "zero-trip loop must print nothing");
    assert_eq!(obs.inner_after, INNER_INITIAL, "inner must be untouched");
    for _ in 0..500 {
        let obs = assert_driver_eq("cfg-17", rng.int_biased(), rng.int_biased(), 0);
        assert!(obs.stdout.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Row 18 -- `driver` with exactly one iteration
// ---------------------------------------------------------------------------

#[test]
fn cfg_18_driver_one_iteration() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..500 {
        let obs = assert_driver_eq("cfg-18", INNER_INITIAL, rng.int_biased(), 1);
        assert_eq!(
            obs.stdout.iter().filter(|&&b| b == b'\n').count(),
            1,
            "one iteration must print exactly one line"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 19 -- `driver` with 2 and 3 iterations
// ---------------------------------------------------------------------------

#[test]
fn cfg_19_driver_two_and_three_iterations() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..400 {
        let v = rng.int_biased();
        assert_driver_eq("cfg-19", INNER_INITIAL, v, 2);
        assert_driver_eq("cfg-19", INNER_INITIAL, v, 3);
    }
    for &v in BOUNDARIES.iter() {
        assert_driver_eq("cfg-19", INNER_INITIAL, v, 2);
        assert_driver_eq("cfg-19", INNER_INITIAL, v, 3);
    }
}

// ---------------------------------------------------------------------------
// Row 20 -- fresh state, `initial_value >= 1` => immediate doubling lock-in
// ---------------------------------------------------------------------------

#[test]
fn cfg_20_driver_if_arm_first_then_doubling() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..300 {
        let v = rng.int_in(1, c_int::MAX);
        let iters = rng.int_in(1, 64);
        assert_driver_eq("cfg-20", INNER_INITIAL, v, iters);
    }
}

// ---------------------------------------------------------------------------
// Row 21 -- fresh state, `initial_value < inner` => creep through the else arm
// ---------------------------------------------------------------------------

#[test]
fn cfg_21_driver_else_arm_first() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 21);
    // With `inner == 1`, `initial_value == 0` and every negative value take the
    // else arm first.
    assert_driver_eq("cfg-21", INNER_INITIAL, 0, 4);
    for _ in 0..300 {
        let v = rng.int_in(c_int::MIN, 0);
        let iters = rng.int_in(1, 64);
        assert_driver_eq("cfg-21", INNER_INITIAL, v, iters);
    }
    // A long creep: inner preset large and positive, initial_value far below it,
    // so the else arm runs many times before the if arm is reached.
    for _ in 0..100 {
        let inner = rng.int_in(2, 50);
        let v = -rng.int_in(0, 500);
        assert_driver_eq("cfg-21", inner, v, rng.int_in(1, 80));
    }
}

// ---------------------------------------------------------------------------
// Row 22 -- full cross-product of preset state x input x iteration count
// ---------------------------------------------------------------------------

#[test]
fn cfg_22_driver_state_input_cross_product() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 22);
    let presets = [1, 0, -1, 7, -7, 12345, -12345, c_int::MAX, c_int::MIN, c_int::MAX - 1, c_int::MIN + 1];
    for &inner in presets.iter() {
        for _ in 0..60 {
            let v = rng.int_biased();
            let iters = rng.int_in(1, 40);
            assert_driver_eq("cfg-22", inner, v, iters);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 23 -- boundary initial values x fixed iteration counts x presets
// ---------------------------------------------------------------------------

#[test]
fn cfg_23_driver_boundary_initial_values() {
    let _g = lock();
    let presets = [1, 0, -1, c_int::MAX, c_int::MIN];
    for &inner in presets.iter() {
        for &v in BOUNDARIES.iter() {
            for &iters in &[1, 2, 3, 7, 33, 64] {
                assert_driver_eq("cfg-23", inner, v, iters);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24 -- long output streams (buffering + repeated wrap-around)
// ---------------------------------------------------------------------------

#[test]
fn cfg_24_driver_many_iterations() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 24);
    for &iters in &[128, 1000, 4096] {
        let obs = assert_driver_eq("cfg-24", INNER_INITIAL, 1, iters);
        assert_eq!(
            obs.stdout.iter().filter(|&&b| b == b'\n').count(),
            iters as usize,
            "one line per iteration"
        );
        for _ in 0..10 {
            assert_driver_eq("cfg-24", rng.int_biased(), rng.int_biased(), iters);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 -- two consecutive `driver` calls, no reset in between
// ---------------------------------------------------------------------------

#[test]
fn cfg_25_driver_state_carries_between_calls() {
    let _g = lock();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..200 {
        let inner0 = rng.int_biased();
        let (v1, n1) = (rng.int_biased(), rng.int_in(0, 20));
        let (v2, n2) = (rng.int_biased(), rng.int_in(0, 20));
        let run = |lib: &Lib| -> (Vec<u8>, Vec<u8>, c_int) {
            set_inner(lib, inner0);
            let o1 = capture_stdout(lib.name, || unsafe { (lib.driver)(v1, n1) });
            // deliberately NO reset here
            let o2 = capture_stdout(lib.name, || unsafe { (lib.driver)(v2, n2) });
            (o1, o2, get_inner(lib))
        };
        let got_c = run(&l.c);
        let got_rust = run(&l.rust);
        assert_eq!(
            got_c.2, got_rust.2,
            "[cfg-25] inner divergence inner0={inner0} ({v1},{n1}) ({v2},{n2})"
        );
        assert!(
            got_c.0 == got_rust.0 && got_c.1 == got_rust.1,
            "[cfg-25] stdout divergence inner0={inner0} ({v1},{n1}) ({v2},{n2})\n  C   : {} | {}\n  Rust: {} | {}",
            preview(&got_c.0),
            preview(&got_c.1),
            preview(&got_rust.0),
            preview(&got_rust.1)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 26 -- interleaved low-level and wrapper calls on the same shared state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Op {
    Alias(c_int),
    AliasSelf,
    Driver(c_int, c_int),
}

#[test]
fn cfg_26_interleaved_entry_points() {
    let _g = lock();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..120 {
        let inner0 = rng.int_biased();
        let ops: Vec<Op> = (0..64)
            .map(|_| match rng.next_u64() % 4 {
                0 => Op::AliasSelf,
                1 => Op::Driver(rng.int_biased(), rng.int_in(0, 8)),
                _ => Op::Alias(rng.int_biased()),
            })
            .collect();
        let run = |lib: &Lib| -> Vec<(u8, bool, bool, c_int, c_int, Vec<u8>)> {
            set_inner(lib, inner0);
            ops.iter()
                .map(|op| match *op {
                    Op::Alias(v) => {
                        let mut outer = v;
                        let p: *mut c_int = &mut outer;
                        let ret = unsafe { (lib.static_alias)(p) };
                        (
                            0u8,
                            ret == lib.inner_addr,
                            ret == p,
                            unsafe { *ret },
                            get_inner(lib),
                            Vec::new(),
                        )
                    }
                    Op::AliasSelf => {
                        let p = lib.inner_addr;
                        let ret = unsafe { (lib.static_alias)(p) };
                        (
                            1u8,
                            ret == lib.inner_addr,
                            ret == p,
                            unsafe { *ret },
                            get_inner(lib),
                            Vec::new(),
                        )
                    }
                    Op::Driver(v, n) => {
                        let out = capture_stdout(lib.name, || unsafe { (lib.driver)(v, n) });
                        (2u8, false, false, 0, get_inner(lib), out)
                    }
                })
                .collect()
        };
        let got_c = run(&l.c);
        let got_rust = run(&l.rust);
        for (i, (a, b)) in got_c.iter().zip(got_rust.iter()).enumerate() {
            assert_eq!(
                a, b,
                "[cfg-26] divergence at op {i} ({:?}) with inner0={inner0}",
                ops[i]
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 27 -- `initial_value` is by value: the caller's variable is untouched
// ---------------------------------------------------------------------------

#[test]
fn cfg_27_driver_argument_passed_by_value() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..300 {
        let v = rng.int_biased();
        let n = rng.int_in(1, 12);
        let a = assert_driver_eq("cfg-27", INNER_INITIAL, v, n);
        assert_eq!(a.caller_arg_after, v, "the caller's argument must not change");
        // Passing the same value again from a fresh state must reproduce the
        // very same output -- proof that the first call did not mutate it.
        let b = assert_driver_eq("cfg-27", INNER_INITIAL, v, n);
        assert_eq!(a.stdout, b.stdout, "replay from fresh state must match");
    }
}

// ---------------------------------------------------------------------------
// Row 29 -- `driver` must write through the *same* stdio buffer as the caller
//
// `printf("%d\n", ...)` in C shares the process-wide `FILE *stdout` with the
// caller, so a caller's own unflushed `printf` output keeps its position
// relative to the library's output. A translation that used Rust's `println!`
// would emit the same *bytes* but through a different, line-flushed buffer,
// reordering them against the caller's. Mutation testing showed this is
// invisible to the other rows, so it gets its own row.
// ---------------------------------------------------------------------------

#[test]
fn cfg_29_shares_the_callers_stdio_buffer() {
    let _g = lock();
    let l = libs();
    let mut out = Vec::new();
    for lib in [&l.c, &l.rust] {
        set_inner(lib, 1);
        let captured = capture_stdout_raw(lib.name, || {
            libc_print("A");                       // unflushed, in libc's buffer
            unsafe { (lib.driver)(1, 3) };         // library output
            libc_print("B");                       // unflushed, in libc's buffer
        });
        out.push(captured);
    }
    assert_eq!(
        out[0], out[1],
        "[cfg-29] interleaving with the caller's own stdio differs\n  C   : {}\n  Rust: {}",
        preview(&out[0]),
        preview(&out[1])
    );
    assert_eq!(
        out[0], b"A2\n4\n8\nB",
        "[cfg-29] the library must write into the caller's stdout buffer in order, got {}",
        preview(&out[0])
    );
}

// ---------------------------------------------------------------------------
// Extra: the chain helper via the generic `driver`-shaped feedback loop
// ---------------------------------------------------------------------------

#[test]
fn cfg_extra_feedback_chain_matches_driver_shape() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xFEED);
    for _ in 0..400 {
        let inner = rng.int_biased();
        let outer = rng.int_biased();
        let steps = 1 + (rng.next_u64() % 40) as usize;
        assert_chain_eq("cfg-extra", inner, outer, steps);
    }
}
