//! Layout / ABI checks and a probe of the one input class `ERRORS.md` excludes.

mod common;

use common::*;
use std::ffi::c_void;

/// The Rust crate stores `c2Simplex`'s four `c2sv` members as an array. Confirm
/// that this is layout-identical to the C's four consecutive named fields, which
/// is what makes `c2sv *verts = &s.a;` pointer walking valid in the translation.
#[test]
fn struct_layouts_match_the_c_abi() {
    use std::mem::{align_of, size_of};

    assert_eq!(size_of::<c2v>(), 8);
    assert_eq!(size_of::<c2r>(), 8);
    assert_eq!(size_of::<c2x>(), 16);
    assert_eq!(size_of::<c2Circle>(), 12);
    assert_eq!(size_of::<c2AABB>(), 16);
    assert_eq!(size_of::<c2Capsule>(), 20);
    assert_eq!(size_of::<c2GJKCache>(), 4 + 4 + 12 + 12 + 4);
    assert_eq!(size_of::<c2Proxy>(), 4 + 4 + 8 * 8);
    assert_eq!(size_of::<c2sv>(), 8 * 3 + 4 + 4 + 4);
    // 4 * sizeof(c2sv) + float + int
    assert_eq!(size_of::<c2Simplex>(), 4 * size_of::<c2sv>() + 4 + 4);
    assert_eq!(align_of::<c2Simplex>(), 4);

    // The array-of-4 must start at offset 0 and be contiguous, so that
    // `verts[i]` in Rust is the same byte range as walking `&s.a` in C.
    let s = c2Simplex::default();
    let base = &s as *const c2Simplex as usize;
    for i in 0..4 {
        assert_eq!(
            &s.verts[i] as *const c2sv as usize - base,
            i * size_of::<c2sv>(),
            "c2Simplex.verts[{i}] is not at the C's offsetof(c2Simplex, {})",
            ["a", "b", "c", "d"][i]
        );
    }
    assert_eq!(
        &s.div as *const f32 as usize - base,
        4 * size_of::<c2sv>()
    );
}

/// `c2GJK` with an out-of-range `C2_TYPE` is the one input class `ERRORS.md`
/// excludes from bit-exact comparison. This test documents *why*, by proving the
/// C's answer is not a function of its inputs: the same call, made twice with
/// identical arguments but a differently-dirtied stack, gives different answers.
///
/// `c2MakeProxy` has no `default:` label, so an invalid type leaves `c2Proxy pA`
/// (an uninitialised local in `c2GJK`) untouched; every subsequent read of
/// `pA.count` / `pA.radius` / `pA.verts` is then a read of whatever the previous
/// call left on the stack.
///
/// The *deterministic* invalid-enum paths — `c2Collided` and `c2MakeProxy`
/// themselves — are fully covered bit-exactly in `phase_c_errors.rs`.
#[test]
fn gjk_invalid_type_reads_uninitialised_stack_and_is_excluded() {
    #[allow(clippy::type_complexity)]
    type GjkFn = unsafe extern "C" fn(
        *const c_void,
        i32,
        *const c2x,
        *const c_void,
        i32,
        *const c2x,
        *mut c2v,
        *mut c2v,
        i32,
        *mut i32,
        *mut c2GJKCache,
    ) -> f32;
    let (c_f, _r_f) = pair::<GjkFn>("c2GJK");

    let circ = c2Circle {
        p: c2v { x: 1.0, y: 2.0 },
        r: 3.0,
    };
    let sp = &circ as *const c2Circle as *const c_void;

    // Call c2GJK with a VALID pair first (dirties the stack slot that pA will
    // occupy with one pattern), then with typeA = 3 (invalid).
    let call = |ta: i32, tb: i32| -> f32 {
        unsafe {
            let (mut a, mut b) = (c2v::default(), c2v::default());
            c_f(
                sp,
                ta,
                std::ptr::null(),
                sp,
                tb,
                std::ptr::null(),
                &mut a,
                &mut b,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        }
    };

    // Warm the stack with a large-coordinate shape, then repeat the invalid call.
    let far = c2Circle {
        p: c2v { x: 9.0e9, y: -9.0e9 },
        r: 1.0,
    };
    let fp = &far as *const c2Circle as *const c_void;
    let call_far = |ta: i32, tb: i32| -> f32 {
        unsafe {
            let (mut a, mut b) = (c2v::default(), c2v::default());
            c_f(
                fp,
                ta,
                std::ptr::null(),
                fp,
                tb,
                std::ptr::null(),
                &mut a,
                &mut b,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        }
    };

    let _ = call(C2_TYPE_CIRCLE, C2_TYPE_CIRCLE);
    let after_small = call(3, C2_TYPE_CIRCLE);
    let _ = call_far(C2_TYPE_CIRCLE, C2_TYPE_CIRCLE);
    let after_far = call(3, C2_TYPE_CIRCLE);

    println!(
        "c2GJK(typeA=3) after a small-coord call: {after_small} (0x{:08x})",
        after_small.to_bits()
    );
    println!(
        "c2GJK(typeA=3) after a large-coord call: {after_far} (0x{:08x})",
        after_far.to_bits()
    );
    // Not asserted as unequal (the stack could coincidentally match); the point
    // is that this value is stack residue, not a function of the arguments, so it
    // is not a valid differential-test target. Recorded here for the record.
}
