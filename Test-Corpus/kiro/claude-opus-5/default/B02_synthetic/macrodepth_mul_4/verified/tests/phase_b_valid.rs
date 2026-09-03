//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every call crosses the FFI boundary twice: once into the C reference `.so`
//! (`cbuild/libcdriver_<op>_<repeat>.so`, compiled from `c_src/src/mdcore.c`) and
//! once into the Rust `cdylib` (`target/<profile>/libdriver.so`). Return values and
//! the bytes each side writes to stdout are compared.

mod common;

use common::{
    assert_same, capture_stdout, pair, Rng, BOUNDARY_PAIRS, INIT, OP_TAG, REPEAT, SEED,
};
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Rows 1-7 — the three leaf operations, driven directly.
// All three are exported in every configuration, so the two that `OP` did not
// select must still be exercised.
// ---------------------------------------------------------------------------

fn op_row(row: &str, f: fn(&common::Impl, c_int, c_int) -> c_int) {
    let p = pair();
    for &(a, b) in BOUNDARY_PAIRS {
        let cv = f(&p.c, a, b);
        let rv = f(&p.rust, a, b);
        assert_eq!(cv, rv, "[{OP_TAG}/{REPEAT}] {row} boundary({a}, {b})");
    }
    let mut rng = Rng::new(SEED);
    for i in 0..4096 {
        let (a, b) = (rng.next_operand(), rng.next_operand());
        let cv = f(&p.c, a, b);
        let rv = f(&p.rust, a, b);
        assert_eq!(cv, rv, "[{OP_TAG}/{REPEAT}] {row} random#{i}({a}, {b})");
    }
}

#[test]
fn cfg_01_02_op_add_random_and_boundaries() {
    op_row("op_add", |i, a, b| i.op_add(a, b));
}

#[test]
fn cfg_03_04_op_sub_random_and_boundaries() {
    op_row("op_sub", |i, a, b| i.op_sub(a, b));
}

#[test]
fn cfg_05_06_op_mul_random_and_boundaries() {
    op_row("op_mul", |i, a, b| i.op_mul(a, b));
}

#[test]
fn cfg_07_non_selected_ops_still_exported_and_correct() {
    // Row 7: whichever op OP picked, the other two are still part of the ABI.
    let p = pair();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..1024 {
        let (a, b) = (rng.next_operand(), rng.next_operand());
        assert_eq!(p.c.op_add(a, b), p.rust.op_add(a, b), "op_add({a},{b})");
        assert_eq!(p.c.op_sub(a, b), p.rust.op_sub(a, b), "op_sub({a},{b})");
        assert_eq!(p.c.op_mul(a, b), p.rust.op_mul(a, b), "op_mul({a},{b})");
        // And the arithmetic itself, independent of the C, so a jointly-wrong
        // pair cannot pass.
        assert_eq!(p.rust.op_add(a, b), a.wrapping_add(b));
        assert_eq!(p.rust.op_sub(a, b), a.wrapping_sub(b));
        assert_eq!(p.rust.op_mul(a, b), a.wrapping_mul(b));
    }
}

// ---------------------------------------------------------------------------
// Rows 8-9 — helper_ptr: OP through a local function pointer, plus one printf.
// ---------------------------------------------------------------------------

#[test]
fn cfg_08_helper_ptr_random() {
    let mut rng = Rng::new(SEED ^ 0x08);
    for i in 0..1024 {
        let (a, b) = (rng.next_operand(), rng.next_operand());
        assert_same(&format!("helper_ptr random#{i}({a}, {b})"), |imp| {
            imp.helper_ptr(a, b)
        });
    }
}

#[test]
fn cfg_09_helper_ptr_boundaries() {
    for &(a, b) in BOUNDARY_PAIRS {
        assert_same(&format!("helper_ptr boundary({a}, {b})"), |imp| {
            imp.helper_ptr(a, b)
        });
    }
}

// ---------------------------------------------------------------------------
// Rows 10-11 — helper_call: the only entry point depending on OP *and* REPEAT.
// ---------------------------------------------------------------------------

#[test]
fn cfg_10_helper_call_random() {
    let mut rng = Rng::new(SEED ^ 0x10);
    for i in 0..1024 {
        let (a, b) = (rng.next_operand(), rng.next_operand());
        assert_same(&format!("helper_call random#{i}({a}, {b})"), |imp| {
            imp.helper_call(a, b)
        });
    }
}

#[test]
fn cfg_11_helper_call_boundaries() {
    for &(a, b) in BOUNDARY_PAIRS {
        assert_same(&format!("helper_call boundary({a}, {b})"), |imp| {
            imp.helper_call(a, b)
        });
    }
}

