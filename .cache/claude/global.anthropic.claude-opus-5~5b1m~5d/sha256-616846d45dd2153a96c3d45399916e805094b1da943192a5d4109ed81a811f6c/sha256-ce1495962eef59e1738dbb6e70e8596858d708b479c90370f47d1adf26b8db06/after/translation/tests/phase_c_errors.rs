//! Phase C — error-path differential tests, one per ERRORS.md row (1..=42).
//!
//! Each test constructs the exact invalid / degenerate input, calls BOTH `.so`s
//! and asserts they return the SAME rejection value (the same sentinel or the
//! same exact bit pattern) — never merely "both failed somehow".

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_int;
use std::ptr;

const SEED: u64 = 0x13198A2E_03707344;
const N: usize = 8_000;

// ===========================================================================
// c2Collided: the `default: return 0` rejection branch (lib.c:112-113)
// ===========================================================================

fn valid_operands() -> (C2Circle, C2Capsule) {
    (
        C2Circle { p: C2v { x: 1.0, y: 2.0 }, r: 3.0 },
        C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 10.0, y: 0.0 },
            r: 1.0,
        },
    )
}

/// Row 1 — `typeB == 3`, the first value one step past `C2_TYPE_CAPSULE`.
#[test]
fn err01_type_one_past_max() {
    let (a, b) = valid_operands();
    let got = diff(
        || "c2Collided(valid, valid, typeB=3)".to_string(),
        |api| unsafe { (api.c2Collided)((&raw const a).cast(), (&raw const b).cast(), 3) },
    );
    assert_eq!(got, 0, "the `default:` arm must return the 0 sentinel");
}

/// Row 2 — `typeB == -1`.
#[test]
fn err02_type_negative_one() {
    let (a, b) = valid_operands();
    let got = diff(
        || "c2Collided(valid, valid, typeB=-1)".to_string(),
        |api| unsafe { (api.c2Collided)((&raw const a).cast(), (&raw const b).cast(), -1) },
    );
    assert_eq!(got, 0);
}

/// Row 3 — `typeB == INT_MAX`.
#[test]
fn err03_type_int_max() {
    let (a, b) = valid_operands();
    let got = diff(
        || format!("c2Collided(valid, valid, typeB={})", c_int::MAX),
        |api| unsafe {
            (api.c2Collided)((&raw const a).cast(), (&raw const b).cast(), c_int::MAX)
        },
    );
    assert_eq!(got, 0);
}

/// Row 4 — `typeB == INT_MIN`.
#[test]
fn err04_type_int_min() {
    let (a, b) = valid_operands();
    let got = diff(
        || format!("c2Collided(valid, valid, typeB={})", c_int::MIN),
        |api| unsafe {
            (api.c2Collided)((&raw const a).cast(), (&raw const b).cast(), c_int::MIN)
        },
    );
    assert_eq!(got, 0);
}

/// Row 5 — 4096 random out-of-range `int`s. A C `enum` accepts any `int` across
/// the FFI boundary, so every value with no valid variant is a real input.
#[test]
fn err05_type_fuzz_out_of_range() {
    let (a, b) = valid_operands();
    let mut rng = Rng::new(SEED ^ 5);
    // exhaustive small neighbourhood around the valid range
    let mut tys: Vec<c_int> = (-64..=64).filter(|t| !(0..=2).contains(t)).collect();
    // plus the powers of two and their negations, and random 32-bit values
    for bit in 0..31 {
        tys.push(1i32 << bit);
        tys.push(-(1i32 << bit));
    }
    tys.retain(|t| !(0..=2).contains(t));
    while tys.len() < 4096 {
        let t = rng.next_u32() as c_int;
        if !(0..=2).contains(&t) {
            tys.push(t);
        }
    }
    for ty in tys {
        let got = diff(
            || format!("c2Collided(valid, valid, typeB={ty})"),
            |api| unsafe { (api.c2Collided)((&raw const a).cast(), (&raw const b).cast(), ty) },
        );
        assert_eq!(got, 0, "typeB={ty} must hit the `default:` arm");
    }
}

/// Row 6 — NULL pointers with an out-of-range `typeB`: the `default:` arm is
/// reached before any load, so neither implementation may dereference. (A NULL
/// with a *valid* `typeB` is a genuine segfault in the C and is therefore not a
/// testable behaviour — the C is the ground truth and it faults.)
#[test]
fn err06_null_pointers_invalid_type() {
    let (a, _b) = valid_operands();
    for ty in [3, -1, 99, c_int::MAX, c_int::MIN] {
        for (pa, pb, label) in [
            (ptr::null(), ptr::null(), "A=NULL,B=NULL"),
            ((&raw const a).cast(), ptr::null(), "A=valid,B=NULL"),
            (ptr::null(), (&raw const a).cast(), "A=NULL,B=valid"),
            // deliberately bogus, never-dereferenced addresses
            (1usize as *const _, 3usize as *const _, "A=0x1,B=0x3 (misaligned)"),
            (usize::MAX as *const _, usize::MAX as *const _, "A=B=~0"),
        ] {
            let got = diff(
                || format!("c2Collided({label}, typeB={ty})"),
                |api| unsafe { (api.c2Collided)(pa, pb, ty) },
            );
            assert_eq!(got, 0);
        }
    }
}

/// Row 7 — type confusion: `A` is reinterpreted as `c2Circle` for *every*
/// `typeB`, never validated. Feed `A` a buffer that is really an AABB / capsule
/// and check the result equals the kernel applied to its first 12 bytes.
#[test]
fn err07_type_confusion_A_always_circle() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..N {
        // `A` is *actually* a c2AABB (16 bytes) / c2Capsule (20 bytes)
        let a_as_aabb = C2Aabb { min: rng.v_finite(), max: rng.v_finite() };
        let a_as_cap = C2Capsule {
            a: rng.v_finite(),
            b: rng.v_finite(),
            r: rng.radius(),
        };
        let b_circle = C2Circle { p: rng.v_finite(), r: rng.radius() };
        let b_aabb = C2Aabb { min: rng.v_finite(), max: rng.v_finite() };
        let b_cap = C2Capsule {
            a: rng.v_finite(),
            b: rng.v_finite(),
            r: rng.radius(),
        };
        // what the C actually sees in `A`: the first 12 bytes as a c2Circle
        let reinterp_aabb: C2Circle = unsafe {
            (&raw const a_as_aabb).cast::<C2Circle>().read_unaligned()
        };
        let reinterp_cap: C2Circle = unsafe {
            (&raw const a_as_cap).cast::<C2Circle>().read_unaligned()
        };
        for (pa, reinterp, tag) in [
            ((&raw const a_as_aabb).cast::<std::ffi::c_void>(), reinterp_aabb, "A=AABB"),
            ((&raw const a_as_cap).cast::<std::ffi::c_void>(), reinterp_cap, "A=capsule"),
        ] {
            diff(
                || format!("type-confusion {tag} reinterp={reinterp:?}"),
                |api| {
                    let c0 = unsafe {
                        (api.c2Collided)(pa, (&raw const b_circle).cast(), C2_TYPE_CIRCLE)
                    };
                    let c1 = unsafe {
                        (api.c2Collided)(pa, (&raw const b_aabb).cast(), C2_TYPE_AABB)
                    };
                    let c2 = unsafe {
                        (api.c2Collided)(pa, (&raw const b_cap).cast(), C2_TYPE_CAPSULE)
                    };
                    assert_eq!(
                        c0,
                        (api.c2CircletoCircle)(reinterp, b_circle),
                        "[{}] A not reinterpreted as c2Circle (CIRCLE)",
                        api.name
                    );
                    assert_eq!(
                        c1,
                        (api.c2CircletoAABB)(reinterp, b_aabb),
                        "[{}] A not reinterpreted as c2Circle (AABB)",
                        api.name
                    );
                    assert_eq!(
                        c2,
                        (api.c2CircletoCapsule)(reinterp, b_cap),
                        "[{}] A not reinterpreted as c2Circle (CAPSULE)",
                        api.name
                    );
                    (c0, c1, c2)
                },
            );
        }
    }
}

