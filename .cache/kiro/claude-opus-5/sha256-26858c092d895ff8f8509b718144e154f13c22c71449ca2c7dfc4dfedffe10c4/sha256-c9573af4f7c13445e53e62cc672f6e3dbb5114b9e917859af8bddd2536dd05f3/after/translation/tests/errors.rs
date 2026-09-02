//! Phase C — error / rejection-path differential tests, one test per `ERRORS.md`
//! row, plus the generic C-API boundaries.
//!
//! The library's whole rejection surface is the three `return 0;` statements
//! reached from a `switch` `default:` label in `collided` (`lib.c:82,92,96`), so
//! every row here drives an out-of-range `C2_TYPE` value across the FFI boundary.
//! A C `enum` accepts any `int`, and the disassembly confirms the tags are read
//! as 4-byte integers (`cmpl $0x0,-0xc(%rbp)` / `cmpl $0x1,...`), so values with
//! no valid variant are real inputs the C handles — and the Rust must reject them
//! with the identical sentinel (`0`), not merely "fail somehow".
#![allow(non_snake_case)]

mod harness;
use harness::*;

use std::ffi::c_void;

/// Every `int` that is NOT a valid `C2_TYPE` variant, including one step past
/// each end of the valid range (`-1` and `2`) and both `i32` extremes.
const INVALID_TAGS: &[i32] = &[
    -1,             // one below the first variant
    2,              // one past the last variant
    3,
    255,
    256,
    0x1_0000,
    0x7FFF_FFFF,    // i32::MAX
    -2,
    -128,
    -0x8000_0000i64 as i32, // i32::MIN
];

const VALID_TAGS: &[i32] = &[C2_TYPE_CIRCLE, C2_TYPE_AABB];

/// Pointers a caller can legitimately hand to `collided` when the tag makes the
/// call reject before any dereference. `null`, a misaligned non-null value and an
/// unmapped address are all safe here precisely *because* the C returns from the
/// `default:` arm without touching them.
fn hostile_ptrs() -> Vec<*const c_void> {
    vec![
        std::ptr::null(),
        1usize as *const c_void,          // non-null, misaligned
        0xFFFF_FFFF_FFFF_FFFFusize as *const c_void, // unmapped
        0x1000usize as *const c_void,     // unmapped, page-aligned
    ]
}

/// A 16-byte valid buffer, used where the tag is valid on one side.
#[repr(C)]
struct Buf([u32; 4]);
impl Buf {
    fn ptr(&self) -> *const c_void {
        self.0.as_ptr() as *const c_void
    }
}

/// `{1.0, 2.0, 3.0, 4.0}`: as a `c2Circle` that is `p=(1,2) r=3`, as a `c2AABB`
/// it is `min=(1,2) max=(3,4)`, and **all four valid tag pairs return 1** for it
/// (asserted by [`assert_discriminating`]).
fn good_buf() -> Buf {
    Buf([0x3F80_0000, 0x4000_0000, 0x4040_0000, 0x4080_0000])
}

/// Asserts, against the C, that every VALID tag pair returns `1` for this buffer.
///
/// Without this, a `0` from an invalid tag pair would be weak evidence: it could
/// mean "rejected" or it could mean "dispatched and the geometry happened to miss".
/// With it, `0` can only mean rejection.
fn assert_discriminating(buf: &Buf) {
    let (c, _) = both();
    for &ta in VALID_TAGS {
        for &tb in VALID_TAGS {
            let got = unsafe { (c.collided)(buf.ptr(), ta, buf.ptr(), tb) };
            assert_eq!(
                got, 1,
                "buffer is not discriminating: C returned {got} for the valid pair ({ta},{tb}), \
                 so a 0 on the rejection path would not prove rejection"
            );
        }
    }
}

