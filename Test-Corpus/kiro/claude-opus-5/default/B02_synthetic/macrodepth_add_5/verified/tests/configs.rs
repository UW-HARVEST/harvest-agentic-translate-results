//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through `dlopen`/`dlsym` on both shared libraries; no
//! `driver` function is ever called directly, so the `#[no_mangle]` export
//! wrappers are part of what is being compared. Each row asserts both the
//! returned `int` and the exact bytes written to stdout.
//!
//! The row numbers match `CONFIGS.md`. The `(OP, REPEAT)` axis is swept by
//! `run_all.sh`, which re-runs this whole file for each of the 24 feature
//! combinations.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Rows 1-6: the three leaf operations
// ---------------------------------------------------------------------------
//
// `op_add`/`op_sub`/`op_mul` are all defined unconditionally in mdcore.c, so
// each must behave identically in every one of the 24 configurations -- a
// regression that made them depend on the selected OP would show up here.

fn leaf_random(what: &'static str, pick: fn(&Lib) -> BinFn) {
    let mut rng = Rng::new();
    for _ in 0..CASES {
        let a = rng.next_operand();
        let b = rng.next_operand();
        diff_bin(what, pick, a, b);
    }
}

fn leaf_bounds(what: &'static str, pick: fn(&Lib) -> BinFn) {
    for &a in BOUNDS {
        for &b in BOUNDS {
            diff_bin(what, pick, a, b);
        }
    }
}

#[test]
fn row01_op_add_random() {
    leaf_random("op_add", |l| l.op_add);
}

#[test]
fn row02_op_add_bounds() {
    leaf_bounds("op_add", |l| l.op_add);
    // The two wraparound cases spelled out, so the row cannot silently pass by
    // both sides panicking or both sides being skipped.
    assert_eq!(diff_bin("op_add", |l| l.op_add, i32::MAX, 1), i32::MIN);
    assert_eq!(diff_bin("op_add", |l| l.op_add, i32::MIN, -1), i32::MAX);
}

#[test]
fn row03_op_sub_random() {
    leaf_random("op_sub", |l| l.op_sub);
}

#[test]
fn row04_op_sub_bounds() {
    leaf_bounds("op_sub", |l| l.op_sub);
    assert_eq!(diff_bin("op_sub", |l| l.op_sub, i32::MIN, 1), i32::MAX);
    assert_eq!(diff_bin("op_sub", |l| l.op_sub, 0, i32::MIN), i32::MIN);
}

#[test]
fn row05_op_mul_random() {
    leaf_random("op_mul", |l| l.op_mul);
}

#[test]
fn row06_op_mul_bounds() {
    leaf_bounds("op_mul", |l| l.op_mul);
    assert_eq!(diff_bin("op_mul", |l| l.op_mul, i32::MAX, i32::MAX), 1);
    assert_eq!(diff_bin("op_mul", |l| l.op_mul, i32::MIN, -1), i32::MIN);
}

// ---------------------------------------------------------------------------
// Rows 7-9: the two exported data symbols
// ---------------------------------------------------------------------------

/// Row 7: `int (*G_OP)(int,int) = OP_FN(OP);` -- load the slot and call through it.
#[test]
fn row07_g_op_dispatch() {
    let p = pair();
    let mut rng = Rng::new();
    for _ in 0..CASES {
        let a = rng.next_operand();
        let b = rng.next_operand();
        let got = diff_g_op(a, b);
        // The pointer must resolve to the configured operation, not merely to
        // "the same thing in both libraries".
        assert_eq!(
            got,
            OP.apply(a, b),
            "[{}] G_OP does not dispatch to op_{}",
            tag(),
            OP.cmake_value()
        );
        // ... and to the very same function the library exports by name.
        let by_name = match OP {
            Op::Add => p.c.op_add,
            Op::Sub => p.c.op_sub,
            Op::Mul => p.c.op_mul,
        };
        assert_eq!(
            p.c.load_g_op() as usize,
            by_name as usize,
            "[{}] C: G_OP is not the exported op_{}",
            tag(),
            OP.cmake_value()
        );
        let by_name = match OP {
            Op::Add => p.rust.op_add,
            Op::Sub => p.rust.op_sub,
            Op::Mul => p.rust.op_mul,
        };
        assert_eq!(
            p.rust.load_g_op() as usize,
            by_name as usize,
            "[{}] Rust: G_OP is not the exported op_{}",
            tag(),
            OP.cmake_value()
        );
    }
    for &a in BOUNDS {
        for &b in BOUNDS {
            diff_g_op(a, b);
        }
    }
}