/// Row 8 — `typeB == C2_TYPE_CAPSULE` reads exactly `sizeof(c2Capsule) == 20`
/// bytes from `B` and no more; `typeB == 1` reads exactly 16 and `typeB == 0`
/// exactly 12. Proven by varying only the *trailing* bytes: the result must not
/// change, in either implementation.
#[test]
fn err08_capsule_reads_exactly_20_bytes() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..N {
        let a = C2Circle { p: rng.v_finite(), r: rng.radius() };
        let payload: [u8; 20] = {
            let cap = C2Capsule {
                a: rng.v_finite(),
                b: rng.v_finite(),
                r: rng.radius(),
            };
            unsafe { std::mem::transmute::<C2Capsule, [u8; 20]>(cap) }
        };
        for (ty, len) in [(C2_TYPE_CIRCLE, 12usize), (C2_TYPE_AABB, 16), (C2_TYPE_CAPSULE, 20)] {
            let mut buf1 = [0u8; 40];
            let mut buf2 = [0u8; 40];
            buf1[..20].copy_from_slice(&payload);
            buf2[..20].copy_from_slice(&payload);
            // poison everything past `len` differently in the two buffers
            for i in len..40 {
                buf1[i] = 0x00;
                buf2[i] = 0xFF;
            }
            let r1 = diff(
                || format!("c2Collided(a, buf1, {ty}) len={len}"),
                |api| unsafe {
                    (api.c2Collided)((&raw const a).cast(), buf1.as_ptr().cast(), ty)
                },
            );
            let r2 = diff(
                || format!("c2Collided(a, buf2, {ty}) len={len}"),
                |api| unsafe {
                    (api.c2Collided)((&raw const a).cast(), buf2.as_ptr().cast(), ty)
                },
            );
            assert_eq!(r1, r2, "typeB={ty} read past {len} bytes of B");
        }
    }
}

// ===========================================================================
// c2CircletoCapsule degenerate / overflow paths
// ===========================================================================

/// Row 9 — degenerate capsule with `da == 0` ⇒ `0.0f / 0.0f` (lib.c:93).
#[test]
fn err09_capsule_degenerate_0_div_0() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..N {
        let m = rng.v_finite();
        // p == B.a == B.b ⇒ ap == (0,0), n == (0,0) ⇒ da == 0, db == 0
        // (db >= 0 so the *shaft* branch is not taken); force the shaft branch
        // by using p == m with a zero-length axis, which gives da == 0 and
        // db == 0 ⇒ after-B-cap branch. Both are exercised here.
        let cap = C2Capsule { a: m, b: m, r: rng.radius() };
        let a = C2Circle { p: m, r: rng.radius() };
        let got = diff(
            || format!("c2CircletoCapsule({a:?}, {cap:?}) [0/0]"),
            |api| (api.c2CircletoCapsule)(a, cap),
        );
        assert!(got == 0 || got == 1);
    }
    // Now really reach the `da >= 0 && db < 0` shaft branch with n == (0,0):
    // impossible (db == da == 0), so instead use a capsule whose axis has zero
    // dot with itself but non-zero components: n = (0, 0) is the only such
    // vector in f32 unless a component is a denormal that squares to 0.
    let tiny = f32::from_bits(1); // 1e-45; tiny*tiny == 0 in f32
    for &sx in &[tiny, -tiny] {
        for &sy in &[0.0f32, tiny, -tiny] {
            let cap = C2Capsule {
                a: C2v { x: 0.0, y: 0.0 },
                b: C2v { x: sx, y: sy },
                r: 1.0,
            };
            for px in [0.0f32, tiny, -tiny, 1.0, -1.0] {
                let a = C2Circle { p: C2v { x: px, y: 0.0 }, r: 1.0 };
                diff(
                    || format!("c2CircletoCapsule({a:?}, {cap:?}) [dot(n,n)==0]"),
                    |api| (api.c2CircletoCapsule)(a, cap),
                );
            }
        }
    }
}

/// Row 10 — degenerate capsule with `da > 0` ⇒ division by `+0.0` = `+inf`.
#[test]
fn err10_capsule_degenerate_div_by_zero() {
    // A capsule whose axis squares to zero (denormal components) but whose dot
    // with `ap` is non-zero is not reachable in f32; the reachable form is
    // `dot(n,n) == 0` with `da == 0`, plus the *near*-degenerate case where
    // `dot(n,n)` is a denormal and the quotient overflows to +/-inf.
    let mut rng = Rng::new(SEED ^ 10);
    let tinies = [
        f32::from_bits(1),
        f32::from_bits(0x0000_0002),
        f32::from_bits(0x0080_0000), // FLT_MIN
        f32::from_bits(0x0000_8000),
    ];
    for &t in &tinies {
        for _ in 0..(N / 4) {
            let base = C2v { x: rng.range(-4.0, 4.0), y: rng.range(-4.0, 4.0) };
            let cap = C2Capsule {
                a: base,
                b: C2v { x: base.x + t, y: base.y },
                r: rng.radius(),
            };
            let p = C2v {
                x: base.x + rng.range(0.0, 8.0),
                y: base.y + rng.range(-8.0, 8.0),
            };
            let a = C2Circle { p, r: rng.radius() };
            diff(
                || format!("c2CircletoCapsule({a:?}, {cap:?}) [near-degenerate axis]"),
                |api| (api.c2CircletoCapsule)(a, cap),
            );
        }
    }
    // exactly-zero axis with p strictly "ahead" so da == +0.0 (not -0.0)
    for _ in 0..N {
        let m = rng.v_finite();
        let cap = C2Capsule { a: m, b: m, r: rng.radius() };
        let a = C2Circle {
            p: C2v { x: m.x + rng.range(0.1, 50.0), y: m.y },
            r: rng.radius(),
        };
        diff(
            || format!("c2CircletoCapsule({a:?}, {cap:?}) [zero axis, da=+0]"),
            |api| (api.c2CircletoCapsule)(a, cap),
        );
    }
}

