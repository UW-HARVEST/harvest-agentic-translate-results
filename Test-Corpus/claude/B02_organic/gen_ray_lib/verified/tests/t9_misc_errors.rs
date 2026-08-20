//! Phase C leftovers: E30, E31, E32 and E36 (misaligned out-pointer).

mod common;
use common::*;

/// E31: `sqrtf` of NaN / negative-zero / infinity, both directly through
/// `c2Len` and indirectly through `c2RaytoCircle`'s `disc`.
#[test]
fn e31_sqrtf_nan_parity() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xE31);

    // Direct: every special value in both components.
    for &x in &SPECIALS {
        for &y in &SPECIALS {
            let v = c2v { x, y };
            d.check(f_eq((c.c2Len)(v), (r.c2Len)(v)), || {
                format!(
                    "c2Len({}): C={} RUST={}",
                    fmt_v(v),
                    fmt_f((c.c2Len)(v)),
                    fmt_f((r.c2Len)(v))
                )
            });
            d.check(v_eq((c.c2Norm)(v), (r.c2Norm)(v)), || {
                format!(
                    "c2Norm({}): C={} RUST={}",
                    fmt_v(v),
                    fmt_v((c.c2Norm)(v)),
                    fmt_v((r.c2Norm)(v))
                )
            });
        }
    }
    // sNaN payloads survive quieting identically.
    for bits in [
        0x7f80_0001u32,
        0x7f80_1234,
        0xff80_0001,
        0xffbf_ffff,
        0x7fc0_0000,
        0xffc0_0000,
        0x7fff_ffff,
        0xffff_ffff,
    ] {
        let v = c2v {
            x: f32::from_bits(bits),
            y: 0.0,
        };
        d.check(f_eq((c.c2Len)(v), (r.c2Len)(v)), || {
            format!(
                "c2Len(nan {bits:#x}): C={} RUST={}",
                fmt_f((c.c2Len)(v)),
                fmt_f((r.c2Len)(v))
            )
        });
    }

    // Indirect: `disc` is NaN inside c2RaytoCircle (inf - inf) but `b` is
    // finite, so the NaN produced by sqrtf is what reaches the subtraction.
    for _ in 0..5_000 {
        let a = c2Ray {
            p: c2v {
                x: rng.nice(),
                y: rng.nice(),
            },
            d: c2v {
                x: f32::INFINITY,
                y: rng.nice(),
            },
            t: f32::INFINITY,
        };
        for b in [
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: f32::INFINITY,
            },
            c2Circle {
                p: c2v {
                    x: f32::INFINITY,
                    y: 0.0,
                },
                r: f32::INFINITY,
            },
            c2Circle {
                p: c2v {
                    x: rng.nice(),
                    y: rng.nice(),
                },
                r: f32::MAX,
            },
        ] {
            let rc = call_circle(c, a, b);
            let rr = call_circle(r, a, b);
            d.ray("E31/disc-nan", || format!("{:?} {:?}", a, b), rc, rr);
        }
    }
    d.finish("E31 sqrtf NaN parity");
}

