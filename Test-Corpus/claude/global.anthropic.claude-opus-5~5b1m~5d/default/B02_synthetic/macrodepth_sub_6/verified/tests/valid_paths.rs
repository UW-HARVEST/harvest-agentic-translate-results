//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every call goes through `dlopen`/`dlsym` on
//! both the C `.so` and the Rust `.so`; the Rust crate is never invoked
//! directly, so the `#[no_mangle]` wrappers are exercised too.

mod common;

use common::{capture_stdout, pair, Rng, EDGE_I32, INIT, OP, REPEAT};
use std::ffi::c_int;

/* =============================== rows 1-6 =============================== */
/* The three unconditional operation primitives.                            */

fn diff_bin2(sym: &str, seed: u64, iters: usize) {
    let p = pair();
    let c = p.c.bin2(sym);
    let r = p.r.bin2(sym);

    // Boundary operands: full cross-product of the edge set.
    for &a in EDGE_I32 {
        for &b in EDGE_I32 {
            let cv = unsafe { c(a, b) };
            let rv = unsafe { r(a, b) };
            assert_eq!(
                cv, rv,
                "{}({}, {}) mismatch [OP={} REPEAT={}]",
                sym, a, b, OP, REPEAT
            );
        }
    }

    // Randomized full-range operands.
    let mut rng = Rng::new(seed);
    for _ in 0..iters {
        let a = rng.next_i32();
        let b = rng.next_i32();
        let cv = unsafe { c(a, b) };
        let rv = unsafe { r(a, b) };
        assert_eq!(
            cv, rv,
            "{}({}, {}) mismatch [OP={} REPEAT={}]",
            sym, a, b, OP, REPEAT
        );
    }
}

#[test]
fn row01_row02_op_add() {
    diff_bin2("op_add", 0x0D15_EA5E_0000_0001, 512);
}

#[test]
fn row03_row04_op_sub() {
    diff_bin2("op_sub", 0x0D15_EA5E_0000_0002, 512);
}

#[test]
fn row05_row06_op_mul() {
    diff_bin2("op_mul", 0x0D15_EA5E_0000_0003, 512);
}

/* ============================== rows 7-9 ================================ */
/* The `G_OP` data slot: identity *and* behaviour.                          */

#[test]
fn row07_row09_g_op_slot_points_at_selected_op() {
    let p = pair();
    let expected_sym = format!("op_{}", OP);

    // Pointer identity: the slot must hold the address of `op_<OP>` in the very
    // same object, for both implementations.
    for imp in [&p.c, &p.r] {
        let slot_addr = imp.g_op() as usize;
        let sym_addr = imp.addr(&expected_sym);
        assert_eq!(
            slot_addr, sym_addr,
            "{}: G_OP does not point at {} [OP={} REPEAT={}]",
            imp.name, expected_sym, OP, REPEAT
        );
        // and must *not* point at either of the other two.
        for other in ["add", "sub", "mul"] {
            if other == OP {
                continue;
            }
            assert_ne!(
                slot_addr,
                imp.addr(&format!("op_{}", other)),
                "{}: G_OP wrongly points at op_{}",
                imp.name,
                other
            );
        }
    }

    // Behavioural parity when called through the slot.
    let cg = p.c.g_op();
    let rg = p.r.g_op();
    let cref = p.c.bin2(&expected_sym);
    let mut rng = Rng::new(0x0D15_EA5E_0000_0007);
    for i in 0..512 {
        let (a, b) = if i < EDGE_I32.len() * EDGE_I32.len() {
            (EDGE_I32[i / EDGE_I32.len()], EDGE_I32[i % EDGE_I32.len()])
        } else {
            (rng.next_i32(), rng.next_i32())
        };
        let cv = unsafe { cg(a, b) };
        let rv = unsafe { rg(a, b) };
        let expect = unsafe { cref(a, b) };
        assert_eq!(cv, rv, "G_OP({}, {}) mismatch [OP={}]", a, b, OP);
        assert_eq!(cv, expect, "C: G_OP != op_{} for ({}, {})", OP, a, b);
    }
}