/// Row 11 — `A.r + B.r` overflows to `+inf`, so `r*r == inf` and everything
/// finite "collides". No overflow check in the C.
#[test]
fn err11_capsule_radius_overflow() {
    let mut rng = Rng::new(SEED ^ 11);
    let bigs = [f32::MAX, f32::MAX / 2.0, 1e38f32, 3e38f32];
    for &ra in &bigs {
        for &rb in &bigs {
            for _ in 0..(N / 16) {
                let cap = C2Capsule {
                    a: rng.v_finite(),
                    b: rng.v_finite(),
                    r: rb,
                };
                let a = C2Circle { p: rng.v_finite(), r: ra };
                diff(
                    || format!("c2CircletoCapsule({a:?}, {cap:?}) [radius overflow]"),
                    |api| (api.c2CircletoCapsule)(a, cap),
                );
            }
        }
    }
    // the canonical case: both FLT_MAX, small finite geometry -> C returns 1
    let cap = C2Capsule {
        a: C2v { x: 0.0, y: 0.0 },
        b: C2v { x: 1.0, y: 0.0 },
        r: f32::MAX,
    };
    let a = C2Circle { p: C2v { x: 1000.0, y: 1000.0 }, r: f32::MAX };
    let got = diff(
        || "c2CircletoCapsule(FLT_MAX radii)".to_string(),
        |api| (api.c2CircletoCapsule)(a, cap),
    );
    assert_eq!(got, 1, "inf radius must swallow everything finite");
}

/// Row 12 — negative radii (no non-negativity check); `r*r` is positive again.
#[test]
fn err12_capsule_negative_radii() {
    let mut rng = Rng::new(SEED ^ 12);
    let mut any_hit = false;
    for _ in 0..N {
        let r = rng.range(0.0, 40.0);
        let ax = rng.range(-20.0, 20.0);
        let ay = rng.range(-20.0, 20.0);
        let cap = C2Capsule {
            a: C2v { x: ax, y: ay },
            b: C2v { x: ax + rng.range(-40.0, 40.0), y: ay + rng.range(-40.0, 40.0) },
            r: -r,
        };
        let a = C2Circle { p: rng.v_finite(), r: -rng.range(0.0, 40.0) };
        let got = diff(
            || format!("c2CircletoCapsule({a:?}, {cap:?}) [negative radii]"),
            |api| (api.c2CircletoCapsule)(a, cap),
        );
        any_hit |= got != 0;
    }
    assert!(
        any_hit,
        "the C squares the negative radius sum, so hits must still be reachable"
    );
}

/// Row 13 — NaN in any field ⇒ every `comiss` unordered ⇒ `0`.
#[test]
fn err13_capsule_nan_inputs() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..N {
        let nan = if rng.next_u32() & 1 == 0 {
            qnan(rng.next_u32(), rng.next_u32() & 1 == 0)
        } else {
            snan(rng.next_u32(), rng.next_u32() & 1 == 0)
        };
        let base_a = C2Circle { p: rng.v_finite(), r: rng.radius() };
        let base_cap = C2Capsule {
            a: rng.v_finite(),
            b: rng.v_finite(),
            r: rng.radius(),
        };
        for field in 0..7 {
            let mut a = base_a;
            let mut cap = base_cap;
            match field {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.r = nan,
                3 => cap.a.x = nan,
                4 => cap.a.y = nan,
                5 => cap.b.x = nan,
                _ => cap.r = nan,
            }
            let got = diff(
                || format!("c2CircletoCapsule({a:?}, {cap:?}) [NaN in field {field}]"),
                |api| (api.c2CircletoCapsule)(a, cap),
            );
            // Fields 3/4 poison `B.a`, which only feeds `n` and `ap`. Both
            // `da` and `db` then become NaN, so the C takes the *after-B cap*
            // branch, where `d2 = c2Dot(A.p - B.b, A.p - B.b)` is fully finite
            // — a genuine `1` is possible. Do not "fix" that expectation: only
            // the fields that actually reach `d2` force a rejection.
            if field != 3 && field != 4 {
                assert_eq!(got, 0, "NaN must make the final `<` false (field {field})");
            }
        }
    }
}

