//! Phase B — valid-path differential tests, one per `CONFIGS.md` row.
//!
//! Every call goes through `dlopen`/`dlsym` on both shared objects; return values
//! *and* the exact `printf` bytes are compared.

mod common;

use common::*;
use std::ffi::c_int;

/* ------------------------------------------------------------------ */
/* Guard: the loaded Rust .so really is the configuration we expect    */
/* ------------------------------------------------------------------ */

/// Protects every other test in the suite from silently passing against a stale
/// `libdriver.so` left behind by a different feature combination.
#[test]
fn row0_loaded_rust_so_matches_the_feature_set() {
    let (c, r) = libs();

    // OP is observable through G_OP_NAME ...
    assert_eq!(
        show(&r.g_op_name_str()),
        OP,
        "the loaded Rust .so was built for OP={} but this test binary for OP={OP}. \
         Rebuild with matching features (./check_all.sh).",
        show(&r.g_op_name_str())
    );
    assert_eq!(show(&c.g_op_name_str()), OP, "C .so built for the wrong OP");

    // ... and REPEAT through the `helper.acc=` field printed by helper_call.
    let expect_acc = expected_acc(REPEAT);
    for (lbl, l) in [("C", c), ("Rust", r)] {
        let f = l.func2("helper_call");
        let (_, out) = capture(|| unsafe { f(0, 0) });
        let s = show(&out);
        let acc: c_int = s
            .split("helper.acc=")
            .nth(1)
            .and_then(|t| t.trim().lines().next())
            .and_then(|t| t.trim().parse().ok())
            .unwrap_or_else(|| panic!("cannot parse helper.acc from {s:?}"));
        assert_eq!(
            acc, expect_acc,
            "{lbl} .so was built for a different REPEAT than {REPEAT}"
        );
    }
}

/// `RUN_LOOP(OP, INIT_FOR(OP), n)` computed independently of the library, from
/// the macro definitions in `mdmacros.h`.
fn expected_acc(n: c_int) -> c_int {
    let mut acc = init_for_op();
    for i in 0..n {
        acc = match OP {
            "add" => acc.wrapping_add(i),
            "sub" => acc.wrapping_sub(i),
            _ => acc.wrapping_mul(i.wrapping_add(1)),
        };
    }
    acc
}

fn inputs() -> Vec<(c_int, c_int)> {
    let mut v = Vec::new();
    // Rows 1-3: full boundary cross-product (81 pairs).
    for a in BOUNDARIES {
        for b in BOUNDARIES {
            v.push((a, b));
        }
    }
    // ... plus 256 randomized pairs from the fixed seed.
    let mut rng = Rng::new();
    for _ in 0..256 {
        v.push((rng.next_mixed(), rng.next_mixed()));
    }
    v
}

/* ------------------------------------------------------------------ */
/* Rows 1-4: the lowest-level entry points                            */
/* ------------------------------------------------------------------ */

#[test]
fn row1_op_add() {
    for (a, b) in inputs() {
        let got = diff2("op_add", a, b);
        assert_eq!(got, a.wrapping_add(b), "op_add({a},{b})");
    }
}

#[test]
fn row2_op_sub() {
    for (a, b) in inputs() {
        let got = diff2("op_sub", a, b);
        assert_eq!(got, a.wrapping_sub(b), "op_sub({a},{b})");
    }
}

#[test]
fn row3_op_mul() {
    for (a, b) in inputs() {
        let got = diff2("op_mul", a, b);
        assert_eq!(got, a.wrapping_mul(b), "op_mul({a},{b})");
    }
}

#[test]
fn row4_non_selected_ops_are_still_exported_and_correct() {
    // mdcore.c defines all three op_* unconditionally, whatever OP is; a
    // translation that only emitted the selected one would fail here.
    let mut rng = Rng::with_seed(0xA5A5_1111);
    for _ in 0..128 {
        let (a, b) = (rng.next_mixed(), rng.next_mixed());
        for f in ["op_add", "op_sub", "op_mul"] {
            diff2(f, a, b);
        }
    }
}

/* ------------------------------------------------------------------ */
/* Rows 5-7: helper_ptr (depends on OP only)                          */
/* ------------------------------------------------------------------ */

#[test]
fn rows5_7_helper_ptr() {
    for (a, b) in inputs() {
        let got = diff2("helper_ptr", a, b);
        let want = match OP {
            "add" => a.wrapping_add(b),
            "sub" => a.wrapping_sub(b),
            _ => a.wrapping_mul(b),
        };
        assert_eq!(got, want, "helper_ptr({a},{b}) with OP={OP}");
    }
}

/* ------------------------------------------------------------------ */
/* Rows 8-17: helper_call (depends on OP x REPEAT)                    */
/* ------------------------------------------------------------------ */

#[test]
fn rows8_17_helper_call() {
    let acc = expected_acc(REPEAT);
    for (a, b) in inputs() {
        let got = diff2("helper_call", a, b);
        let r = match OP {
            "add" => a.wrapping_add(b),
            "sub" => a.wrapping_sub(b),
            _ => a.wrapping_mul(b),
        };
        assert_eq!(
            got,
            r.wrapping_add(acc),
            "helper_call({a},{b}) with OP={OP} REPEAT={REPEAT} (acc should be {acc})"
        );
    }
}