fn assert_same(ctx: &str, a: *const c_void, ta: i32, b: *const c_void, tb: i32, expect: i32) {
    let (c, r) = both();
    let cv = unsafe { (c.collided)(a, ta, b, tb) };
    let rv = unsafe { (r.collided)(a, ta, b, tb) };
    assert_eq!(
        cv, rv,
        "collided DIVERGED on the rejection path\n  case  : {ctx}\n  C     : {cv}\n  Rust  : {rv}"
    );
    // Not merely "both failed": both must return the exact sentinel the C's
    // `default:` arm returns.
    assert_eq!(cv, expect, "C returned {cv}, expected the `default:` sentinel {expect} — {ctx}");
    assert_eq!(rv, expect, "Rust returned {rv}, expected the `default:` sentinel {expect} — {ctx}");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — typeA invalid, typeB = C2_TYPE_CIRCLE
// ---------------------------------------------------------------------------

#[test]
fn err_row01_typeA_invalid_typeB_circle() {
    let good = good_buf();
    assert_discriminating(&good);
    for &ta in INVALID_TAGS {
        for pa in hostile_ptrs() {
            let ctx = format!("typeA={ta} (invalid), typeB=CIRCLE, A={pa:?}");
            assert_same(&ctx, pa, ta, good.ptr(), C2_TYPE_CIRCLE, 0);
        }
        // Also with a perfectly valid A pointer: the tag alone must cause the
        // rejection, independent of the payload.
        let ctx = format!("typeA={ta} (invalid), typeB=CIRCLE, A=valid buffer");
        assert_same(&ctx, good.ptr(), ta, good.ptr(), C2_TYPE_CIRCLE, 0);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — typeA invalid, typeB = C2_TYPE_AABB
// ---------------------------------------------------------------------------

#[test]
fn err_row02_typeA_invalid_typeB_aabb() {
    let good = good_buf();
    assert_discriminating(&good);
    for &ta in INVALID_TAGS {
        for pa in hostile_ptrs() {
            let ctx = format!("typeA={ta} (invalid), typeB=AABB, A={pa:?}");
            assert_same(&ctx, pa, ta, good.ptr(), C2_TYPE_AABB, 0);
        }
        let ctx = format!("typeA={ta} (invalid), typeB=AABB, A=valid buffer");
        assert_same(&ctx, good.ptr(), ta, good.ptr(), C2_TYPE_AABB, 0);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 — both tags invalid
// ---------------------------------------------------------------------------

#[test]
fn err_row03_both_types_invalid() {
    for &ta in INVALID_TAGS {
        for &tb in INVALID_TAGS {
            // Both pointers null: the outer `default:` returns before any deref.
            let ctx = format!("typeA={ta}, typeB={tb} (both invalid), A=B=NULL");
            assert_same(&ctx, std::ptr::null(), ta, std::ptr::null(), tb, 0);
        }
        for pa in hostile_ptrs() {
            for pb in hostile_ptrs() {
                let ctx = format!("typeA={ta}, typeB=2 (both invalid), A={pa:?} B={pb:?}");
                assert_same(&ctx, pa, ta, pb, 2, 0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 4 — typeA = C2_TYPE_CIRCLE, typeB invalid  (lib.c:82)
// ---------------------------------------------------------------------------

#[test]
fn err_row04_typeA_circle_typeB_invalid() {
    let good = good_buf();
    assert_discriminating(&good);
    for &tb in INVALID_TAGS {
        // A is a valid circle buffer; only typeB is bad.
        let ctx = format!("typeA=CIRCLE, typeB={tb} (invalid), A=valid");
        assert_same(&ctx, good.ptr(), C2_TYPE_CIRCLE, good.ptr(), tb, 0);
        // The inner `default:` returns before either operand is dereferenced, so
        // even a null/unmapped A is a defined input on this path.
        for pa in hostile_ptrs() {
            for pb in hostile_ptrs() {
                let ctx = format!("typeA=CIRCLE, typeB={tb} (invalid), A={pa:?} B={pb:?}");
                assert_same(&ctx, pa, C2_TYPE_CIRCLE, pb, tb, 0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 5 — typeA = C2_TYPE_AABB, typeB invalid  (lib.c:92)
// ---------------------------------------------------------------------------

#[test]
fn err_row05_typeA_aabb_typeB_invalid() {
    let good = good_buf();
    assert_discriminating(&good);
    for &tb in INVALID_TAGS {
        let ctx = format!("typeA=AABB, typeB={tb} (invalid), A=valid");
        assert_same(&ctx, good.ptr(), C2_TYPE_AABB, good.ptr(), tb, 0);
        for pa in hostile_ptrs() {
            for pb in hostile_ptrs() {
                let ctx = format!("typeA=AABB, typeB={tb} (invalid), A={pa:?} B={pb:?}");
                assert_same(&ctx, pa, C2_TYPE_AABB, pb, tb, 0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic boundaries: exhaustive small-integer tag sweep
// ---------------------------------------------------------------------------

/// Sweeps every tag pair in `-64..=64` (plus the extremes) against buffers whose
/// contents make **all four** valid dispatches return `1`.
///
/// The buffer choice is what gives this test teeth: if the Rust `default:` arm
/// wrongly dispatched instead of rejecting, it would return `1` where the C
/// returns `0`, and the assertion fires. With a buffer that happens to make the
/// dispatch return `0` too, the same bug would be invisible.
#[test]
fn err_generic_exhaustive_small_tag_sweep() {
    let (c, r) = both();
    // As c2AABB: min=(0,0) max=(1,1) -> self-overlap -> 1.
    // As c2Circle: p=(0,0) r=1      -> self-overlap -> 1.
    // Mixed circle/AABB              -> centre inside box, d2=0 < r2=1 -> 1.
    let positive = Buf([0x0000_0000, 0x0000_0000, 0x3F80_0000, 0x3F80_0000]);
    // A second buffer with the opposite property, so the sweep is not tuned to
    // one shape: as c2AABB this is min=(1,-1) max=(2,-2), an inverted box that
    // does NOT self-overlap.
    let negative = Buf([0x3F80_0000, 0xBF80_0000, 0x4000_0000, 0xC000_0000]);

    // Non-vacuity: confirm the C really returns 1 for all four valid pairs on
    // `positive`, otherwise the rejections below prove nothing.
    for &ta in VALID_TAGS {
        for &tb in VALID_TAGS {
            let got = unsafe { (c.collided)(positive.ptr(), ta, positive.ptr(), tb) };
            assert_eq!(
                got, 1,
                "sweep buffer is not discriminating: C returned {got} for the valid pair ({ta},{tb})"
            );
        }
    }

    let mut sweep: Vec<i32> = (-64..=64).collect();
    sweep.extend_from_slice(&[i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX, 0x1000, -0x1000]);
    let mut rejected = 0usize;
    let mut accepted = 0usize;
    for buf in [&positive, &negative] {
        for &ta in &sweep {
            for &tb in &sweep {
                let cv = unsafe { (c.collided)(buf.ptr(), ta, buf.ptr(), tb) };
                let rv = unsafe { (r.collided)(buf.ptr(), ta, buf.ptr(), tb) };
                assert_eq!(cv, rv, "collided DIVERGED at typeA={ta} typeB={tb}: C={cv} Rust={rv}");
                if VALID_TAGS.contains(&ta) && VALID_TAGS.contains(&tb) {
                    accepted += 1;
                } else {
                    assert_eq!(
                        cv, 0,
                        "C accepted the invalid tag pair ({ta},{tb}) with result {cv}"
                    );
                    assert_eq!(
                        rv, 0,
                        "Rust dispatched instead of rejecting the invalid tag pair ({ta},{tb})"
                    );
                    rejected += 1;
                }
            }
        }
    }
    assert_eq!(accepted, 8, "expected the 4 valid tag pairs on each of the 2 buffers");
    assert!(rejected > 35_000, "sweep covered too few rejections: {rejected}");
}

/// Null pointers combined with *valid* tags are deliberately NOT tested: the C
/// dereferences unconditionally on those paths (`lib.c:78,80,88,90`), which is
/// undefined behaviour rather than a defined rejection, so there is no C result
/// to match against. This test documents the boundary by asserting the only part
/// that IS defined — that a null pointer is fine as long as its own arm is not
/// the one taken.
#[test]
fn err_generic_null_is_only_safe_on_the_rejecting_arm() {
    let good = good_buf();
    assert_discriminating(&good);
    // typeA invalid => A is never read, so a null A is defined.
    assert_same("null A, invalid typeA", std::ptr::null(), 7, good.ptr(), C2_TYPE_CIRCLE, 0);
    // typeB invalid => neither operand is read, so a null B is defined.
    assert_same("null B, invalid typeB", good.ptr(), C2_TYPE_CIRCLE, std::ptr::null(), 7, 0);
    assert_same("null B, invalid typeB, A=aabb", good.ptr(), C2_TYPE_AABB, std::ptr::null(), 7, 0);
}

/// There is no length, size or count parameter anywhere in the public API, so
/// "zero and oversized lengths" degenerate to the size of the pointed-to object.
/// The nearest real analogue is a buffer that is only just large enough: a
/// 12-byte allocation read as a 12-byte `c2Circle`. Verifying both libraries read
/// exactly the same number of bytes catches an over-read in the Rust wrapper.
#[test]
fn err_generic_minimum_sized_buffers() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..2000 {
        // Exactly 12 bytes — the size of c2Circle. Reading more would be an
        // over-read; both libraries must stay within it.
        let circle: [u32; 3] = [rng.next_u32(), rng.next_u32(), rng.next_u32()];
        let p = circle.as_ptr() as *const c_void;
        let cv = unsafe { (c.collided)(p, C2_TYPE_CIRCLE, p, C2_TYPE_CIRCLE) };
        let rv = unsafe { (r.collided)(p, C2_TYPE_CIRCLE, p, C2_TYPE_CIRCLE) };
        assert_eq!(cv, rv, "12-byte circle buffer #{i} diverged: C={cv} Rust={rv}");

        // Exactly 16 bytes — the size of c2AABB.
        let aabb: [u32; 4] = [rng.next_u32(), rng.next_u32(), rng.next_u32(), rng.next_u32()];
        let q = aabb.as_ptr() as *const c_void;
        let cv = unsafe { (c.collided)(q, C2_TYPE_AABB, q, C2_TYPE_AABB) };
        let rv = unsafe { (r.collided)(q, C2_TYPE_AABB, q, C2_TYPE_AABB) };
        assert_eq!(cv, rv, "16-byte aabb buffer #{i} diverged: C={cv} Rust={rv}");
    }
}

/// The predicates take structs by value and have no rejection surface of their
/// own — every bit pattern is accepted. This asserts that property holds
/// identically in both libraries for the inputs a caller would consider
/// "invalid" (NaN radius, inverted box, negative radius): both must RETURN a
/// value rather than trap, and the values must agree.
#[test]
fn err_generic_predicates_never_reject() {
    let (c, r) = both();
    let nan = f32::NAN;
    let snan = f32::from_bits(0x7F80_0001);
    let inf = f32::INFINITY;
    let v = |x: f32, y: f32| C2v { x, y };
    for &bad in &[nan, snan, inf, -inf, -1.0f32, f32::MAX] {
        let A = C2Circle { p: v(bad, bad), r: bad };
        let B = C2Aabb { min: v(bad, -bad), max: v(-bad, bad) };
        assert_eq!(
            (c.c2CircletoCircle)(A, A),
            (r.c2CircletoCircle)(A, A),
            "c2CircletoCircle diverged for {bad:e}/{:#010x}",
            bad.to_bits()
        );
        assert_eq!(
            (c.c2CircletoAABB)(A, B),
            (r.c2CircletoAABB)(A, B),
            "c2CircletoAABB diverged for {bad:e}/{:#010x}",
            bad.to_bits()
        );
        assert_eq!(
            (c.c2AABBtoAABB)(B, B),
            (r.c2AABBtoAABB)(B, B),
            "c2AABBtoAABB diverged for {bad:e}/{:#010x}",
            bad.to_bits()
        );
    }
}