/// E32: the x86 "indefinite" QNaN produced by invalid operations
/// (`inf * 0`, `inf - inf`, `0 / 0`) is `0xffc00000` - sign bit SET.
#[test]
fn e32_indefinite_nan_bits() {
    let (c, r) = apis();
    let mut d = Diff::new();

    // inf * 0 through c2Mulvs / c2Dot / c2MulmvT
    let inf_vec = c2v {
        x: f32::INFINITY,
        y: f32::NEG_INFINITY,
    };
    let zero_vec = c2v { x: 0.0, y: -0.0 };
    for (label, cv, rv) in [
        (
            "c2Mulvs(inf,0)",
            (c.c2Mulvs)(inf_vec, 0.0),
            (r.c2Mulvs)(inf_vec, 0.0),
        ),
        (
            "c2Mulvs(0,inf)",
            (c.c2Mulvs)(zero_vec, f32::INFINITY),
            (r.c2Mulvs)(zero_vec, f32::INFINITY),
        ),
        (
            "c2Div(inf,inf)",
            (c.c2Div)(inf_vec, f32::INFINITY),
            (r.c2Div)(inf_vec, f32::INFINITY),
        ),
        (
            "c2Norm(inf)",
            (c.c2Norm)(inf_vec),
            (r.c2Norm)(inf_vec),
        ),
        (
            "c2Sub(inf,inf)",
            (c.c2Sub)(inf_vec, inf_vec),
            (r.c2Sub)(inf_vec, inf_vec),
        ),
        (
            "c2Add(inf,-inf)",
            (c.c2Add)(
                inf_vec,
                c2v {
                    x: f32::NEG_INFINITY,
                    y: f32::INFINITY,
                },
            ),
            (r.c2Add)(
                inf_vec,
                c2v {
                    x: f32::NEG_INFINITY,
                    y: f32::INFINITY,
                },
            ),
        ),
    ] {
        d.check(v_eq(cv, rv), || {
            format!("{label}: C={} RUST={}", fmt_v(cv), fmt_v(rv))
        });
        // Document the actual bit pattern the C produces.
        eprintln!("{label} -> {}", fmt_v(cv));
    }

    let dc = (c.c2Dot)(inf_vec, zero_vec);
    let dr = (r.c2Dot)(inf_vec, zero_vec);
    d.check(f_eq(dc, dr), || {
        format!("c2Dot(inf,0): C={} RUST={}", fmt_f(dc), fmt_f(dr))
    });
    eprintln!("c2Dot(inf,0) -> {}", fmt_f(dc));

    let m = c2m {
        x: inf_vec,
        y: zero_vec,
    };
    let mc = (c.c2MulmvT)(m, zero_vec);
    let mr = (r.c2MulmvT)(m, zero_vec);
    d.check(v_eq(mc, mr), || {
        format!("c2MulmvT(inf,0): C={} RUST={}", fmt_v(mc), fmt_v(mr))
    });

    // Exhaustive inf/zero cross product over every operand slot.
    let vals = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
    ];
    for &a in &vals {
        for &b in &vals {
            for &e in &vals {
                for &f in &vals {
                    let u = c2v { x: a, y: b };
                    let v = c2v { x: e, y: f };
                    d.check(f_eq((c.c2Dot)(u, v), (r.c2Dot)(u, v)), || {
                        format!("c2Dot({},{})", fmt_v(u), fmt_v(v))
                    });
                    d.check(v_eq((c.c2Add)(u, v), (r.c2Add)(u, v)), || {
                        format!("c2Add({},{})", fmt_v(u), fmt_v(v))
                    });
                    d.check(v_eq((c.c2Sub)(u, v), (r.c2Sub)(u, v)), || {
                        format!("c2Sub({},{})", fmt_v(u), fmt_v(v))
                    });
                    d.check(v_eq((c.c2Mulvs)(u, e), (r.c2Mulvs)(u, e)), || {
                        format!("c2Mulvs({},{})", fmt_v(u), fmt_f(e))
                    });
                    d.check(v_eq((c.c2Div)(u, e), (r.c2Div)(u, e)), || {
                        format!("c2Div({},{})", fmt_v(u), fmt_f(e))
                    });
                    let mm = c2m { x: u, y: v };
                    d.check(
                        v_eq((c.c2MulmvT)(mm, u), (r.c2MulmvT)(mm, u)),
                        || format!("c2MulmvT({},{},{})", fmt_v(u), fmt_v(v), fmt_v(u)),
                    );
                }
            }
        }
    }
    d.finish("E32 indefinite-NaN bit patterns");
}

/// E36: a MISALIGNED `c2Raycast *out`.  x86 `movss`/`movq` do not require
/// alignment, so the C simply writes; the Rust translation must not introduce
/// an alignment check (and must not use an alignment-requiring instruction).
#[test]
fn e36_misaligned_out_pointer() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xE36);
    for off in 1..=7usize {
        for i in 0..2_000 {
            // 12 bytes of payload at a deliberately odd address.
            let mut buf_c = [0u8; 32];
            let mut buf_r = [0u8; 32];
            for b in buf_c.iter_mut() {
                *b = 0xA5;
            }
            buf_r.copy_from_slice(&buf_c);
            let pc = unsafe { buf_c.as_mut_ptr().add(off) } as *mut c2Raycast;
            let pr = unsafe { buf_r.as_mut_ptr().add(off) } as *mut c2Raycast;

            let a = c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 100.0,
            };
            let (rc, rr) = match i % 3 {
                0 => {
                    let s = c2Circle {
                        p: c2v { x: 5.0, y: rng.uniform(0.5) },
                        r: 1.0,
                    };
                    unsafe {
                        (
                            (c.c2RaytoCircle)(a, s, pc),
                            (r.c2RaytoCircle)(a, s, pr),
                        )
                    }
                }
                1 => {
                    let s = c2AABB {
                        min: c2v { x: -1.0, y: -1.0 },
                        max: c2v { x: 1.0, y: 1.0 },
                    };
                    unsafe { ((c.c2RaytoAABB)(a, s, pc), (r.c2RaytoAABB)(a, s, pr)) }
                }
                _ => {
                    let s = c2Capsule {
                        a: c2v { x: 5.0, y: -2.0 },
                        b: c2v { x: 5.0, y: 2.0 },
                        r: 1.0,
                    };
                    unsafe {
                        (
                            (c.c2RaytoCapsule)(a, s, pc),
                            (r.c2RaytoCapsule)(a, s, pr),
                        )
                    }
                }
            };
            d.check(rc == rr, || format!("E36 ret mismatch off={off}: {rc} vs {rr}"));
            d.check(buf_c == buf_r, || {
                format!(
                    "E36 buffer mismatch off={off} case={}:\n  C   ={:02x?}\n  RUST={:02x?}",
                    i % 3,
                    buf_c,
                    buf_r
                )
            });
        }
    }
    d.finish("E36 misaligned out pointer");
}