/// Row 14 — ±inf coordinates ⇒ `inf - inf = NaN` inside `c2Sub` ⇒ `0`.
#[test]
fn err14_capsule_inf_coords() {
    let mut rng = Rng::new(SEED ^ 14);
    let infs = [f32::INFINITY, f32::NEG_INFINITY];
    for &i1 in &infs {
        for &i2 in &infs {
            for _ in 0..(N / 4) {
                let base_a = C2Circle { p: rng.v_finite(), r: rng.radius() };
                let base_cap = C2Capsule {
                    a: rng.v_finite(),
                    b: rng.v_finite(),
                    r: rng.radius(),
                };
                for field in 0..6 {
                    let mut a = base_a;
                    let mut cap = base_cap;
                    match field {
                        0 => { a.p.x = i1; cap.a.x = i2; }
                        1 => { a.p.y = i1; cap.a.y = i2; }
                        2 => { cap.a.x = i1; cap.b.x = i2; }
                        3 => { cap.a.y = i1; cap.b.y = i2; }
                        4 => { a.p.x = i1; cap.b.x = i2; }
                        _ => { a.p = C2v { x: i1, y: i2 }; }
                    }
                    diff(
                        || format!("c2CircletoCapsule({a:?}, {cap:?}) [inf coords {field}]"),
                        |api| (api.c2CircletoCapsule)(a, cap),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// c2CircletoCircle degenerate / overflow paths
// ===========================================================================

/// Row 15 — negative radii: `r2 = (A.r + B.r)²` is non-negative, so a
/// "negative-radius" circle still reports collisions.
#[test]
fn err15_circle_negative_radii() {
    let mut rng = Rng::new(SEED ^ 15);
    let mut any_hit = false;
    for _ in 0..N {
        let a = C2Circle {
            p: C2v { x: rng.range(-20.0, 20.0), y: rng.range(-20.0, 20.0) },
            r: -rng.range(0.0, 30.0),
        };
        let b = C2Circle {
            p: C2v { x: rng.range(-20.0, 20.0), y: rng.range(-20.0, 20.0) },
            r: -rng.range(0.0, 30.0),
        };
        let got = diff(
            || format!("c2CircletoCircle({a:?}, {b:?}) [negative radii]"),
            |api| (api.c2CircletoCircle)(a, b),
        );
        any_hit |= got != 0;
    }
    assert!(any_hit);
    // radii that cancel to exactly zero -> r2 == 0 -> `d2 < 0` is always false
    for _ in 0..N {
        let r = rng.range(0.0, 30.0);
        let a = C2Circle { p: rng.v_finite(), r };
        let b = C2Circle { p: rng.v_finite(), r: -r };
        let got = diff(
            || format!("c2CircletoCircle({a:?}, {b:?}) [radii cancel]"),
            |api| (api.c2CircletoCircle)(a, b),
        );
        assert_eq!(got, 0, "r2 == 0 means `d2 < 0` is never true");
    }
}

/// Row 16 — radius sum overflows to `+inf`.
#[test]
fn err16_circle_radius_overflow() {
    // NOTE: the centres must be close enough that `d2` stays FINITE — with
    // `1e30` apart, `d2` also overflows and `inf < inf` is false, which is what
    // the C does. Keep the distance small so `r2 == +inf` is the only infinity.
    let a = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: f32::MAX };
    let b = C2Circle { p: C2v { x: 1e18, y: -1e18 }, r: f32::MAX };
    let got = diff(
        || "c2CircletoCircle(FLT_MAX, FLT_MAX)".to_string(),
        |api| (api.c2CircletoCircle)(a, b),
    );
    assert_eq!(got, 1);
    // the symmetric check: d2 also overflowing makes `inf < inf` false
    let far = C2Circle { p: C2v { x: 1e30, y: -1e30 }, r: f32::MAX };
    let got = diff(
        || "c2CircletoCircle(FLT_MAX, FLT_MAX, d2 overflows too)".to_string(),
        |api| (api.c2CircletoCircle)(a, far),
    );
    assert_eq!(got, 0, "`inf < inf` is false");
    let mut rng = Rng::new(SEED ^ 16);
    let bigs = [f32::MAX, f32::MAX / 2.0, 2e38f32, 1e38f32, f32::INFINITY];
    for &ra in &bigs {
        for &rb in &bigs {
            for _ in 0..(N / 25) {
                let a = C2Circle { p: rng.v_finite(), r: ra };
                let b = C2Circle { p: rng.v_finite(), r: rb };
                diff(
                    || format!("c2CircletoCircle({a:?}, {b:?}) [radius overflow]"),
                    |api| (api.c2CircletoCircle)(a, b),
                );
            }
        }
    }
}

/// Row 17 — `A.r + B.r` = `+inf + -inf` ⇒ NaN ⇒ `0`.
#[test]
fn err17_circle_inf_minus_inf_radius() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..N {
        for (ra, rb) in [
            (f32::INFINITY, f32::NEG_INFINITY),
            (f32::NEG_INFINITY, f32::INFINITY),
        ] {
            let a = C2Circle { p: rng.v_finite(), r: ra };
            let b = C2Circle { p: rng.v_finite(), r: rb };
            let got = diff(
                || format!("c2CircletoCircle({a:?}, {b:?}) [inf-inf radius]"),
                |api| (api.c2CircletoCircle)(a, b),
            );
            assert_eq!(got, 0);
        }
    }
}

/// Row 18 — NaN coordinate or NaN radius ⇒ `0`.
#[test]
fn err18_circle_nan_inputs() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..N {
        let nan = if rng.next_u32() & 1 == 0 {
            qnan(rng.next_u32(), rng.next_u32() & 1 == 0)
        } else {
            snan(rng.next_u32(), rng.next_u32() & 1 == 0)
        };
        let ba = C2Circle { p: rng.v_finite(), r: rng.radius() };
        let bb = C2Circle { p: rng.v_finite(), r: rng.radius() };
        for field in 0..6 {
            let mut a = ba;
            let mut b = bb;
            match field {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.r = nan,
                3 => b.p.x = nan,
                4 => b.p.y = nan,
                _ => b.r = nan,
            }
            let got = diff(
                || format!("c2CircletoCircle({a:?}, {b:?}) [NaN field {field}]"),
                |api| (api.c2CircletoCircle)(a, b),
            );
            assert_eq!(got, 0, "NaN field {field}");
        }
    }
}

/// Row 19 — coordinate difference overflows ⇒ `d2 = +inf` ⇒ `inf < r2` false.
#[test]
fn err19_circle_distance_overflow() {
    let a = C2Circle { p: C2v { x: f32::MAX, y: 0.0 }, r: 1e30 };
    let b = C2Circle { p: C2v { x: -f32::MAX, y: 0.0 }, r: 1e30 };
    let got = diff(
        || "c2CircletoCircle(+FLT_MAX vs -FLT_MAX)".to_string(),
        |api| (api.c2CircletoCircle)(a, b),
    );
    assert_eq!(got, 0, "d2 == +inf must not be `< r2` for finite r2");
    let mut rng = Rng::new(SEED ^ 19);
    let big = [f32::MAX, -f32::MAX, 1e30f32, -1e30f32, 1e20f32, -1e20f32];
    for &x1 in &big {
        for &x2 in &big {
            for _ in 0..(N / 36) {
                let a = C2Circle { p: C2v { x: x1, y: rng.finite_f32() }, r: rng.radius() };
                let b = C2Circle { p: C2v { x: x2, y: rng.finite_f32() }, r: rng.radius() };
                diff(
                    || format!("c2CircletoCircle({a:?}, {b:?}) [distance overflow]"),
                    |api| (api.c2CircletoCircle)(a, b),
                );
            }
        }
    }
}

// ===========================================================================
// c2CircletoAABB degenerate paths
// ===========================================================================

/// Row 20 — inverted AABB (`min > max`), never validated.
#[test]
fn err20_aabb_inverted_box() {
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..N {
        let lo = C2v { x: rng.range(-40.0, 40.0), y: rng.range(-40.0, 40.0) };
        let hi = C2v { x: lo.x + rng.range(0.0, 40.0), y: lo.y + rng.range(0.0, 40.0) };
        let a = C2Circle { p: rng.v_finite(), r: rng.radius() };
        for b in [
            C2Aabb { min: hi, max: lo },
            C2Aabb { min: C2v { x: hi.x, y: lo.y }, max: C2v { x: lo.x, y: hi.y } },
            C2Aabb { min: C2v { x: lo.x, y: hi.y }, max: C2v { x: hi.x, y: lo.y } },
        ] {
            diff(
                || format!("c2CircletoAABB({a:?}, {b:?}) [inverted box]"),
                |api| (api.c2CircletoAABB)(a, b),
            );
        }
    }
    // With min > max, c2Clampv always returns `min` (max wins over min(a,max)),
    // so the nearest point is `min` regardless of `a` — verify the C agrees.
    let b = C2Aabb {
        min: C2v { x: 10.0, y: 10.0 },
        max: C2v { x: -10.0, y: -10.0 },
    };
    for &(x, y, r) in &[
        (10.0f32, 10.0f32, 1.0f32),
        (0.0, 0.0, 1.0),
        (0.0, 0.0, 20.0),
        (-10.0, -10.0, 1.0),
    ] {
        diff(
            || format!("c2CircletoAABB(({x},{y}),{r}, inverted)"),
            |api| (api.c2CircletoAABB)(C2Circle { p: C2v { x, y }, r }, b),
        );
    }
}

/// Row 21 — negative `A.r`: `r2 = A.r * A.r` ≥ 0, so no rejection.
#[test]
fn err21_aabb_negative_radius() {
    let mut rng = Rng::new(SEED ^ 21);
    let mut any_hit = false;
    for _ in 0..N {
        let x0 = rng.range(-40.0, 40.0);
        let y0 = rng.range(-40.0, 40.0);
        let b = C2Aabb {
            min: C2v { x: x0, y: y0 },
            max: C2v { x: x0 + rng.range(0.0, 40.0), y: y0 + rng.range(0.0, 40.0) },
        };
        let a = C2Circle {
            p: C2v { x: rng.range(-60.0, 60.0), y: rng.range(-60.0, 60.0) },
            r: -rng.range(0.0, 30.0),
        };
        let got = diff(
            || format!("c2CircletoAABB({a:?}, {b:?}) [negative radius]"),
            |api| (api.c2CircletoAABB)(a, b),
        );
        any_hit |= got != 0;
    }
    assert!(any_hit, "A.r*A.r is positive, so hits must still be reachable");
}

/// Row 22 — NaN in any field.
#[test]
fn err22_aabb_nan_inputs() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..N {
        let nan = if rng.next_u32() & 1 == 0 {
            qnan(rng.next_u32(), rng.next_u32() & 1 == 0)
        } else {
            snan(rng.next_u32(), rng.next_u32() & 1 == 0)
        };
        let ba = C2Circle { p: rng.v_finite(), r: rng.radius() };
        let bb = C2Aabb { min: rng.v_finite(), max: rng.v_finite() };
        for field in 0..7 {
            let mut a = ba;
            let mut b = bb;
            match field {
                0 => a.p.x = nan,
                1 => a.p.y = nan,
                2 => a.r = nan,
                3 => b.min.x = nan,
                4 => b.min.y = nan,
                5 => b.max.x = nan,
                _ => b.max.y = nan,
            }
            let got = diff(
                || format!("c2CircletoAABB({a:?}, {b:?}) [NaN field {field}]"),
                |api| (api.c2CircletoAABB)(a, b),
            );
            if field <= 2 {
                assert_eq!(got, 0, "NaN in the circle must always reject (field {field})");
            }
        }
    }
}

/// Row 23 — `A.r` = `+inf` ⇒ `r2 = inf` ⇒ `1`; `A.r` = NaN ⇒ `0`.
#[test]
fn err23_aabb_inf_radius() {
    let b = C2Aabb {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    // `p` must stay close enough that `d2` remains FINITE: at 1e30 the squared
    // distance also overflows to `+inf` and `inf < inf` is false in the C.
    let far = C2v { x: 1e18, y: -1e18 };
    let got_inf = diff(
        || "c2CircletoAABB(r=+inf)".to_string(),
        |api| (api.c2CircletoAABB)(C2Circle { p: far, r: f32::INFINITY }, b),
    );
    assert_eq!(got_inf, 1);
    let got_ninf = diff(
        || "c2CircletoAABB(r=-inf)".to_string(),
        |api| (api.c2CircletoAABB)(C2Circle { p: far, r: f32::NEG_INFINITY }, b),
    );
    assert_eq!(got_ninf, 1, "(-inf)*(-inf) == +inf");
    let got_nan = diff(
        || "c2CircletoAABB(r=NaN)".to_string(),
        |api| (api.c2CircletoAABB)(C2Circle { p: far, r: qnan(5, false) }, b),
    );
    assert_eq!(got_nan, 0);
    // `d2` overflowing as well: `inf < inf` is false
    let farther = C2v { x: 1e30, y: -1e30 };
    let got = diff(
        || "c2CircletoAABB(r=+inf, d2 overflows too)".to_string(),
        |api| (api.c2CircletoAABB)(C2Circle { p: farther, r: f32::INFINITY }, b),
    );
    assert_eq!(got, 0, "`inf < inf` is false");
    // and inf radius with inf coordinates -> d2 becomes NaN -> 0
    let got = diff(
        || "c2CircletoAABB(r=+inf, p=inf)".to_string(),
        |api| {
            (api.c2CircletoAABB)(
                C2Circle { p: C2v { x: f32::INFINITY, y: 0.0 }, r: f32::INFINITY },
                C2Aabb {
                    min: C2v { x: f32::INFINITY, y: -1.0 },
                    max: C2v { x: f32::INFINITY, y: 1.0 },
                },
            )
        },
    );
    assert_eq!(got, 0);
}

/// Row 24 — box with `±inf` bounds ⇒ `c2Sub` yields `inf - inf = NaN`.
#[test]
fn err24_aabb_inf_bounds() {
    let mut rng = Rng::new(SEED ^ 24);
    let infs = [f32::INFINITY, f32::NEG_INFINITY];
    for &i1 in &infs {
        for &i2 in &infs {
            for _ in 0..(N / 4) {
                let a = C2Circle { p: rng.v_finite(), r: rng.radius() };
                let boxes = [
                    C2Aabb { min: C2v { x: i1, y: i1 }, max: C2v { x: i2, y: i2 } },
                    C2Aabb { min: C2v { x: i1, y: -1.0 }, max: C2v { x: 1.0, y: i2 } },
                    C2Aabb { min: C2v { x: -1.0, y: i1 }, max: C2v { x: i2, y: 1.0 } },
                ];
                for b in boxes {
                    diff(
                        || format!("c2CircletoAABB({a:?}, {b:?}) [inf bounds]"),
                        |api| (api.c2CircletoAABB)(a, b),
                    );
                }
                // circle centre also at infinity -> inf - inf
                let a2 = C2Circle { p: C2v { x: i1, y: i2 }, r: rng.radius() };
                for b in boxes {
                    let got = diff(
                        || format!("c2CircletoAABB({a2:?}, {b:?}) [inf p and bounds]"),
                        |api| (api.c2CircletoAABB)(a2, b),
                    );
                    let _ = got;
                }
            }
        }
    }
}

// ===========================================================================
// Ternary min/max: NaN and signed zero
// ===========================================================================

/// Row 25 — `c2Maxv` with a NaN operand returns **`b`**, bit-exact (including
/// an un-quieted SNaN payload — the C copies with `movss`, it does not compute).
#[test]
fn err25_maxv_nan_returns_b() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..N {
        let na = if rng.next_u32() & 1 == 0 {
            qnan(rng.next_u32(), rng.next_u32() & 1 == 0)
        } else {
            snan(rng.next_u32(), rng.next_u32() & 1 == 0)
        };
        let nb = snan(rng.next_u32(), rng.next_u32() & 1 == 0);
        let f = rng.finite_f32();
        let cases = [
            (C2v { x: na, y: f }, C2v { x: f, y: f }),   // NaN in a only
            (C2v { x: f, y: f }, C2v { x: na, y: f }),   // NaN in b only
            (C2v { x: na, y: nb }, C2v { x: nb, y: na }), // NaN in both
        ];
        for (a, b) in cases {
            let got = diff(
                || format!("c2Maxv({a:?}, {b:?}) [NaN]"),
                |api| vbits((api.c2Maxv)(a, b)),
            );
            if a.x.is_nan() || b.x.is_nan() {
                assert_eq!(
                    got.0,
                    b.x.to_bits(),
                    "unordered `a.x > b.x` must select b.x verbatim"
                );
            }
        }
    }
}

/// Row 26 — `c2Minv` with a NaN operand returns **`b`**, bit-exact.
#[test]
fn err26_minv_nan_returns_b() {
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..N {
        let na = if rng.next_u32() & 1 == 0 {
            qnan(rng.next_u32(), rng.next_u32() & 1 == 0)
        } else {
            snan(rng.next_u32(), rng.next_u32() & 1 == 0)
        };
        let nb = snan(rng.next_u32(), rng.next_u32() & 1 == 0);
        let f = rng.finite_f32();
        let cases = [
            (C2v { x: na, y: f }, C2v { x: f, y: f }),
            (C2v { x: f, y: f }, C2v { x: na, y: f }),
            (C2v { x: na, y: nb }, C2v { x: nb, y: na }),
        ];
        for (a, b) in cases {
            let got = diff(
                || format!("c2Minv({a:?}, {b:?}) [NaN]"),
                |api| vbits((api.c2Minv)(a, b)),
            );
            if a.x.is_nan() || b.x.is_nan() {
                assert_eq!(got.0, b.x.to_bits());
            }
        }
    }
}

/// Row 27 — signed-zero pairs: neither `>` nor `<` holds, so `b` is selected
/// and the sign bit of the zero survives (no canonicalisation).
#[test]
fn err27_minmax_signed_zero() {
    for (ax, bx) in [
        (0.0f32, -0.0f32),
        (-0.0f32, 0.0f32),
        (0.0f32, 0.0f32),
        (-0.0f32, -0.0f32),
    ] {
        for (ay, by) in [(0.0f32, -0.0f32), (-0.0f32, 0.0f32)] {
            let a = C2v { x: ax, y: ay };
            let b = C2v { x: bx, y: by };
            let mx = diff(
                || format!("c2Maxv({a:?}, {b:?}) [signed zero]"),
                |api| vbits((api.c2Maxv)(a, b)),
            );
            assert_eq!(mx, (bx.to_bits(), by.to_bits()), "max of ±0 must be `b`");
            let mn = diff(
                || format!("c2Minv({a:?}, {b:?}) [signed zero]"),
                |api| vbits((api.c2Minv)(a, b)),
            );
            assert_eq!(mn, (bx.to_bits(), by.to_bits()), "min of ±0 must be `b`");
        }
    }
    // and through c2Clampv
    for &z in &[0.0f32, -0.0f32] {
        for &w in &[0.0f32, -0.0f32] {
            for &v in &[0.0f32, -0.0f32] {
                let a = C2v { x: z, y: w };
                let lo = C2v { x: w, y: v };
                let hi = C2v { x: v, y: z };
                diff(
                    || format!("c2Clampv({a:?}, {lo:?}, {hi:?}) [signed zero]"),
                    |api| vbits((api.c2Clampv)(a, lo, hi)),
                );
            }
        }
    }
}

/// Row 28 — `c2Clampv` with `lo > hi` (invalid range, never checked).
#[test]
fn err28_clampv_lo_greater_than_hi() {
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..N {
        let hi = C2v { x: rng.range(-40.0, 0.0), y: rng.range(-40.0, 0.0) };
        let lo = C2v { x: rng.range(0.0, 40.0), y: rng.range(0.0, 40.0) };
        let a = C2v { x: rng.range(-80.0, 80.0), y: rng.range(-80.0, 80.0) };
        let got = diff(
            || format!("c2Clampv({a:?}, {lo:?}, {hi:?}) [lo>hi]"),
            |api| vbits((api.c2Clampv)(a, lo, hi)),
        );
        // c2Maxv(lo, c2Minv(a, hi)) with lo > hi >= min(a,hi) always yields lo
        assert_eq!(got, (lo.x.to_bits(), lo.y.to_bits()));
    }
}

/// Row 29 — `c2Clampv` with NaN in `a`, `lo`, or `hi`.
#[test]
fn err29_clampv_nan() {
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..N {
        let nan = if rng.next_u32() & 1 == 0 {
            qnan(rng.next_u32(), rng.next_u32() & 1 == 0)
        } else {
            snan(rng.next_u32(), rng.next_u32() & 1 == 0)
        };
        let ba = rng.v_finite();
        let blo = rng.v_finite();
        let bhi = rng.v_finite();
        for field in 0..6 {
            let mut a = ba;
            let mut lo = blo;
            let mut hi = bhi;
            match field {
                0 => a.x = nan,
                1 => a.y = nan,
                2 => lo.x = nan,
                3 => lo.y = nan,
                4 => hi.x = nan,
                _ => hi.y = nan,
            }
            diff(
                || format!("c2Clampv({a:?}, {lo:?}, {hi:?}) [NaN field {field}]"),
                |api| vbits((api.c2Clampv)(a, lo, hi)),
            );
        }
    }
}

// ===========================================================================
// Arithmetic NaN / invalid-operation surface
// ===========================================================================

/// Row 30 — `c2Dot` with both products NaN and *different* payloads: SSE
/// returns the destination operand, so this pins GCC's exact `mulss`/`addss`
/// operand order (`px = mulss(a.x, b.x)`, `py = mulss(b.y, a.y)`,
/// `res = addss(py, px)`).
#[test]
fn err30_dot_nan_operand_order() {
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..N {
        let n1 = qnan(rng.next_u32(), rng.next_u32() & 1 == 0);
        let n2 = qnan(rng.next_u32(), rng.next_u32() & 1 == 0);
        let n3 = qnan(rng.next_u32(), rng.next_u32() & 1 == 0);
        let n4 = qnan(rng.next_u32(), rng.next_u32() & 1 == 0);
        let f = rng.finite_f32();
        let cases = [
            (C2v { x: n1, y: n3 }, C2v { x: n2, y: n4 }),
            (C2v { x: n1, y: f }, C2v { x: n2, y: f }),
            (C2v { x: f, y: n3 }, C2v { x: f, y: n4 }),
            (C2v { x: n1, y: n3 }, C2v { x: f, y: f }),
            (C2v { x: f, y: f }, C2v { x: n2, y: n4 }),
        ];
        for (a, b) in cases {
            diff(
                || format!("c2Dot({a:?}, {b:?}) [NaN operand order]"),
                |api| (api.c2Dot)(a, b).to_bits(),
            );
        }
    }
    // regression pin: the exact values that exposed the original divergence
    let na = f32::from_bits(0x7FC0_0001);
    let nb = f32::from_bits(0xFFC0_0002);
    let nc = f32::from_bits(0x7FC0_0003);
    let nd = f32::from_bits(0xFFC0_0004);
    let pins: &[((f32, f32), (f32, f32), u32)] = &[
        ((na, 1.0), (nb, 1.0), 0x7FC0_0001),
        ((1.0, na), (1.0, nb), 0xFFC0_0002),
        ((na, nc), (nb, nd), 0xFFC0_0004),
        ((na, nc), (1.0, 1.0), 0x7FC0_0003),
    ];
    for &((ax, ay), (bx, by), want) in pins {
        let a = C2v { x: ax, y: ay };
        let b = C2v { x: bx, y: by };
        let got = diff(
            || format!("c2Dot({a:?}, {b:?}) [pin]"),
            |api| (api.c2Dot)(a, b).to_bits(),
        );
        assert_eq!(got, want, "C reference value changed?!");
    }
}

/// Row 31 — invalid operations inside `c2Dot`: `0 * inf` and `inf + -inf`.
#[test]
fn err31_dot_invalid_operations() {
    let zs = [0.0f32, -0.0f32];
    let is = [f32::INFINITY, f32::NEG_INFINITY];
    for &z in &zs {
        for &i in &is {
            for &z2 in &zs {
                for &i2 in &is {
                    let cases = [
                        (C2v { x: z, y: 1.0 }, C2v { x: i, y: 1.0 }),
                        (C2v { x: i, y: 1.0 }, C2v { x: z, y: 1.0 }),
                        (C2v { x: 1.0, y: z2 }, C2v { x: 1.0, y: i2 }),
                        (C2v { x: 1.0, y: i2 }, C2v { x: 1.0, y: z2 }),
                        (C2v { x: i, y: i2 }, C2v { x: 1.0, y: 1.0 }),
                        (C2v { x: 1.0, y: 1.0 }, C2v { x: i, y: i2 }),
                        (C2v { x: z, y: i2 }, C2v { x: i, y: z2 }),
                    ];
                    for (a, b) in cases {
                        diff(
                            || format!("c2Dot({a:?}, {b:?}) [invalid op]"),
                            |api| (api.c2Dot)(a, b).to_bits(),
                        );
                    }
                }
            }
        }
    }
}

/// Row 32 — SNaN input: `mulss` quiets it (sets bit 22) but keeps the payload.
#[test]
fn err32_dot_snan_quieting() {
    let mut rng = Rng::new(SEED ^ 32);
    for _ in 0..N {
        let s1 = snan(rng.next_u32(), rng.next_u32() & 1 == 0);
        let s2 = snan(rng.next_u32(), rng.next_u32() & 1 == 0);
        let f = rng.finite_f32();
        let cases = [
            (C2v { x: s1, y: f }, C2v { x: f, y: f }),
            (C2v { x: f, y: f }, C2v { x: s1, y: f }),
            (C2v { x: f, y: s1 }, C2v { x: f, y: f }),
            (C2v { x: f, y: f }, C2v { x: f, y: s1 }),
            (C2v { x: s1, y: s2 }, C2v { x: s2, y: s1 }),
        ];
        for (a, b) in cases {
            let got = diff(
                || format!("c2Dot({a:?}, {b:?}) [SNaN]"),
                |api| (api.c2Dot)(a, b).to_bits(),
            );
            assert!(
                f32::from_bits(got).is_nan(),
                "SNaN input must yield some NaN, got {got:#010x}"
            );
        }
    }
}

/// Row 33 — `c2Mulvs` with a NaN scalar *and* NaN components: pins which
/// `mulss` operand is the destination (the vector component, not the scalar).
#[test]
fn err33_mulvs_nan_operand_order() {
    let mut rng = Rng::new(SEED ^ 33);
    for _ in 0..N {
        let n1 = qnan(rng.next_u32(), rng.next_u32() & 1 == 0);
        let n2 = qnan(rng.next_u32(), rng.next_u32() & 1 == 0);
        let n3 = qnan(rng.next_u32(), rng.next_u32() & 1 == 0);
        let f = rng.finite_f32();
        for (a, b) in [
            (C2v { x: n1, y: n3 }, n2),
            (C2v { x: n1, y: f }, n2),
            (C2v { x: f, y: n3 }, n2),
            (C2v { x: n1, y: n3 }, f),
        ] {
            diff(
                || format!("c2Mulvs({a:?}, {:#010x}) [NaN order]", b.to_bits()),
                |api| vbits((api.c2Mulvs)(a, b)),
            );
        }
    }
    // regression pin
    let na = f32::from_bits(0x7FC0_0001);
    let nb = f32::from_bits(0xFFC0_0002);
    let nc = f32::from_bits(0x7FC0_0003);
    let got = diff(
        || "c2Mulvs((n1,n3), n2) [pin]".to_string(),
        |api| vbits((api.c2Mulvs)(C2v { x: na, y: nc }, nb)),
    );
    assert_eq!(
        got,
        (0x7FC0_0001, 0x7FC0_0003),
        "the vector component must be the mulss destination"
    );
}

/// Row 34 — `c2Mulvs` with `0 * inf` in either order.
#[test]
fn err34_mulvs_zero_times_inf() {
    for &z in &[0.0f32, -0.0f32] {
        for &i in &[f32::INFINITY, f32::NEG_INFINITY] {
            for (a, b) in [
                (C2v { x: z, y: z }, i),
                (C2v { x: i, y: i }, z),
                (C2v { x: z, y: i }, i),
                (C2v { x: i, y: z }, z),
            ] {
                diff(
                    || format!("c2Mulvs({a:?}, {:#010x}) [0*inf]", b.to_bits()),
                    |api| vbits((api.c2Mulvs)(a, b)),
                );
            }
        }
    }
}

/// Row 35 — `c2Sub`: `inf - inf` ⇒ NaN; `-0.0 - 0.0` ⇒ `-0.0`;
/// `0.0 - 0.0` ⇒ `+0.0`.
#[test]
fn err35_sub_inf_and_signed_zero() {
    let vals = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        qnan(1, false),
        snan(2, true),
    ];
    for &ax in &vals {
        for &bx in &vals {
            for &ay in &vals {
                for &by in &vals {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    diff(
                        || format!("c2Sub({a:?}, {b:?}) [inf/signed zero]"),
                        |api| vbits((api.c2Sub)(a, b)),
                    );
                }
            }
        }
    }
    let got = diff(
        || "c2Sub((-0,0),(0,0))".to_string(),
        |api| {
            vbits((api.c2Sub)(
                C2v { x: -0.0, y: 0.0 },
                C2v { x: 0.0, y: 0.0 },
            ))
        },
    );
    assert_eq!(got, (0x8000_0000, 0x0000_0000), "-0.0 - 0.0 == -0.0");
}

/// Row 36 — subtraction overflow ⇒ `±inf`.
#[test]
fn err36_sub_overflow() {
    let vals = [f32::MAX, f32::MIN, f32::MAX / 2.0, -f32::MAX / 2.0];
    for &ax in &vals {
        for &bx in &vals {
            let a = C2v { x: ax, y: bx };
            let b = C2v { x: bx, y: ax };
            let got = diff(
                || format!("c2Sub({a:?}, {b:?}) [overflow]"),
                |api| vbits((api.c2Sub)(a, b)),
            );
            let _ = got;
        }
    }
    let got = diff(
        || "c2Sub((FLT_MAX,-FLT_MAX),(-FLT_MAX,FLT_MAX))".to_string(),
        |api| {
            vbits((api.c2Sub)(
                C2v { x: f32::MAX, y: -f32::MAX },
                C2v { x: -f32::MAX, y: f32::MAX },
            ))
        },
    );
    assert_eq!(
        got,
        (f32::INFINITY.to_bits(), f32::NEG_INFINITY.to_bits()),
        "FLT_MAX - -FLT_MAX must overflow to +inf"
    );
}

/// Row 37 — `c2V` passes every pathological bit pattern through unchanged.
#[test]
fn err37_c2v_bit_passthrough() {
    let mut rng = Rng::new(SEED ^ 37);
    for _ in 0..(N * 4) {
        let x = rng.any_f32();
        let y = rng.any_f32();
        let got = diff(
            || format!("c2V({:#010x}, {:#010x})", x.to_bits(), y.to_bits()),
            |api| vbits((api.c2V)(x, y)),
        );
        assert_eq!(
            got,
            (x.to_bits(), y.to_bits()),
            "c2V must not canonicalise anything"
        );
    }
    for &x in special_values() {
        for &y in special_values() {
            let got = diff(
                || format!("c2V({:#010x}, {:#010x})", x.to_bits(), y.to_bits()),
                |api| vbits((api.c2V)(x, y)),
            );
            assert_eq!(got, (x.to_bits(), y.to_bits()));
        }
    }
}

// ===========================================================================
// circle_collide
// ===========================================================================

/// Row 38 — NaN `x`, `y`, or `r` ⇒ all three sub-tests reject ⇒ `0`.
#[test]
fn err38_circle_collide_nan() {
    let mut rng = Rng::new(SEED ^ 38);
    for _ in 0..N {
        let nan = if rng.next_u32() & 1 == 0 {
            qnan(rng.next_u32(), rng.next_u32() & 1 == 0)
        } else {
            snan(rng.next_u32(), rng.next_u32() & 1 == 0)
        };
        let fx = rng.range(-100.0, 100.0);
        let fy = rng.range(-100.0, 100.0);
        let fr = rng.range(0.0, 60.0);
        for (x, y, r) in [
            (nan, fy, fr),
            (fx, nan, fr),
            (fx, fy, nan),
            (nan, nan, fr),
            (nan, nan, nan),
        ] {
            let got = diff(
                || {
                    format!(
                        "circle_collide({:#010x}, {:#010x}, {:#010x})",
                        x.to_bits(),
                        y.to_bits(),
                        r.to_bits()
                    )
                },
                |api| (api.circle_collide)(x, y, r),
            );
            assert_eq!(got, 0, "NaN input must reject all three shapes");
        }
    }
}

/// Row 39 — ±inf `x`, `y`, or `r`.
#[test]
fn err39_circle_collide_inf() {
    let vals = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0f32,
        -0.0,
        -70.0,
        -27.0,
        20.0,
    ];
    for &x in &vals {
        for &y in &vals {
            for &r in &vals {
                diff(
                    || format!("circle_collide({x}, {y}, {r})"),
                    |api| (api.circle_collide)(x, y, r),
                );
            }
        }
    }
    // +inf radius with finite centre: every shape is "hit" -> 7
    let got = diff(
        || "circle_collide(0, 0, +inf)".to_string(),
        |api| (api.circle_collide)(0.0, 0.0, f32::INFINITY),
    );
    assert_eq!(got, 7);
    // -inf radius: squares/sums to +inf for circle & aabb; the capsule uses
    // A.r + B.r = -inf + 10 = -inf, whose square is +inf too -> 7
    let got = diff(
        || "circle_collide(0, 0, -inf)".to_string(),
        |api| (api.circle_collide)(0.0, 0.0, f32::NEG_INFINITY),
    );
    assert_eq!(got, 7);
    // infinite centre: inf - inf = NaN somewhere -> 0
    let got = diff(
        || "circle_collide(+inf, 0, 1)".to_string(),
        |api| (api.circle_collide)(f32::INFINITY, 0.0, 1.0),
    );
    assert_eq!(got, 0);
}

