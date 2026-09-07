//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact rejecting condition, calls BOTH `.so`s and
//! asserts the SAME sentinel/error value is returned *and* that `*out` ends in
//! the same state (pre-filled with `SENTINEL`, so "untouched" is checked too).

mod common;
use common::*;
use std::ffi::c_void;
use std::ptr;

// ---------------------------------------------------------------------------
// Trampoline for the `c2CastRay` fall-off-the-end path (ERRORS.md row 36).
//
// For a `typeB` outside {0,1,2} the C `switch` has no `default`, so the C
// function returns without ever writing `%eax`: the value the caller observes
// IS the value the caller had in `%rax` at the call instruction. Calling the two
// libraries from two adjacent Rust statements does NOT establish the same entry
// state (in unoptimized code the two call sequences leave different values in
// `%rax`), so a naive side-by-side call compares the *harness's* register
// allocation rather than the libraries.
//
// This trampoline fixes the entry state explicitly: it seeds `%rax` with a
// caller-chosen value and then calls the target with the three integer
// arguments. The by-value `c2Ray` stack argument is left uninitialised on
// purpose — the fall-through path provably never reads it (GCC only touches
// `0x10(%rbp)`.. inside the three `case` arms) — so this helper is ONLY valid
// for invalid `typeB` values.
// ---------------------------------------------------------------------------
#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".globl tramp_castray_invalid",
    ".hidden tramp_castray_invalid",
    "tramp_castray_invalid:",
    "push rbp",
    "mov rbp, rsp",
    "sub rsp, 48", // scratch for the (never-read) by-value c2Ray stack arg
    "mov r9, rdi", // fn
    "mov rdi, rsi", // B
    "mov esi, edx", // typeB
    "mov rdx, rcx", // out
    "mov rax, r8", // seed the return register
    "call r9",
    "leave",
    "ret",
);

#[cfg(target_arch = "x86_64")]
extern "C" {
    /// `tramp_castray_invalid(fn, B, typeB, out, rax_seed) -> i64`
    fn tramp_castray_invalid(
        f: FnCastRay,
        b: *const c_void,
        ty: i32,
        out: *mut C2Raycast,
        rax_seed: u64,
    ) -> u64;
}

/// Calls `f` with an invalid `typeB` and a known `%rax` at entry.
#[cfg(target_arch = "x86_64")]
fn call_invalid(f: FnCastRay, b: *const c_void, ty: i32, out: *mut C2Raycast, seed: u64) -> u32 {
    (unsafe { tramp_castray_invalid(f, b, ty, out, seed) } & 0xFFFF_FFFF) as u32
}

fn cir(l: &Pair, ray: C2Ray, c: C2Circle, ctx: String) -> i32 {
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoCircle)(ray, c, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoCircle)(ray, c, &mut or) };
    diff_eq!(ctx, (rc, rcb(oc)), (rr, rcb(or)));
    rc
}

fn aabb(l: &Pair, ray: C2Ray, b: C2AABB, ctx: String) -> i32 {
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoAABB)(ray, b, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoAABB)(ray, b, &mut or) };
    diff_eq!(ctx, (rc, rcb(oc)), (rr, rcb(or)));
    rc
}

fn caps(l: &Pair, ray: C2Ray, c: C2Capsule, ctx: String) -> (i32, C2Raycast) {
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoCapsule)(ray, c, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoCapsule)(ray, c, &mut or) };
    diff_eq!(ctx, (rc, rcb(oc)), (rr, rcb(or)));
    (rc, oc)
}

fn genray(l: &Pair, a: &[f32; 16], ctx: String) -> i32 {
    let mut c1 = SENTINEL;
    let mut c2 = SENTINEL;
    let mut c3 = SENTINEL;
    let mut r1 = SENTINEL;
    let mut r2 = SENTINEL;
    let mut r3 = SENTINEL;
    let rc = unsafe {
        (l.c.gen_ray)(
            &mut c1, &mut c2, &mut c3, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
            a[9], a[10], a[11], a[12], a[13], a[14], a[15],
        )
    };
    let rr = unsafe {
        (l.r.gen_ray)(
            &mut r1, &mut r2, &mut r3, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
            a[9], a[10], a[11], a[12], a[13], a[14], a[15],
        )
    };
    diff_eq!(
        ctx,
        (rc, rcb(c1), rcb(c2), rcb(c3)),
        (rr, rcb(r1), rcb(r2), rcb(r3))
    );
    rc
}

// ===========================================================================
// Rows 1-7: c2RaytoCircle rejections
// ===========================================================================

/// row 1: `disc < 0` — the ray *line* misses the circle entirely.
#[test]
fn err01_circle_disc_negative() {
    let l = libs();
    let mut rng = Rng::new(1);
    let mut n = 0;
    for _ in 0..50_000 {
        let c = C2Circle {
            p: rng.v(20.0),
            r: rng.unit() * 5.0 + 0.1,
        };
        // perpendicular offset strictly greater than r => disc < 0
        let d = rng.unit_dir();
        let perp = C2v { x: -d.y, y: d.x };
        let off = c.r * (1.01 + rng.unit() * 5.0);
        let back = 1.0 + rng.unit() * 50.0;
        let ray = C2Ray {
            p: C2v {
                x: c.p.x + perp.x * off - d.x * back,
                y: c.p.y + perp.y * off - d.y * back,
            },
            d,
            t: back * 3.0,
        };
        let r = cir(l, ray, c, format!("err01 {ray:?} {c:?}"));
        assert_eq!(r, 0, "row 1 must reject: {ray:?} {c:?}");
        n += 1;
    }
    assert!(n > 0);
}

/// row 2: nearest root behind the origin (`t < 0`).
#[test]
fn err02_circle_t_negative() {
    let l = libs();
    let mut rng = Rng::new(2);
    for _ in 0..50_000 {
        let c = C2Circle {
            p: rng.v(20.0),
            r: rng.unit() * 5.0 + 0.1,
        };
        let d = rng.unit_dir();
        // origin strictly past the circle along +d
        let ahead = c.r * (1.01 + rng.unit() * 20.0);
        let ray = C2Ray {
            p: C2v {
                x: c.p.x + d.x * ahead,
                y: c.p.y + d.y * ahead,
            },
            d,
            t: 1e4,
        };
        let r = cir(l, ray, c, format!("err02 {ray:?} {c:?}"));
        assert_eq!(r, 0, "row 2 must reject: {ray:?} {c:?}");
    }
}

/// row 3: intersection beyond the ray length (`t > A.t`).
#[test]
fn err03_circle_t_beyond_length() {
    let l = libs();
    let mut rng = Rng::new(3);
    for _ in 0..50_000 {
        let c = C2Circle {
            p: rng.v(20.0),
            r: rng.unit() * 5.0 + 0.1,
        };
        let d = rng.unit_dir();
        let back = c.r + 1.0 + rng.unit() * 50.0;
        let ray = C2Ray {
            p: C2v {
                x: c.p.x - d.x * back,
                y: c.p.y - d.y * back,
            },
            d,
            // stops strictly short of the entry point (back - r)
            t: (back - c.r) * (1.0 - 0.01 - rng.unit() * 0.5),
        };
        let r = cir(l, ray, c, format!("err03 {ray:?} {c:?}"));
        assert_eq!(r, 0, "row 3 must reject: {ray:?} {c:?}");
    }
}