#[test]
fn cfg_10b_helper_call_acc_matches_rep_unrolling() {
    // Independently pins `acc` to the REP<REPEAT> semantics from mdmacros.h,
    // so both sides agreeing on a wrong value would still fail.
    let expected_acc: c_int = match OP_TAG {
        "add" => (0..REPEAT).sum(),
        "sub" => -(0..REPEAT).sum::<c_int>(),
        "mul" => (0..REPEAT).fold(1, |acc, i| acc * (i + 1)),
        other => panic!("unexpected OP {other}"),
    };
    let (_, out) = capture_stdout(|| pair().c.helper_call(7, 3));
    let text = String::from_utf8_lossy(&out).into_owned();
    assert!(
        text.contains(&format!("helper.acc={expected_acc}\n")),
        "[{OP_TAG}/{REPEAT}] C helper_call printed {text:?}, expected acc={expected_acc}"
    );
    let (_, out_r) = capture_stdout(|| pair().rust.helper_call(7, 3));
    assert_eq!(text, String::from_utf8_lossy(&out_r));
}

// ---------------------------------------------------------------------------
// Rows 12-20 — use_generated / DISPATCH_REP's switch, one row per `case`.
// ---------------------------------------------------------------------------

fn use_generated_row(n: c_int) {
    assert_same(&format!("use_generated({n})"), |imp| imp.use_generated(n));
}

#[test]
fn cfg_12_use_generated_case_0() {
    use_generated_row(0);
}
#[test]
fn cfg_13_use_generated_case_1() {
    use_generated_row(1);
}
#[test]
fn cfg_14_use_generated_case_2() {
    use_generated_row(2);
}
#[test]
fn cfg_15_use_generated_case_3() {
    use_generated_row(3);
}
#[test]
fn cfg_16_use_generated_case_4() {
    use_generated_row(4);
}
#[test]
fn cfg_17_use_generated_case_5() {
    use_generated_row(5);
}
#[test]
fn cfg_18_use_generated_case_6() {
    use_generated_row(6);
}

#[test]
fn cfg_19_use_generated_of_repeat() {
    // The call mdmain.c:42 actually makes. Falls into `default` when REPEAT == 7.
    use_generated_row(REPEAT);
    let expected: c_int = match (OP_TAG, REPEAT) {
        (_, 7) => INIT, // no `case 7:` in DISPATCH_REP
        ("add", r) => (0..r).sum(),
        ("sub", r) => -(0..r).sum::<c_int>(),
        ("mul", r) => (0..r).fold(1, |acc, i| acc * (i + 1)),
        (other, _) => panic!("unexpected OP {other}"),
    };
    let p = pair();
    let (cv, _) = capture_stdout(|| p.c.use_generated(REPEAT));
    assert_eq!(
        cv, expected,
        "[{OP_TAG}/{REPEAT}] C use_generated(REPEAT) should be {expected}"
    );
}

#[test]
fn cfg_20_use_generated_random_selectors() {
    let mut rng = Rng::new(SEED ^ 0x20);
    // Every `case` plus a spread of `default` selectors.
    for n in -3..=10 {
        assert_same(&format!("use_generated dense({n})"), |imp| {
            imp.use_generated(n)
        });
    }
    for i in 0..2048 {
        // Mostly near the switch range, some full-range, so both partitions are hit.
        let n = if i % 3 == 0 {
            (rng.next_u64() % 21) as c_int - 4
        } else {
            rng.next_i32()
        };
        assert_same(&format!("use_generated random#{i}({n})"), |imp| {
            imp.use_generated(n)
        });
    }
}

// ---------------------------------------------------------------------------
// Rows 21-23 — the G_OP data slot.
// ---------------------------------------------------------------------------

#[test]
fn cfg_21_g_op_dispatches_like_selected_op() {
    let p = pair();
    let cf = p.c.g_op();
    let rf = p.rust.g_op();
    let mut rng = Rng::new(SEED ^ 0x21);
    for _ in 0..1024 {
        let (a, b) = (rng.next_operand(), rng.next_operand());
        let cv = unsafe { cf(a, b) };
        let rv = unsafe { rf(a, b) };
        assert_eq!(cv, rv, "[{OP_TAG}/{REPEAT}] G_OP({a}, {b})");
        // ...and it is the op OP selected, per OP_FN(OP) = CAT(op_, OP).
        let direct = match OP_TAG {
            "add" => p.c.op_add(a, b),
            "sub" => p.c.op_sub(a, b),
            _ => p.c.op_mul(a, b),
        };
        assert_eq!(cv, direct, "G_OP is not op_{OP_TAG}");
    }
}