/* ============================= rows 10-12 =============================== */
/* The `G_OP_NAME` data slot: `STR(OP)` byte-for-byte.                      */

#[test]
fn row10_row12_g_op_name_string() {
    let p = pair();
    let cn = p.c.g_op_name();
    let rn = p.r.g_op_name();
    assert_eq!(
        cn, rn,
        "G_OP_NAME bytes differ (C={:?} Rust={:?}) [OP={}]",
        cn, rn, OP
    );
    let mut want = OP.as_bytes().to_vec();
    want.push(0);
    assert_eq!(cn, want, "C: G_OP_NAME != STR(OP) for OP={}", OP);
}

/* ============================= rows 13-14 =============================== */
/* helper_ptr: return value and printed bytes.                              */

#[test]
fn row13_helper_ptr_return_value() {
    // Silence the helpers' printf while checking only return values.
    let ((), _bytes) = capture_stdout(|| {
        let p = pair();
        let c = p.c.bin2("helper_ptr");
        let r = p.r.bin2("helper_ptr");
        let mut rng = Rng::new(0x0D15_EA5E_0000_0013);
        for &a in EDGE_I32 {
            for &b in EDGE_I32 {
                assert_eq!(
                    unsafe { c(a, b) },
                    unsafe { r(a, b) },
                    "helper_ptr({}, {}) [OP={} REPEAT={}]",
                    a,
                    b,
                    OP,
                    REPEAT
                );
            }
        }
        for _ in 0..256 {
            let (a, b) = (rng.next_i32(), rng.next_i32());
            assert_eq!(
                unsafe { c(a, b) },
                unsafe { r(a, b) },
                "helper_ptr({}, {}) [OP={} REPEAT={}]",
                a,
                b,
                OP,
                REPEAT
            );
        }
    });
}

#[test]
fn row14_helper_ptr_stdout() {
    let p = pair();
    let c = p.c.bin2("helper_ptr");
    let r = p.r.bin2("helper_ptr");
    let mut rng = Rng::new(0x0D15_EA5E_0000_0014);
    let mut cases: Vec<(c_int, c_int)> = EDGE_I32
        .iter()
        .flat_map(|&a| EDGE_I32.iter().map(move |&b| (a, b)))
        .collect();
    for _ in 0..128 {
        cases.push((rng.next_i32(), rng.next_i32()));
    }
    for (a, b) in cases {
        let (cv, cout) = capture_stdout(|| unsafe { c(a, b) });
        let (rv, rout) = capture_stdout(|| unsafe { r(a, b) });
        assert_eq!(cv, rv, "helper_ptr({}, {}) return", a, b);
        assert_eq!(
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            "helper_ptr({}, {}) stdout [OP={} REPEAT={}]",
            a,
            b,
            OP,
            REPEAT
        );
    }
}

/* ============================= rows 15-18 =============================== */
/* helper_call: OP *and* REPEAT dependent, plus printed bytes.              */

#[test]
fn row15_row17_helper_call_return_value() {
    let ((), _bytes) = capture_stdout(|| {
        let p = pair();
        let c = p.c.bin2("helper_call");
        let r = p.r.bin2("helper_call");

        // Independently derived expectation for the REPEAT-driven accumulator.
        let expected_acc: c_int = match OP {
            "add" => (0..REPEAT).sum(),
            "sub" => -(0..REPEAT).sum::<c_int>(),
            _ => (1..=REPEAT).product(),
        };
        let op_ref = p.c.bin2(&format!("op_{}", OP));

        let mut rng = Rng::new(0x0D15_EA5E_0000_0015);
        let mut cases: Vec<(c_int, c_int)> = EDGE_I32
            .iter()
            .flat_map(|&a| EDGE_I32.iter().map(move |&b| (a, b)))
            .collect();
        for _ in 0..256 {
            cases.push((rng.next_i32(), rng.next_i32()));
        }
        for (a, b) in cases {
            let cv = unsafe { c(a, b) };
            let rv = unsafe { r(a, b) };
            assert_eq!(
                cv, rv,
                "helper_call({}, {}) [OP={} REPEAT={}]",
                a, b, OP, REPEAT
            );
            let want = unsafe { op_ref(a, b) }.wrapping_add(expected_acc);
            assert_eq!(
                cv, want,
                "C: helper_call({}, {}) != op_{}(a,b) + {} [REPEAT={}]",
                a, b, OP, expected_acc, REPEAT
            );
        }
    });
}

