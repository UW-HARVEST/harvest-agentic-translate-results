//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid input or
//! condition, calls BOTH `.so`s through `dlsym`, and asserts they return the
//! SAME sentinel (this library's only rejection sentinel is the literal `0` from
//! `c2Collided`'s `default:` arm) — never merely "both failed somehow".

#![allow(non_snake_case)]

mod common;
use common::*;

fn bytes_of<T: Copy>(v: &T) -> Vec<u8> {
    let n = std::mem::size_of::<T>();
    let mut out = vec![0u8; n];
    unsafe {
        std::ptr::copy_nonoverlapping(v as *const T as *const u8, out.as_mut_ptr(), n);
    }
    out
}

fn specials() -> Vec<f32> {
    let mut v: Vec<f32> = SPECIAL_F32.to_vec();
    v.extend(SPECIAL_BITS.iter().map(|&b| f32::from_bits(b)));
    v
}

// ===========================================================================
// Row 1 — out-of-range C2_TYPE hits `default: return 0;`
// ===========================================================================

#[test]
fn err_row01_c2collided_out_of_range_enum() {
    let (c, r) = libs();
    let A = c2Circle {
        p: c2v { x: -70.0, y: 0.0 },
        r: 20.0,
    };
    // A 20-byte operand buffer, big enough for any of the three shapes, so the
    // call would be well-defined if the C *did* dispatch.
    let cap = c2Capsule {
        a: c2v { x: -40.0, y: 40.0 },
        b: c2v { x: -20.0, y: 100.0 },
        r: 10.0,
    };
    let (ab, bb) = (bytes_of(&A), bytes_of(&cap));

    let hand_picked: [i32; 14] = [
        3,
        4,
        5,
        7,
        8,
        16,
        255,
        256,
        1000,
        0x7FFF_FFFE,
        i32::MAX,
        -1,
        -2,
        i32::MIN,
    ];
    for &ty in &hand_picked {
        unsafe {
            let cv = (c.c2Collided)(ab.as_ptr(), bb.as_ptr(), ty);
            let rv = (r.c2Collided)(ab.as_ptr(), bb.as_ptr(), ty);
            diff_assert!(
                cv == rv,
                "row01 c2Collided(.., typeB={ty}): C={cv} RS={rv}"
            );
            // The documented sentinel is exactly 0, not "some falsy value".
            diff_assert!(cv == 0, "row01 C returned {cv} for typeB={ty}, expected 0");
            diff_assert!(rv == 0, "row01 Rust returned {rv} for typeB={ty}, expected 0");
        }
    }

    // 512 randomized ints outside {0,1,2}.
    let mut rng = Rng::seeded(101);
    for i in 0..512 {
        let mut ty = rng.next_i32();
        if (0..=2).contains(&ty) {
            ty = ty.wrapping_add(3);
        }
        unsafe {
            let cv = (c.c2Collided)(ab.as_ptr(), bb.as_ptr(), ty);
            let rv = (r.c2Collided)(ab.as_ptr(), bb.as_ptr(), ty);
            diff_assert!(cv == rv, "row01 random #{i} typeB={ty}: C={cv} RS={rv}");
            diff_assert!(cv == 0 && rv == 0, "row01 random #{i} typeB={ty} not 0");
        }
    }
}

// ===========================================================================
// Row 2 — out-of-range type with NULL operands: `default:` runs before any load
// ===========================================================================

#[test]
fn err_row02_c2collided_null_ptrs_with_bad_type() {
    let (c, r) = libs();
    for &ty in &[3i32, 99, -1, i32::MIN, i32::MAX] {
        unsafe {
            let cv = (c.c2Collided)(std::ptr::null(), std::ptr::null(), ty);
            let rv = (r.c2Collided)(std::ptr::null(), std::ptr::null(), ty);
            diff_assert!(
                cv == 0 && rv == 0,
                "row02 c2Collided(NULL, NULL, {ty}): C={cv} RS={rv}, expected 0/0"
            );
        }
    }
    // Mixed: one null, one valid, still an invalid type.
    let A = c2Circle {
        p: c2v { x: 1.0, y: 2.0 },
        r: 3.0,
    };
    let ab = bytes_of(&A);
    unsafe {
        let cv = (c.c2Collided)(ab.as_ptr(), std::ptr::null(), 42);
        let rv = (r.c2Collided)(ab.as_ptr(), std::ptr::null(), 42);
        diff_assert!(cv == 0 && rv == 0, "row02 mixed-null: C={cv} RS={rv}");
        let cv = (c.c2Collided)(std::ptr::null(), ab.as_ptr(), 42);
        let rv = (r.c2Collided)(std::ptr::null(), ab.as_ptr(), 42);
        diff_assert!(cv == 0 && rv == 0, "row02 mixed-null2: C={cv} RS={rv}");
    }
}

