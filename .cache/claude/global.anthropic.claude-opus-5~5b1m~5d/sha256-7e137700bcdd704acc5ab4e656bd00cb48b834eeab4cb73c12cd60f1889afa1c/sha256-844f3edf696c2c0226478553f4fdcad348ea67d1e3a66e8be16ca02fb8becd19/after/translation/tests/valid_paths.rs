//! Phase B — valid-path differential tests, gated on `CONFIGS.md`.
//!
//! Every test loads **both** shared objects via `libloading` and drives the
//! exported symbols directly; no Rust function is ever called in-process. The
//! row of `CONFIGS.md` a test discharges is named in its doc comment. Which row
//! of the 24 `(OP, REPEAT)` rows is active is decided by the Cargo feature set,
//! so the whole file is re-run once per configuration by
//! `run_all_configs.sh` (see `CONFIGS.md`).

mod common;

use std::ffi::c_int;

use common::{load_pair, same, Api, Rng, CORNERS, INIT_FOR, N_SHAPES, OP_NAME, REPEAT, SEED};

/// Number of randomised inputs per entry point per row. Property-style: a single
/// hand-picked scalar hits one code path and misses value-dependent bugs.
const ITERS: usize = 1000;

// ---------------------------------------------------------------------------
// Axis 5, level 1: the leaf operations `op_add` / `op_sub` / `op_mul`
// ---------------------------------------------------------------------------

/// `CONFIGS.md` rows 1–24, entry points `op_add`/`op_sub`/`op_mul`.
///
/// All three are exported from *every* build regardless of which one `OP`
/// selected, so all three are driven in every configuration.
#[test]
fn op_leaves_match_on_corners_and_random() {
    let (c, r) = load_pair();

    // Full cross-product of the corner values: 20 x 20 = 400 pairs, covering
    // INT_MAX/INT_MIN overflow for add, sub and mul alike.
    for &a in CORNERS {
        for &b in CORNERS {
            check_leaves(&c, &r, a, b);
        }
    }

    let mut rng = Rng::new(SEED);
    for _ in 0..ITERS {
        let a = rng.next_i32_biased();
        let b = rng.next_i32_biased();
        check_leaves(&c, &r, a, b);
    }
    // A second sweep with pure-uniform full-range values, which is where the
    // multiply wraps essentially every time.
    let mut rng = Rng::new(SEED ^ 0xA5A5_A5A5);
    for _ in 0..ITERS {
        let a = rng.next_i32();
        let b = rng.next_i32();
        check_leaves(&c, &r, a, b);
    }
}

fn check_leaves(c: &Api, r: &Api, a: c_int, b: c_int) {
    let args = format!("{a}, {b}");
    // SAFETY: signatures match the C prototypes; no pointers involved.
    unsafe {
        same("op_add", &args, (c.op_add)(a, b), (r.op_add)(a, b));
        same("op_sub", &args, (c.op_sub)(a, b), (r.op_sub)(a, b));
        same("op_mul", &args, (c.op_mul)(a, b), (r.op_mul)(a, b));
    }
}

// ---------------------------------------------------------------------------
// Axis 5, level 2/3: the exported globals `G_OP` and `G_OP_NAME`
// ---------------------------------------------------------------------------

/// `CONFIGS.md` rows 1–24, entry point `G_OP` (data symbol).
///
/// Checks three things an external consumer relies on:
/// 1. `G_OP` is non-null and points at this library's own selected `op_*`
///    export (`int (*G_OP)(int,int) = OP_FN(OP);`),
/// 2. calling *through* the global gives the same answer in both libraries,
/// 3. the object is 8 bytes of writable `.data` (see `errors.rs` for the store).
#[test]
fn g_op_global_matches() {
    let (c, r) = load_pair();

    let cv = c.g_op_value().expect("C: G_OP must be non-null");
    let rv = r.g_op_value().expect("Rust: G_OP must be non-null");

    // `G_OP` must be the address of the *same* library's selected op export.
    assert_eq!(
        cv as usize, c.selected_op() as usize,
        "C: G_OP should be &op_{OP_NAME}"
    );
    assert_eq!(
        rv as usize, r.selected_op() as usize,
        "Rust: G_OP should be &op_{OP_NAME} (got a different exported function)"
    );

    for &a in CORNERS {
        for &b in CORNERS {
            same(
                "G_OP",
                &format!("{a}, {b}"),
                c.call_g_op(a, b),
                r.call_g_op(a, b),
            );
        }
    }
    let mut rng = Rng::new(SEED ^ 0x1111);
    for _ in 0..ITERS {
        let (a, b) = (rng.next_i32_biased(), rng.next_i32_biased());
        same(
            "G_OP",
            &format!("{a}, {b}"),
            c.call_g_op(a, b),
            r.call_g_op(a, b),
        );
    }
}

