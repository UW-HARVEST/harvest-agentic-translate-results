//! Phase B — valid-path differential tests, CONFIGS.md rows 1..=26.
//!
//! The L0/L1 entry points: `c2V`, `c2Sub`, `c2Dot`, `c2Mulvs`, `c2Maxv`,
//! `c2Minv`, `c2Clampv`. Every call goes through the exported C symbol of BOTH
//! shared objects; results are compared bit-for-bit (`to_bits`), so `-0.0` vs
//! `+0.0` and NaN payload/sign differences are failures.

mod common;
use common::*;

const SEED: u64 = 0x243F_6A88_85A3_08D3;
const N: usize = 20_000;

// ===========================================================================
// c2V — rows 1..=3
// ===========================================================================

/// Row 1 — `c2V` with random finite `f32`s.
#[test]
fn cfg01_c2v_random_finite() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..N {
        let (x, y) = (rng.finite_f32(), rng.finite_f32());
        diff(
            || format!("c2V({:#010x}, {:#010x})", x.to_bits(), y.to_bits()),
            |api| vbits((api.c2V)(x, y)),
        );
    }
}

/// Row 2 — `c2V` with arbitrary `u32`-as-`f32` encodings: NaN (both signs, all
/// payloads), infinities, denormals, `-0.0`. Pure bit pass-through.
#[test]
fn cfg02_c2v_random_any_bits() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N {
        let (x, y) = (rng.any_f32(), rng.any_f32());
        diff(
            || format!("c2V({:#010x}, {:#010x})", x.to_bits(), y.to_bits()),
            |api| vbits((api.c2V)(x, y)),
        );
    }
}

/// Row 3 — `c2V` over the exhaustive special-value grid.
#[test]
fn cfg03_c2v_special_grid() {
    let sv = special_values();
    for &x in sv {
        for &y in sv {
            diff(
                || format!("c2V({:#010x}, {:#010x})", x.to_bits(), y.to_bits()),
                |api| vbits((api.c2V)(x, y)),
            );
        }
    }
}

// ===========================================================================
// c2Sub — rows 4..=7
// ===========================================================================

/// Row 4 — `c2Sub` with random finite vectors in normal magnitude ranges.
#[test]
fn cfg04_c2sub_random_finite() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        let (a, b) = (rng.v_finite(), rng.v_finite());
        diff(
            || format!("c2Sub({a:?}, {b:?})"),
            |api| vbits((api.c2Sub)(a, b)),
        );
    }
}

/// Row 5 — `c2Sub` with arbitrary bit patterns.
#[test]
fn cfg05_c2sub_random_any_bits() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..N {
        let (a, b) = (rng.v_any(), rng.v_any());
        diff(
            || format!("c2Sub({a:?}, {b:?})"),
            |api| vbits((api.c2Sub)(a, b)),
        );
    }
}

/// Row 6 — `c2Sub` over the special-value grid (sign of zero must survive:
/// `-0.0 - 0.0 == -0.0` but `0.0 - 0.0 == +0.0`).
#[test]
fn cfg06_c2sub_special_grid() {
    let sv = special_values();
    for &ax in sv {
        for &bx in sv {
            for &ay in sv {
                let a = C2v { x: ax, y: ay };
                let b = C2v { x: bx, y: ax };
                diff(
                    || format!("c2Sub({a:?}, {b:?})"),
                    |api| vbits((api.c2Sub)(a, b)),
                );
            }
        }
    }
}