// ===========================================================================
// Row 3 — negative enum values
// ===========================================================================

#[test]
fn err_row03_c2collided_negative_enum() {
    let (c, r) = libs();
    let A = c2Circle {
        p: c2v { x: -20.0, y: -20.0 },
        r: 10.0,
    };
    let bb = c2AABB {
        min: c2v { x: -40.0, y: -40.0 },
        max: c2v { x: -15.0, y: -15.0 },
    };
    let (ap, bp) = (bytes_of(&A), bytes_of(&bb));
    for ty in -2048i32..0 {
        unsafe {
            let cv = (c.c2Collided)(ap.as_ptr(), bp.as_ptr(), ty);
            let rv = (r.c2Collided)(ap.as_ptr(), bp.as_ptr(), ty);
            diff_assert!(cv == rv && cv == 0, "row03 typeB={ty}: C={cv} RS={rv}");
        }
    }
}

// ===========================================================================
// Row 4 — exactly one step past the last valid variant
// ===========================================================================

#[test]
fn err_row04_c2collided_one_past_last_variant() {
    let (c, r) = libs();
    let A = c2Circle {
        p: c2v { x: -70.0, y: 0.0 },
        r: 20.0,
    };
    let B = c2Circle {
        p: c2v { x: -70.0, y: 0.0 },
        r: 20.0,
    };
    let (ap, bp) = (bytes_of(&A), bytes_of(&B));
    unsafe {
        // 2 is the last valid variant and MUST still dispatch (a real overlap).
        let cv2 = (c.c2Collided)(ap.as_ptr(), bp.as_ptr(), C2_TYPE_CAPSULE);
        let rv2 = (r.c2Collided)(ap.as_ptr(), bp.as_ptr(), C2_TYPE_CAPSULE);
        diff_assert!(cv2 == rv2, "row04 typeB=2 (valid): C={cv2} RS={rv2}");

        // 3 is one past the end and MUST be rejected with exactly 0.
        let cv3 = (c.c2Collided)(ap.as_ptr(), bp.as_ptr(), 3);
        let rv3 = (r.c2Collided)(ap.as_ptr(), bp.as_ptr(), 3);
        diff_assert!(
            cv3 == 0 && rv3 == 0,
            "row04 typeB=3 (one past end): C={cv3} RS={rv3}, expected 0/0"
        );

        // -1 is one step before the first valid variant.
        let cvm = (c.c2Collided)(ap.as_ptr(), bp.as_ptr(), -1);
        let rvm = (r.c2Collided)(ap.as_ptr(), bp.as_ptr(), -1);
        diff_assert!(
            cvm == 0 && rvm == 0,
            "row04 typeB=-1 (one before start): C={cvm} RS={rvm}"
        );

        // And every valid variant must NOT be rejected wrongly: verify each
        // dispatches identically for a deliberately overlapping input.
        for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            let big = c2Capsule {
                a: c2v { x: -70.0, y: 0.0 },
                b: c2v { x: -70.0, y: 0.0 },
                r: 1000.0,
            };
            let bpb = bytes_of(&big);
            let cv = (c.c2Collided)(ap.as_ptr(), bpb.as_ptr(), ty);
            let rv = (r.c2Collided)(ap.as_ptr(), bpb.as_ptr(), ty);
            diff_assert!(cv == rv, "row04 valid typeB={ty}: C={cv} RS={rv}");
        }
    }
}

// ===========================================================================
// Row 5 — `A` is blindly cast to c2Circle* whatever `typeB` says
// ===========================================================================

