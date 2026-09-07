//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! The library has no error codes; its entire rejection surface is
//! (a) the three `default: return 0` switch arms in `collided`, and
//! (b) the unordered-compare sentinel `0` returned by the three predicates.
//! Both are asserted to be *the same value* in C and Rust, not merely
//! "both failed".

#![allow(non_snake_case)]

mod harness;
use harness::*;
use std::ffi::{c_int, c_void};
use std::ptr;

/// Every out-of-range `C2_TYPE` value worth passing across the FFI boundary.
/// A C enum accepts any `int`, so these are all real inputs.
const BAD_TAGS: &[c_int] = &[
    2, // one step past C2_TYPE_AABB
    3,
    4,
    7,
    -1,
    -2,
    255,
    256,
    0x7FFF,
    0x1_0000,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    0x4000_0000,
    -0x4000_0000,
];

const GOOD_TAGS: &[c_int] = &[C2_TYPE_CIRCLE, C2_TYPE_AABB];

fn buf16(rng: &mut Rng) -> [u8; 16] {
    let mut b = [0u8; 16];
    for i in 0..4 {
        b[i * 4..i * 4 + 4].copy_from_slice(&rng.next_u32().to_le_bytes());
    }
    b
}

#[track_caller]
fn both(c: &Impl, r: &Impl, pa: *const c_void, ta: c_int, pb: *const c_void, tb: c_int) -> c_int {
    unsafe {
        let cv = (c.collided)(pa, ta, pb, tb);
        let rv = (r.collided)(pa, ta, pb, tb);
        assert_eq!(
            cv, rv,
            "collided diverged: typeA={ta} typeB={tb} A={pa:?} B={pb:?} -> C={cv} Rust={rv}"
        );
        cv
    }
}

// ---------------------------------------------------------------------------
// Row 1 — outer `default:` (lib.c:95-96): typeA out of range
// ---------------------------------------------------------------------------