/// Row 7 — `c2Sub` overflow shapes.
#[test]
fn cfg07_c2sub_overflow() {
    let big = [f32::MAX, f32::MIN, f32::MAX / 2.0, -f32::MAX / 2.0];
    for &ax in &big {
        for &bx in &big {
            for &ay in &big {
                for &by in &big {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    diff(
                        || format!("c2Sub({a:?}, {b:?})"),
                        |api| vbits((api.c2Sub)(a, b)),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// c2Dot — rows 8..=13
// ===========================================================================

/// Row 8 — `c2Dot` with random finite vectors.
#[test]
fn cfg08_c2dot_random_finite() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..N {
        let (a, b) = (rng.v_finite(), rng.v_finite());
        diff(
            || format!("c2Dot({a:?}, {b:?})"),
            |api| (api.c2Dot)(a, b).to_bits(),
        );
    }
}

/// Row 9 — `c2Dot` with arbitrary bit patterns. This is the row that pins
/// GCC's `mulss`/`addss` destination-operand choices (SSE returns the
/// destination when it is a NaN), so distinct NaN payloads are essential.
#[test]
fn cfg09_c2dot_random_any_bits() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..N {
        let (a, b) = (rng.v_any(), rng.v_any());
        diff(
            || format!("c2Dot({a:?}, {b:?})"),
            |api| (api.c2Dot)(a, b).to_bits(),
        );
    }
    // ... and pathological-weighted, so NaN/inf show up far more often.
    let mut rng = Rng::new(SEED ^ 0x9999);
    for _ in 0..N {
        let (a, b) = (rng.v_path(), rng.v_path());
        diff(
            || format!("c2Dot({a:?}, {b:?})"),
            |api| (api.c2Dot)(a, b).to_bits(),
        );
    }
}

/// Row 10 — `c2Dot` special-value grid on the `x` lane while the `y` lane
/// carries distinct random NaN payloads (exposes both `mulss` orders and the
/// `addss` order simultaneously).
#[test]
fn cfg10_c2dot_special_grid_with_nan_y() {
    let sv = special_values();
    let mut rng = Rng::new(SEED ^ 10);
    for &ax in sv {
        for &bx in sv {
            for k in 0..8u32 {
                let ay = if k & 1 == 0 {
                    qnan(rng.next_u32(), k & 2 != 0)
                } else {
                    snan(rng.next_u32(), k & 2 != 0)
                };
                let by = if k & 4 == 0 {
                    qnan(rng.next_u32(), k & 2 == 0)
                } else {
                    rng.finite_f32()
                };
                let a = C2v { x: ax, y: ay };
                let b = C2v { x: bx, y: by };
                diff(
                    || format!("c2Dot({a:?}, {b:?})"),
                    |api| (api.c2Dot)(a, b).to_bits(),
                );
            }
        }
    }
}

/// Row 11 — the two products cancel exactly, so the sign of the resulting zero
/// is the observable (`+0.0 + -0.0 == +0.0`, `-0.0 + -0.0 == -0.0`).
#[test]
fn cfg11_c2dot_exact_cancellation() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..N {
        // choose powers of two so the products are exact
        let e1 = rng.below(40) as i32 - 20;
        let e2 = rng.below(40) as i32 - 20;
        let p = (2.0f32).powi(e1);
        let q = (2.0f32).powi(e2);
        // a.x*b.x = p*q, a.y*b.y = -(p*q)
        let a = C2v { x: p, y: p };
        let b = C2v { x: q, y: -q };
        diff(
            || format!("c2Dot({a:?}, {b:?})"),
            |api| (api.c2Dot)(a, b).to_bits(),
        );
        // and the signed-zero variants
        for (sx, sy) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            let a = C2v { x: 0.0 * sx, y: 0.0 * sy };
            let b = C2v { x: q, y: -q };
            diff(
                || format!("c2Dot({a:?}, {b:?})"),
                |api| (api.c2Dot)(a, b).to_bits(),
            );
        }
    }
}

/// Row 12 — products overflow to `±inf`; `inf + -inf` must yield the same NaN.
#[test]
fn cfg12_c2dot_overflow_and_inf_cancellation() {
    let vals = [
        f32::MAX,
        f32::MIN,
        1e30f32,
        -1e30f32,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
    ];
    for &ax in &vals {
        for &bx in &vals {
            for &ay in &vals {
                for &by in &vals {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    diff(
                        || format!("c2Dot({a:?}, {b:?})"),
                        |api| (api.c2Dot)(a, b).to_bits(),
                    );
                }
            }
        }
    }
}

/// Row 13 — the self-dot `c2Dot(v, v)` that all three kernels use.
#[test]
fn cfg13_c2dot_self() {
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..N {
        let v = match i % 3 {
            0 => rng.v_finite(),
            1 => rng.v_any(),
            _ => rng.v_path(),
        };
        diff(
            || format!("c2Dot({v:?}, {v:?})"),
            |api| (api.c2Dot)(v, v).to_bits(),
        );
    }
    for &x in special_values() {
        for &y in special_values() {
            let v = C2v { x, y };
            diff(
                || format!("c2Dot({v:?}, {v:?})"),
                |api| (api.c2Dot)(v, v).to_bits(),
            );
        }
    }
}

// ===========================================================================
// c2Mulvs — rows 14..=16
// ===========================================================================

/// Row 14 — `c2Mulvs` with random finite vector and scalar.
#[test]
fn cfg14_c2mulvs_random_finite() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..N {
        let a = rng.v_finite();
        let b = rng.finite_f32();
        diff(
            || format!("c2Mulvs({a:?}, {:#010x})", b.to_bits()),
            |api| vbits((api.c2Mulvs)(a, b)),
        );
    }
}

/// Row 15 — `c2Mulvs` with arbitrary bit patterns: pins which operand of
/// `mulss` is the destination, and covers `0 * inf`.
#[test]
fn cfg15_c2mulvs_random_any_bits() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..N {
        let a = rng.v_any();
        let b = rng.any_f32();
        diff(
            || format!("c2Mulvs({a:?}, {:#010x})", b.to_bits()),
            |api| vbits((api.c2Mulvs)(a, b)),
        );
    }
    let mut rng = Rng::new(SEED ^ 0x1515);
    for _ in 0..N {
        let a = rng.v_path();
        let b = rng.pathological_f32();
        diff(
            || format!("c2Mulvs({a:?}, {:#010x})", b.to_bits()),
            |api| vbits((api.c2Mulvs)(a, b)),
        );
    }
}

