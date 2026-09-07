//! Phase B -- valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both sides are driven exclusively through `.so` exports resolved by name.

mod harness;

use core::ffi::c_int;
use harness::{capture_stdout, Pair, Rng, Side, GRID};

/// Rows 1, 3, 5: randomized full-range sweep over a leaf `op_*`.
fn leaf_random(name: &str) {
    let p = Pair::load();
    let c = p.fn2(Side::C, name);
    let r = p.fn2(Side::Rust, name);
    let mut rng = Rng::new();
    for k in 0..4096 {
        let (a, b) = (rng.next_mixed_i32(), rng.next_mixed_i32());
        assert_eq!(
            c(a, b),
            r(a, b),
            "{name}({a}, {b}) diverged at iteration {k}"
        );
    }
}

/// Rows 2, 4, 6: the 13x13 boundary grid (overflow corners included).
fn leaf_grid(name: &str) {
    let p = Pair::load();
    let c = p.fn2(Side::C, name);
    let r = p.fn2(Side::Rust, name);
    for &a in GRID.iter() {
        for &b in GRID.iter() {
            assert_eq!(c(a, b), r(a, b), "{name}({a}, {b}) diverged");
        }
    }
}

#[test]
fn row01_op_add_random() {
    leaf_random("op_add");
}

#[test]
fn row02_op_add_grid() {
    leaf_grid("op_add");
}

#[test]
fn row03_op_sub_random() {
    leaf_random("op_sub");
}

#[test]
fn row04_op_sub_grid() {
    leaf_grid("op_sub");
}

#[test]
fn row05_op_mul_random() {
    leaf_random("op_mul");
}

#[test]
fn row06_op_mul_grid() {
    leaf_grid("op_mul");
}

/// Row 7: small-magnitude sweep for all three leaves.
#[test]
fn row07_leaves_small_sweep() {
    let p = Pair::load();
    for name in ["op_add", "op_sub", "op_mul"] {
        let c = p.fn2(Side::C, name);
        let r = p.fn2(Side::Rust, name);
        for a in -8..=8 {
            for b in -8..=8 {
                assert_eq!(c(a, b), r(a, b), "{name}({a}, {b}) diverged");
            }
        }
    }
}

/// Row 8: the `G_OP` data export -- behaviour must match C's *and* must be the
/// `op_<OP>` the active build selected.
#[test]
fn row08_g_op_random_and_selection() {
    let p = Pair::load();
    let gc = p.g_op(Side::C);
    let gr = p.g_op(Side::Rust);

    let mut rng = Rng::new();
    for k in 0..4096 {
        let (a, b) = (rng.next_mixed_i32(), rng.next_mixed_i32());
        assert_eq!(gc(a, b), gr(a, b), "G_OP({a}, {b}) diverged at {k}");
    }

    // G_OP must point at op_<MD_OP> in *both* libraries.
    let expected = format!("op_{}", p.cfg.op);
    let leaf_c = p.fn2(Side::C, &expected);
    let leaf_r = p.fn2(Side::Rust, &expected);
    let mut rng = Rng::new();
    for _ in 0..512 {
        let (a, b) = (rng.next_mixed_i32(), rng.next_mixed_i32());
        assert_eq!(gc(a, b), leaf_c(a, b), "C G_OP is not {expected}");
        assert_eq!(gr(a, b), leaf_r(a, b), "Rust G_OP is not {expected}");
    }
}

/// Row 9: `G_OP` non-NULL (asserted while reading) plus grid behaviour.
#[test]
fn row09_g_op_grid() {
    let p = Pair::load();
    let gc = p.g_op(Side::C);
    let gr = p.g_op(Side::Rust);
    for &a in GRID.iter() {
        for &b in GRID.iter() {
            assert_eq!(gc(a, b), gr(a, b), "G_OP({a}, {b}) diverged");
        }
    }
}