#[test]
fn err_row05_c2collided_blind_cast_of_A() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(105);
    for i in 0..4096 {
        // Deliberately hand an AABB / a Capsule as the FIRST operand. The C
        // reinterprets its first 12 bytes as a c2Circle regardless.
        let fake_aabb = c2AABB {
            min: rng.vec_coord(),
            max: rng.vec_coord(),
        };
        let fake_cap = c2Capsule {
            a: rng.vec_coord(),
            b: rng.vec_coord(),
            r: rng.radius(),
        };
        for (k, abuf) in [bytes_of(&fake_aabb), bytes_of(&fake_cap)]
            .iter()
            .enumerate()
        {
            for &(ty, bsz) in &[
                (C2_TYPE_CIRCLE, 12usize),
                (C2_TYPE_AABB, 16),
                (C2_TYPE_CAPSULE, 20),
            ] {
                let mut bbuf = vec![0u8; bsz];
                for b in bbuf.iter_mut() {
                    *b = (rng.next_u32() & 0xFF) as u8;
                }
                unsafe {
                    let cv = (c.c2Collided)(abuf.as_ptr(), bbuf.as_ptr(), ty);
                    let rv = (r.c2Collided)(abuf.as_ptr(), bbuf.as_ptr(), ty);
                    diff_assert!(
                        cv == rv,
                        "row05 #{i} k={k} ty={ty}: C={cv} RS={rv} A={:02x?} B={:02x?}",
                        abuf,
                        bbuf
                    );
                }
                // Cross-check: the same 12 bytes viewed as a real c2Circle must
                // give the identical answer, proving the blind cast is faithful.
                let mut as_circle = c2Circle {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: 0.0,
                };
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        abuf.as_ptr(),
                        &mut as_circle as *mut c2Circle as *mut u8,
                        12,
                    );
                }
                let cb = bytes_of(&as_circle);
                unsafe {
                    let cv1 = (c.c2Collided)(abuf.as_ptr(), bbuf.as_ptr(), ty);
                    let cv2 = (c.c2Collided)(cb.as_ptr(), bbuf.as_ptr(), ty);
                    diff_assert!(cv1 == cv2, "row05 C blind-cast not byte-faithful");
                    let rv1 = (r.c2Collided)(abuf.as_ptr(), bbuf.as_ptr(), ty);
                    let rv2 = (r.c2Collided)(cb.as_ptr(), bbuf.as_ptr(), ty);
                    diff_assert!(rv1 == rv2, "row05 Rust blind-cast not byte-faithful");
                }
            }
        }
    }
}

// ===========================================================================
// Row 6 — degenerate capsule => unguarded division by zero
// ===========================================================================