/// A caller-supplied target for the `G_OP` rebind check.
extern "C" fn injected(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(3).wrapping_sub(b)
}

/// Row 8: `G_OP` is a non-`const` global, so an external caller may overwrite it.
#[test]
fn row_g_op_rebind() {
    let p = pair();
    let saved_c = p.c.load_g_op();
    let saved_rust = p.rust.load_g_op();

    p.c.store_g_op(injected);
    p.rust.store_g_op(injected);
    let mut rng = Rng::new();
    for _ in 0..64 {
        let a = rng.next_operand();
        let b = rng.next_operand();
        let got = diff_g_op(a, b);
        assert_eq!(got, injected(a, b), "[{}] rebound G_OP not honoured", tag());
    }
    // `helper_call`/`helper_ptr` expand `OP_FN(OP)` directly, so they must be
    // unaffected by the rebind -- exactly as in C.
    let a = 11;
    let b = 4;
    assert_eq!(
        diff_bin("helper_ptr", |l| l.helper_ptr, a, b),
        OP.apply(a, b),
        "[{}] helper_ptr must not read G_OP",
        tag()
    );

    p.c.store_g_op(saved_c);
    p.rust.store_g_op(saved_rust);
    assert_eq!(diff_g_op(a, b), OP.apply(a, b));
}