/// `CONFIGS.md` rows 1–24, entry point `G_OP_NAME` (data symbol).
///
/// `const char *G_OP_NAME = STR(OP);` must stringify to exactly the `OP` token.
#[test]
fn g_op_name_global_matches() {
    let (c, r) = load_pair();
    let cn = c.g_op_name_bytes();
    let rn = r.g_op_name_bytes();
    assert_eq!(
        cn, rn,
        "G_OP_NAME differs: C={:?} Rust={:?}",
        String::from_utf8_lossy(&cn),
        String::from_utf8_lossy(&rn)
    );
    assert_eq!(
        cn,
        OP_NAME.as_bytes(),
        "G_OP_NAME should be STR(OP) == {OP_NAME:?}"
    );
    assert!(!c.g_op_name_ptr().is_null());
    assert!(!r.g_op_name_ptr().is_null());
}

// ---------------------------------------------------------------------------
// Axis 5, level 4: `helper_ptr`
// ---------------------------------------------------------------------------

/// `CONFIGS.md` rows 1–24, entry point `helper_ptr`.
#[test]
fn helper_ptr_matches() {
    let (c, r) = load_pair();
    for &a in CORNERS {
        for &b in CORNERS {
            // SAFETY: `int helper_ptr(int, int)`.
            unsafe {
                same(
                    "helper_ptr",
                    &format!("{a}, {b}"),
                    (c.helper_ptr)(a, b),
                    (r.helper_ptr)(a, b),
                );
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 0x2222);
    for _ in 0..ITERS {
        let (a, b) = (rng.next_i32_biased(), rng.next_i32_biased());
        // SAFETY: as above.
        unsafe {
            same(
                "helper_ptr",
                &format!("{a}, {b}"),
                (c.helper_ptr)(a, b),
                (r.helper_ptr)(a, b),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Axis 5, level 5: `use_generated` — the only door to `static accum_<OP>`
// ---------------------------------------------------------------------------

/// `CONFIGS.md` rows 1–24, entry point `use_generated`, axis 3 (all `switch`
/// arms).
///
/// This is the important low-level row: `mdmain.c` only ever calls
/// `use_generated(REPEAT)`, so driving it directly is the only way to reach the
/// other six `case` arms of `DISPATCH_REP`. `n` is independent of the build-time
/// `REPEAT`.
#[test]
fn use_generated_matches_all_switch_arms() {
    let (c, r) = load_pair();
    for &n in N_SHAPES {
        // SAFETY: `int use_generated(int)`.
        unsafe {
            same(
                "use_generated",
                &format!("{n}"),
                (c.use_generated)(n),
                (r.use_generated)(n),
            );
        }
    }

    // Randomised: full-range `int`, plus a dense sweep of the in-range window
    // and its immediate neighbourhood.
    let mut rng = Rng::new(SEED ^ 0x3333);
    for _ in 0..ITERS {
        let n = rng.next_i32();
        // SAFETY: as above.
        unsafe {
            same(
                "use_generated",
                &format!("{n}"),
                (c.use_generated)(n),
                (r.use_generated)(n),
            );
        }
    }
    for n in -20..=20 {
        // SAFETY: as above.
        unsafe {
            same(
                "use_generated",
                &format!("{n}"),
                (c.use_generated)(n),
                (r.use_generated)(n),
            );
        }
    }
}

/// Harness sanity check: the C `.so` must agree with the accumulator semantics
/// read straight off `mdmacros.h`, so that a *mutual* bug in the harness cannot
/// let a real divergence slip through.
///
/// `DISPATCH_REP` maps `case k` (`k` in `0..=6`) to `REPk`, which applies
/// `STEP_OP` at indices `0..k`. Hence, starting from `INIT_FOR(OP)`:
/// `add` ⇒ `0+1+..+(k-1)`, `sub` ⇒ `-(0+1+..+(k-1))`, `mul` ⇒ `k!`.
/// Every other `n` takes `default:` and yields `INIT_FOR(OP)`.
#[test]
fn c_accumulator_matches_macro_semantics() {
    let (c, r) = load_pair();
    for &n in N_SHAPES {
        let expected = model_accum(n);
        // SAFETY: `int use_generated(int)`.
        let (cv, rv) = unsafe { ((c.use_generated)(n), (r.use_generated)(n)) };
        assert_eq!(cv, expected, "C use_generated({n}) vs macro model");
        assert_eq!(rv, expected, "Rust use_generated({n}) vs macro model");
    }
}

/// The `DISPATCH_REP` semantics, transcribed from `mdmacros.h`.
fn model_accum(n: c_int) -> c_int {
    if !(0..=6).contains(&n) {
        return INIT_FOR; // `default: break;`
    }
    let mut acc = INIT_FOR;
    for i in 0..n {
        acc = step_model(acc, i);
    }
    acc
}

/// `STEP_OP(OP, acc, i)`, transcribed from `mdmacros.h`.
fn step_model(acc: c_int, i: c_int) -> c_int {
    match OP_NAME {
        "mul" => acc.wrapping_mul(i.wrapping_add(1)),
        "sub" => acc.wrapping_sub(i),
        _ => acc.wrapping_add(i),
    }
}

// ---------------------------------------------------------------------------
// Axis 5, level 6: `helper_call` — the composed pipeline
// ---------------------------------------------------------------------------

/// `CONFIGS.md` rows 1–24, entry point `helper_call`.
///
/// `helper_call` composes three separately-configured things — the selected op,
/// the statically unrolled `RUN_LOOP(OP, acc, REPEAT)`, and the `r + acc` sum —
/// so it is where a `REPEAT`-dependent bug shows up. Bugs in the composed
/// pipeline are invisible to the per-leaf tests above.
#[test]
fn helper_call_matches() {
    let (c, r) = load_pair();
    for &a in CORNERS {
        for &b in CORNERS {
            // SAFETY: `int helper_call(int, int)`.
            unsafe {
                same(
                    "helper_call",
                    &format!("{a}, {b}"),
                    (c.helper_call)(a, b),
                    (r.helper_call)(a, b),
                );
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 0x4444);
    for _ in 0..ITERS {
        let (a, b) = (rng.next_i32_biased(), rng.next_i32_biased());
        // SAFETY: as above.
        unsafe {
            same(
                "helper_call",
                &format!("{a}, {b}"),
                (c.helper_call)(a, b),
                (r.helper_call)(a, b),
            );
        }
    }
}

/// Harness sanity check for `helper_call`, pinning the `RUN_LOOP`/`REPEAT`
/// unroll (which no other test can observe in isolation, since `RUN_LOOP` is
/// inlined into `helper_call` and never exported).
///
/// `RUN_LOOP(op, acc, REPEAT)` = `REP<REPEAT>` = `STEP_OP` at indices
/// `0..REPEAT`. Note this uses `REPEAT` (build-time), *not* the `switch` in
/// `DISPATCH_REP`, so `REPEAT == 7` is fine here even though
/// `use_generated(7)` takes `default:`.
#[test]
fn helper_call_run_loop_unroll_is_repeat_steps() {
    let (c, r) = load_pair();
    let mut acc = INIT_FOR;
    for i in 0..REPEAT {
        acc = step_model(acc, i);
    }
    for &a in CORNERS {
        for &b in CORNERS {
            let op = match OP_NAME {
                "mul" => a.wrapping_mul(b),
                "sub" => a.wrapping_sub(b),
                _ => a.wrapping_add(b),
            };
            let expected = op.wrapping_add(acc);
            // SAFETY: `int helper_call(int, int)`.
            let (cv, rv) = unsafe { ((c.helper_call)(a, b), (r.helper_call)(a, b)) };
            assert_eq!(cv, expected, "C helper_call({a},{b}) vs macro model");
            assert_eq!(rv, expected, "Rust helper_call({a},{b}) vs macro model");
        }
    }
}

// ---------------------------------------------------------------------------
// Cross-entry-point consistency (axis 1: the four things `OP` selects at once)
// ---------------------------------------------------------------------------

/// `CONFIGS.md` rows 25–28: the four selections `OP` drives must stay mutually
/// consistent in the Rust build — `G_OP_NAME` (`STR(OP)`), `G_OP`/`OP_FN(OP)`,
/// `INIT_FOR(OP)` and `STEP_OP(OP, ...)`. A feature-resolution mistake that
/// picked, say, `mul` for `INIT_FOR` but `add` for `op_fn` would still pass the
/// per-symbol tests, but shows up here.
#[test]
fn op_selection_is_self_consistent() {
    let (c, r) = load_pair();

    // (a) STR(OP) agrees with which op_* `G_OP` points at.
    let name = String::from_utf8(r.g_op_name_bytes()).unwrap();
    let gv = r.g_op_value().unwrap() as usize;
    let expected_fn = match name.as_str() {
        "add" => r.op_add,
        "sub" => r.op_sub,
        "mul" => r.op_mul,
        other => panic!("Rust G_OP_NAME is {other:?}, not one of add/sub/mul"),
    } as usize;
    assert_eq!(gv, expected_fn, "Rust: G_OP disagrees with G_OP_NAME");

    // (b) INIT_FOR(OP) is observable as `use_generated(0)` (REP0 is empty).
    // SAFETY: `int use_generated(int)`.
    let (c0, r0) = unsafe { ((c.use_generated)(0), (r.use_generated)(0)) };
    same("use_generated", "0 (== INIT_FOR)", c0, r0);
    assert_eq!(c0, INIT_FOR, "INIT_FOR({OP_NAME}) should be {INIT_FOR}");

    // (c) STEP_OP(OP, ...) is observable as use_generated(1)..use_generated(6).
    for n in 1..=6 {
        // SAFETY: as above.
        let (cv, rv) = unsafe { ((c.use_generated)(n), (r.use_generated)(n)) };
        same("use_generated", &format!("{n}"), cv, rv);
        assert_eq!(cv, model_accum(n));
    }

    // (d) The same op reached three different ways must agree, for both libs.
    let mut rng = Rng::new(SEED ^ 0x5555);
    for _ in 0..ITERS {
        let (a, b) = (rng.next_i32_biased(), rng.next_i32_biased());
        for api in [&c, &r] {
            // SAFETY: signatures match the C prototypes.
            let via_helper_ptr = unsafe { (api.helper_ptr)(a, b) };
            let via_global = api.call_g_op(a, b);
            // SAFETY: as above.
            let via_named = unsafe { (api.selected_op())(a, b) };
            assert_eq!(
                via_helper_ptr, via_global,
                "{}: helper_ptr vs G_OP disagree at ({a},{b})",
                api.tag
            );
            assert_eq!(
                via_global, via_named,
                "{}: G_OP vs op_{OP_NAME} disagree at ({a},{b})",
                api.tag
            );
        }
    }
}

/// `CONFIGS.md` axis 2 boundary: `REPEAT == 0` must produce an *empty* unroll
/// (`REP0` expands to nothing), leaving `acc == INIT_FOR`; and `REPEAT == 7`
/// must produce seven steps. Asserted only in the configurations where it
/// applies, so the test is meaningful rather than vacuous.
#[test]
fn repeat_boundaries() {
    let (c, r) = load_pair();
    let mut acc = INIT_FOR;
    for i in 0..REPEAT {
        acc = step_model(acc, i);
    }
    if REPEAT == 0 {
        assert_eq!(acc, INIT_FOR, "REP0 must be an empty expansion");
    }
    // helper_call(0, 0) isolates `acc`, because op(0,0) is 0 for add/sub/mul.
    // SAFETY: `int helper_call(int, int)`.
    let (cv, rv) = unsafe { ((c.helper_call)(0, 0), (r.helper_call)(0, 0)) };
    same("helper_call", "0, 0", cv, rv);
    assert_eq!(cv, acc, "helper_call(0,0) should expose acc for REPEAT={REPEAT}");
}