/// Row 16 — `c2Mulvs` special-scalar × special-vector grid.
#[test]
fn cfg16_c2mulvs_special_grid() {
    let sv = special_values();
    for &b in sv {
        for &x in sv {
            for &y in sv {
                let a = C2v { x, y };
                diff(
                    || format!("c2Mulvs({a:?}, {:#010x})", b.to_bits()),
                    |api| vbits((api.c2Mulvs)(a, b)),
                );
            }
        }
    }
}

// ===========================================================================
// c2Maxv — rows 17..=19
// ===========================================================================

/// Row 17 — `c2Maxv` random finite; asserts all four lane-branch combinations
/// were actually reached.
#[test]
fn cfg17_c2maxv_random_finite() {
    let mut rng = Rng::new(SEED ^ 17);
    let mut seen = [false; 4];
    for _ in 0..N {
        let (a, b) = (rng.v_finite(), rng.v_finite());
        let idx = ((a.x > b.x) as usize) | (((a.y > b.y) as usize) << 1);
        seen[idx] = true;
        diff(
            || format!("c2Maxv({a:?}, {b:?})"),
            |api| vbits((api.c2Maxv)(a, b)),
        );
    }
    assert!(seen.iter().all(|s| *s), "lane-branch coverage gap: {seen:?}");
}

/// Row 18 — equal lanes force the `else` (i.e. `b`) arm of both ternaries.
#[test]
fn cfg18_c2maxv_equal_lanes() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..N {
        let a = rng.v_finite();
        for b in [
            a,
            C2v { x: a.x, y: rng.finite_f32() },
            C2v { x: rng.finite_f32(), y: a.y },
            C2v { x: -a.x, y: -a.y },
        ] {
            diff(
                || format!("c2Maxv({a:?}, {b:?})"),
                |api| vbits((api.c2Maxv)(a, b)),
            );
        }
    }
}