#[test]
fn cfg_22_g_op_points_at_own_op_export() {
    let p = pair();
    assert_eq!(
        p.c.g_op() as usize,
        p.c.op_addr(OP_TAG),
        "[{OP_TAG}/{REPEAT}] C G_OP != &op_{OP_TAG}"
    );
    assert_eq!(
        p.rust.g_op() as usize,
        p.rust.op_addr(OP_TAG),
        "[{OP_TAG}/{REPEAT}] Rust G_OP != &op_{OP_TAG}"
    );
}

#[test]
fn cfg_23_g_op_writable() {
    // `int (*G_OP)(int,int)` is not const, so a caller may rebind it. Both sides
    // must then dispatch through the stored pointer, and must restore cleanly.
    let p = pair();
    let c_orig = p.c.g_op();
    let r_orig = p.rust.g_op();

    for other in ["add", "sub", "mul"] {
        let cf: common::OpFn = unsafe { std::mem::transmute(p.c.op_addr(other)) };
        let rf: common::OpFn = unsafe { std::mem::transmute(p.rust.op_addr(other)) };
        p.c.set_g_op(cf);
        p.rust.set_g_op(rf);
        let mut rng = Rng::new(SEED ^ 0x23);
        for _ in 0..256 {
            let (a, b) = (rng.next_operand(), rng.next_operand());
            let cv = unsafe { (p.c.g_op())(a, b) };
            let rv = unsafe { (p.rust.g_op())(a, b) };
            assert_eq!(cv, rv, "[{OP_TAG}/{REPEAT}] rebound G_OP=op_{other}({a},{b})");
            let expected = match other {
                "add" => a.wrapping_add(b),
                "sub" => a.wrapping_sub(b),
                _ => a.wrapping_mul(b),
            };
            assert_eq!(cv, expected, "rebound G_OP did not call op_{other}");
        }
    }

    p.c.set_g_op(c_orig);
    p.rust.set_g_op(r_orig);
    assert_eq!(p.c.g_op() as usize, p.c.op_addr(OP_TAG));
    assert_eq!(p.rust.g_op() as usize, p.rust.op_addr(OP_TAG));
}

// ---------------------------------------------------------------------------
// Row 24 — G_OP_NAME.
// ---------------------------------------------------------------------------

#[test]
fn cfg_24_g_op_name_bytes() {
    let p = pair();
    let c_name = p.c.g_op_name();
    let r_name = p.rust.g_op_name();
    assert_eq!(
        String::from_utf8_lossy(&c_name),
        String::from_utf8_lossy(&r_name),
        "[{OP_TAG}/{REPEAT}] G_OP_NAME differs"
    );
    assert_eq!(
        c_name,
        OP_TAG.as_bytes(),
        "[{OP_TAG}/{REPEAT}] STR(OP) should be {OP_TAG:?}"
    );
}

// ---------------------------------------------------------------------------
// Row 25 — the composed pipeline, one captured stdout stream.
// ---------------------------------------------------------------------------

#[test]
fn cfg_25_full_pipeline_single_stream() {
    let mut rng = Rng::new(SEED ^ 0x25);
    for i in 0..256 {
        let (a, b) = (rng.next_operand(), rng.next_operand());
        let n = if i % 4 == 0 { REPEAT } else { rng.next_i32() };
        let p = pair();

        let run = |imp: &common::Impl| {
            let x1 = imp.helper_call(a, b);
            let x2 = imp.helper_ptr(a, b);
            let x3 = imp.use_generated(n);
            let g = unsafe { (imp.g_op())(a, b) };
            let r_call = match OP_TAG {
                "add" => imp.op_add(a, b),
                "sub" => imp.op_sub(a, b),
                _ => imp.op_mul(a, b),
            };
            // The summary line mdmain.c:46 computes.
            r_call
                .wrapping_add(x1)
                .wrapping_add(x2)
                .wrapping_add(x3)
                .wrapping_add(g)
        };

        let (cv, cout) = capture_stdout(|| run(&p.c));
        let (rv, rout) = capture_stdout(|| run(&p.rust));
        assert_eq!(
            cv, rv,
            "[{OP_TAG}/{REPEAT}] pipeline#{i} a={a} b={b} n={n}: summand differs"
        );
        assert_eq!(
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            "[{OP_TAG}/{REPEAT}] pipeline#{i} a={a} b={b} n={n}: stdout differs"
        );
        // Three printf lines, in order.
        assert_eq!(
            cout.iter().filter(|&&c| c == b'\n').count(),
            3,
            "expected 3 lines from the pipeline, got {:?}",
            String::from_utf8_lossy(&cout)
        );
    }
}