/// Row 10: the `G_OP_NAME` data export.
#[test]
fn row10_g_op_name() {
    let p = Pair::load();
    let c = p.g_op_name(Side::C);
    let r = p.g_op_name(Side::Rust);
    assert_eq!(
        c,
        r,
        "G_OP_NAME diverged: C={:?} Rust={:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    assert_eq!(
        c.as_slice(),
        p.cfg.op.as_bytes(),
        "G_OP_NAME must equal STR(OP) for this build"
    );
}

/// Rows 11-14: the printing helpers. Return value *and* the exact bytes each
/// `.so` writes to stdout are compared.
fn helper_case(name: &str, inputs: &[(c_int, c_int)]) {
    let p = Pair::load();

    let out_c = {
        let c = p.fn2(Side::C, name);
        let mut rets = Vec::with_capacity(inputs.len());
        let bytes = capture_stdout(&format!("c-{name}"), || {
            for &(a, b) in inputs {
                rets.push(c(a, b));
            }
        });
        (rets, bytes)
    };
    let out_r = {
        let r = p.fn2(Side::Rust, name);
        let mut rets = Vec::with_capacity(inputs.len());
        let bytes = capture_stdout(&format!("rs-{name}"), || {
            for &(a, b) in inputs {
                rets.push(r(a, b));
            }
        });
        (rets, bytes)
    };

    for (i, (&(a, b), (cv, rv))) in inputs
        .iter()
        .zip(out_c.0.iter().zip(out_r.0.iter()))
        .enumerate()
    {
        assert_eq!(cv, rv, "{name}({a}, {b}) return diverged at index {i}");
    }
    assert_eq!(
        out_c.1.len(),
        out_r.1.len(),
        "{name} stdout length diverged ({} vs {})",
        out_c.1.len(),
        out_r.1.len()
    );
    if out_c.1 != out_r.1 {
        let at = out_c
            .1
            .iter()
            .zip(out_r.1.iter())
            .position(|(x, y)| x != y)
            .unwrap();
        let lo = at.saturating_sub(80);
        panic!(
            "{name} stdout diverged at byte {at}\n C: {:?}\n R: {:?}",
            String::from_utf8_lossy(&out_c.1[lo..(at + 80).min(out_c.1.len())]),
            String::from_utf8_lossy(&out_r.1[lo..(at + 80).min(out_r.1.len())]),
        );
    }
}

fn random_pairs(n: usize) -> Vec<(c_int, c_int)> {
    let mut rng = Rng::new();
    (0..n)
        .map(|_| (rng.next_mixed_i32(), rng.next_mixed_i32()))
        .collect()
}

fn grid_pairs() -> Vec<(c_int, c_int)> {
    let mut v = Vec::new();
    for &a in GRID.iter() {
        for &b in GRID.iter() {
            v.push((a, b));
        }
    }
    v
}

#[test]
fn row11_helper_ptr_random() {
    helper_case("helper_ptr", &random_pairs(4096));
}

#[test]
fn row12_helper_ptr_grid() {
    helper_case("helper_ptr", &grid_pairs());
}

#[test]
fn row13_helper_call_random() {
    helper_case("helper_call", &random_pairs(4096));
}

#[test]
fn row14_helper_call_grid() {
    helper_case("helper_call", &grid_pairs());
}

/// Rows 15-18: `use_generated`, i.e. the `static` macro-generated
/// `accum_<OP>(n)` behind `DISPATCH_REP`'s `switch`.
fn use_generated_case(tag: &str, inputs: &[c_int]) {
    let p = Pair::load();

    let (rc, bc) = {
        let f = p.fn1(Side::C, "use_generated");
        let mut rets = Vec::new();
        let bytes = capture_stdout(&format!("c-ug-{tag}"), || {
            for &n in inputs {
                rets.push(f(n));
            }
        });
        (rets, bytes)
    };
    let (rr, br) = {
        let f = p.fn1(Side::Rust, "use_generated");
        let mut rets = Vec::new();
        let bytes = capture_stdout(&format!("rs-ug-{tag}"), || {
            for &n in inputs {
                rets.push(f(n));
            }
        });
        (rets, bytes)
    };

    for (i, &n) in inputs.iter().enumerate() {
        assert_eq!(rc[i], rr[i], "use_generated({n}) diverged at index {i}");
    }
    assert_eq!(bc, br, "use_generated stdout diverged for {tag}");
}

