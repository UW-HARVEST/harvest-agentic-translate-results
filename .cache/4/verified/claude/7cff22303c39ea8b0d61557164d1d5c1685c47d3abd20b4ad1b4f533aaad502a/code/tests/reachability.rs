//! The reachability map behind `ERRORS.md` and `CONFIGS.md`.
//!
//! Several rows in those tables describe a condition that the C source *writes*
//! but that cannot actually be reached through the public API. Rather than
//! asserting such rows by hand-waving, this test mechanically classifies a large
//! search of the input space and asserts, for every condition, whether it is
//! REACHABLE (with a minimum hit count, so no row passes vacuously) or
//! UNREACHABLE (zero hits over the whole search).
//!
//! Every classified input is *also* run through both `.so`s and compared
//! bit-for-bit, so this doubles as a broad differential sweep.

mod common;
use common::*;

/// Minimum hit count required before a condition counts as "reachable".
const MIN_HITS: usize = 8;

struct Cond {
    name: &'static str,
    reachable: bool,
    pred: fn(&Trace) -> bool,
}

const CONDS: &[Cond] = &[
    // --- reachable ----------------------------------------------------------
    Cond { name: "if arm taken",                     reachable: true,  pred: |t| t.arm_if },
    Cond { name: "else arm taken",                   reachable: true,  pred: |t| !t.arm_if },
    Cond { name: "ERRORS 13 / CONFIGS 5: sqd < 0 (clamp taken)", reachable: true, pred: |t| t.sqd < 0.0 },
    Cond { name: "ERRORS 14 / CONFIGS 9: sqd is NaN",            reachable: true, pred: |t| t.sqd.is_nan() },
    Cond { name: "ERRORS 16 / CONFIGS 7: sqd == +0.0",           reachable: true, pred: |t| t.sqd.to_bits() == 0 },
    Cond { name: "ERRORS 17: sqd == 0x80000001",     reachable: true,  pred: |t| t.sqd.to_bits() == 0x8000_0001 },
    Cond { name: "ERRORS 18: sqd == 0x00000001",     reachable: true,  pred: |t| t.sqd.to_bits() == 0x0000_0001 },
    Cond { name: "sqd positive subnormal",           reachable: true,  pred: |t| { let b = t.sqd.to_bits(); b != 0 && b < 0x0080_0000 } },
    Cond { name: "sqd negative subnormal",           reachable: true,  pred: |t| { let b = t.sqd.to_bits(); b > 0x8000_0000 && b < 0x8080_0000 } },
    Cond { name: "ERRORS 19: dy2*dy2 overflows to inf", reachable: true, pred: |t| t.dy2.is_finite() && t.dy2_sq.is_infinite() },
    Cond {
        name: "ERRORS 20 / CONFIGS 11: inf - inf inside sqd",
        reachable: true,
        pred: |t| {
            t.dy2_sq.is_infinite()
                && t.two_dx2_dy2.is_infinite()
                && t.dy2_sq.is_sign_positive() == t.two_dx2_dy2.is_sign_positive()
        },
    },
    Cond {
        name: "ERRORS 21a / CONFIGS 12: 0 * inf inside 2*dx2*dy2",
        reachable: true,
        pred: |t| !t.dx2.is_nan() && !t.dy2.is_nan() && t.two_dx2_dy2.is_nan(),
    },
    Cond {
        name: "ERRORS 22: inf + (-inf) inside dy2 + dx2",
        reachable: true,
        pred: |t| t.dy2.is_infinite() && t.dx2.is_infinite() && (t.dy2 + t.dx2).is_nan(),
    },
    Cond { name: "CONFIGS 10: sqd == +inf",          reachable: true,  pred: |t| t.sqd == f32::INFINITY },
    Cond { name: "root == +inf",                     reachable: true,  pred: |t| t.root == f32::INFINITY },
    Cond { name: "lambda == +inf",                   reachable: true,  pred: |t| t.lambda == f32::INFINITY },
    Cond { name: "lambda is NaN",                    reachable: true,  pred: |t| t.lambda.is_nan() },
    Cond {
        name: "sum NaN manufactured from non-NaN operands",
        reachable: true,
        pred: |t| !t.dy2.is_nan() && !t.dx2.is_nan() && !t.root.is_nan() && t.sum.is_nan(),
    },
    Cond { name: "(dy2 + dx2) == -0.0",              reachable: true,  pred: |t| (t.dy2 + t.dx2).to_bits() == 0x8000_0000 },

    // --- unreachable --------------------------------------------------------
    Cond {
        name: "ERRORS 15 / CONFIGS 8: sqd == -0.0",
        reachable: false,
        pred: |t| t.sqd.to_bits() == 0x8000_0000,
    },
    Cond {
        name: "ERRORS 21b: 0 * inf inside 4*dxy*dxy",
        reachable: false,
        pred: |t| !t.dxy.is_nan() && t.term4.is_nan(),
    },
    Cond {
        name: "ERRORS 23: sqrtf receives a strictly negative argument",
        reachable: false,
        pred: |t| t.clamped < 0.0,
    },
    Cond {
        name: "ERRORS 23: sqrtf receives -0.0",
        reachable: false,
        pred: |t| t.clamped.to_bits() == 0x8000_0000,
    },
    Cond {
        name: "ERRORS 26: dx2 - lambda with both infinite and same-signed",
        reachable: false,
        pred: |t| {
            t.dx2.is_infinite()
                && t.lambda.is_infinite()
                && t.dx2.is_sign_positive() == t.lambda.is_sign_positive()
        },
    },
    Cond { name: "lambda == -inf",                   reachable: false, pred: |t| t.lambda == f32::NEG_INFINITY },
    Cond { name: "(4*dxy)*dxy strictly negative",    reachable: false, pred: |t| t.term4 < 0.0 },
    Cond { name: "(4*dxy)*dxy == -0.0",              reachable: false, pred: |t| t.term4.to_bits() == 0x8000_0000 },
    Cond {
        name: "clamp taken AND (dy2 + dx2) == -0.0",
        reachable: false,
        pred: |t| t.sqd < 0.0 && (t.dy2 + t.dx2).to_bits() == 0x8000_0000,
    },
    Cond {
        name: "if arm taken with a NaN dx2 or dy2",
        reachable: false,
        pred: |t| t.arm_if && (t.dx2.is_nan() || t.dy2.is_nan()),
    },
];