/// row 4: degenerate point circle `r == 0`.
#[test]
fn err04_circle_zero_radius() {
    let l = libs();
    let mut rng = Rng::new(4);
    for _ in 0..20_000 {
        let c = C2Circle { p: rng.v(20.0), r: 0.0 };
        let d = rng.unit_dir();
        let back = 1.0 + rng.unit() * 50.0;
        // exactly through the centre
        let ray = C2Ray {
            p: C2v {
                x: c.p.x - d.x * back,
                y: c.p.y - d.y * back,
            },
            d,
            t: back * 2.0,
        };
        cir(l, ray, c, format!("err04 through {ray:?} {c:?}"));
        // and generic rays
        let ray2 = C2Ray {
            p: rng.v(20.0),
            d: rng.unit_dir(),
            t: rng.unit() * 60.0,
        };
        let r2 = cir(l, ray2, c, format!("err04 generic {ray2:?} {c:?}"));
        assert!(r2 == 0 || r2 == 1);
    }
    // also -0.0
    for _ in 0..5_000 {
        let c = C2Circle { p: rng.v(20.0), r: -0.0 };
        let ray = C2Ray {
            p: rng.v(20.0),
            d: rng.unit_dir(),
            t: rng.unit() * 60.0,
        };
        cir(l, ray, c, format!("err04 negzero {ray:?} {c:?}"));
    }
}

/// row 5: `r < 0` is NOT rejected; it behaves exactly like `|r|`.
#[test]
fn err05_circle_negative_radius_equals_abs() {
    let l = libs();
    let mut rng = Rng::new(5);
    for _ in 0..20_000 {
        let rad = rng.unit() * 8.0 + 0.1;
        let p = rng.v(20.0);
        let ray = C2Ray {
            p: rng.v(30.0),
            d: rng.unit_dir(),
            t: rng.unit() * 80.0,
        };
        let a = C2Circle { p, r: rad };
        let b = C2Circle { p, r: -rad };
        // C's own claim: +r and -r agree
        let mut o1 = SENTINEL;
        let mut o2 = SENTINEL;
        let c1 = unsafe { (l.c.c2RaytoCircle)(ray, a, &mut o1) };
        let c2 = unsafe { (l.c.c2RaytoCircle)(ray, b, &mut o2) };
        assert_eq!((c1, rcb(o1)), (c2, rcb(o2)), "row 5: C differs for ±r");
        // and Rust matches C for the negative radius
        cir(l, ray, b, format!("err05 {ray:?} {b:?}"));
    }
}

/// row 6: `A.t < 0` can never accept.
#[test]
fn err06_circle_negative_length() {
    let l = libs();
    let mut rng = Rng::new(6);
    for &t in &[-0.0f32, -1e-30, -1.0, -1e30, f32::NEG_INFINITY] {
        for _ in 0..10_000 {
            let c = C2Circle {
                p: rng.v(20.0),
                r: rng.unit() * 5.0 + 0.1,
            };
            let d = rng.unit_dir();
            let back = c.r + 1.0 + rng.unit() * 20.0;
            let ray = C2Ray {
                p: C2v {
                    x: c.p.x - d.x * back,
                    y: c.p.y - d.y * back,
                },
                d,
                t,
            };
            let r = cir(l, ray, c, format!("err06 t={t} {ray:?} {c:?}"));
            if t.is_sign_negative() && t != -0.0 {
                assert_eq!(r, 0, "row 6 must reject for t={t}");
            }
        }
    }
}

/// row 7: any NaN input reaches the final `return 0` with `*out` untouched.
#[test]
fn err07_circle_nan_rejects() {
    let l = libs();
    let mut rng = Rng::new(7);
    for &s in &specials_nan_payloads() {
        if !s.is_nan() {
            continue;
        }
        for slot in 0..6 {
            for _ in 0..2_000 {
                let mut ray = C2Ray {
                    p: rng.v(20.0),
                    d: rng.unit_dir(),
                    t: 50.0,
                };
                let mut c = C2Circle {
                    p: rng.v(10.0),
                    r: 3.0,
                };
                match slot {
                    0 => ray.p.x = s,
                    1 => ray.p.y = s,
                    2 => ray.d.x = s,
                    3 => ray.d.y = s,
                    4 => c.p.x = s,
                    _ => c.r = s,
                }
                let r = cir(l, ray, c, format!("err07 slot{slot} {ray:?} {c:?}"));
                assert_eq!(r, 0, "row 7: NaN input must reject (slot {slot})");
            }
        }
    }
}

// ===========================================================================
// Rows 8-12: c2AABBtoAABB rejections
// ===========================================================================

#[test]
fn err08_11_aabb_to_aabb_each_separation() {
    let l = libs();
    let mut rng = Rng::new(8);
    for dir in 0..4 {
        for _ in 0..20_000 {
            let a = C2AABB {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: 10.0, y: 10.0 },
            };
            let gap = 1e-4 + rng.unit() * 100.0;
            let b = match dir {
                0 => C2AABB {
                    // B.max.x < A.min.x
                    min: C2v { x: -gap - 20.0, y: 0.0 },
                    max: C2v { x: -gap, y: 10.0 },
                },
                1 => C2AABB {
                    // A.max.x < B.min.x
                    min: C2v { x: 10.0 + gap, y: 0.0 },
                    max: C2v { x: 40.0, y: 10.0 },
                },
                2 => C2AABB {
                    // B.max.y < A.min.y
                    min: C2v { x: 0.0, y: -gap - 20.0 },
                    max: C2v { x: 10.0, y: -gap },
                },
                _ => C2AABB {
                    // A.max.y < B.min.y
                    min: C2v { x: 0.0, y: 10.0 + gap },
                    max: C2v { x: 10.0, y: 40.0 },
                },
            };
            let rc = (l.c.c2AABBtoAABB)(a, b);
            let rr = (l.r.c2AABBtoAABB)(a, b);
            diff_eq!(format!("err08_11 dir{dir} {a:?} {b:?}"), rc, rr);
            assert_eq!(rc, 0, "row {} must reject", 8 + dir);
        }
    }
}