/// Row 19 — `c2Maxv` with arbitrary bit patterns, plus the full special grid
/// (NaN in either/both operands, all four `±0` pairings).
#[test]
fn cfg19_c2maxv_any_bits_and_special_grid() {
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..N {
        let (a, b) = if i % 2 == 0 {
            (rng.v_any(), rng.v_any())
        } else {
            (rng.v_path(), rng.v_path())
        };
        diff(
            || format!("c2Maxv({a:?}, {b:?})"),
            |api| vbits((api.c2Maxv)(a, b)),
        );
    }
    let sv = special_values();
    for &ax in sv {
        for &bx in sv {
            for &ay in sv {
                let a = C2v { x: ax, y: ay };
                let b = C2v { x: bx, y: bx };
                diff(
                    || format!("c2Maxv({a:?}, {b:?})"),
                    |api| vbits((api.c2Maxv)(a, b)),
                );
            }
        }
    }
}

// ===========================================================================
// c2Minv — rows 20..=22
// ===========================================================================

/// Row 20 — `c2Minv` random finite; all four lane-branch combinations.
#[test]
fn cfg20_c2minv_random_finite() {
    let mut rng = Rng::new(SEED ^ 20);
    let mut seen = [false; 4];
    for _ in 0..N {
        let (a, b) = (rng.v_finite(), rng.v_finite());
        let idx = ((a.x < b.x) as usize) | (((a.y < b.y) as usize) << 1);
        seen[idx] = true;
        diff(
            || format!("c2Minv({a:?}, {b:?})"),
            |api| vbits((api.c2Minv)(a, b)),
        );
    }
    assert!(seen.iter().all(|s| *s), "lane-branch coverage gap: {seen:?}");
}

/// Row 21 — equal lanes force the `else` arm.
#[test]
fn cfg21_c2minv_equal_lanes() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..N {
        let a = rng.v_finite();
        for b in [
            a,
            C2v { x: a.x, y: rng.finite_f32() },
            C2v { x: rng.finite_f32(), y: a.y },
            C2v { x: -a.x, y: -a.y },
        ] {
            diff(
                || format!("c2Minv({a:?}, {b:?})"),
                |api| vbits((api.c2Minv)(a, b)),
            );
        }
    }
}

/// Row 22 — `c2Minv` arbitrary bit patterns + special grid.
#[test]
fn cfg22_c2minv_any_bits_and_special_grid() {
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..N {
        let (a, b) = if i % 2 == 0 {
            (rng.v_any(), rng.v_any())
        } else {
            (rng.v_path(), rng.v_path())
        };
        diff(
            || format!("c2Minv({a:?}, {b:?})"),
            |api| vbits((api.c2Minv)(a, b)),
        );
    }
    let sv = special_values();
    for &ax in sv {
        for &bx in sv {
            for &ay in sv {
                let a = C2v { x: ax, y: ay };
                let b = C2v { x: bx, y: bx };
                diff(
                    || format!("c2Minv({a:?}, {b:?})"),
                    |api| vbits((api.c2Minv)(a, b)),
                );
            }
        }
    }
}

// ===========================================================================
// c2Clampv — rows 23..=26
// ===========================================================================