#[test]
fn reachability_map_matches_the_artifacts() {
    let mut hits = vec![0usize; CONDS.len()];
    let mut first: Vec<Option<[u32; 3]>> = vec![None; CONDS.len()];

    let mut rng = Rng::new(SEED ^ 0x3001);
    // Randomized search; every candidate is also differentially checked.
    let mut diffed = 0usize;
    for _ in 0..2_000_000 {
        let (a, b, c) = rng.candidate();
        let t = trace(a, b, c);
        for (k, cond) in CONDS.iter().enumerate() {
            if (cond.pred)(&t) {
                hits[k] += 1;
                if first[k].is_none() {
                    first[k] = Some([a, b, c]);
                    // Differentially check the first witness of every condition.
                    diff(&format!("reachability witness: {}", cond.name), &[a, b, c], 1, 2);
                }
            }
        }
        if diffed < 100_000 {
            diff("reachability sweep", &[a, b, c], 1, 2);
            diffed += 1;
        }
    }
    // Exhaustive deterministic sweep over the specials cross-product.
    for &a in SPECIALS {
        for &b in SPECIALS {
            for &c in SPECIALS {
                let t = trace(a, b, c);
                for (k, cond) in CONDS.iter().enumerate() {
                    if (cond.pred)(&t) {
                        hits[k] += 1;
                        if first[k].is_none() {
                            first[k] = Some([a, b, c]);
                        }
                    }
                }
                diff("reachability specials", &[a, b, c], 1, 2);
            }
        }
    }
    // The constructed sqd < 0 witness from ERRORS.md rows 13/17.
    {
        let x = (0.75f64 * 2f64.powi(-75)) as f32;
        let t = trace(x.to_bits(), x.to_bits(), 0);
        assert!(t.sqd < 0.0);
        for (k, cond) in CONDS.iter().enumerate() {
            if (cond.pred)(&t) {
                hits[k] += 1;
                if first[k].is_none() {
                    first[k] = Some([x.to_bits(), x.to_bits(), 0]);
                }
            }
        }
        diff("reachability constructed witness", &[x.to_bits(), x.to_bits(), 0], 1, 2);
    }

    let mut problems = Vec::new();
    for (k, cond) in CONDS.iter().enumerate() {
        let status = if cond.reachable { "REACHABLE  " } else { "UNREACHABLE" };
        println!(
            "{status} hits={:>9}  {}   first={:?}",
            hits[k],
            cond.name,
            first[k].map(|w| w.map(|b| format!("{b:#010x}")))
        );
        if cond.reachable && hits[k] < MIN_HITS {
            problems.push(format!(
                "`{}` is documented REACHABLE but was hit only {} time(s)",
                cond.name, hits[k]
            ));
        }
        if !cond.reachable && hits[k] != 0 {
            problems.push(format!(
                "`{}` is documented UNREACHABLE but was hit {} time(s), e.g. {:?}",
                cond.name, hits[k], first[k]
            ));
        }
    }
    assert!(problems.is_empty(), "reachability map is stale:\n  {}", problems.join("\n  "));
}
