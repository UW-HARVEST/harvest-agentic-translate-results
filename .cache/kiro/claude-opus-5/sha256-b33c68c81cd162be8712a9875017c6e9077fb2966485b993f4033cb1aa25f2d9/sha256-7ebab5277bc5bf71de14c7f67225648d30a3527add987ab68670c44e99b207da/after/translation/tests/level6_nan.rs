//! Level 6: focused NaN / infinity propagation.
//!
//! Every mismatch found while verifying this translation was a NaN sign or
//! payload difference: x86 scalar SSE ops return the *destination* operand when
//! both operands are NaN, and produce a sign-set "QNaN indefinite" on invalid
//! operations. These tests drive the float functions with dense combinations of
//! signed/signaling NaNs and infinities so those paths cannot regress silently.

mod harness;

use harness::*;

/// Operands chosen to make both sides of a `*` or `+` NaN at once, and to
/// trigger invalid operations (`0 * inf`, `inf - inf`).
fn hostile() -> Vec<f32> {
    let mut v = nan_pool();
    v.extend_from_slice(&[
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0,
        -1.0,
        0.5,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
    ]);
    v
}

#[test]
fn c2dot_nan_exhaustive() {
    let i = impls();
    let (c, r) = i.sym::<FnDot>("c2Dot");
    let h = hostile();
    // Full 4-way product over the hostile pool: every (dest, src) NaN pairing.
    for &ax in h.iter() {
        for &ay in h.iter() {
            for &bx in h.iter() {
                for &by in h.iter() {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    let x = unsafe { c(a, b) };
                    let y = unsafe { r(a, b) };
                    eq_f32(
                        &format!(
                            "c2Dot(({:#010x},{:#010x}),({:#010x},{:#010x}))",
                            ax.to_bits(),
                            ay.to_bits(),
                            bx.to_bits(),
                            by.to_bits()
                        ),
                        x,
                        y,
                    );
                }
            }
        }
    }
}

#[test]
fn f9_nan_dense() {
    let i = impls();
    let (c, r) = i.sym::<FnF9>("f9");
    let h = hostile();
    // 8 independent coordinates is too many to enumerate; sweep pairs of
    // coordinates over the hostile pool with the rest pinned to hostile values.
    for &a in h.iter() {
        for &b in h.iter() {
            let combos = [
                [a, b, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5],
                [1.0, 0.0, a, b, 0.0, 1.0, 0.5, 0.5],
                [1.0, 0.0, 0.0, 1.0, a, b, 0.5, 0.5],
                [1.0, 0.0, 0.0, 1.0, 0.5, 0.5, a, b],
                [a, a, b, b, a, b, b, a],
                [a, b, b, a, a, b, b, a],
            ];
            for cm in combos.iter() {
                let (p1, p2, p3, p) = (
                    LmVec2 { x: cm[0], y: cm[1] },
                    LmVec2 { x: cm[2], y: cm[3] },
                    LmVec2 { x: cm[4], y: cm[5] },
                    LmVec2 { x: cm[6], y: cm[7] },
                );
                let x = unsafe { c(p1, p2, p3, p) };
                let y = unsafe { r(p1, p2, p3, p) };
                eq_vec2(&format!("f9 nan {cm:?}"), (x.x, x.y), (y.x, y.y));
            }
        }
    }
}

fn triple_nan_exhaustive(name: &str) {
    let i = impls();
    let (c, r) = i.sym::<FnTriple>(name);
    let mut pool = hostile();
    // Hue values that select each branch, so NaN operands are combined inside
    // every arm rather than only in the fallthrough.
    pool.extend_from_slice(&[30.0, 90.0, 150.0, 210.0, 270.0, 330.0, 400.0, -30.0, 60.0]);
    for &a in pool.iter() {
        for &b in pool.iter() {
            for &cc in pool.iter() {
                let src = [a, b, cc];
                let mut cd = [0f32; 3];
                let mut rd = [0f32; 3];
                unsafe { c(cd.as_mut_ptr(), src.as_ptr()) };
                unsafe { r(rd.as_mut_ptr(), src.as_ptr()) };
                for k in 0..3 {
                    eq_f32(
                        &format!(
                            "{name}([{:#010x},{:#010x},{:#010x}])[{k}]",
                            a.to_bits(),
                            b.to_bits(),
                            cc.to_bits()
                        ),
                        cd[k],
                        rd[k],
                    );
                }
            }
        }
    }
}

#[test]
fn f11_nan_exhaustive() {
    triple_nan_exhaustive("f11");
}

#[test]
fn f12_nan_exhaustive() {
    triple_nan_exhaustive("f12");
}

#[test]
fn f13_nan_exhaustive() {
    triple_nan_exhaustive("f13");
}