#[test]
fn row15_use_generated_in_range() {
    use_generated_case("in-range", &[0, 1, 2, 3, 4, 5, 6]);
}

/// Row 16: exactly what `mdmain.c:42` passes -- `use_generated(REPEAT)`.
/// For `REPEAT == 7` this must fall to `DISPATCH_REP`'s `default:` arm.
#[test]
fn row16_use_generated_at_repeat() {
    let p = Pair::load();
    let n = p.cfg.repeat;
    drop(p);
    use_generated_case("at-repeat", &[n]);

    let p = Pair::load();
    let c = p.fn1(Side::C, "use_generated");
    let got = capture_stdout("c-ug-repeat-value", || {
        let _ = c(p.cfg.repeat);
    });
    // Sanity-check the C ground truth against the documented asymmetry.
    if p.cfg.repeat == 7 {
        assert_eq!(
            String::from_utf8_lossy(&got),
            format!("gen.acc={}\n", p.cfg.init()),
            "REPEAT=7 must hit DISPATCH_REP's default arm"
        );
    }
}

#[test]
fn row17_use_generated_random() {
    let mut rng = Rng::new();
    let mut v: Vec<c_int> = (0..4096).map(|_| rng.next_mixed_i32()).collect();
    // Guarantee in-range hits are mixed in with the `default`-arm majority.
    for n in 0..=6 {
        v.push(n);
    }
    use_generated_case("random", &v);
}

#[test]
fn row18_use_generated_edge_sweep() {
    let v: Vec<c_int> = (-16..=16).collect();
    use_generated_case("edges", &v);
}

/// Row 19: the composed pipeline, in the order `mdmain.c` performs it, using the
/// low-level exports rather than any convenience wrapper.
///
/// `RUN_LOOP(OP, acc, REPEAT)` is not an exported symbol (it is preprocessor-only
/// in C), so it is recovered the only way an external caller can:
/// `helper_call(a, b) - OP_FN(a, b)`, derived identically on both sides.
#[test]
fn row19_composed_pipeline() {
    let p = Pair::load();
    let leaf = format!("op_{}", p.cfg.op);

    let pairs = {
        let mut v = random_pairs(1024);
        v.extend(grid_pairs());
        v
    };

    let run = |side: harness::Side, tag: &str| -> (Vec<[c_int; 7]>, Vec<u8>) {
        let op = p.fn2(side, &leaf);
        let hc = p.fn2(side, "helper_call");
        let hp = p.fn2(side, "helper_ptr");
        let ug = p.fn1(side, "use_generated");
        let g = p.g_op(side);
        let repeat = p.cfg.repeat;

        let mut rows = Vec::with_capacity(pairs.len());
        let bytes = capture_stdout(tag, || {
            for &(a, b) in pairs.iter() {
                let r_call = op(a, b);
                let x1 = hc(a, b);
                let acc = x1.wrapping_sub(r_call);
                let x2 = hp(a, b);
                let x3 = ug(repeat);
                let gv = g(a, b);
                let summary = r_call
                    .wrapping_add(acc)
                    .wrapping_add(x1)
                    .wrapping_add(x2)
                    .wrapping_add(x3)
                    .wrapping_add(gv);
                rows.push([r_call, acc, x1, x2, x3, gv, summary]);
            }
        });
        (rows, bytes)
    };

    let (rows_c, bytes_c) = run(Side::C, "c-pipeline");
    let (rows_r, bytes_r) = run(Side::Rust, "rs-pipeline");

    for (i, (&(a, b), (rc, rr))) in pairs
        .iter()
        .zip(rows_c.iter().zip(rows_r.iter()))
        .enumerate()
    {
        assert_eq!(
            rc, rr,
            "pipeline diverged at index {i} for (a, b) = ({a}, {b})\n\
             fields: [r_call, acc, x1, x2, x3, g, summary]"
        );
    }
    assert_eq!(bytes_c, bytes_r, "pipeline stdout diverged");
}