/// row 12: NaN coordinates make every `<` false, so the C ACCEPTS.
#[test]
fn err12_aabb_to_aabb_nan_accepts() {
    let l = libs();
    for &s in &specials_nan_payloads() {
        if !s.is_nan() {
            continue;
        }
        for slot in 0..8 {
            let mut a = C2AABB {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: 1.0, y: 1.0 },
            };
            let mut b = C2AABB {
                min: C2v { x: 100.0, y: 100.0 },
                max: C2v { x: 101.0, y: 101.0 },
            };
            match slot {
                0 => a.min.x = s,
                1 => a.min.y = s,
                2 => a.max.x = s,
                3 => a.max.y = s,
                4 => b.min.x = s,
                5 => b.min.y = s,
                6 => b.max.x = s,
                _ => b.max.y = s,
            }
            let rc = (l.c.c2AABBtoAABB)(a, b);
            let rr = (l.r.c2AABBtoAABB)(a, b);
            diff_eq!(format!("err12 slot{slot} {a:?} {b:?}"), rc, rr);
        }
    }
    // the documented all-NaN case: C returns 1
    let n = C2AABB {
        min: C2v { x: f32::NAN, y: f32::NAN },
        max: C2v { x: f32::NAN, y: f32::NAN },
    };
    assert_eq!(
        (l.c.c2AABBtoAABB)(n, n),
        1,
        "row 12: C is documented to accept all-NaN boxes"
    );
    diff_eq!("err12 allnan", (l.c.c2AABBtoAABB)(n, n), (l.r.c2AABBtoAABB)(n, n));
}

// ===========================================================================
// Rows 13-20: c2RaytoAABB rejections
// ===========================================================================

/// row 13: the ray's own bounding box does not overlap `B`.
#[test]
fn err13_aabb_bbox_reject() {
    let l = libs();
    let mut rng = Rng::new(13);
    for _ in 0..50_000 {
        let b = C2AABB {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v { x: 10.0, y: 10.0 },
        };
        // a short ray entirely to the left of the box
        let x0 = -100.0 - rng.unit() * 100.0;
        let ray = C2Ray {
            p: C2v { x: x0, y: rng.sym(50.0) },
            d: rng.unit_dir(),
            t: rng.unit() * 10.0,
        };
        let r = aabb(l, ray, b, format!("err13 {ray:?} {b:?}"));
        assert_eq!(r, 0, "row 13 must reject: {ray:?}");
    }
}