#[test]
fn row18_helper_call_stdout() {
    let p = pair();
    let c = p.c.bin2("helper_call");
    let r = p.r.bin2("helper_call");
    let mut rng = Rng::new(0x0D15_EA5E_0000_0018);
    let mut cases: Vec<(c_int, c_int)> = EDGE_I32
        .iter()
        .flat_map(|&a| EDGE_I32.iter().map(move |&b| (a, b)))
        .collect();
    for _ in 0..128 {
        cases.push((rng.next_i32(), rng.next_i32()));
    }
    for (a, b) in cases {
        let (cv, cout) = capture_stdout(|| unsafe { c(a, b) });
        let (rv, rout) = capture_stdout(|| unsafe { r(a, b) });
        assert_eq!(cv, rv, "helper_call({}, {}) return", a, b);
        assert_eq!(
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            "helper_call({}, {}) stdout [OP={} REPEAT={}]",
            a,
            b,
            OP,
            REPEAT
        );
    }
}

/* ============================= rows 19-23 =============================== */
/* use_generated: every `DISPATCH_REP` switch case, plus a full sweep.      */

fn diff_use_generated(n: c_int) {
    let p = pair();
    let c = p.c.un1("use_generated");
    let r = p.r.un1("use_generated");
    let (cv, cout) = capture_stdout(|| unsafe { c(n) });
    let (rv, rout) = capture_stdout(|| unsafe { r(n) });
    assert_eq!(
        cv, rv,
        "use_generated({}) return [OP={} REPEAT={}]",
        n, OP, REPEAT
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "use_generated({}) stdout [OP={} REPEAT={}]",
        n,
        OP,
        REPEAT
    );
}

/// Independent model of `accum_<OP>(n)` straight from `mdmacros.h`.
fn model_accum(n: c_int) -> c_int {
    if !(0..=6).contains(&n) {
        return INIT; // `default: break;`
    }
    let mut acc = INIT;
    for i in 0..n {
        acc = match OP {
            "add" => acc.wrapping_add(i),
            "sub" => acc.wrapping_sub(i),
            _ => acc.wrapping_mul(i.wrapping_add(1)),
        };
    }
    acc
}

#[test]
fn row19_use_generated_n0() {
    diff_use_generated(0);
    let p = pair();
    let c = p.c.un1("use_generated");
    let (cv, _) = capture_stdout(|| unsafe { c(0) });
    assert_eq!(cv, INIT, "C: use_generated(0) should be INIT_FOR({})", OP);
}

#[test]
fn row20_use_generated_n1_to_n5() {
    for n in 1..=5 {
        diff_use_generated(n);
        let p = pair();
        let c = p.c.un1("use_generated");
        let (cv, _) = capture_stdout(|| unsafe { c(n) });
        assert_eq!(cv, model_accum(n), "C model mismatch for n={} OP={}", n, OP);
    }
}

#[test]
fn row21_use_generated_n6() {
    diff_use_generated(6);
    let p = pair();
    let c = p.c.un1("use_generated");
    let (cv, _) = capture_stdout(|| unsafe { c(6) });
    assert_eq!(cv, model_accum(6), "C model mismatch for n=6 OP={}", OP);
    // Sanity: the documented values for the highest in-range case.
    let want = match OP {
        "add" => 15,
        "sub" => -15,
        _ => 720,
    };
    assert_eq!(cv, want);
}