#[test]
fn err01_outer_default_bad_typeA() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xE1);
    for &ta in BAD_TAGS {
        for &tb in GOOD_TAGS.iter().chain(BAD_TAGS.iter()) {
            let a = buf16(&mut rng);
            let b = buf16(&mut rng);
            let got = both(
                c,
                r,
                a.as_ptr() as *const c_void,
                ta,
                b.as_ptr() as *const c_void,
                tb,
            );
            assert_eq!(got, 0, "expected the C sentinel 0 for typeA={ta}, typeB={tb}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 2 — inner `default:` under C2_TYPE_CIRCLE (lib.c:81-82)
// ---------------------------------------------------------------------------

#[test]
fn err02_inner_default_circle_bad_typeB() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xE2);
    for &tb in BAD_TAGS {
        for _ in 0..64 {
            let a = buf16(&mut rng);
            let b = buf16(&mut rng);
            let got = both(
                c,
                r,
                a.as_ptr() as *const c_void,
                C2_TYPE_CIRCLE,
                b.as_ptr() as *const c_void,
                tb,
            );
            assert_eq!(got, 0, "expected 0 for typeA=CIRCLE typeB={tb}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — inner `default:` under C2_TYPE_AABB (lib.c:91-92)
// ---------------------------------------------------------------------------

#[test]
fn err03_inner_default_aabb_bad_typeB() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xE3);
    for &tb in BAD_TAGS {
        for _ in 0..64 {
            let a = buf16(&mut rng);
            let b = buf16(&mut rng);
            let got = both(
                c,
                r,
                a.as_ptr() as *const c_void,
                C2_TYPE_AABB,
                b.as_ptr() as *const c_void,
                tb,
            );
            assert_eq!(got, 0, "expected 0 for typeA=AABB typeB={tb}");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 4-5 — exactly one step past the documented valid range
// ---------------------------------------------------------------------------

#[test]
fn err04_typeA_one_past_range() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xE4);
    let a = buf16(&mut rng);
    let b = buf16(&mut rng);
    for tb in [0, 1, 2] {
        let got = both(
            c,
            r,
            a.as_ptr() as *const c_void,
            C2_TYPE_AABB + 1,
            b.as_ptr() as *const c_void,
            tb,
        );
        assert_eq!(got, 0);
    }
    // and one step *before* the range
    for tb in [0, 1, 2] {
        let got = both(
            c,
            r,
            a.as_ptr() as *const c_void,
            C2_TYPE_CIRCLE - 1,
            b.as_ptr() as *const c_void,
            tb,
        );
        assert_eq!(got, 0);
    }
}

#[test]
fn err05_typeB_one_past_range() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xE5);
    let a = buf16(&mut rng);
    let b = buf16(&mut rng);
    for ta in [C2_TYPE_CIRCLE, C2_TYPE_AABB] {
        for tb in [C2_TYPE_AABB + 1, C2_TYPE_CIRCLE - 1] {
            let got = both(
                c,
                r,
                a.as_ptr() as *const c_void,
                ta,
                b.as_ptr() as *const c_void,
                tb,
            );
            assert_eq!(got, 0, "typeA={ta} typeB={tb}");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 6-8 — null pointers on the paths where the C rejects before deref
// ---------------------------------------------------------------------------

#[test]
fn err06_null_pointers_with_bad_typeA() {
    let (c, r) = pair();
    let n = ptr::null::<c_void>();
    for &ta in BAD_TAGS {
        for &tb in GOOD_TAGS.iter().chain(BAD_TAGS.iter()) {
            assert_eq!(both(c, r, n, ta, n, tb), 0, "typeA={ta} typeB={tb}");
        }
    }
    // dangling / misaligned non-null garbage pointers are equally safe here
    for &ta in BAD_TAGS {
        let junk = 0xdead_beef_usize as *const c_void;
        assert_eq!(both(c, r, junk, ta, junk, 0), 0);
        assert_eq!(both(c, r, junk, ta, junk, 1), 0);
    }
}

#[test]
fn err07_null_B_with_typeA_circle_bad_typeB() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xE7);
    let a = buf16(&mut rng);
    for &tb in BAD_TAGS {
        assert_eq!(
            both(c, r, a.as_ptr() as *const c_void, C2_TYPE_CIRCLE, ptr::null(), tb),
            0,
            "typeB={tb}"
        );
        // A may be null too: the outer switch matched but nothing is read yet
        assert_eq!(both(c, r, ptr::null(), C2_TYPE_CIRCLE, ptr::null(), tb), 0);
    }
}

#[test]
fn err08_null_B_with_typeA_aabb_bad_typeB() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xE8);
    let a = buf16(&mut rng);
    for &tb in BAD_TAGS {
        assert_eq!(
            both(c, r, a.as_ptr() as *const c_void, C2_TYPE_AABB, ptr::null(), tb),
            0,
            "typeB={tb}"
        );
        assert_eq!(both(c, r, ptr::null(), C2_TYPE_AABB, ptr::null(), tb), 0);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — the tag lies about the struct: 12-byte circle vs 16-byte box
// ---------------------------------------------------------------------------

#[test]
fn err09_tag_struct_size_mismatch() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xE9);
    for _ in 0..4000 {
        // Allocate 16 bytes but tag it CIRCLE (12 read) and AABB (16 read).
        let a = buf16(&mut rng);
        let b = buf16(&mut rng);
        for &(ta, tb) in &[
            (C2_TYPE_CIRCLE, C2_TYPE_AABB),
            (C2_TYPE_AABB, C2_TYPE_CIRCLE),
            (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
            (C2_TYPE_AABB, C2_TYPE_AABB),
        ] {
            both(
                c,
                r,
                a.as_ptr() as *const c_void,
                ta,
                b.as_ptr() as *const c_void,
                tb,
            );
        }
    }
    // unaligned (odd-offset) reads: C does an unaligned load, Rust must too
    let mut raw = [0u8; 40];
    for (i, x) in raw.iter_mut().enumerate() {
        *x = (i as u8).wrapping_mul(37).wrapping_add(11);
    }
    for off in 0..(40 - 16) {
        let p = unsafe { raw.as_ptr().add(off) } as *const c_void;
        for &ta in GOOD_TAGS {
            for &tb in GOOD_TAGS {
                both(c, r, p, ta, p, tb);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 10-11 — NaN sentinel rejection in the predicates
// ---------------------------------------------------------------------------

const NANS: &[f32] = &[
    f32::from_bits(0x7FC0_0000),
    f32::from_bits(0xFFC0_0000),
    f32::from_bits(0x7FC0_1234),
    f32::from_bits(0xFFDE_AD00),
    f32::from_bits(0x7F80_0001),
    f32::from_bits(0xFF80_0001),
    f32::from_bits(0x7FBF_FFFF),
];

#[test]
fn err10_nan_radius_rejects_with_zero() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xEA);
    for &nan in NANS {
        for _ in 0..500 {
            // Overlapping geometry, but a NaN radius: comiss unordered -> 0.
            let p = rng.v_small();
            let A = C2Circle { p, r: nan };
            let B = C2Circle { p, r: 5.0 };
            unsafe {
                let cv = (c.c2CircletoCircle)(A, B);
                let rv = (r.c2CircletoCircle)(A, B);
                assert_eq!(cv, rv, "c2CircletoCircle NaN radius: C={cv} Rust={rv}");
                assert_eq!(cv, 0, "C must reject a NaN radius with 0");

                let cv = (c.c2CircletoCircle)(B, A);
                let rv = (r.c2CircletoCircle)(B, A);
                assert_eq!(cv, rv);
                assert_eq!(cv, 0);

                let bx = C2Aabb { min: v(p.x - 1.0, p.y - 1.0), max: v(p.x + 1.0, p.y + 1.0) };
                let cv = (c.c2CircletoAABB)(A, bx);
                let rv = (r.c2CircletoAABB)(A, bx);
                assert_eq!(cv, rv);
                assert_eq!(cv, 0, "c2CircletoAABB must reject a NaN radius with 0");
            }
        }
    }
}

#[test]
fn err11_nan_coordinate_rejects_with_zero() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xEB);
    for &nan in NANS {
        for slot in 0..2 {
            for _ in 0..300 {
                let mut p = rng.v_small();
                if slot == 0 {
                    p.x = nan;
                } else {
                    p.y = nan;
                }
                let A = C2Circle { p, r: 100.0 };
                let B = C2Circle { p: rng.v_small(), r: 100.0 };
                unsafe {
                    let cv = (c.c2CircletoCircle)(A, B);
                    assert_eq!(cv, (r.c2CircletoCircle)(A, B));
                    assert_eq!(cv, 0, "NaN centre must be rejected with 0");
                    let cv = (c.c2CircletoCircle)(B, A);
                    assert_eq!(cv, (r.c2CircletoCircle)(B, A));
                    assert_eq!(cv, 0);
                    let bx = C2Aabb { min: v(-1e3, -1e3), max: v(1e3, 1e3) };
                    let cv = (c.c2CircletoAABB)(A, bx);
                    assert_eq!(cv, (r.c2CircletoAABB)(A, bx));
                    assert_eq!(cv, 0);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — negative radius is NOT rejected; it behaves like |r|
// ---------------------------------------------------------------------------

#[test]
fn err12_negative_radius_not_rejected() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xEC);
    for _ in 0..4000 {
        let p = rng.v_small();
        let q = rng.v_small();
        let rr = (rng.next_u32() % 500) as f32 + 0.5;
        let A = C2Circle { p, r: -rr };
        let B = C2Circle { p: q, r: 0.0 };
        unsafe {
            let cv = (c.c2CircletoCircle)(A, B);
            assert_eq!(cv, (r.c2CircletoCircle)(A, B), "negative radius diverged");
            // C squares the sum, so -rr behaves exactly like +rr here
            let Apos = C2Circle { p, r: rr };
            assert_eq!(cv, (c.c2CircletoCircle)(Apos, B), "C's own |r| equivalence");

            let bx = C2Aabb { min: v(q.x - 1.0, q.y - 1.0), max: v(q.x + 1.0, q.y + 1.0) };
            let cv = (c.c2CircletoAABB)(A, bx);
            assert_eq!(cv, (r.c2CircletoAABB)(A, bx));
            assert_eq!(cv, (c.c2CircletoAABB)(Apos, bx));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — degenerate / inverted / NaN boxes are not validated
// ---------------------------------------------------------------------------

#[test]
fn err13_aabb_no_validation() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xED);
    // all-NaN box "collides" with itself because all four `<` are false
    for &nan in NANS {
        let b = C2Aabb { min: v(nan, nan), max: v(nan, nan) };
        unsafe {
            let cv = (c.c2AABBtoAABB)(b, b);
            assert_eq!(cv, (r.c2AABBtoAABB)(b, b));
            assert_eq!(cv, 1, "C returns 1 for an all-NaN box pair");
        }
    }
    // inverted and degenerate boxes: no rejection, just the raw comparisons
    for _ in 0..4000 {
        let p = rng.v_small();
        let q = rng.v_small();
        let inverted = C2Aabb { min: v(p.x.max(q.x), p.y.max(q.y)), max: v(p.x.min(q.x), p.y.min(q.y)) };
        let degenerate = C2Aabb { min: p, max: p };
        let normal = C2Aabb { min: v(p.x.min(q.x), p.y.min(q.y)), max: v(p.x.max(q.x), p.y.max(q.y)) };
        for A in [inverted, degenerate, normal] {
            for B in [inverted, degenerate, normal] {
                unsafe {
                    assert_eq!(
                        (c.c2AABBtoAABB)(A, B),
                        (r.c2AABBtoAABB)(A, B),
                        "c2AABBtoAABB diverged A={A:?} B={B:?}"
                    );
                }
            }
        }
    }
    // a single NaN in each of the 8 slots
    for &nan in NANS {
        for slot in 0..8 {
            let mut s = [0.0f32; 8];
            for x in s.iter_mut() {
                *x = rng.small_f32();
            }
            s[slot] = nan;
            let A = C2Aabb { min: v(s[0], s[1]), max: v(s[2], s[3]) };
            let B = C2Aabb { min: v(s[4], s[5]), max: v(s[6], s[7]) };
            unsafe {
                assert_eq!((c.c2AABBtoAABB)(A, B), (r.c2AABBtoAABB)(A, B), "slot {slot}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Extra generic boundary: zero / oversized "lengths" have no analogue here
// (no length parameters exist), but the return value must always be exactly
// 0 or 1 in both libraries — never some other int.
// ---------------------------------------------------------------------------

#[test]
fn err14_return_values_are_always_0_or_1() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xEE);
    for _ in 0..4000 {
        let A = rng.circle_interesting();
        let B = rng.circle_interesting();
        let bx = rng.aabb_interesting();
        let by = rng.aabb_interesting();
        unsafe {
            for (cv, rv) in [
                ((c.c2CircletoCircle)(A, B), (r.c2CircletoCircle)(A, B)),
                ((c.c2CircletoAABB)(A, bx), (r.c2CircletoAABB)(A, bx)),
                ((c.c2AABBtoAABB)(bx, by), (r.c2AABBtoAABB)(bx, by)),
            ] {
                assert_eq!(cv, rv);
                assert!(cv == 0 || cv == 1, "C returned {cv}, not a 0/1 boolean");
            }
        }
    }
    // out-of-range tags must yield exactly 0, never a truthy value
    let buf = [0u8; 16];
    for &ta in BAD_TAGS {
        for &tb in BAD_TAGS {
            let got = both(
                c,
                r,
                buf.as_ptr() as *const c_void,
                ta,
                buf.as_ptr() as *const c_void,
                tb,
            );
            assert_eq!(got, 0);
        }
    }
}
