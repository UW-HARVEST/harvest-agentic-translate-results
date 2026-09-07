//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both `.so`s are loaded with `libloading`; every call crosses the FFI
//! boundary. Lowest-level entry points (`op_add`/`op_sub`/`op_mul`, the
//! `G_OP`/`G_OP_NAME` data objects) are exercised directly, then the
//! `helper_ptr`/`helper_call`/`use_generated` layers, then the whole composed
//! pipeline in `main`'s order.

mod common;

use common::*;

const DRAWS: usize = 512;

/// Compare an `(int,int) -> int` export across both libraries over the edge
/// grid plus `DRAWS` randomized draws.
fn diff2(name: &str) {
    let (c, r) = pair();
    let cf = c.f2(name);
    let rf = r.f2(name);

    for &a in EDGE.iter() {
        for &b in EDGE.iter() {
            assert_eq!(cf(a, b), rf(a, b), "{name}({a},{b}) [OP={OP} REPEAT={REPEAT}]");
        }
    }
    let mut rng = Rng::new();
    for _ in 0..DRAWS {
        let (a, b) = (rng.i32_mixed(), rng.i32_mixed());
        assert_eq!(cf(a, b), rf(a, b), "{name}({a},{b}) [OP={OP} REPEAT={REPEAT}]");
    }
}

/* ---- rows 1-2: op_add ------------------------------------------------ */

#[test]
fn row01_row02_op_add_random_and_boundaries() {
    diff2("op_add");
    let (c, r) = pair();
    let (cf, rf) = (c.f2("op_add"), r.f2("op_add"));
    for &(a, b) in &[
        (0, 0),
        (0, 1),
        (1, 0),
        (-1, 1),
        (i32::MAX, 0),
        (i32::MAX, 1),
        (i32::MIN, -1),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
    ] {
        assert_eq!(cf(a, b), rf(a, b), "op_add({a},{b})");
    }
}

/* ---- rows 3-4: op_sub ------------------------------------------------ */

#[test]
fn row03_row04_op_sub_random_and_boundaries() {
    diff2("op_sub");
    let (c, r) = pair();
    let (cf, rf) = (c.f2("op_sub"), r.f2("op_sub"));
    for &(a, b) in &[
        (i32::MAX, -1),
        (i32::MIN, 1),
        (0, i32::MIN),
        (0, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN),
    ] {
        assert_eq!(cf(a, b), rf(a, b), "op_sub({a},{b})");
    }
}

/* ---- rows 5-6: op_mul ----------------------------------------------- */

#[test]
fn row05_row06_op_mul_random_and_boundaries() {
    diff2("op_mul");
    let (c, r) = pair();
    let (cf, rf) = (c.f2("op_mul"), r.f2("op_mul"));
    for &(a, b) in &[
        (0, i32::MIN),
        (1, i32::MIN),
        (-1, i32::MIN),
        (i32::MAX, 2),
        (65_536, 65_536),
        (i32::MIN, i32::MIN),
        (46_341, 46_341),
        (-46_341, 46_341),
    ] {
        assert_eq!(cf(a, b), rf(a, b), "op_mul({a},{b})");
    }
}

/* ---- rows 7-9: G_OP identity ---------------------------------------- */

#[test]
fn row07_row08_row09_g_op_points_at_the_selected_op() {
    let _g = g_op_guard();
    let (c, r) = pair();
    let expected = format!("op_{OP}");

    // In each library, G_OP must hold the address of that library's own op_<OP>.
    assert_eq!(
        unsafe { *c.g_op_slot() },
        c.addr(&expected),
        "C: G_OP != &{expected}"
    );
    assert_eq!(
        unsafe { *r.g_op_slot() },
        r.addr(&expected),
        "Rust: G_OP != &{expected} (OP={OP})"
    );

    // And it must not be any of the other two ops.
    for other in ["op_add", "op_sub", "op_mul"] {
        if other != expected {
            assert_ne!(unsafe { *r.g_op_slot() }, r.addr(other), "Rust: G_OP == &{other}");
        }
    }
}

/* ---- row 10: calling through G_OP ---------------------------------- */

#[test]
fn row10_g_op_call_through() {
    let _g = g_op_guard();
    let (c, r) = pair();
    let (cf, rf) = (c.g_op(), r.g_op());
    for &a in EDGE.iter() {
        for &b in EDGE.iter() {
            assert_eq!(cf(a, b), rf(a, b), "G_OP({a},{b}) [OP={OP}]");
        }
    }
    let mut rng = Rng::new();
    for _ in 0..DRAWS {
        let (a, b) = (rng.i32_mixed(), rng.i32_mixed());
        assert_eq!(cf(a, b), rf(a, b), "G_OP({a},{b}) [OP={OP}]");
    }
}