#[test]
fn row22_row23_use_generated_full_sweep() {
    // Contiguous window across the switch boundary in both directions.
    for n in -8..=16 {
        diff_use_generated(n);
    }
    // Randomized full-range `n`.
    let mut rng = Rng::new(0x0D15_EA5E_0000_0022);
    let p = pair();
    let c = p.c.un1("use_generated");
    let r = p.r.un1("use_generated");
    let ((), _) = capture_stdout(|| {
        for _ in 0..256 {
            let n = rng.next_i32();
            assert_eq!(
                unsafe { c(n) },
                unsafe { r(n) },
                "use_generated({}) [OP={} REPEAT={}]",
                n,
                OP,
                REPEAT
            );
        }
        for &n in EDGE_I32 {
            assert_eq!(
                unsafe { c(n) },
                unsafe { r(n) },
                "use_generated({}) [OP={} REPEAT={}]",
                n,
                OP,
                REPEAT
            );
        }
    });
}

/* ============================= rows 24-25 =============================== */
/* The composed pipeline, driven exactly the way `mdmain.c` drives it.      */

#[test]
fn row24_row25_composed_pipeline() {
    let p = pair();

    let mut rng = Rng::new(0x0D15_EA5E_0000_0024);
    let mut cases: Vec<(c_int, c_int)> = vec![
        (3, 4),
        (0, 0),
        (-5, 9),
        (i32::MAX, 1),
        (i32::MIN, -1),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
    ];
    for _ in 0..128 {
        cases.push((rng.next_i32(), rng.next_i32()));
    }

    for (a, b) in cases {
        // Reproduce mdmain.c lines 36-46 through the .so, in order.
        let run = |imp: &common::Impl| {
            let hc = imp.bin2("helper_call");
            let hp = imp.bin2("helper_ptr");
            let ug = imp.un1("use_generated");
            let op = imp.bin2(&format!("op_{}", OP));
            let g = imp.g_op();
            capture_stdout(|| unsafe {
                let r_call = op(a, b);
                let x1 = hc(a, b);
                let x2 = hp(a, b);
                let x3 = ug(REPEAT);
                let gv = g(a, b);
                [r_call, x1, x2, x3, gv]
            })
        };

        let (cvals, cout) = run(&p.c);
        let (rvals, rout) = run(&p.r);

        assert_eq!(
            cvals, rvals,
            "pipeline values for ({}, {}) [OP={} REPEAT={}]",
            a, b, OP, REPEAT
        );
        assert_eq!(
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            "pipeline stdout for ({}, {}) [OP={} REPEAT={}]",
            a,
            b,
            OP,
            REPEAT
        );
        // Four printf lines, in mdmain's order.
        assert_eq!(
            cout.iter().filter(|&&x| x == b'\n').count(),
            3,
            "expected 3 lines from the helper sequence, got {:?}",
            String::from_utf8_lossy(&cout)
        );

        // And the summary arithmetic from mdmain.c:46.
        let csum = cvals.iter().fold(0i32, |acc, &v| acc.wrapping_add(v));
        let rsum = rvals.iter().fold(0i32, |acc, &v| acc.wrapping_add(v));
        assert_eq!(csum, rsum, "summary for ({}, {})", a, b);
    }
}

/* =============================== row 26 ================================= */
/* Whole program: `driver` executable, stdout + stderr + exit status.        */