/// row 14: separating-axis test on the skew normal (`d > 0`).
#[test]
fn err14_aabb_separating_axis_reject() {
    let l = libs();
    let mut rng = Rng::new(14);
    let b = C2AABB {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    let mut hits = 0usize;
    for _ in 0..200_000 {
        // long ray whose bbox covers the box but which passes beside it
        let ang = rng.unit() * std::f32::consts::TAU;
        let d = C2v { x: ang.cos(), y: ang.sin() };
        let perp = C2v { x: -d.y, y: d.x };
        let off = 1.5 + rng.unit() * 3.0; // beyond the box half-diagonal
        let ray = C2Ray {
            p: C2v {
                x: perp.x * off - d.x * 20.0,
                y: perp.y * off - d.y * 20.0,
            },
            d,
            t: 40.0,
        };
        // only count the cases that actually reach the d>0 test
        if (l.c.c2AABBtoAABB)(
            C2AABB {
                min: (l.c.c2Minv)(ray.p, (l.c.c2Add)(ray.p, (l.c.c2Mulvs)(ray.d, ray.t))),
                max: (l.c.c2Maxv)(ray.p, (l.c.c2Add)(ray.p, (l.c.c2Mulvs)(ray.d, ray.t))),
            },
            b,
        ) != 0
        {
            let r = aabb(l, ray, b, format!("err14 {ray:?}"));
            if r == 0 {
                hits += 1;
            }
        }
    }
    assert!(hits > 1_000, "row 14: only {hits} separating-axis rejections generated");
}

/// row 15: all four plane parameters exceed 1 (`hit == 0`).
#[test]
fn err15_aabb_no_plane_hit() {
    let l = libs();
    let mut rng = Rng::new(15);
    let b = C2AABB {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    let mut zero = 0usize;
    for _ in 0..200_000 {
        let ray = C2Ray {
            p: rng.v(4.0),
            d: rng.unit_dir(),
            t: rng.unit() * 6.0,
        };
        if aabb(l, ray, b, format!("err15 {ray:?}")) == 0 {
            zero += 1;
        }
    }
    assert!(zero > 0, "row 15: no rejections generated");
}

/// row 16: inverted / empty box — C never validates `min <= max`.
#[test]
fn err16_aabb_inverted_box() {
    let l = libs();
    let mut rng = Rng::new(16);
    let boxes = [
        C2AABB {
            min: C2v { x: 5.0, y: 7.0 },
            max: C2v { x: -2.0, y: -3.0 },
        },
        C2AABB {
            min: C2v { x: 1.0, y: -3.0 },
            max: C2v { x: -1.0, y: 3.0 },
        },
        C2AABB {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v { x: -0.0, y: -0.0 },
        },
    ];
    for (i, &bb) in boxes.iter().enumerate() {
        for _ in 0..20_000 {
            let ray = C2Ray {
                p: rng.v(20.0),
                d: rng.unit_dir(),
                t: rng.unit() * 60.0,
            };
            aabb(l, ray, bb, format!("err16 box{i} {ray:?} {bb:?}"));
        }
    }
}

/// row 17: `A.t == 0` → `p1 == p0`, zero skew normal, `d <= 0`.
#[test]
fn err17_aabb_zero_length_ray() {
    let l = libs();
    let mut rng = Rng::new(17);
    let b = C2AABB {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    for &t in &[0.0f32, -0.0] {
        for _ in 0..20_000 {
            let ray = C2Ray {
                p: rng.v(3.0),
                d: rng.unit_dir(),
                t,
            };
            aabb(l, ray, b, format!("err17 t={t} {ray:?}"));
        }
    }
}

/// row 18: NaN input. Only a NaN that poisons **all four** plane tests makes
/// `hit == 0`; a NaN on a single axis leaves the other axis's `tN` finite, so
/// the C ACCEPTS and writes `*out`. Both behaviours must match.
#[test]
fn err18_aabb_nan_rejects() {
    let l = libs();
    let mut rng = Rng::new(18);
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for &s in &specials_nan_payloads() {
        if !s.is_nan() {
            continue;
        }
        for slot in 0..5 {
            for _ in 0..2_000 {
                let mut ray = C2Ray {
                    p: C2v { x: -10.0, y: 0.0 },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: 40.0,
                };
                match slot {
                    0 => ray.p.x = s,
                    1 => ray.p.y = s,
                    2 => ray.d.x = s,
                    3 => ray.d.y = s,
                    _ => ray.t = s,
                }
                let b = C2AABB {
                    min: rng.v(2.0),
                    max: C2v { x: 5.0, y: 5.0 },
                };
                let r = aabb(l, ray, b, format!("err18 slot{slot} {ray:?} {b:?}"));
                if r == 0 {
                    rejected += 1;
                } else {
                    accepted += 1;
                }
            }
        }
        // both axes NaN -> all four planes poisoned -> reject
        for _ in 0..500 {
            let ray = C2Ray {
                p: C2v { x: s, y: s },
                d: C2v { x: 1.0, y: 0.0 },
                t: 40.0,
            };
            let b = C2AABB {
                min: C2v { x: -1.0, y: -1.0 },
                max: C2v { x: 1.0, y: 1.0 },
            };
            let r = aabb(l, ray, b, format!("err18 bothaxes {ray:?}"));
            assert_eq!(r, 0, "row 18: NaN on both axes must reject");
        }
    }
    eprintln!("row 18: {rejected} rejected / {accepted} accepted (both compared bit-exactly)");
    assert!(rejected > 0 && accepted > 0, "row 18: expected both outcomes");
}

/// rows 19,20: the two `c2RayToPlane_OneDimensional` zero-returns, reached
/// through `c2RaytoAABB` (`da < 0`, and `da == db` with `da*db <= 0`).
#[test]
fn err19_20_ray_to_plane_zero_paths() {
    let l = libs();
    let mut rng = Rng::new(19);
    let b = C2AABB {
        min: C2v { x: -2.0, y: -3.0 },
        max: C2v { x: 5.0, y: 7.0 },
    };
    // row 19: origin on the far side of a plane => da < 0 for that plane
    for _ in 0..20_000 {
        let ray = C2Ray {
            p: C2v {
                x: -50.0 - rng.unit() * 50.0,
                y: rng.sym(6.0),
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: 120.0,
        };
        aabb(l, ray, b, format!("err19 {ray:?}"));
    }
    // row 20: ray exactly parallel to a plane => da == db (d == 0), and the
    // fully-degenerate da == db == 0 case (ray lying in the plane)
    let planes = [b.min.x, b.max.x, b.min.y, b.max.y, 0.0];
    for &v in &planes {
        for horiz in [true, false] {
            for _ in 0..5_000 {
                let s = -60.0 + rng.unit() * 5.0;
                let ray = if horiz {
                    C2Ray {
                        p: C2v { x: s, y: v },
                        d: C2v { x: 1.0, y: 0.0 },
                        t: 130.0,
                    }
                } else {
                    C2Ray {
                        p: C2v { x: v, y: s },
                        d: C2v { x: 0.0, y: 1.0 },
                        t: 130.0,
                    }
                };
                aabb(l, ray, b, format!("err20 v={v} h={horiz} {ray:?}"));
            }
        }
    }
    // ray of zero direction: da == db on BOTH axes simultaneously
    for _ in 0..20_000 {
        let ray = C2Ray {
            p: rng.v(8.0),
            d: C2v { x: 0.0, y: 0.0 },
            t: rng.unit() * 50.0,
        };
        aabb(l, ray, b, format!("err20 zerodir {ray:?}"));
    }
}

// ===========================================================================
// Rows 21-25: c2AABBtoPoint rejections
// ===========================================================================

#[test]
fn err21_24_aabb_to_point_each_side() {
    let l = libs();
    let mut rng = Rng::new(21);
    let b = C2AABB {
        min: C2v { x: -1.0, y: -2.0 },
        max: C2v { x: 3.0, y: 4.0 },
    };
    for side in 0..4 {
        for _ in 0..20_000 {
            let eps = 1e-5 + rng.unit() * 50.0;
            let p = match side {
                0 => C2v { x: b.min.x - eps, y: rng.sym(1.0) }, // B.x < min.x
                1 => C2v { x: rng.sym(1.0), y: b.min.y - eps }, // B.y < min.y
                2 => C2v { x: b.max.x + eps, y: rng.sym(1.0) }, // B.x > max.x
                _ => C2v { x: rng.sym(1.0), y: b.max.y + eps }, // B.y > max.y
            };
            let rc = (l.c.c2AABBtoPoint)(b, p);
            let rr = (l.r.c2AABBtoPoint)(b, p);
            diff_eq!(format!("err21_24 side{side} {p:?}"), rc, rr);
            assert_eq!(rc, 0, "row {} must reject", 21 + side);
        }
    }
}

/// row 25: NaN coordinate → C ACCEPTS.
#[test]
fn err25_aabb_to_point_nan_accepts() {
    let l = libs();
    let b = C2AABB {
        min: C2v { x: -1.0, y: -2.0 },
        max: C2v { x: 3.0, y: 4.0 },
    };
    for &s in &specials_nan_payloads() {
        if !s.is_nan() {
            continue;
        }
        for p in [
            C2v { x: s, y: 0.0 },
            C2v { x: 0.0, y: s },
            C2v { x: s, y: s },
            C2v { x: s, y: 1e9 },
        ] {
            let rc = (l.c.c2AABBtoPoint)(b, p);
            let rr = (l.r.c2AABBtoPoint)(b, p);
            diff_eq!(format!("err25 {p:?}"), rc, rr);
        }
    }
    assert_eq!(
        (l.c.c2AABBtoPoint)(b, C2v { x: f32::NAN, y: f32::NAN }),
        1,
        "row 25: C is documented to accept a NaN point"
    );
}

// ===========================================================================
// Rows 26-28: c2CircleToPoint rejections
// ===========================================================================

/// row 26: a point exactly on the rim is rejected (`<`, not `<=`).
#[test]
fn err26_circle_to_point_on_rim() {
    let l = libs();
    let mut rng = Rng::new(26);
    let mut exact = 0usize;
    for _ in 0..50_000 {
        let c = C2Circle {
            p: rng.v(20.0),
            r: rng.unit() * 10.0 + 0.1,
        };
        // axis-aligned rim points give exactly d2 == r*r
        for (dx, dy) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let p = C2v {
                x: c.p.x + dx * c.r,
                y: c.p.y + dy * c.r,
            };
            let rc = (l.c.c2CircleToPoint)(c, p);
            let rr = (l.r.c2CircleToPoint)(c, p);
            diff_eq!(format!("err26 {c:?} {p:?}"), rc, rr);
            if rc == 0 {
                exact += 1;
            }
        }
    }
    assert!(exact > 0, "row 26: no exact-rim rejection observed");
}

/// row 27: `r == 0` can never accept.
#[test]
fn err27_circle_to_point_zero_radius() {
    let l = libs();
    let mut rng = Rng::new(27);
    for &r in &[0.0f32, -0.0] {
        for _ in 0..20_000 {
            let c = C2Circle { p: rng.v(20.0), r };
            for p in [c.p, rng.v(20.0), C2v { x: 0.0, y: 0.0 }] {
                let rc = (l.c.c2CircleToPoint)(c, p);
                let rr = (l.r.c2CircleToPoint)(c, p);
                diff_eq!(format!("err27 r={r} {c:?} {p:?}"), rc, rr);
                assert_eq!(rc, 0, "row 27 must reject");
            }
        }
    }
}

/// row 28: NaN → reject.
#[test]
fn err28_circle_to_point_nan() {
    let l = libs();
    for &s in &specials_nan_payloads() {
        if !s.is_nan() {
            continue;
        }
        for slot in 0..5 {
            let mut c = C2Circle {
                p: C2v { x: 1.0, y: 2.0 },
                r: 5.0,
            };
            let mut p = C2v { x: 1.0, y: 2.0 };
            match slot {
                0 => c.p.x = s,
                1 => c.p.y = s,
                2 => c.r = s,
                3 => p.x = s,
                _ => p.y = s,
            }
            let rc = (l.c.c2CircleToPoint)(c, p);
            let rr = (l.r.c2CircleToPoint)(c, p);
            diff_eq!(format!("err28 slot{slot} {c:?} {p:?}"), rc, rr);
            assert_eq!(rc, 0, "row 28: NaN must reject (slot {slot})");
        }
    }
}

// ===========================================================================
// Rows 29-35: c2RaytoCapsule rejections
// ===========================================================================

fn rand_capsule(rng: &mut Rng) -> C2Capsule {
    let a = rng.v(30.0);
    let ang = rng.unit() * std::f32::consts::TAU;
    let ln = rng.unit() * 40.0 + 0.5;
    C2Capsule {
        a,
        b: C2v {
            x: a.x + ang.cos() * ln,
            y: a.y + ang.sin() * ln,
        },
        r: rng.unit() * 8.0 + 0.05,
    }
}

fn cap_basis(cap: C2Capsule) -> (C2v, C2v, f32) {
    let dx = cap.b.x - cap.a.x;
    let dy = cap.b.y - cap.a.y;
    let ln = (dx * dx + dy * dy).sqrt();
    let along = C2v { x: dx / ln, y: dy / ln };
    let perp = C2v { x: along.y, y: -along.x };
    (perp, along, ln)
}

/// row 29: the final `return 0` — and `*out` HAS already been written with
/// `t = 0`, `n = norm(b - a)`, which both sides must produce identically.
#[test]
fn err29_capsule_final_reject_writes_out() {
    let l = libs();
    let mut rng = Rng::new(29);
    let mut rejects = 0usize;
    for _ in 0..100_000 {
        let cap = rand_capsule(&mut rng);
        let (perp, along, ln) = cap_basis(cap);
        let sgn = if rng.next_u32() & 1 == 0 { 1.0f32 } else { -1.0 };
        let off = sgn * (cap.r + 1.0 + rng.unit() * 40.0);
        let axial = rng.sym(ln);
        let p = C2v {
            x: cap.a.x + along.x * axial + perp.x * off,
            y: cap.a.y + along.y * axial + perp.y * off,
        };
        // heading AWAY from the capsule, so no sign change and no slab entry
        let ray = C2Ray {
            p,
            d: C2v { x: perp.x * sgn, y: perp.y * sgn },
            t: rng.unit() * 50.0,
        };
        let (r, oc) = caps(l, ray, cap, format!("err29 {ray:?} {cap:?}"));
        if r == 0 {
            rejects += 1;
            let n = (l.c.c2Norm)((l.c.c2Sub)(cap.b, cap.a));
            assert_eq!(
                rcb(oc),
                (fb(0.0), fb(n.x), fb(n.y)),
                "row 29: unexpected *out on the reject path"
            );
        }
    }
    assert!(rejects > 1_000, "row 29: only {rejects} rejections generated");
}

/// row 30: degenerate capsule `a == b` → all-NaN basis → C returns 1 with
/// `out->n = (NaN, NaN)`, `out->t = 0`.
#[test]
fn err30_capsule_degenerate_accepts_with_nan() {
    let l = libs();
    let mut rng = Rng::new(30);
    for _ in 0..30_000 {
        let a = rng.v(20.0);
        let cap = C2Capsule {
            a,
            b: a,
            r: rng.unit() * 5.0,
        };
        let ray = C2Ray {
            p: rng.v(20.0),
            d: rng.unit_dir(),
            t: rng.unit() * 60.0,
        };
        let (r, oc) = caps(l, ray, cap, format!("err30 {ray:?} {cap:?}"));
        assert_eq!(r, 1, "row 30: degenerate capsule must accept");
        assert!(oc.n.x.is_nan() && oc.n.y.is_nan(), "row 30: n must be NaN");
        assert_eq!(fb(oc.t), fb(0.0), "row 30: t must be 0");
    }
}

/// row 31: `B.r == 0`.
#[test]
fn err31_capsule_zero_radius() {
    let l = libs();
    let mut rng = Rng::new(31);
    for &r in &[0.0f32, -0.0] {
        for _ in 0..30_000 {
            let mut cap = rand_capsule(&mut rng);
            cap.r = r;
            let (_p, along, ln) = cap_basis(cap);
            let s = rng.unit();
            let target = C2v {
                x: cap.a.x + along.x * (s * ln),
                y: cap.a.y + along.y * (s * ln),
            };
            let p = rng.v(50.0);
            let (dx, dy) = (target.x - p.x, target.y - p.y);
            let dl = (dx * dx + dy * dy).sqrt();
            let ray = C2Ray {
                p,
                d: C2v { x: dx / dl, y: dy / dl },
                t: dl * (0.5 + rng.unit() * 1.5),
            };
            caps(l, ray, cap, format!("err31 r={r} {ray:?} {cap:?}"));
        }
    }
}

/// row 32: `B.r < 0` → inverted `capsule_bb`; not rejected by the C.
#[test]
fn err32_capsule_negative_radius() {
    let l = libs();
    let mut rng = Rng::new(32);
    for _ in 0..30_000 {
        let mut cap = rand_capsule(&mut rng);
        cap.r = -(rng.unit() * 8.0 + 0.05);
        let ray = C2Ray {
            p: rng.v(50.0),
            d: rng.unit_dir(),
            t: rng.unit() * 100.0,
        };
        caps(l, ray, cap, format!("err32 {ray:?} {cap:?}"));
        let (_p, along, ln) = cap_basis(cap);
        let s = rng.unit();
        let target = C2v {
            x: cap.a.x + along.x * (s * ln),
            y: cap.a.y + along.y * (s * ln),
        };
        let (dx, dy) = (target.x - ray.p.x, target.y - ray.p.y);
        let dl = (dx * dx + dy * dy).sqrt();
        let ray2 = C2Ray {
            p: ray.p,
            d: C2v { x: dx / dl, y: dy / dl },
            t: dl * 1.4,
        };
        caps(l, ray2, cap, format!("err32 aimed {ray2:?} {cap:?}"));
    }
}

/// row 33: NaN ray → `c2AABBtoPoint` accepts → returns 1 with `t = 0`.
#[test]
fn err33_capsule_nan_ray_accepts() {
    let l = libs();
    let mut rng = Rng::new(33);
    for &s in &specials_nan_payloads() {
        if !s.is_nan() {
            continue;
        }
        for slot in 0..5 {
            for _ in 0..2_000 {
                let cap = rand_capsule(&mut rng);
                let mut ray = C2Ray {
                    p: rng.v(40.0),
                    d: rng.unit_dir(),
                    t: 50.0,
                };
                match slot {
                    0 => ray.p.x = s,
                    1 => ray.p.y = s,
                    2 => ray.d.x = s,
                    3 => ray.d.y = s,
                    _ => ray.t = s,
                }
                let (r, oc) = caps(l, ray, cap, format!("err33 slot{slot} {ray:?} {cap:?}"));
                if slot <= 1 {
                    assert_eq!(r, 1, "row 33: NaN origin must accept (slot {slot})");
                    let n = (l.c.c2Norm)((l.c.c2Sub)(cap.b, cap.a));
                    assert_eq!(rcb(oc), (fb(0.0), fb(n.x), fb(n.y)));
                }
            }
        }
    }
}

/// rows 34,35: delegated rejections — the chosen end circle misses.
#[test]
fn err34_35_capsule_delegated_circle_miss() {
    let l = libs();
    let mut rng = Rng::new(34);
    let mut deleg_rejects = 0usize;
    let mut endcap_rejects = 0usize;
    for _ in 0..200_000 {
        let cap = rand_capsule(&mut rng);
        let (perp, along, ln) = cap_basis(cap);
        if rng.next_u32() & 1 == 0 {
            // row 34: inside the slab width, far past a cap, ray pointing away
            let off = rng.sym(cap.r * 0.95);
            let axial = -(cap.r * 2.0 + rng.unit() * 30.0);
            let p = C2v {
                x: cap.a.x + along.x * axial + perp.x * off,
                y: cap.a.y + along.y * axial + perp.y * off,
            };
            let ray = C2Ray {
                p,
                d: C2v { x: -along.x, y: -along.y },
                t: rng.unit() * 50.0,
            };
            let (r, oc) = caps(l, ray, cap, format!("err34 {ray:?} {cap:?}"));
            if r == 0 {
                deleg_rejects += 1;
                let n = (l.c.c2Norm)((l.c.c2Sub)(cap.b, cap.a));
                assert_eq!(
                    rcb(oc),
                    (fb(0.0), fb(n.x), fb(n.y)),
                    "row 34: *out must keep the pre-delegation values"
                );
            }
        } else {
            // row 35: shallow crossing well beyond a cap => y outside [0, len]
            let sgn = if rng.next_u32() & 1 == 0 { 1.0f32 } else { -1.0 };
            let off = sgn * (cap.r + 0.5 + rng.unit() * 20.0);
            let axial = if rng.next_u32() & 1 == 0 {
                -(ln * (1.0 + rng.unit() * 5.0))
            } else {
                ln * (2.0 + rng.unit() * 5.0)
            };
            let p = C2v {
                x: cap.a.x + along.x * axial + perp.x * off,
                y: cap.a.y + along.y * axial + perp.y * off,
            };
            let ray = C2Ray {
                p,
                d: C2v { x: -perp.x * sgn, y: -perp.y * sgn },
                t: off.abs() * (1.0 + rng.unit() * 3.0),
            };
            let (r, oc) = caps(l, ray, cap, format!("err35 {ray:?} {cap:?}"));
            if r == 0 {
                endcap_rejects += 1;
                let n = (l.c.c2Norm)((l.c.c2Sub)(cap.b, cap.a));
                assert_eq!(rcb(oc), (fb(0.0), fb(n.x), fb(n.y)));
            }
        }
    }
    assert!(deleg_rejects > 100, "row 34: only {deleg_rejects} rejections");
    assert!(endcap_rejects > 100, "row 35: only {endcap_rejects} rejections");
}

// ===========================================================================
// Rows 36-39: c2CastRay — invalid enum values and survivable null pointers
// ===========================================================================

/// row 36: `typeB` outside `{0,1,2}`. The C `switch` has no `default` arm and
/// the function is not `void`, so the C returns without writing the return
/// register: the caller observes its own incoming `%rax`. Verified through the
/// trampoline, which fixes that entry state, so C and Rust are compared under
/// *identical* conditions.
#[test]
fn err36_castray_invalid_enum() {
    let l = libs();
    let mut rng = Rng::new(36);
    let cir_shape = C2Circle {
        p: C2v { x: 0.0, y: 0.0 },
        r: 2.0,
    };
    let mut bad: Vec<i32> = vec![
        3, 4, 5, 7, 8, 16, 255, 256, 1000, -1, -2, -3, -128, i32::MIN, i32::MAX,
        i32::MIN + 1, i32::MAX - 1, 0x1_0000, 0x7FFF_FFFE,
    ];
    for _ in 0..2_000 {
        let v = rng.next_u32() as i32;
        if !(0..=2).contains(&v) {
            bad.push(v);
        }
    }
    let seeds = [
        0u64,
        1,
        0xDEAD_BEEF,
        0x42200000,
        u32::MAX as u64,
        0x1234_5678_9ABC_DEF0,
    ];
    for ty in bad {
        for &seed in &seeds {
            let mut oc = SENTINEL;
            let mut or = SENTINEL;
            let p = &cir_shape as *const C2Circle as *const c_void;
            let rc = call_invalid(l.c.c2CastRay, p, ty, &mut oc, seed);
            let rr = call_invalid(l.r.c2CastRay, p, ty, &mut or, seed);
            diff_eq!(
                format!("err36 ty={ty} seed={seed:#x}"),
                (rc, rcb(oc)),
                (rr, rcb(or))
            );
            // The documented C behaviour: the return register is untouched.
            assert_eq!(
                rc, seed as u32,
                "row 36: C did not preserve the incoming return register"
            );
            assert_eq!(
                rr, seed as u32,
                "row 36: Rust did not preserve the incoming return register"
            );
            // and `*out` is never written on this path
            assert_eq!(rcb(oc), rcb(SENTINEL), "row 36: C wrote to *out");
            assert_eq!(rcb(or), rcb(SENTINEL), "row 36: Rust wrote to *out");
        }
    }
}

/// row 37: invalid enum with BOTH pointers null — the C never dereferences on
/// this path, so it must not crash and must agree with Rust.
#[test]
fn err37_castray_invalid_enum_null_pointers() {
    let l = libs();
    for ty in [3i32, -1, 99, i32::MIN, i32::MAX, 4, 256] {
        for seed in [0u64, 7, 0xFFFF_FFFF, 0xABCD_1234] {
            let rc = call_invalid(l.c.c2CastRay, ptr::null(), ty, ptr::null_mut(), seed);
            let rr = call_invalid(l.r.c2CastRay, ptr::null(), ty, ptr::null_mut(), seed);
            diff_eq!(format!("err37 ty={ty} seed={seed:#x}"), rc, rr);
            assert_eq!(rc, seed as u32, "row 37: C clobbered the return register");
            assert_eq!(rr, seed as u32, "row 37: Rust clobbered the return register");
        }
    }
}

/// row 38: `out == NULL` with a CIRCLE that the ray misses — the C never writes
/// `*out` on the reject path, so a null `out` is survivable.
#[test]
fn err38_null_out_circle_reject() {
    let l = libs();
    let mut rng = Rng::new(38);
    for _ in 0..20_000 {
        let c = C2Circle {
            p: C2v { x: 1e6, y: 1e6 },
            r: 0.5,
        };
        let ray = C2Ray {
            p: rng.v(10.0),
            d: rng.unit_dir(),
            t: rng.unit() * 10.0,
        };
        let rc = unsafe { (l.c.c2RaytoCircle)(ray, c, ptr::null_mut()) };
        let rr = unsafe { (l.r.c2RaytoCircle)(ray, c, ptr::null_mut()) };
        diff_eq!(format!("err38 direct {ray:?}"), rc, rr);
        assert_eq!(rc, 0);
        let p = &c as *const C2Circle as *const c_void;
        let rc2 = unsafe { (l.c.c2CastRay)(ray, p, C2_TYPE_CIRCLE, ptr::null_mut()) };
        let rr2 = unsafe { (l.r.c2CastRay)(ray, p, C2_TYPE_CIRCLE, ptr::null_mut()) };
        diff_eq!(format!("err38 dispatch {ray:?}"), rc2, rr2);
    }
    // NaN cases: probe with a real buffer first, then re-run the confirmed
    // rejections with a null `out`.
    for slot in 0..4 {
        for _ in 0..2_000 {
            let mut ray = C2Ray {
                p: C2v { x: 0.0, y: 0.0 },
                d: C2v { x: 1.0, y: 0.0 },
                t: 10.0,
            };
            match slot {
                0 => ray.p.x = f32::NAN,
                1 => ray.p.y = f32::NAN,
                2 => ray.d.x = f32::NAN,
                _ => ray.d.y = f32::NAN,
            }
            let c = C2Circle {
                p: C2v { x: 5.0, y: 0.0 },
                r: 1.0,
            };
            let probe = cir(l, ray, c, format!("err38 nan probe slot{slot}"));
            if probe != 0 {
                continue;
            }
            let rc = unsafe { (l.c.c2RaytoCircle)(ray, c, ptr::null_mut()) };
            let rr = unsafe { (l.r.c2RaytoCircle)(ray, c, ptr::null_mut()) };
            diff_eq!(format!("err38 nan slot{slot}"), rc, rr);
            assert_eq!(rc, 0);
        }
    }
}

/// row 39: `out == NULL` with an AABB the ray's bbox misses.
#[test]
fn err39_null_out_aabb_reject() {
    let l = libs();
    let mut rng = Rng::new(39);
    for _ in 0..20_000 {
        let b = C2AABB {
            min: C2v { x: 1e6, y: 1e6 },
            max: C2v { x: 1e6 + 1.0, y: 1e6 + 1.0 },
        };
        let ray = C2Ray {
            p: rng.v(10.0),
            d: rng.unit_dir(),
            t: rng.unit() * 10.0,
        };
        let rc = unsafe { (l.c.c2RaytoAABB)(ray, b, ptr::null_mut()) };
        let rr = unsafe { (l.r.c2RaytoAABB)(ray, b, ptr::null_mut()) };
        diff_eq!(format!("err39 direct {ray:?}"), rc, rr);
        assert_eq!(rc, 0);
        let p = &b as *const C2AABB as *const c_void;
        let rc2 = unsafe { (l.c.c2CastRay)(ray, p, C2_TYPE_AABB, ptr::null_mut()) };
        let rr2 = unsafe { (l.r.c2CastRay)(ray, p, C2_TYPE_AABB, ptr::null_mut()) };
        diff_eq!(format!("err39 dispatch {ray:?}"), rc2, rr2);
    }
    // NaN cases: `c2RaytoAABB` only leaves `*out` untouched when it actually
    // rejects, so probe with a real buffer first and only then re-run with a
    // null `out` for the confirmed-rejecting inputs.
    for slot in 0..5 {
        for _ in 0..2_000 {
            let mut ray = C2Ray {
                p: C2v { x: -5.0, y: 0.0 },
                d: C2v { x: 1.0, y: 0.0 },
                t: 20.0,
            };
            match slot {
                0 => ray.p.x = f32::NAN,
                1 => ray.p.y = f32::NAN,
                2 => ray.d.x = f32::NAN,
                3 => ray.d.y = f32::NAN,
                _ => ray.t = f32::NAN,
            }
            let b = C2AABB {
                min: C2v { x: -1.0, y: -1.0 },
                max: C2v { x: 1.0, y: 1.0 },
            };
            let probe = aabb(l, ray, b, format!("err39 nan probe slot{slot}"));
            if probe != 0 {
                continue; // this input writes `*out`; null would be a real UB deref
            }
            let rc = unsafe { (l.c.c2RaytoAABB)(ray, b, ptr::null_mut()) };
            let rr = unsafe { (l.r.c2RaytoAABB)(ray, b, ptr::null_mut()) };
            diff_eq!(format!("err39 nan slot{slot}"), rc, rr);
            assert_eq!(rc, 0);
        }
    }
}

// ===========================================================================
// Rows 40-41: c2Div / c2Norm degenerate values
// ===========================================================================

/// row 40: division by zero and `c2Norm` of the zero vector.
#[test]
fn err40_div_and_norm_by_zero() {
    let l = libs();
    for &b in &[0.0f32, -0.0] {
        for &x in &specials_nan_payloads() {
            for &y in &specials_nan_payloads() {
                let a = C2v { x, y };
                diff_eq!(
                    format!("err40 c2Div ({:#x},{:#x})/{:#x}", fb(x), fb(y), fb(b)),
                    vb((l.c.c2Div)(a, b)),
                    vb((l.r.c2Div)(a, b))
                );
            }
        }
    }
    for &x in &[0.0f32, -0.0] {
        for &y in &[0.0f32, -0.0] {
            let a = C2v { x, y };
            let cv = (l.c.c2Norm)(a);
            diff_eq!(
                format!("err40 c2Norm ({:#x},{:#x})", fb(x), fb(y)),
                vb(cv),
                vb((l.r.c2Norm)(a))
            );
            assert!(
                cv.x.is_nan() && cv.y.is_nan(),
                "row 40: c2Norm(0,0) must be NaN, got {cv:?}"
            );
        }
    }
    for &b in &[f32::from_bits(1), -f32::from_bits(1), f32::MIN_POSITIVE] {
        for &x in &specials_nan_payloads() {
            let a = C2v { x, y: 1.0 };
            diff_eq!(
                format!("err40 denorm div {:#x}/{:#x}", fb(x), fb(b)),
                vb((l.c.c2Div)(a, b)),
                vb((l.r.c2Div)(a, b))
            );
        }
    }
}

/// row 41: `c2Norm` propagates NaN / inf.
#[test]
fn err41_norm_nan_inf() {
    let l = libs();
    let mut rng = Rng::new(41);
    for &x in &specials_nan_payloads() {
        for &y in &specials_nan_payloads() {
            let a = C2v { x, y };
            diff_eq!(
                format!("err41 c2Norm ({:#x},{:#x})", fb(x), fb(y)),
                vb((l.c.c2Norm)(a)),
                vb((l.r.c2Norm)(a))
            );
            diff_eq!(
                format!("err41 c2Len ({:#x},{:#x})", fb(x), fb(y)),
                fb((l.c.c2Len)(a)),
                fb((l.r.c2Len)(a))
            );
        }
    }
    for _ in 0..50_000 {
        let a = C2v {
            x: rng.any_bits(),
            y: rng.any_bits(),
        };
        diff_eq!(
            format!("err41 fuzz ({:#x},{:#x})", fb(a.x), fb(a.y)),
            vb((l.c.c2Norm)(a)),
            vb((l.r.c2Norm)(a))
        );
    }
}

// ===========================================================================
// Rows 42-44: gen_ray degenerate inputs
// ===========================================================================

/// row 42: `mp == ray.p` → NaN direction for every sub-cast.
#[test]
fn err42_gen_ray_zero_length_ray() {
    let l = libs();
    let mut rng = Rng::new(42);
    let mut masks = std::collections::BTreeMap::<i32, usize>::new();
    for _ in 0..30_000 {
        let px = rng.sym(30.0);
        let py = rng.sym(30.0);
        let a = [
            px, py, px, py,
            rng.sym(40.0), rng.sym(40.0), rng.unit() * 20.0,
            rng.sym(40.0), rng.sym(40.0), rng.sym(40.0), rng.sym(40.0),
            rng.unit() * 8.0 + 0.05,
            rng.sym(40.0), rng.sym(40.0), rng.sym(40.0), rng.sym(40.0),
        ];
        let m = genray(l, &a, format!("err42 {a:?}"));
        *masks.entry(m).or_insert(0) += 1;
    }
    eprintln!("row 42 observed masks: {masks:?}");
}

/// row 43: all-zero arguments.
#[test]
fn err43_gen_ray_all_zero() {
    let l = libs();
    genray(l, &[0.0f32; 16], "err43 all-zero".to_string());
    genray(l, &[-0.0f32; 16], "err43 all-negative-zero".to_string());
    for mask in 0u32..(1 << 10) {
        let mut a = [0.0f32; 16];
        for i in 0..10 {
            a[i] = if mask & (1 << i) != 0 { -0.0 } else { 0.0 };
        }
        genray(l, &a, format!("err43 signmask={mask}"));
    }
}

/// row 44: NaN / inf arguments are not validated at all.
#[test]
fn err44_gen_ray_nan_inf_args() {
    let l = libs();
    let mut rng = Rng::new(44);
    for &s in &specials_nan_payloads() {
        for slot in 0..16 {
            for _ in 0..200 {
                let mut a = [0.0f32; 16];
                for v in a.iter_mut() {
                    *v = rng.sym(30.0);
                }
                a[slot] = s;
                genray(l, &a, format!("err44 slot{slot}={:#x} {a:?}", fb(s)));
            }
        }
    }
    for _ in 0..30_000 {
        let mut a = [0.0f32; 16];
        for v in a.iter_mut() {
            *v = rng.special(1e6);
        }
        genray(l, &a, format!("err44 all-special {a:?}"));
    }
    for _ in 0..30_000 {
        let mut a = [0.0f32; 16];
        for v in a.iter_mut() {
            *v = rng.any_bits();
        }
        genray(l, &a, format!("err44 bits {a:?}"));
    }
}

// ===========================================================================
// Generic boundary sweeps required in addition to the table
// ===========================================================================

/// Out-of-range enum values for a dense sweep, with each shape payload present.
/// Compared through the trampoline (see row 36) so the entry state is identical.
#[test]
fn err_generic_enum_sweep() {
    let l = libs();
    let mut rng = Rng::new(0xEEEE);
    let cap = C2Capsule {
        a: C2v { x: -1.0, y: -2.0 },
        b: C2v { x: 3.0, y: 4.0 },
        r: 1.5,
    };
    let p = &cap as *const C2Capsule as *const c_void;
    for ty in -300i32..=300 {
        if (0..=2).contains(&ty) {
            continue; // valid values are covered by CONFIGS.md rows 49-52
        }
        for seed in [0u64, 0x5A5A_5A5A, 0xFFFF_FFFF] {
            let mut oc = SENTINEL;
            let mut or = SENTINEL;
            let rc = call_invalid(l.c.c2CastRay, p, ty, &mut oc, seed);
            let rr = call_invalid(l.r.c2CastRay, p, ty, &mut or, seed);
            diff_eq!(
                format!("enum sweep ty={ty} seed={seed:#x}"),
                (rc, rcb(oc)),
                (rr, rcb(or))
            );
            assert_eq!(rc, seed as u32);
            assert_eq!(rr, seed as u32);
        }
    }
    // and the three VALID values still dispatch correctly (regression guard for
    // the naked shim's range check)
    for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
        for _ in 0..20_000 {
            let mut oc = SENTINEL;
            let mut or = SENTINEL;
            let ray = C2Ray {
                p: rng.v(30.0),
                d: rng.unit_dir(),
                t: rng.unit() * 60.0,
            };
            let rc = unsafe { (l.c.c2CastRay)(ray, p, ty, &mut oc) };
            let rr = unsafe { (l.r.c2CastRay)(ray, p, ty, &mut or) };
            diff_eq!(
                format!("enum sweep valid ty={ty} {ray:?}"),
                (rc, rcb(oc)),
                (rr, rcb(or))
            );
        }
    }
    // randomized invalid enums
    for _ in 0..20_000 {
        let ty = loop {
            let v = rng.next_u32() as i32;
            if !(0..=2).contains(&v) {
                break v;
            }
        };
        let seed = rng.next_u32() as u64;
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = call_invalid(l.c.c2CastRay, p, ty, &mut oc, seed);
        let rr = call_invalid(l.r.c2CastRay, p, ty, &mut or, seed);
        diff_eq!(
            format!("enum sweep rnd ty={ty} seed={seed:#x}"),
            (rc, rcb(oc)),
            (rr, rcb(or))
        );
        assert_eq!(rc, seed as u32);
        assert_eq!(rr, seed as u32);
    }
}

/// One step past every documented boundary of the raycast predicates.
#[test]
fn err_generic_one_step_past_boundaries() {
    let l = libs();
    let mut rng = Rng::new(0xB0DE);
    let step = |v: f32, k: i32| -> f32 {
        if v == 0.0 {
            return if k > 0 { f32::from_bits(1) } else { -f32::from_bits(1) };
        }
        let b = v.to_bits() as i32;
        f32::from_bits((if v > 0.0 { b + k } else { b - k }) as u32)
    };
    for _ in 0..20_000 {
        let c = C2Circle {
            p: rng.v(10.0),
            r: rng.unit() * 5.0 + 0.5,
        };
        let d = rng.unit_dir();
        let back = 1.0 + rng.unit() * 20.0;
        let entry = back - c.r;
        let base = C2Ray {
            p: C2v {
                x: c.p.x - d.x * back,
                y: c.p.y - d.y * back,
            },
            d,
            t: entry,
        };
        for k in [-2i32, -1, 0, 1, 2] {
            let ray = C2Ray {
                t: step(base.t, k),
                ..base
            };
            cir(l, ray, c, format!("boundary t k={k} {ray:?} {c:?}"));
        }
        let perp = C2v { x: -d.y, y: d.x };
        for k in [-2i32, -1, 0, 1, 2] {
            let rr = step(c.r, k);
            let ray = C2Ray {
                p: C2v {
                    x: c.p.x + perp.x * rr - d.x * back,
                    y: c.p.y + perp.y * rr - d.y * back,
                },
                d,
                t: back * 2.0,
            };
            cir(l, ray, c, format!("boundary tangent k={k} {ray:?} {c:?}"));
        }
        let b = C2AABB {
            min: C2v { x: -1.0, y: -1.0 },
            max: C2v { x: 1.0, y: 1.0 },
        };
        for k in [-1i32, 0, 1] {
            let ray = C2Ray {
                p: C2v { x: -10.0, y: step(1.0, k) },
                d: C2v { x: 1.0, y: 0.0 },
                t: 30.0,
            };
            aabb(l, ray, b, format!("boundary aabb face k={k} {ray:?}"));
        }
    }
}