/* ---- row 11: G_OP is writable exported data ------------------------ */

#[test]
fn row11_g_op_is_writable_and_takes_effect() {
    let _g = g_op_guard();
    let (c, r) = pair();
    let (cs, rs) = (c.g_op_slot(), r.g_op_slot());
    let (c_orig, r_orig) = unsafe { (*cs, *rs) };

    // Overwrite with each of the three ops in turn and call through the slot.
    for name in ["op_add", "op_sub", "op_mul"] {
        unsafe {
            *cs = c.addr(name);
            *rs = r.addr(name);
        }
        let (cf, rf) = (c.g_op(), r.g_op());
        let mut rng = Rng::seeded(0xC0FFEE);
        for _ in 0..128 {
            let (a, b) = (rng.i32_mixed(), rng.i32_mixed());
            assert_eq!(cf(a, b), rf(a, b), "G_OP:={name} then G_OP({a},{b})");
        }
    }

    unsafe {
        *cs = c_orig;
        *rs = r_orig;
    }
    assert_eq!(unsafe { *cs }, c_orig);
    assert_eq!(unsafe { *rs }, r_orig);
}

/* ---- rows 12-14: G_OP_NAME ---------------------------------------- */

#[test]
fn row12_row13_row14_g_op_name() {
    let (c, r) = pair();
    let cn = c.g_op_name();
    let rn = r.g_op_name();
    assert_eq!(cn, rn, "G_OP_NAME bytes differ");
    assert_eq!(cn, OP.as_bytes(), "G_OP_NAME != STR(OP) for OP={OP}");
    assert_eq!(cn.len(), 3);
}

/* ---- rows 15-16: helper_ptr --------------------------------------- */

#[test]
fn row15_helper_ptr() {
    diff2("helper_ptr");
}

#[test]
fn row16_helper_ptr_ignores_a_mutated_g_op() {
    let _g = g_op_guard();
    // C's helper_ptr does `int (*fp)(int,int) = OP_FN(OP);` — it uses the
    // token-pasted op directly, NOT the global G_OP. Overwriting G_OP must
    // therefore leave helper_ptr's result unchanged.
    let (c, r) = pair();
    let (chp, rhp) = (c.f2("helper_ptr"), r.f2("helper_ptr"));
    let (cs, rs) = (c.g_op_slot(), r.g_op_slot());
    let (c_orig, r_orig) = unsafe { (*cs, *rs) };

    let mut rng = Rng::seeded(0xBADF00D);
    let probes: Vec<(i32, i32)> = (0..64).map(|_| (rng.i32_mixed(), rng.i32_mixed())).collect();
    let baseline: Vec<i32> = probes.iter().map(|&(a, b)| chp(a, b)).collect();

    for name in ["op_add", "op_sub", "op_mul"] {
        unsafe {
            *cs = c.addr(name);
            *rs = r.addr(name);
        }
        for (k, &(a, b)) in probes.iter().enumerate() {
            let (cv, rv) = (chp(a, b), rhp(a, b));
            assert_eq!(cv, rv, "helper_ptr({a},{b}) after G_OP:={name} [OP={OP}]");
            assert_eq!(
                cv, baseline[k],
                "C helper_ptr changed after G_OP:={name} — invariant broken"
            );
        }
    }

    unsafe {
        *cs = c_orig;
        *rs = r_orig;
    }
}

/* ---- rows 17-20: helper_call -------------------------------------- */

#[test]
fn row17_row18_row19_helper_call_per_op_and_repeat() {
    diff2("helper_call");

    let (c, r) = pair();
    let (cf, rf) = (c.f2("helper_call"), r.f2("helper_call"));
    let op = c.f2(&format!("op_{OP}"));

    // helper_call returns op(a,b) + <REPEAT-unrolled accumulator>.
    let acc = unrolled(REPEAT);
    let mut rng = Rng::seeded(0x1234_5678);
    for _ in 0..DRAWS {
        let (a, b) = (rng.i32_mixed(), rng.i32_mixed());
        let want = op(a, b).wrapping_add(acc);
        assert_eq!(cf(a, b), want, "C helper_call({a},{b}) [OP={OP} REPEAT={REPEAT}]");
        assert_eq!(rf(a, b), want, "Rust helper_call({a},{b}) [OP={OP} REPEAT={REPEAT}]");
    }

    // Pin the expected accumulator table per (OP, REPEAT).
    let table: [i32; 8] = match OP {
        "add" => [0, 0, 1, 3, 6, 10, 15, 21],
        "sub" => [0, 0, -1, -3, -6, -10, -15, -21],
        _ => [1, 1, 2, 6, 24, 120, 720, 5040],
    };
    assert_eq!(acc, table[REPEAT as usize], "accumulator table mismatch");
}