/// Row 40 — negative `r` (no non-negativity check).
#[test]
fn err40_circle_collide_negative_r() {
    let mut rng = Rng::new(SEED ^ 40);
    let mut any_nonzero = false;
    for _ in 0..(N * 4) {
        let x = rng.range(-150.0, 150.0);
        let y = rng.range(-150.0, 150.0);
        let r = -rng.range(0.0, 60.0);
        let got = diff(
            || format!("circle_collide({x}, {y}, {r})"),
            |api| (api.circle_collide)(x, y, r),
        );
        any_nonzero |= got != 0;
    }
    assert!(
        any_nonzero,
        "negative radii are squared, so collisions must still be reported"
    );
    for r in [-0.0f32, -1.0, -20.0, -1e30, -f32::MAX] {
        for &(x, y) in &[(-70.0f32, 0.0f32), (-27.0, -27.0), (-30.0, 60.0), (0.0, 0.0)] {
            diff(
                || format!("circle_collide({x}, {y}, {r})"),
                |api| (api.circle_collide)(x, y, r),
            );
        }
    }
}

/// Row 41 — `r` large enough that `r*r` / `(r+R)²` overflow to `+inf` ⇒ `7`.
#[test]
fn err41_circle_collide_radius_overflow() {
    for r in [f32::MAX, f32::MAX / 2.0, 1e38f32, 2e19f32, 1e20f32] {
        let got = diff(
            || format!("circle_collide(0, 0, {r})"),
            |api| (api.circle_collide)(0.0, 0.0, r),
        );
        assert_eq!(got, 7, "r={r} should swallow all three shapes");
    }
    // and with the centre far away but still finite
    let mut rng = Rng::new(SEED ^ 41);
    for _ in 0..N {
        let x = rng.range(-1e20, 1e20);
        let y = rng.range(-1e20, 1e20);
        for r in [f32::MAX, 1e38f32, 1e20f32, f32::MIN] {
            diff(
                || format!("circle_collide({x}, {y}, {r})"),
                |api| (api.circle_collide)(x, y, r),
            );
        }
    }
}