/// Row 9: `const char *G_OP_NAME = STR(OP);`
#[test]
fn row09_g_op_name() {
    let p = pair();
    let c = p.c.g_op_name_bytes();
    let r = p.rust.g_op_name_bytes();
    assert_eq!(
        c,
        r,
        "[{}] G_OP_NAME diverged: C={:?} Rust={:?}",
        tag(),
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    assert_eq!(
        c,
        OP.cmake_value().as_bytes(),
        "[{}] G_OP_NAME is not STR(OP)",
        tag()
    );
    assert_eq!(c.len(), 3, "[{}] G_OP_NAME should be 3 bytes + NUL", tag());
}

// ---------------------------------------------------------------------------
// Rows 10-11: helper_ptr (leaf call through a local function pointer)
// ---------------------------------------------------------------------------

#[test]
fn row10_helper_ptr_random() {
    let mut rng = Rng::new();
    for _ in 0..CASES {
        let a = rng.next_operand();
        let b = rng.next_operand();
        let got = diff_bin("helper_ptr", |l| l.helper_ptr, a, b);
        assert_eq!(got, OP.apply(a, b), "[{}] helper_ptr value", tag());
    }
}

#[test]
fn row11_helper_ptr_bounds() {
    for &a in BOUNDS {
        for &b in BOUNDS {
            diff_bin("helper_ptr", |l| l.helper_ptr, a, b);
        }
    }
    // `%d` of INT_MIN is the classic printf edge case; diff_bin compares the
    // printed bytes, so a bad formatting path fails here.
    diff_bin("helper_ptr", |l| l.helper_ptr, i32::MIN, 0);
    diff_bin("helper_ptr", |l| l.helper_ptr, i32::MIN, 1);
}

// ---------------------------------------------------------------------------
// Rows 12-15: helper_call (leaf call + the full RUN_LOOP unrolling)
// ---------------------------------------------------------------------------
//
// Rows 12/13/14 in CONFIGS.md are the per-OP sweeps of REPEAT; a single test
// body covers whichever (OP, REPEAT) this binary was built for, and run_all.sh
// supplies the other 23.

#[test]
fn row12_13_14_helper_call_random() {
    let expected_acc = rep_reference(REPEAT);
    let mut rng = Rng::new();
    for _ in 0..CASES {
        let a = rng.next_operand();
        let b = rng.next_operand();
        let got = diff_bin("helper_call", |l| l.helper_call, a, b);
        assert_eq!(
            got,
            OP.apply(a, b).wrapping_add(expected_acc),
            "[{}] helper_call = r + acc",
            tag()
        );
    }
}

/// The `acc` half of `helper_call` depends only on REPEAT, and is checked
/// against the hand-computed table from `CONFIGS.md` rows 12-14.
#[test]
fn row12_13_14_helper_call_acc_table() {
    let table: [c_int; 8] = match OP {
        Op::Add => [0, 0, 1, 3, 6, 10, 15, 21],
        Op::Sub => [0, 0, -1, -3, -6, -10, -15, -21],
        Op::Mul => [1, 1, 2, 6, 24, 120, 720, 5040],
    };
    let expected_acc = table[REPEAT as usize];
    // helper_call(0, 0) isolates acc for add/sub (r == 0) and, for mul, r == 0
    // as well, so the return value is exactly acc for add/sub.
    let got = diff_bin("helper_call", |l| l.helper_call, 0, 0);
    assert_eq!(
        got,
        OP.apply(0, 0).wrapping_add(expected_acc),
        "[{}] helper_call acc should be {expected_acc}",
        tag()
    );
    assert_eq!(
        rep_reference(REPEAT),
        expected_acc,
        "[{}] harness REP table disagrees with CONFIGS.md",
        tag()
    );
    // And the printed line must carry that acc verbatim.
    let p = pair();
    let cf = p.c.helper_call;
    let (_, out) = capture_stdout(|| unsafe { cf(0, 0) });
    let want = format!("helper.call={} helper.acc={}\n", OP.apply(0, 0), expected_acc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        want,
        "[{}] C helper_call printed line",
        tag()
    );
}

#[test]
fn row15_helper_call_bounds() {
    for &a in BOUNDS {
        for &b in BOUNDS {
            diff_bin("helper_call", |l| l.helper_call, a, b);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 16-20: use_generated (the switch-dispatched generated accumulator)
// ---------------------------------------------------------------------------

#[test]
fn row16_use_generated_zero() {
    let got = diff_un("use_generated", |l| l.use_generated, 0);
    assert_eq!(got, OP.init(), "[{}] use_generated(0) is INIT_FOR(OP)", tag());
}

#[test]
fn row17_use_generated_one() {
    let got = diff_un("use_generated", |l| l.use_generated, 1);
    // REP1 applies STEP_OP(op, acc, 0): a no-op for add/sub, `*1` for mul.
    assert_eq!(got, OP.step(OP.init(), 0), "[{}] use_generated(1)", tag());
    assert_eq!(got, OP.init(), "[{}] i == 0 makes REP1 a no-op", tag());
}

#[test]
fn row18_use_generated_cases_2_to_6() {
    for n in 2..=6 {
        let got = diff_un("use_generated", |l| l.use_generated, n);
        assert_eq!(
            got,
            rep_reference(n),
            "[{}] use_generated({n}) should be REP{n}",
            tag()
        );
    }
}

#[test]
fn row19_use_generated_sweep() {
    for n in -16..=16 {
        let got = diff_un("use_generated", |l| l.use_generated, n);
        assert_eq!(
            got,
            dispatch_reference(n),
            "[{}] use_generated({n}) vs DISPATCH_REP",
            tag()
        );
    }
}

#[test]
fn row20_use_generated_random() {
    let mut rng = Rng::new();
    for _ in 0..CASES {
        let n = rng.next_i32();
        let got = diff_un("use_generated", |l| l.use_generated, n);
        assert_eq!(got, dispatch_reference(n), "[{}] use_generated({n})", tag());
    }
    // Plus a deliberate mix of small in-range and just-out-of-range values.
    for n in [0, 6, 7, 8, -1, 100, i32::MAX, i32::MIN] {
        diff_un("use_generated", |l| l.use_generated, n);
    }
}

// ---------------------------------------------------------------------------
// Row 21: the unrolled path vs the switch path
// ---------------------------------------------------------------------------
//
// `helper_call` uses RUN_LOOP -> REP<REPEAT> (compile-time, REP7 exists);
// `use_generated` uses DISPATCH_REP -> switch (runtime, no case 7). They must
// agree for REPEAT <= 6 and MUST NOT agree at REPEAT == 7.

#[test]
fn row21_unrolled_vs_switch() {
    let acc_from_helper = diff_bin("helper_call", |l| l.helper_call, 0, 0)
        .wrapping_sub(OP.apply(0, 0));
    let acc_from_generated = diff_un("use_generated", |l| l.use_generated, REPEAT);

    if REPEAT <= 6 {
        assert_eq!(
            acc_from_helper, acc_from_generated,
            "[{}] REP{REPEAT} and case {REPEAT} must agree",
            tag()
        );
    } else {
        assert_eq!(REPEAT, 7);
        assert_eq!(
            acc_from_generated,
            OP.init(),
            "[{}] DISPATCH_REP has no case 7, so it must return INIT",
            tag()
        );
        assert_eq!(
            acc_from_helper,
            rep_reference(7),
            "[{}] RUN_LOOP does expand REP7",
            tag()
        );
        assert_ne!(
            acc_from_helper, acc_from_generated,
            "[{}] the REPEAT=7 asymmetry in mdmacros.h must be preserved",
            tag()
        );
    }
}

// ---------------------------------------------------------------------------
// Row 22: the whole pipeline, the way main() drives it
// ---------------------------------------------------------------------------

#[test]
fn row22_full_pipeline_transcript() {
    let p = pair();
    let mut rng = Rng::new();
    for _ in 0..128 {
        let a = rng.next_operand();
        let b = rng.next_operand();

        let run = |l: &Lib| -> (c_int, Vec<u8>) {
            capture_stdout(|| unsafe {
                let r_call = (l.op_add)(a, b); // leaf, OP-independent
                let x1 = (l.helper_call)(a, b);
                let x2 = (l.helper_ptr)(a, b);
                let x3 = (l.use_generated)(REPEAT);
                let g = (l.load_g_op())(a, b);
                r_call
                    .wrapping_add(x1)
                    .wrapping_add(x2)
                    .wrapping_add(x3)
                    .wrapping_add(g)
            })
        };
        let c = run(&p.c);
        let r = run(&p.rust);
        assert_eq!(
            c.0,
            r.0,
            "[{}] pipeline sum for ({a}, {b}): C={} Rust={}",
            tag(),
            c.0,
            r.0
        );
        assert_eq!(
            String::from_utf8_lossy(&c.1),
            String::from_utf8_lossy(&r.1),
            "[{}] pipeline transcript for ({a}, {b})",
            tag()
        );
    }
}

// ---------------------------------------------------------------------------
// Row 23: statelessness across repeated calls
// ---------------------------------------------------------------------------

#[test]
fn row23_repeated_calls_are_stateless() {
    let first = diff_bin("helper_call", |l| l.helper_call, 5, 3);
    diff_bin("helper_call", |l| l.helper_call, -9_999, 12_345);
    let again = diff_bin("helper_call", |l| l.helper_call, 5, 3);
    assert_eq!(first, again, "[{}] helper_call is not stateless", tag());

    let g1 = diff_un("use_generated", |l| l.use_generated, 4);
    diff_un("use_generated", |l| l.use_generated, 99);
    let g2 = diff_un("use_generated", |l| l.use_generated, 4);
    assert_eq!(g1, g2, "[{}] use_generated is not stateless", tag());

    let p1 = diff_bin("helper_ptr", |l| l.helper_ptr, 7, 8);
    let p2 = diff_bin("helper_ptr", |l| l.helper_ptr, 7, 8);
    assert_eq!(p1, p2, "[{}] helper_ptr is not stateless", tag());

    // The data symbols must not have drifted either.
    assert_eq!(pair().c.g_op_name_bytes(), pair().rust.g_op_name_bytes());
}