#[test]
fn rows8_17_helper_call_prints_the_unrolled_acc() {
    // The `helper.acc=` field pins the REPEAT-selected unrolling, and is the only
    // externally visible evidence of REP0..REP7.
    let (c, r) = libs();
    let cf = c.func2("helper_call");
    let rf = r.func2("helper_call");
    let (_, cout) = capture(|| unsafe { cf(7, 3) });
    let (_, rout) = capture(|| unsafe { rf(7, 3) });
    assert_eq!(show(&cout), show(&rout));
    let want = format!(
        "helper.call={} helper.acc={}\n",
        match OP {
            "add" => 10,
            "sub" => 4,
            _ => 21,
        },
        expected_acc(REPEAT)
    );
    assert_eq!(show(&cout), want, "helper_call stdout for OP={OP} REPEAT={REPEAT}");
}

/* ------------------------------------------------------------------ */
/* Rows 18-21: use_generated / accum_<OP>                             */
/* ------------------------------------------------------------------ */

#[test]
fn rows18_20_use_generated_every_switch_case() {
    // DISPATCH_REP has case labels 0..=6; each is a distinct code path.
    for n in 0..=6 {
        let got = diff1("use_generated", n);
        assert_eq!(
            got,
            expected_acc(n),
            "use_generated({n}) with OP={OP} (REPEAT must not influence this)"
        );
    }
}

#[test]
fn rows18_20_use_generated_is_repeat_independent() {
    // accum_<OP>'s switch is emitted in full regardless of REPEAT, so the result
    // depends only on OP and n -- a Rust version that reused the REPEAT-unrolled
    // loop here would diverge for n != REPEAT.
    let expect: [c_int; 7] = match OP {
        "add" => [0, 0, 1, 3, 6, 10, 15],
        "sub" => [0, 0, -1, -3, -6, -10, -15],
        _ => [1, 1, 2, 6, 24, 120, 720],
    };
    for (n, want) in expect.iter().enumerate() {
        let got = diff1("use_generated", n as c_int);
        assert_eq!(got, *want, "use_generated({n}) with OP={OP}");
    }
}

#[test]
fn row21_use_generated_sweep_valid_and_default_arms() {
    for n in -8..=15 {
        diff1("use_generated", n);
    }
    for n in [c_int::MIN, c_int::MIN + 1, c_int::MAX - 1, c_int::MAX] {
        diff1("use_generated", n);
    }
    let mut rng = Rng::with_seed(0xBEEF_0021);
    for _ in 0..200 {
        diff1("use_generated", rng.next_mixed());
    }
}

/* ------------------------------------------------------------------ */
/* Rows 26-27: composition / interleaving                             */
/* ------------------------------------------------------------------ */

#[test]
fn interleaved_call_sequence() {
    // Row 26 (also ERRORS.md G5): drive the library the way a consumer does --
    // many calls in one process, interleaved, sharing the globals and the stdio
    // buffer. Compared as one contiguous stdout blob per implementation.
    let (c, r) = libs();
    let mut rng_c = Rng::with_seed(0xC0FFEE);
    let mut rng_r = Rng::with_seed(0xC0FFEE);

    let run = |l: &Lib, rng: &mut Rng| -> Vec<c_int> {
        let hc = l.func2("helper_call");
        let hp = l.func2("helper_ptr");
        let ug = l.func1("use_generated");
        let oa = l.func2("op_add");
        let os = l.func2("op_sub");
        let om = l.func2("op_mul");
        let gop = l.g_op();
        let mut acc = Vec::new();
        for k in 0..40 {
            let (a, b) = (rng.next_mixed(), rng.next_mixed());
            unsafe {
                acc.push(hc(a, b));
                acc.push(ug(rng.range(-3, 10)));
                acc.push(hp(a, b));
                acc.push(oa(a, b));
                acc.push(os(a, b));
                acc.push(om(a, b));
                acc.push((*gop)(a, b));
                if k % 7 == 3 {
                    acc.push(ug(REPEAT));
                }
            }
        }
        acc
    };

    let (cv, cout) = capture(|| run(c, &mut rng_c));
    let (rv, rout) = capture(|| run(r, &mut rng_r));
    assert_eq!(cv, rv, "interleaved return values diverge");
    assert_eq!(
        show(&cout),
        show(&rout),
        "interleaved stdout diverges [OP={OP} REPEAT={REPEAT}]"
    );
}

#[test]
fn composed_pipeline_feedback() {
    // Row 27: feed each result into the next call so the inputs become
    // value-dependent (and self-induce overflow), covering states a
    // per-function test with hand-picked scalars never reaches.
    let (c, r) = libs();

    let run = |l: &Lib| -> Vec<c_int> {
        let hc = l.func2("helper_call");
        let hp = l.func2("helper_ptr");
        let ug = l.func1("use_generated");
        let gop = l.g_op();
        let mut x: c_int = 3;
        let mut y: c_int = -7;
        let mut trace = Vec::new();
        for _ in 0..64 {
            unsafe {
                let t = hc(x, y);
                let u = hp(y, t);
                let v = ug(t & 0xF);
                let w = (*gop)(u, v);
                trace.extend_from_slice(&[t, u, v, w]);
                x = w;
                y = t ^ v;
            }
        }
        trace
    };

    let (cv, cout) = capture(|| run(c));
    let (rv, rout) = capture(|| run(r));
    assert_eq!(cv, rv, "composed pipeline return values diverge");
    assert_eq!(show(&cout), show(&rout), "composed pipeline stdout diverges");
}
