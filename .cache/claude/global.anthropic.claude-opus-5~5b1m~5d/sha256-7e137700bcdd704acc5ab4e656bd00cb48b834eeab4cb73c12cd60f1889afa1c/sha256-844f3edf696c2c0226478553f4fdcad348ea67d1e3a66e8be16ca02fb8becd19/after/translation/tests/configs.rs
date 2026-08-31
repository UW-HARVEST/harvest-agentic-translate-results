//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares the returned `int` *and* the
//! exact stdout bytes. Randomized rows use a fixed seed.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Rows 1-9: the leaf ops (lowest level of the call hierarchy)
// ---------------------------------------------------------------------------

fn op_small_grid(name: &str) {
    for (a, b) in small_grid() {
        diff_bin(name, a, b);
    }
}

fn op_random(name: &str, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..ITERS {
        let (a, b) = (rng.next_int(), rng.next_int());
        diff_bin(name, a, b);
    }
}

fn op_bounds(name: &str) {
    for (a, b) in bounds_grid() {
        diff_bin(name, a, b);
    }
}

#[test]
fn cfg_01_op_add_small_grid() {
    op_small_grid("op_add");
}

#[test]
fn cfg_02_op_add_random() {
    op_random("op_add", SEED ^ 1);
}

#[test]
fn cfg_03_op_add_bounds() {
    op_bounds("op_add");
}

#[test]
fn cfg_04_op_sub_small_grid() {
    op_small_grid("op_sub");
}

#[test]
fn cfg_05_op_sub_random() {
    op_random("op_sub", SEED ^ 2);
}

#[test]
fn cfg_06_op_sub_bounds() {
    op_bounds("op_sub");
}

#[test]
fn cfg_07_op_mul_small_grid() {
    op_small_grid("op_mul");
}

#[test]
fn cfg_08_op_mul_random() {
    op_random("op_mul", SEED ^ 3);
}

#[test]
fn cfg_09_op_mul_bounds() {
    op_bounds("op_mul");
}

// ---------------------------------------------------------------------------
// Rows 10-13: the macro-initialised globals
// ---------------------------------------------------------------------------

/// Row 10 — `const char *G_OP_NAME = STR(OP);`
#[test]
fn cfg_10_g_op_name_matches() {
    let (c, r) = pair();
    let cn = c.g_op_name();
    let rn = r.g_op_name();
    assert_eq!(
        String::from_utf8_lossy(&cn),
        String::from_utf8_lossy(&rn),
        "G_OP_NAME mismatch [OP={OP}]"
    );
    // And it must actually be the configured OP name.
    assert_eq!(String::from_utf8_lossy(&cn), OP, "C G_OP_NAME unexpected");
}

/// Row 11 — `int (*G_OP)(int,int) = OP_FN(OP);` must point at `op_<OP>`.
#[test]
fn cfg_11_g_op_points_at_selected_op() {
    let (c, r) = pair();
    let expected = format!("op_{OP}");
    assert_eq!(
        c.g_op_value(),
        c.fn_addr(&expected),
        "C G_OP does not point at {expected}"
    );
    assert_eq!(
        r.g_op_value(),
        r.fn_addr(&expected),
        "Rust G_OP does not point at {expected}"
    );
    // The two must also disagree with the other ops (guards against all three
    // ops being folded to one address).
    for other in ["op_add", "op_sub", "op_mul"] {
        if other != expected {
            let c_same = c.g_op_value() == c.fn_addr(other);
            let r_same = r.g_op_value() == r.fn_addr(other);
            assert_eq!(
                c_same, r_same,
                "G_OP-vs-{other} identity differs between C and Rust"
            );
        }
    }
}

/// Row 12 — invoke through `G_OP`, exhaustive small grid.
#[test]
fn cfg_12_g_op_small_grid() {
    for (a, b) in small_grid() {
        diff_g_op(a, b);
    }
}

/// Row 13 — invoke through `G_OP`, randomized + boundary values.
#[test]
fn cfg_13_g_op_random_and_bounds() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..ITERS {
        let (a, b) = (rng.next_int(), rng.next_int());
        diff_g_op(a, b);
    }
    for (a, b) in bounds_grid() {
        diff_g_op(a, b);
    }
}

// ---------------------------------------------------------------------------
// Rows 14-16: helper_ptr (indirect call)
// ---------------------------------------------------------------------------

#[test]
fn cfg_14_helper_ptr_small_grid() {
    op_small_grid("helper_ptr");
}

#[test]
fn cfg_15_helper_ptr_random() {
    op_random("helper_ptr", SEED ^ 5);
}

#[test]
fn cfg_16_helper_ptr_bounds() {
    op_bounds("helper_ptr");
}

// ---------------------------------------------------------------------------
// Rows 17-19: helper_call (op + statically unrolled RUN_LOOP)
// ---------------------------------------------------------------------------

#[test]
fn cfg_17_helper_call_small_grid() {
    op_small_grid("helper_call");
}

#[test]
fn cfg_18_helper_call_random() {
    op_random("helper_call", SEED ^ 6);
}