/// Row 42 — denormal / subnormal inputs everywhere (neither `.so` may run with
/// flush-to-zero; the default MXCSR is shared by both).
#[test]
fn err42_denormals() {
    let mut rng = Rng::new(SEED ^ 42);
    let denorms: Vec<f32> = (0..24)
        .map(|i| f32::from_bits(1u32 << i))
        .filter(|f| f.is_subnormal() || *f == 0.0)
        .chain([f32::from_bits(0x007F_FFFF), f32::from_bits(0x807F_FFFF)])
        .collect();
    assert!(denorms.len() >= 8);
    for &d1 in &denorms {
        for &d2 in &denorms {
            // primitives
            diff(
                || format!("c2V({:#010x}, {:#010x}) [denormal]", d1.to_bits(), d2.to_bits()),
                |api| vbits((api.c2V)(d1, d2)),
            );
            let a = C2v { x: d1, y: d2 };
            let b = C2v { x: d2, y: -d1 };
            diff(
                || format!("c2Sub({a:?}, {b:?}) [denormal]"),
                |api| vbits((api.c2Sub)(a, b)),
            );
            diff(
                || format!("c2Dot({a:?}, {b:?}) [denormal]"),
                |api| (api.c2Dot)(a, b).to_bits(),
            );
            diff(
                || format!("c2Mulvs({a:?}, {:#010x}) [denormal]", d2.to_bits()),
                |api| vbits((api.c2Mulvs)(a, d2)),
            );
            diff(
                || format!("c2Maxv({a:?}, {b:?}) [denormal]"),
                |api| vbits((api.c2Maxv)(a, b)),
            );
            diff(
                || format!("c2Minv({a:?}, {b:?}) [denormal]"),
                |api| vbits((api.c2Minv)(a, b)),
            );
            diff(
                || format!("c2Clampv({a:?}, {b:?}, {a:?}) [denormal]"),
                |api| vbits((api.c2Clampv)(a, b, a)),
            );
            // kernels
            let ca = C2Circle { p: a, r: d1 };
            let cb = C2Circle { p: b, r: d2 };
            diff(
                || format!("c2CircletoCircle({ca:?}, {cb:?}) [denormal]"),
                |api| (api.c2CircletoCircle)(ca, cb),
            );
            let bx = C2Aabb { min: a, max: b };
            diff(
                || format!("c2CircletoAABB({ca:?}, {bx:?}) [denormal]"),
                |api| (api.c2CircletoAABB)(ca, bx),
            );
            let cap = C2Capsule { a, b, r: d1 };
            diff(
                || format!("c2CircletoCapsule({ca:?}, {cap:?}) [denormal]"),
                |api| (api.c2CircletoCapsule)(ca, cap),
            );
            diff(
                || format!("circle_collide({:#010x}, {:#010x}, {:#010x}) [denormal]",
                           d1.to_bits(), d2.to_bits(), d1.to_bits()),
                |api| (api.circle_collide)(d1, d2, d1),
            );
        }
    }
    // mix denormals with normals
    for _ in 0..N {
        let d = denorms[rng.below(denorms.len() as u32) as usize];
        let f = rng.finite_f32();
        let a = C2v { x: d, y: f };
        let b = C2v { x: f, y: d };
        diff(
            || format!("c2Dot({a:?}, {b:?}) [denormal x normal]"),
            |api| (api.c2Dot)(a, b).to_bits(),
        );
        diff(
            || format!("c2Mulvs({a:?}, {:#010x}) [denormal x normal]", f.to_bits()),
            |api| vbits((api.c2Mulvs)(a, f)),
        );
    }
}