#[test]
fn row26_whole_program() {
    use std::process::Command;
    let cexe = common::c_exe_path();
    let rexe = common::rust_exe_path();

    let mut rng = Rng::new(0x0D15_EA5E_0000_0026);
    let mut argsets: Vec<Vec<String>> = vec![
        vec!["3".into(), "4".into()],
        vec!["0".into(), "0".into()],
        vec!["-5".into(), "9".into()],
        vec!["2147483647".into(), "1".into()],
        vec!["-2147483648".into(), "-1".into()],
        vec!["2147483647".into(), "2147483647".into()],
        vec!["-2147483648".into(), "-2147483648".into()],
        vec!["+8".into(), "-8".into()],
        vec!["5".into(), "6".into(), "7".into()],
    ];
    for _ in 0..48 {
        argsets.push(vec![
            rng.next_i32().to_string(),
            rng.next_i32().to_string(),
        ]);
    }

    for args in argsets {
        let co = Command::new(&cexe).args(&args).output().expect("run C driver");
        let ro = Command::new(&rexe)
            .args(&args)
            .output()
            .expect("run Rust driver");
        assert_eq!(
            co.status.code(),
            ro.status.code(),
            "exit status for {:?} [OP={} REPEAT={}]",
            args,
            OP,
            REPEAT
        );
        assert_eq!(
            String::from_utf8_lossy(&co.stdout),
            String::from_utf8_lossy(&ro.stdout),
            "stdout for {:?} [OP={} REPEAT={}]",
            args,
            OP,
            REPEAT
        );
        assert_eq!(
            String::from_utf8_lossy(&co.stderr),
            String::from_utf8_lossy(&ro.stderr),
            "stderr for {:?} [OP={} REPEAT={}]",
            args,
            OP,
            REPEAT
        );
    }
}

/* =============================== row 27 ================================= */
/* The implicit default configuration (no -DOP / -DREPEAT at all).          */

#[test]
fn row27_implicit_default_matches_add_5() {
    // Only meaningful when this test run *is* the default configuration.
    if !(OP == "add" && REPEAT == 5) {
        return;
    }
    use std::process::Command;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_src = root.parent().unwrap().join("c_src").join("src");
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("cdiff_default");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let exe = dir.join("cdriver_implicit");

    // Note: NO -DOP / -DREPEAT — exercises the `#ifndef` fallbacks.
    let st = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()))
        .arg("-O2")
        .arg("-o")
        .arg(&exe)
        .arg(c_src.join("mdcore.c"))
        .arg(c_src.join("mdmain.c"))
        .status()
        .expect("spawn cc");
    assert!(st.success(), "implicit-default C build failed");

    let rexe = common::rust_exe_path();
    for args in [["3", "4"], ["-7", "11"], ["2147483647", "1"]] {
        let co = Command::new(&exe).args(args).output().expect("run");
        let ro = Command::new(&rexe).args(args).output().expect("run");
        assert_eq!(co.status.code(), ro.status.code());
        assert_eq!(
            String::from_utf8_lossy(&co.stdout),
            String::from_utf8_lossy(&ro.stdout),
            "implicit C default (OP=add REPEAT=5) vs Rust default for {:?}",
            args
        );
    }
}

/* =============================== row 28 ================================= */
/* Degenerate / conflicting Rust feature sets resolve as documented.        */

#[test]
fn row28_feature_priority_is_self_consistent() {
    // The harness resolves OP/REPEAT with the documented priority; the library
    // must agree with it. This catches a drift between `mdconfig.rs` and the
    // documented resolution order under conflicting feature sets.
    let p = pair();
    let mut want = OP.as_bytes().to_vec();
    want.push(0);
    assert_eq!(
        p.r.g_op_name(),
        want,
        "Rust .so resolved a different OP than the documented priority \
         (add > sub > mul); enabled features: add={} sub={} mul={}",
        cfg!(feature = "add"),
        cfg!(feature = "sub"),
        cfg!(feature = "mul")
    );

    let expected_acc: c_int = match OP {
        "add" => (0..REPEAT).sum(),
        "sub" => -(0..REPEAT).sum::<c_int>(),
        _ => (1..=REPEAT).product(),
    };
    let ((cv, rv), _) = capture_stdout(|| {
        let hc_c = p.c.bin2("helper_call");
        let hc_r = p.r.bin2("helper_call");
        (unsafe { hc_c(0, 0) }, unsafe { hc_r(0, 0) })
    });
    // op_<OP>(0, 0) == 0 for all three operations.
    let base: c_int = 0;
    assert_eq!(cv, rv);
    assert_eq!(
        rv,
        base.wrapping_add(expected_acc),
        "Rust .so resolved a different REPEAT than the documented priority \
         (lowest selected wins)"
    );
}