#[test]
fn row20_helper_call_overflow_boundaries() {
    let (c, r) = pair();
    let (cf, rf) = (c.f2("helper_call"), r.f2("helper_call"));
    for &a in EDGE.iter() {
        for &b in EDGE.iter() {
            assert_eq!(cf(a, b), rf(a, b), "helper_call({a},{b}) [OP={OP} REPEAT={REPEAT}]");
        }
    }
}

/* ---- rows 21-26: use_generated ------------------------------------ */

#[test]
fn row21_row22_row23_row24_use_generated_switch_range() {
    let (c, r) = pair();
    let (cf, rf) = (c.f1("use_generated"), r.f1("use_generated"));
    for n in -2..=10 {
        let cv = cf(n);
        let rv = rf(n);
        assert_eq!(cv, rv, "use_generated({n}) [OP={OP} REPEAT={REPEAT}]");
        assert_eq!(cv, accum_ref(n), "C use_generated({n}) vs reference model");
    }
    // n == 7 is RUN_LOOP-legal but outside DISPATCH_REP's switch.
    assert_eq!(cf(7), INIT, "C use_generated(7) should be INIT_FOR({OP})");
    assert_eq!(rf(7), INIT, "Rust use_generated(7) should be INIT_FOR({OP})");
}

#[test]
fn row25_use_generated_random_full_i32() {
    let (c, r) = pair();
    let (cf, rf) = (c.f1("use_generated"), r.f1("use_generated"));
    let mut rng = Rng::seeded(0xDEAD_BEEF);
    for _ in 0..DRAWS {
        let n = rng.i32_mixed();
        assert_eq!(cf(n), rf(n), "use_generated({n}) [OP={OP} REPEAT={REPEAT}]");
    }
    for &n in EDGE.iter() {
        assert_eq!(cf(n), rf(n), "use_generated({n}) [OP={OP}]");
    }
}

#[test]
fn row26_use_generated_at_repeat() {
    let (c, r) = pair();
    let (cf, rf) = (c.f1("use_generated"), r.f1("use_generated"));
    assert_eq!(cf(REPEAT), rf(REPEAT), "use_generated(REPEAT={REPEAT}) [OP={OP}]");
    assert_eq!(cf(REPEAT), accum_ref(REPEAT));
}

/* ---- row 27: composed pipeline, exactly as `main` drives it -------- */

#[test]
fn row27_composed_pipeline() {
    let _g = g_op_guard();
    let (c, r) = pair();

    let pipeline = |l: &Lib, a: i32, b: i32| -> [i32; 6] {
        let op = l.f2(&format!("op_{OP}"));
        let r_call = op(a, b);
        let acc = {
            // mirrors `int acc = INIT_FOR(OP); RUN_LOOP(OP, acc, REPEAT);`
            let mut acc = INIT;
            let mut i = 0;
            while i < REPEAT {
                acc = step(acc, i);
                i += 1;
            }
            acc
        };
        let x1 = l.f2("helper_call")(a, b);
        let x2 = l.f2("helper_ptr")(a, b);
        let x3 = l.f1("use_generated")(REPEAT);
        let g = l.g_op()(a, b);
        let summary = r_call
            .wrapping_add(acc)
            .wrapping_add(x1)
            .wrapping_add(x2)
            .wrapping_add(x3)
            .wrapping_add(g);
        [r_call, acc, x1, x2, x3, summary]
    };

    let mut rng = Rng::seeded(0xFEED_FACE);
    let mut cases: Vec<(i32, i32)> = Vec::new();
    for &a in EDGE.iter() {
        for &b in EDGE.iter() {
            cases.push((a, b));
        }
    }
    for _ in 0..256 {
        cases.push((rng.i32_mixed(), rng.i32_mixed()));
    }

    for (a, b) in cases {
        assert_eq!(
            pipeline(&c, a, b),
            pipeline(&r, a, b),
            "pipeline({a},{b}) [OP={OP} REPEAT={REPEAT}]"
        );
    }
}