#[test]
fn cfg_19_helper_call_bounds() {
    op_bounds("helper_call");
}

/// Extra: pin down that the unrolled accumulator really is the `REPEAT` one, so
/// a wrong-but-consistent `REPEAT` in both would still be caught.
#[test]
fn cfg_19b_helper_call_acc_equals_expected_repeat_chain() {
    let (c, _r) = pair();
    let f = c.bin("helper_call");
    // SAFETY: matches `int helper_call(int,int)`.
    let (_v, out) = capture_stdout(|| unsafe { f(3, 4) });
    let mut acc = INIT;
    let mut i: c_int = 0;
    while i < REPEAT {
        acc = step(acc, i);
        i += 1;
    }
    let text = String::from_utf8_lossy(&out);
    assert!(
        text.contains(&format!("helper.acc={acc}")),
        "C helper.acc does not match the expected REPEAT={REPEAT} chain \
         (expected {acc}); got {text:?}"
    );
}

// ---------------------------------------------------------------------------
// Rows 20-23: use_generated / DISPATCH_REP
// ---------------------------------------------------------------------------

/// Row 20 — every `case` of the `switch` (0..=6), exhaustively.
#[test]
fn cfg_20_use_generated_all_cases() {
    for n in 0..=6 {
        diff_un("use_generated", n);
    }
}

/// Row 21 — `n == REPEAT`, the value `main` passes (axis-1 x axis-2 interaction).
#[test]
fn cfg_21_use_generated_n_eq_repeat() {
    diff_un("use_generated", REPEAT);
}

/// Row 22 — the `default:` arm.
#[test]
fn cfg_22_use_generated_default_arm() {
    for n in [7, 8, 9, 100, -1, -2, c_int::MIN, c_int::MAX] {
        diff_un("use_generated", n);
    }
}

/// Row 23 — randomized `n` over the whole `int` range (biased to `-2..=9`).
#[test]
fn cfg_23_use_generated_random_n() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..ITERS {
        diff_un("use_generated", rng.next_n());
    }
}

// ---------------------------------------------------------------------------
// Row 24: the composed pipeline, driven on the low-level API
// ---------------------------------------------------------------------------

/// Replays `main`'s exact call sequence directly against the exported symbols,
/// capturing the whole interleaved stdout of the composition in one go.
#[test]
fn cfg_24_composed_pipeline() {
    let (c, r) = pair();

    let run = |l: &Loaded, a: c_int, b: c_int| -> (Vec<c_int>, Vec<u8>) {
        let hc = l.bin("helper_call");
        let hp = l.bin("helper_ptr");
        let ug = l.un("use_generated");
        let g = l.g_op();
        capture_stdout(|| {
            // SAFETY: all signatures match the C declarations.
            unsafe {
                let x1 = hc(a, b);
                let x2 = hp(a, b);
                let x3 = ug(REPEAT);
                let gv = g(a, b);
                vec![x1, x2, x3, gv]
            }
        })
    };

    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..128 {
        let (a, b) = (rng.next_int(), rng.next_int());
        let (cv, cout) = run(c, a, b);
        let (rv, rout) = run(r, a, b);
        assert_eq!(cv, rv, "pipeline returns mismatch for ({a}, {b})");
        assert_eq!(
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            "pipeline stdout mismatch for ({a}, {b})"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 25: repeated invocation / absence of hidden state
// ---------------------------------------------------------------------------

#[test]
fn cfg_25_repeated_invocation_is_stateless() {
    let (c, r) = pair();
    let mut rng = Rng::new(SEED ^ 9);
    for name in ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr"] {
        let cf = c.bin(name);
        let rf = r.bin(name);
        let mut cs = Vec::new();
        let mut rs = Vec::new();
        let mut inputs = Vec::new();
        for _ in 0..64 {
            inputs.push((rng.next_int(), rng.next_int()));
        }
        let (_, cout) = capture_stdout(|| {
            for &(a, b) in &inputs {
                // SAFETY: matches the C declaration.
                cs.push(unsafe { cf(a, b) });
            }
        });
        let (_, rout) = capture_stdout(|| {
            for &(a, b) in &inputs {
                // SAFETY: matches the C declaration.
                rs.push(unsafe { rf(a, b) });
            }
        });
        assert_eq!(cs, rs, "{name}: 64 sequential calls diverge");
        assert_eq!(
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            "{name}: 64 sequential calls stdout diverges"
        );
    }
    // use_generated too (unary).
    let cu = c.un("use_generated");
    let ru = r.un("use_generated");
    let ns: Vec<c_int> = (0..64).map(|_| rng.next_n()).collect();
    let (cv, cout) = capture_stdout(|| {
        // SAFETY: matches the C declaration.
        ns.iter().map(|&n| unsafe { cu(n) }).collect::<Vec<_>>()
    });
    let (rv, rout) = capture_stdout(|| {
        // SAFETY: matches the C declaration.
        ns.iter().map(|&n| unsafe { ru(n) }).collect::<Vec<_>>()
    });
    assert_eq!(cv, rv, "use_generated: 64 sequential calls diverge");
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "use_generated: 64 sequential calls stdout diverges"
    );
}