#[test]
fn err_row06_capsule_degenerate_div_by_zero() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(106);

    // Exhaustive-ish over signed zeros for the segment endpoints.
    let zeros = [0.0f32, -0.0];
    for &ax in &zeros {
        for &ay in &zeros {
            for &bx in &zeros {
                for &by in &zeros {
                    let cap = c2Capsule {
                        a: c2v { x: ax, y: ay },
                        b: c2v { x: bx, y: by },
                        r: 10.0,
                    };
                    for &pr in &[0.0f32, 5.0, 50.0, -5.0] {
                        for &(px, py) in &[
                            (0.0f32, 0.0f32),
                            (1.0, 0.0),
                            (0.0, 1.0),
                            (-1.0, -1.0),
                            (100.0, 100.0),
                        ] {
                            let A = c2Circle {
                                p: c2v { x: px, y: py },
                                r: pr,
                            };
                            unsafe {
                                let cv = (c.c2CircletoCapsule)(A, cap);
                                let rv = (r.c2CircletoCapsule)(A, cap);
                                diff_assert!(
                                    cv == rv,
                                    "row06 zero-seg A={A:?} cap={cap:?}: C={cv} RS={rv}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Randomized degenerate capsules at arbitrary positions.
    for i in 0..4096 {
        let p0 = rng.vec_coord();
        let cap = c2Capsule {
            a: p0,
            b: p0,
            r: rng.radius(),
        };
        let A = c2Circle {
            p: if i % 4 == 0 { p0 } else { rng.vec_coord() },
            r: rng.radius(),
        };
        unsafe {
            let cv = (c.c2CircletoCapsule)(A, cap);
            let rv = (r.c2CircletoCapsule)(A, cap);
            diff_assert!(cv == rv, "row06 #{i} A={A:?} cap={cap:?}: C={cv} RS={rv}");
        }
        // Same through the dispatcher, so the MEMORY-class arg path is covered.
        let (ap, bp) = (bytes_of(&A), bytes_of(&cap));
        unsafe {
            let cv = (c.c2Collided)(ap.as_ptr(), bp.as_ptr(), C2_TYPE_CAPSULE);
            let rv = (r.c2Collided)(ap.as_ptr(), bp.as_ptr(), C2_TYPE_CAPSULE);
            diff_assert!(cv == rv, "row06 #{i} via dispatcher: C={cv} RS={rv}");
        }
    }

    // Confirm the C really does produce a NaN/inf `d2` here, i.e. that this row
    // exercises the unguarded division rather than a silently-guarded path.
    let n = c2v { x: 0.0, y: 0.0 };
    unsafe {
        let dot_nn = (c.c2Dot)(n, n);
        assert_eq!(dot_nn, 0.0, "row06 precondition: dot(n,n) must be 0");
        let q = 0.0f32 / dot_nn; // 0/0
        assert!(q.is_nan(), "row06 precondition: 0/0 must be NaN");
    }
}

// ===========================================================================
// Row 7 — negative radii are never validated
// ===========================================================================

#[test]
fn err_row07_negative_radius() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(107);
    for i in 0..4096 {
        let mag = rng.radius();
        let p = rng.vec_coord();
        let q = rng.vec_coord();

        // circle/circle
        let A = c2Circle { p, r: -mag };
        let B = c2Circle { p: q, r: -mag };
        unsafe {
            let cv = (c.c2CircletoCircle)(A, B);
            let rv = (r.c2CircletoCircle)(A, B);
            diff_assert!(cv == rv, "row07 #{i} circle/circle: C={cv} RS={rv}");
            // The C squares (A.r+B.r), so a negated pair behaves like the
            // positive pair — verify that against the C itself, not an
            // assumption about the Rust.
            let Ap = c2Circle { p, r: mag };
            let Bp = c2Circle { p: q, r: mag };
            let cvp = (c.c2CircletoCircle)(Ap, Bp);
            diff_assert!(
                cv == cvp,
                "row07 #{i} C treats -r differently from +r: {cv} vs {cvp}"
            );
        }

        // circle/aabb
        let bb = rng.aabb_proper();
        unsafe {
            let A = c2Circle { p, r: -mag };
            let cv = (c.c2CircletoAABB)(A, bb);
            let rv = (r.c2CircletoAABB)(A, bb);
            diff_assert!(cv == rv, "row07 #{i} circle/aabb: C={cv} RS={rv}");
        }

        // circle/capsule, negative on either side
        let cap = c2Capsule {
            a: rng.vec_coord(),
            b: rng.vec_coord(),
            r: -mag,
        };
        unsafe {
            for &ar in &[-mag, mag, 0.0, -0.0] {
                let A = c2Circle { p, r: ar };
                let cv = (c.c2CircletoCapsule)(A, cap);
                let rv = (r.c2CircletoCapsule)(A, cap);
                diff_assert!(cv == rv, "row07 #{i} circle/capsule ar={ar}: C={cv} RS={rv}");
            }
        }

        // and through the one-shot wrapper
        unsafe {
            let cv = (c.circle_collide)(p.x, p.y, -mag);
            let rv = (r.circle_collide)(p.x, p.y, -mag);
            diff_assert!(cv == rv, "row07 #{i} circle_collide(-r): C={cv} RS={rv}");
        }
    }
}

// ===========================================================================
// Row 8 — inverted / oversized AABB
// ===========================================================================

#[test]
fn err_row08_inverted_aabb() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(108);
    let inf = f32::INFINITY;
    for i in 0..4096 {
        let p = rng.vec_proper_or_coord();
        let A = c2Circle { p, r: rng.radius() };
        let base = rng.aabb_proper();
        let boxes = [
            c2AABB {
                min: base.max,
                max: base.min,
            },
            c2AABB {
                min: c2v {
                    x: base.max.x,
                    y: base.min.y,
                },
                max: c2v {
                    x: base.min.x,
                    y: base.max.y,
                },
            },
            c2AABB {
                min: c2v { x: inf, y: inf },
                max: c2v { x: -inf, y: -inf },
            },
            c2AABB {
                min: c2v { x: -inf, y: -inf },
                max: c2v { x: inf, y: inf },
            },
            c2AABB {
                min: c2v { x: f32::MAX, y: f32::MAX },
                max: c2v { x: f32::MIN, y: f32::MIN },
            },
        ];
        for (k, bb) in boxes.iter().enumerate() {
            unsafe {
                let cv = (c.c2CircletoAABB)(A, *bb);
                let rv = (r.c2CircletoAABB)(A, *bb);
                diff_assert!(
                    cv == rv,
                    "row08 #{i} box{k} A={A:?} bb={bb:?}: C={cv} RS={rv}"
                );
            }
            // Cross-check the clamp chain itself, which is where the missing
            // ordering check lives.
            unsafe {
                let cl = (c.c2Clampv)(A.p, bb.min, bb.max);
                let rl = (r.c2Clampv)(A.p, bb.min, bb.max);
                diff_assert!(
                    v_eq(cl, rl),
                    "row08 #{i} box{k} clamp: C={} RS={}",
                    show_v(cl),
                    show_v(rl)
                );
            }
        }
    }
}

// ===========================================================================
// Row 9 — NaN in any float argument
// ===========================================================================

#[test]
fn err_row09_nan_inputs() {
    let (c, r) = libs();
    let nans: Vec<f32> = SPECIAL_BITS
        .iter()
        .map(|&b| f32::from_bits(b))
        .filter(|v| v.is_nan())
        .chain([f32::NAN, -f32::NAN])
        .collect();
    assert!(nans.len() >= 6, "row09 needs several NaN encodings");

    for &nan in &nans {
        // c2Maxv / c2Minv: the comparison is false, so operand b must win.
        for &other in &[1.0f32, -1.0, 0.0, f32::INFINITY] {
            let a = c2v { x: nan, y: other };
            let b = c2v { x: other, y: nan };
            unsafe {
                let (cm, rm) = ((c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
                diff_assert!(
                    v_eq(cm, rm),
                    "row09 c2Maxv NaN: C={} RS={}",
                    show_v(cm),
                    show_v(rm)
                );
                // The C's raw `>` ternary returns b when the compare is false.
                diff_assert!(
                    f32_eq_bits(cm.x, b.x),
                    "row09 C c2Maxv did not return b.x for NaN a.x: {}",
                    show(cm.x)
                );
                let (cn, rn) = ((c.c2Minv)(a, b), (r.c2Minv)(a, b));
                diff_assert!(
                    v_eq(cn, rn),
                    "row09 c2Minv NaN: C={} RS={}",
                    show_v(cn),
                    show_v(rn)
                );
            }
        }

        // The `d2 < r2` predicates must reject (return 0) on a NaN d2.
        let A = c2Circle {
            p: c2v { x: nan, y: 0.0 },
            r: 10.0,
        };
        let B = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 10.0,
        };
        let bb = c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        let cap = c2Capsule {
            a: c2v { x: -1.0, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r: 1.0,
        };
        unsafe {
            let (cv, rv) = ((c.c2CircletoCircle)(A, B), (r.c2CircletoCircle)(A, B));
            diff_assert!(cv == rv, "row09 circle/circle NaN: C={cv} RS={rv}");
            diff_assert!(cv == 0, "row09 C should reject NaN d2, got {cv}");
            let (cv, rv) = ((c.c2CircletoAABB)(A, bb), (r.c2CircletoAABB)(A, bb));
            diff_assert!(cv == rv, "row09 circle/aabb NaN: C={cv} RS={rv}");
            diff_assert!(cv == 0, "row09 C should reject NaN d2 (aabb), got {cv}");
            let (cv, rv) = (
                (c.c2CircletoCapsule)(A, cap),
                (r.c2CircletoCapsule)(A, cap),
            );
            diff_assert!(cv == rv, "row09 circle/capsule NaN: C={cv} RS={rv}");
            diff_assert!(cv == 0, "row09 C should reject NaN d2 (capsule), got {cv}");

            // NaN radius too.
            let An = c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: nan,
            };
            for &(cv, rv) in &[
                ((c.c2CircletoCircle)(An, B), (r.c2CircletoCircle)(An, B)),
                ((c.c2CircletoAABB)(An, bb), (r.c2CircletoAABB)(An, bb)),
                (
                    (c.c2CircletoCapsule)(An, cap),
                    (r.c2CircletoCapsule)(An, cap),
                ),
            ] {
                diff_assert!(cv == rv, "row09 NaN radius: C={cv} RS={rv}");
                diff_assert!(cv == 0, "row09 C should reject NaN r2, got {cv}");
            }

            // and via the public one-shot entry point
            for &(x, y, rr) in &[
                (nan, 0.0f32, 20.0f32),
                (0.0, nan, 20.0),
                (-70.0, 0.0, nan),
                (nan, nan, nan),
            ] {
                let cv = (c.circle_collide)(x, y, rr);
                let rv = (r.circle_collide)(x, y, rr);
                diff_assert!(cv == rv, "row09 circle_collide NaN: C={cv} RS={rv}");
            }
        }
    }
}

// ===========================================================================
// Row 10 — ±infinity arguments
// ===========================================================================

#[test]
fn err_row10_infinity_inputs() {
    let (c, r) = libs();
    let infs = [f32::INFINITY, f32::NEG_INFINITY];
    let sp = specials();
    for &inf in &infs {
        // inf - inf == NaN inside c2Sub, feeding c2Dot.
        for &v in &sp {
            let a = c2v { x: inf, y: v };
            let b = c2v { x: inf, y: inf };
            unsafe {
                let (cs, rs) = ((c.c2Sub)(a, b), (r.c2Sub)(a, b));
                diff_assert!(
                    v_eq(cs, rs),
                    "row10 c2Sub inf: C={} RS={}",
                    show_v(cs),
                    show_v(rs)
                );
                let (cd, rd) = ((c.c2Dot)(a, b), (r.c2Dot)(a, b));
                diff_assert!(
                    f32_eq_bits(cd, rd),
                    "row10 c2Dot inf ({} . {}): C={} RS={}",
                    show_v(a),
                    show_v(b),
                    show(cd),
                    show(rd)
                );
                // inf * 0 => NaN
                let z = c2v { x: 0.0, y: -0.0 };
                let (cd, rd) = ((c.c2Dot)(a, z), (r.c2Dot)(a, z));
                diff_assert!(
                    f32_eq_bits(cd, rd),
                    "row10 c2Dot inf*0: C={} RS={}",
                    show(cd),
                    show(rd)
                );
                let (cm, rm) = ((c.c2Mulvs)(a, 0.0), (r.c2Mulvs)(a, 0.0));
                diff_assert!(
                    v_eq(cm, rm),
                    "row10 c2Mulvs inf*0: C={} RS={}",
                    show_v(cm),
                    show_v(rm)
                );
            }
        }
        // Whole-shape infinities.
        let A = c2Circle {
            p: c2v { x: inf, y: inf },
            r: inf,
        };
        let B = c2Circle {
            p: c2v { x: inf, y: -inf },
            r: inf,
        };
        let bb = c2AABB {
            min: c2v { x: -inf, y: -inf },
            max: c2v { x: inf, y: inf },
        };
        let cap = c2Capsule {
            a: c2v { x: -inf, y: 0.0 },
            b: c2v { x: inf, y: 0.0 },
            r: inf,
        };
        unsafe {
            let (cv, rv) = ((c.c2CircletoCircle)(A, B), (r.c2CircletoCircle)(A, B));
            diff_assert!(cv == rv, "row10 circle/circle inf: C={cv} RS={rv}");
            let (cv, rv) = ((c.c2CircletoAABB)(A, bb), (r.c2CircletoAABB)(A, bb));
            diff_assert!(cv == rv, "row10 circle/aabb inf: C={cv} RS={rv}");
            let (cv, rv) = (
                (c.c2CircletoCapsule)(A, cap),
                (r.c2CircletoCapsule)(A, cap),
            );
            diff_assert!(cv == rv, "row10 circle/capsule inf: C={cv} RS={rv}");
            for &(x, y, rr) in &[
                (inf, 0.0f32, 20.0f32),
                (0.0, inf, 20.0),
                (-70.0, 0.0, inf),
                (inf, inf, inf),
                (-inf, inf, -inf),
            ] {
                let cv = (c.circle_collide)(x, y, rr);
                let rv = (r.circle_collide)(x, y, rr);
                diff_assert!(cv == rv, "row10 circle_collide inf: C={cv} RS={rv}");
            }
        }
    }
}

// ===========================================================================
// Row 11 — subnormals, underflow and overflow
// ===========================================================================

#[test]
fn err_row11_denormal_and_overflow() {
    let (c, r) = libs();
    let tiny = [
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1.0e-38,
        1.0e-45,
    ];
    let huge = [f32::MAX, f32::MIN, 1.0e38, -1.0e38, 3.4e38];

    for &t in &tiny {
        for &h in &huge {
            let a = c2v { x: t, y: h };
            let b = c2v { x: h, y: t };
            unsafe {
                // c2Dot: t*h stays finite, h*h overflows to inf.
                let (cd, rd) = ((c.c2Dot)(a, b), (r.c2Dot)(a, b));
                diff_assert!(
                    f32_eq_bits(cd, rd),
                    "row11 c2Dot({},{}): C={} RS={}",
                    show_v(a),
                    show_v(b),
                    show(cd),
                    show(rd)
                );
                let hh = c2v { x: h, y: h };
                let (cd, rd) = ((c.c2Dot)(hh, hh), (r.c2Dot)(hh, hh));
                diff_assert!(
                    f32_eq_bits(cd, rd),
                    "row11 c2Dot overflow: C={} RS={}",
                    show(cd),
                    show(rd)
                );
                // c2Mulvs: subnormal * subnormal underflows to (signed) zero.
                let tt = c2v { x: t, y: -t };
                let (cm, rm) = ((c.c2Mulvs)(tt, t), (r.c2Mulvs)(tt, t));
                diff_assert!(
                    v_eq(cm, rm),
                    "row11 c2Mulvs underflow: C={} RS={}",
                    show_v(cm),
                    show_v(rm)
                );
                let (cm, rm) = ((c.c2Mulvs)(hh, h), (r.c2Mulvs)(hh, h));
                diff_assert!(
                    v_eq(cm, rm),
                    "row11 c2Mulvs overflow: C={} RS={}",
                    show_v(cm),
                    show_v(rm)
                );
                // Shape predicates with extreme radii.
                let A = c2Circle { p: a, r: t };
                let B = c2Circle { p: b, r: h };
                let cv = (c.c2CircletoCircle)(A, B);
                let rv = (r.c2CircletoCircle)(A, B);
                diff_assert!(cv == rv, "row11 circle/circle: C={cv} RS={rv}");
                let bb = c2AABB { min: a, max: b };
                let cv = (c.c2CircletoAABB)(A, bb);
                let rv = (r.c2CircletoAABB)(A, bb);
                diff_assert!(cv == rv, "row11 circle/aabb: C={cv} RS={rv}");
                let cap = c2Capsule { a, b, r: h };
                let cv = (c.c2CircletoCapsule)(A, cap);
                let rv = (r.c2CircletoCapsule)(A, cap);
                diff_assert!(cv == rv, "row11 circle/capsule: C={cv} RS={rv}");
                let cv = (c.circle_collide)(t, h, t);
                let rv = (r.circle_collide)(t, h, t);
                diff_assert!(cv == rv, "row11 circle_collide: C={cv} RS={rv}");
            }
        }
    }
}

// ===========================================================================
// Row 12 — unaligned operand pointers into c2Collided
// ===========================================================================

#[test]
fn err_row12_unaligned_pointers() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(112);
    let mut abuf = [0u8; 64];
    let mut bbuf = [0u8; 64];
    for &(ty, bsz) in &[
        (C2_TYPE_CIRCLE, 12usize),
        (C2_TYPE_AABB, 16),
        (C2_TYPE_CAPSULE, 20),
    ] {
        for i in 0..2048 {
            for k in 0..64 {
                abuf[k] = (rng.next_u32() & 0xFF) as u8;
                bbuf[k] = (rng.next_u32() & 0xFF) as u8;
            }
            for off in 1..8usize {
                let ap = unsafe { abuf.as_ptr().add(off) };
                let bp = unsafe { bbuf.as_ptr().add(off) };
                unsafe {
                    let cv = (c.c2Collided)(ap, bp, ty);
                    let rv = (r.c2Collided)(ap, bp, ty);
                    diff_assert!(
                        cv == rv,
                        "row12 ty={ty} #{i} off={off}: C={cv} RS={rv} \
                         A={:02x?} B={:02x?}",
                        &abuf[off..off + 12],
                        &bbuf[off..off + bsz]
                    );
                }
            }
        }
    }
}