/// `f11` divides the hue by 60 and takes `fmodf(.., 2)`; confirm Rust's `%`
/// agrees with glibc's `fmodf` across magnitudes, signs and subnormals.
#[test]
fn f11_fmod_domain() {
    let i = impls();
    let (c, r) = i.sym::<FnTriple>("f11");
    let mut hues: Vec<f32> = Vec::new();
    for k in -2000i32..=2000 {
        hues.push(k as f32 * 0.37);
    }
    for e in -40i32..=40 {
        let m = (2.0f32).powi(e);
        hues.push(m);
        hues.push(-m);
        hues.push(m * 60.0);
        hues.push(-m * 60.0);
    }
    hues.extend_from_slice(&[
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-45,
        -1e-45,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ]);
    let mut rng = Rng::new(0xF11D);
    for _ in 0..20_000 {
        hues.push(rng.next_f32_bits());
    }

    for &h in hues.iter() {
        for &(s, l) in [(0.5f32, 0.5f32), (1.0, 0.5), (0.25, 0.9), (1.0, 1.0)].iter() {
            let src = [h, s, l];
            let mut cd = [0f32; 3];
            let mut rd = [0f32; 3];
            unsafe { c(cd.as_mut_ptr(), src.as_ptr()) };
            unsafe { r(rd.as_mut_ptr(), src.as_ptr()) };
            for k in 0..3 {
                eq_f32(&format!("f11({src:?})[{k}]"), cd[k], rd[k]);
            }
        }
    }
}

/// `f12` casts `floorf(h)` to `int`; the C cast is UB out of range but the
/// binary uses `cvttss2si`, which yields INT_MIN. Probe the boundary densely.
#[test]
fn f12_int_cast_boundary() {
    let i = impls();
    let (c, r) = i.sym::<FnTriple>("f12");
    let mut hs: Vec<f32> = Vec::new();
    // h is divided by 60 before the cast, so scale the interesting points up.
    for &edge in [
        2_147_483_648.0f32,
        -2_147_483_648.0,
        2_147_483_520.0,
        -2_147_483_520.0,
        16_777_216.0,
        -16_777_216.0,
    ]
    .iter()
    {
        for d in -4i32..=4 {
            hs.push(edge * 60.0 + d as f32);
            hs.push(edge + d as f32);
        }
    }
    for k in -400i32..=400 {
        hs.push(k as f32 * 60.0);
        hs.push(k as f32 * 60.0 + 30.0);
        hs.push(k as f32);
    }
    hs.extend_from_slice(&[
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e38,
        -1e38,
    ]);
    hs.extend(nan_pool());
    let mut rng = Rng::new(0xF12C);
    for _ in 0..20_000 {
        hs.push(rng.next_f32_bits());
    }

    for &h in hs.iter() {
        for &(s, v) in [(0.5f32, 0.5f32), (1.0, 1.0), (0.25, 0.75)].iter() {
            let src = [h, s, v];
            let mut cd = [0f32; 3];
            let mut rd = [0f32; 3];
            unsafe { c(cd.as_mut_ptr(), src.as_ptr()) };
            unsafe { r(rd.as_mut_ptr(), src.as_ptr()) };
            for k in 0..3 {
                eq_f32(
                    &format!("f12([{:#010x},{s},{v}])[{k}]", h.to_bits()),
                    cd[k],
                    rd[k],
                );
            }
        }
    }
}

/// The shape predicates return `int`, but feed NaN through them anyway to
/// confirm the comparison sense matches (NaN compares false in both).
#[test]
fn predicates_nan() {
    let i = impls();
    let (cc, rc) = i.sym::<FnCirCir>("c2CircletoCircle");
    let (ca, ra) = i.sym::<FnCirAabb>("c2CircletoAABB");
    let (cb, rb) = i.sym::<FnAabbAabb>("c2AABBtoAABB");
    let h = hostile();
    for &a in h.iter() {
        for &b in h.iter() {
            let c1 = C2Circle {
                p: C2v { x: a, y: b },
                r: a,
            };
            let c2 = C2Circle {
                p: C2v { x: b, y: a },
                r: b,
            };
            let b1 = C2Aabb {
                min: C2v { x: a, y: b },
                max: C2v { x: b, y: a },
            };
            let b2 = C2Aabb {
                min: C2v { x: b, y: a },
                max: C2v { x: a, y: b },
            };
            assert_eq!(unsafe { cc(c1, c2) }, unsafe { rc(c1, c2) }, "cir/cir nan");
            assert_eq!(unsafe { ca(c1, b1) }, unsafe { ra(c1, b1) }, "cir/aabb nan");
            assert_eq!(unsafe { cb(b1, b2) }, unsafe { rb(b1, b2) }, "aabb/aabb nan");
        }
    }
}

/// `agglom` filters NaN with `isnan` before accumulating; make sure the filter
/// and the double accumulation agree when the sub-results are NaN or infinite.
#[test]
fn agglom_nan_inputs() {
    let i = impls();
    let (cs, rs) = i.sym::<FnAgglom>("agglom");
    let (c, r) = (*cs, *rs);
    let h = hostile();

    for &a in h.iter() {
        for &b in h.iter() {
            let v = unsafe {
                c(
                    a, b, a, b, a, b, a, 7, 2, 1, 2, 0x1234, 4096, 2, 16, a, b, b, a, a, b, b, a,
                    0x3C00, a, b, a, b, a, b, a, b, a,
                )
            };
            let w = unsafe {
                r(
                    a, b, a, b, a, b, a, 7, 2, 1, 2, 0x1234, 4096, 2, 16, a, b, b, a, a, b, b, a,
                    0x3C00, a, b, a, b, a, b, a, b, a,
                )
            };
            eq_f64(
                &format!("agglom nan ({:#010x},{:#010x})", a.to_bits(), b.to_bits()),
                v,
                w,
            );
        }
    }
}