/// Row 23 — well-formed range `lo < hi`, with `a` below / inside / above;
/// asserts all 16 composed lane paths of `c2Maxv(lo, c2Minv(a, hi))` are hit.
#[test]
fn cfg23_c2clampv_well_formed_range() {
    let mut rng = Rng::new(SEED ^ 23);
    let mut seen = [false; 16];
    for _ in 0..N {
        let l = C2v { x: rng.range(-100.0, 0.0), y: rng.range(-100.0, 0.0) };
        let h = C2v { x: rng.range(0.0, 100.0), y: rng.range(0.0, 100.0) };
        let a = C2v { x: rng.range(-200.0, 200.0), y: rng.range(-200.0, 200.0) };
        let mx = C2v {
            x: if a.x < h.x { a.x } else { h.x },
            y: if a.y < h.y { a.y } else { h.y },
        };
        let idx = ((a.x < h.x) as usize)
            | (((a.y < h.y) as usize) << 1)
            | (((l.x > mx.x) as usize) << 2)
            | (((l.y > mx.y) as usize) << 3);
        seen[idx] = true;
        diff(
            || format!("c2Clampv({a:?}, {l:?}, {h:?})"),
            |api| vbits((api.c2Clampv)(a, l, h)),
        );
    }
    let hit = seen.iter().filter(|s| **s).count();
    assert!(hit >= 9, "composed lane-path coverage too low: {hit}/16 {seen:?}");
}

/// Row 24 — degenerate range `lo == hi`.
#[test]
fn cfg24_c2clampv_degenerate_range() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..N {
        let l = rng.v_finite();
        let a = rng.v_finite();
        diff(
            || format!("c2Clampv({a:?}, {l:?}, {l:?})"),
            |api| vbits((api.c2Clampv)(a, l, l)),
        );
    }
    for &v in special_values() {
        let l = C2v { x: v, y: v };
        for &w in special_values() {
            let a = C2v { x: w, y: v };
            diff(
                || format!("c2Clampv({a:?}, {l:?}, {l:?})"),
                |api| vbits((api.c2Clampv)(a, l, l)),
            );
        }
    }
}

/// Row 25 — inverted range `lo > hi` (never validated by the C).
#[test]
fn cfg25_c2clampv_inverted_range() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..N {
        let h = C2v { x: rng.range(-100.0, 0.0), y: rng.range(-100.0, 0.0) };
        let l = C2v { x: rng.range(0.0, 100.0), y: rng.range(0.0, 100.0) };
        let a = C2v { x: rng.range(-200.0, 200.0), y: rng.range(-200.0, 200.0) };
        diff(
            || format!("c2Clampv({a:?}, {l:?}, {h:?}) [lo>hi]"),
            |api| vbits((api.c2Clampv)(a, l, h)),
        );
        // one axis inverted only
        let mixed_lo = C2v { x: l.x, y: h.y };
        let mixed_hi = C2v { x: h.x, y: l.y };
        diff(
            || format!("c2Clampv({a:?}, {mixed_lo:?}, {mixed_hi:?}) [one axis inverted]"),
            |api| vbits((api.c2Clampv)(a, mixed_lo, mixed_hi)),
        );
    }
}

/// Row 26 — `c2Clampv` with arbitrary bit patterns for all three arguments.
#[test]
fn cfg26_c2clampv_any_bits() {
    let mut rng = Rng::new(SEED ^ 26);
    for i in 0..N {
        let (a, l, h) = if i % 2 == 0 {
            (rng.v_any(), rng.v_any(), rng.v_any())
        } else {
            (rng.v_path(), rng.v_path(), rng.v_path())
        };
        diff(
            || format!("c2Clampv({a:?}, {l:?}, {h:?})"),
            |api| vbits((api.c2Clampv)(a, l, h)),
        );
    }
    // ±0 and NaN placed at every one of the three arguments
    let interesting = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        qnan(0x11, false),
        qnan(0x22, true),
        snan(0x33, false),
    ];
    for &av in &interesting {
        for &lv in &interesting {
            for &hv in &interesting {
                let a = C2v { x: av, y: lv };
                let l = C2v { x: lv, y: hv };
                let h = C2v { x: hv, y: av };
                diff(
                    || format!("c2Clampv({a:?}, {l:?}, {h:?})"),
                    |api| vbits((api.c2Clampv)(a, l, h)),
                );
            }
        }
    }
}
