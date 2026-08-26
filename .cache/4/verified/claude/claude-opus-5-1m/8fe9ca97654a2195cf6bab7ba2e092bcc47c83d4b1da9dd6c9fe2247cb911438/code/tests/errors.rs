//! Phase C -- one differential test per row of `ERRORS.md`.
//!
//! Each test constructs the exact condition the C code rejects / special-cases
//! and asserts that the C `.so` and the Rust `.so` produce the *same* sentinel,
//! not merely that "both failed".  Where the C sentinel is a fixed value it is
//! also asserted literally, so the test would catch both implementations
//! drifting together.

mod harness;

use core::ffi::{c_int, c_uint};
use harness::*;

const SENTINEL: f32 = -12345.678;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct CPlane {
    normal: [f32; 3],
    dist: f32,
    type_: u8,
    signbits: u8,
    pad: [u8; 2],
}

// ---------------------------------------------------------------------------
// row 1 -- DirToByte(NULL)
// ---------------------------------------------------------------------------

/// `if ( !dir ) { return 0; }` -- the only NULL check in q_math.c.
#[test]
fn dir_to_byte_null() {
    type F = unsafe extern "C" fn(*mut f32) -> c_int;
    let (c, r): (F, F) = both("DirToByte");
    let vc = unsafe { c(std::ptr::null_mut()) };
    let vr = unsafe { r(std::ptr::null_mut()) };
    assert_int("DirToByte(NULL)", vc, vr);
    assert_eq!(vc, 0, "DirToByte(NULL) must return the sentinel 0");
}

// ---------------------------------------------------------------------------
// rows 2, 3, 4 -- ByteToDir index range
// ---------------------------------------------------------------------------

type ByteToDirFn = unsafe extern "C" fn(c_int, *mut f32) -> ();

fn byte_to_dir_pair(c: ByteToDirFn, r: ByteToDirFn, b: c_int) -> ([f32; 3], [f32; 3]) {
    let mut oc = [SENTINEL; 3];
    let mut or_ = [SENTINEL; 3];
    unsafe { c(b, oc.as_mut_ptr()) };
    unsafe { r(b, or_.as_mut_ptr()) };
    (oc, or_)
}

/// `if ( b < 0 || b >= NUMVERTEXNORMALS ) { VectorCopy( vec3_origin, dir ); return; }`
#[test]
fn byte_to_dir_out_of_range() {
    let (c, r): (ByteToDirFn, ByteToDirFn) = both("ByteToDir");

    for b in [
        -1,
        -2,
        -161,
        -162,
        -163,
        162,
        163,
        1000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ] {
        let (oc, or_) = byte_to_dir_pair(c, r, b);
        assert_vec(&format!("ByteToDir({b})"), &oc, &or_);
        assert_eq!(
            oc,
            [0.0, 0.0, 0.0],
            "ByteToDir({b}) must clear dir to vec3_origin"
        );
    }
}

/// One step either side of the valid range.
#[test]
fn byte_to_dir_boundary() {
    let (c, r): (ByteToDirFn, ByteToDirFn) = both("ByteToDir");

    // 161 is the last valid index and must NOT be zeroed
    let (oc, or_) = byte_to_dir_pair(c, r, 161);
    assert_vec("ByteToDir(161)", &oc, &or_);
    assert_ne!(oc, [0.0, 0.0, 0.0], "bytedirs[161] is not the zero vector");

    let (oc, or_) = byte_to_dir_pair(c, r, 162);
    assert_vec("ByteToDir(162)", &oc, &or_);
    assert_eq!(oc, [0.0, 0.0, 0.0]);

    let (oc, or_) = byte_to_dir_pair(c, r, 0);
    assert_vec("ByteToDir(0)", &oc, &or_);
    assert_ne!(oc, [0.0, 0.0, 0.0]);
}

// ---------------------------------------------------------------------------
// row 5 -- NormalizeColor with a zero maximum
// ---------------------------------------------------------------------------

/// `if ( !max ) { VectorClear( out ); }` -- `!max` is true for `+0.0` and for
/// `-0.0`, and the *returned* max keeps its sign.
#[test]
fn normalize_color_zero_max() {
    type F = unsafe extern "C" fn(*const f32, *mut f32) -> f32;
    let (c, r): (F, F) = both("NormalizeColor");

    let cases: &[[f32; 3]] = &[
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [0.0, -0.0, 0.0],
        [-0.0, 0.0, -0.0],
        [-0.0, -1.0, -2.0],
        [-1.0, -0.0, -2.0],
        [-1.0, -2.0, -0.0],
        [0.0, -1.0, -1.0],
        [-1.0, 0.0, -1.0],
        [-1.0, -1.0, 0.0],
    ];
    for v in cases {
        let mut oc = [SENTINEL; 3];
        let mut or_ = [SENTINEL; 3];
        let vc = unsafe { c(v.as_ptr(), oc.as_mut_ptr()) };
        let vr = unsafe { r(v.as_ptr(), or_.as_mut_ptr()) };
        assert_f32(&format!("NormalizeColor({v:?}) return"), vc, vr);
        assert_vec(&format!("NormalizeColor({v:?}) out"), &oc, &or_);
        assert_eq!(oc, [0.0, 0.0, 0.0], "out must be cleared when max == 0");
        assert_eq!(vc, 0.0, "the returned max compares equal to zero");
    }
}

// ---------------------------------------------------------------------------
// rows 6, 7 -- ColorBytes3 / ColorBytes4 conversion edges
// ---------------------------------------------------------------------------

/// `unsigned i;` is uninitialised in `ColorBytes3` and only bytes 0..2 are
/// written, so bit 24..31 of the C result is whatever the callee's stack slot
/// held.  Only the 24 defined bits can be compared; this test pins that down
/// and additionally shows the defined bits agree with `ColorBytes4`.
#[test]
fn color_bytes3_top_byte_is_indeterminate() {
    type F3 = unsafe extern "C" fn(f32, f32, f32) -> c_uint;
    type F4 = unsafe extern "C" fn(f32, f32, f32, f32) -> c_uint;
    let (c3, r3): (F3, F3) = both("ColorBytes3");
    let (c4, r4): (F4, F4) = both("ColorBytes4");

    let mut rng = Rng::new(0xC010_0003);
    for _ in 0..5000 {
        let (a, b, d) = (rng.f32_any(), rng.f32_any(), rng.f32_any());
        let vc3 = unsafe { c3(a, b, d) };
        let vr3 = unsafe { r3(a, b, d) };
        assert_int(
            &format!("ColorBytes3({a:?},{b:?},{d:?}) low 24 bits"),
            vc3 & 0x00ff_ffff,
            vr3 & 0x00ff_ffff,
        );
        // the same three components through ColorBytes4, whose 4th byte IS
        // written, must have identical low 24 bits in both implementations
        let vc4 = unsafe { c4(a, b, d, 0.0) };
        let vr4 = unsafe { r4(a, b, d, 0.0) };
        assert_int(&format!("ColorBytes4({a:?},{b:?},{d:?},0)"), vc4, vr4);
        assert_int(
            &format!("ColorBytes3 vs ColorBytes4 ({a:?},{b:?},{d:?})"),
            vc3 & 0x00ff_ffff,
            vc4 & 0x00ff_ffff,
        );
    }
}

/// `(byte)(x * 255)` is `cvttss2si` into a 32-bit register plus a byte store, so
/// NaN, ±inf and anything outside `INT_MIN..INT_MAX` become `0x80000000` and
/// therefore byte `0x00`; in-range values wrap modulo 256.
#[test]
fn color_bytes_out_of_range() {
    type F4 = unsafe extern "C" fn(f32, f32, f32, f32) -> c_uint;
    let (c4, r4): (F4, F4) = both("ColorBytes4");

    let cases: &[f32] = &[
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e30,
        -1e30,
        f32::MAX,
        -f32::MAX,
        2147483648.0 / 255.0, // exactly at the cvttss2si boundary
        2147483520.0 / 255.0,
        -2147483648.0 / 255.0,
        -0.001,
        -1.0,
        1.0,
        1.00001,
        2.0,
        256.0 / 255.0,
        257.0 / 255.0,
        1000.0,
        f32::MIN_POSITIVE,
        1e-45,
    ];
    for &v in cases {
        let vc = unsafe { c4(v, v, v, v) };
        let vr = unsafe { r4(v, v, v, v) };
        assert_int(&format!("ColorBytes4({v:?} x4)"), vc, vr);
    }
    // the specific documented sentinel: NaN and inf give a zero byte
    for &v in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e30, -1e30] {
        assert_eq!(
            unsafe { c4(v, v, v, v) },
            0,
            "cvttss2si indefinite -> all bytes 0 for {v:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// rows 8..11 -- the two clamps
// ---------------------------------------------------------------------------

#[test]
fn clamp_char_saturation() {
    type F = unsafe extern "C" fn(c_int) -> i8;
    let (c, r): (F, F) = both("ClampChar");
    for i in [
        i32::MIN,
        i32::MIN + 1,
        -100000,
        -1000,
        -130,
        -129,
        -128,
        -127,
        126,
        127,
        128,
        129,
        1000,
        100000,
        i32::MAX - 1,
        i32::MAX,
    ] {
        let (vc, vr) = (unsafe { c(i) }, unsafe { r(i) });
        assert_int(&format!("ClampChar({i})"), vc, vr);
        let expect = if i < -128 {
            -128
        } else if i > 127 {
            127
        } else {
            i as i8
        };
        assert_eq!(vc, expect, "ClampChar({i})");
    }
}

#[test]
fn clamp_short_saturation() {
    type F = unsafe extern "C" fn(c_int) -> i16;
    let (c, r): (F, F) = both("ClampShort");
    for i in [
        i32::MIN,
        i32::MIN + 1,
        -100000,
        -32770,
        -32769,
        -32768,
        -32767,
        32766,
        32767,
        32768,
        32769,
        100000,
        i32::MAX - 1,
        i32::MAX,
    ] {
        let (vc, vr) = (unsafe { c(i) }, unsafe { r(i) });
        assert_int(&format!("ClampShort({i})"), vc, vr);
        let expect = if i < -32768 {
            -32768
        } else if i > 32767 {
            32767
        } else {
            i as i16
        };
        assert_eq!(vc, expect, "ClampShort({i})");
    }
}

// ---------------------------------------------------------------------------
// rows 12, 13 -- PlaneFromPoints returns qfalse for a degenerate triangle
// ---------------------------------------------------------------------------

#[test]
fn plane_from_points_degenerate() {
    type F = unsafe extern "C" fn(*mut f32, *const f32, *const f32, *const f32) -> c_int;
    let (c, r): (F, F) = both("PlaneFromPoints");

    let a = [1.0f32, 2.0, 3.0];
    let degenerate: &[([f32; 3], [f32; 3], [f32; 3])] = &[
        (a, a, a),                                     // all equal
        (a, a, [4.0, 5.0, 6.0]),                       // b == a
        (a, [4.0, 5.0, 6.0], a),                       // c == a
        ([0.0; 3], [0.0; 3], [0.0; 3]),                // all zero
        ([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]), // collinear
        ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [2.0, 2.0, 2.0]), // collinear
        ([1.0, 1.0, 1.0], [2.0, 2.0, 2.0], [3.0, 3.0, 3.0]), // collinear
    ];
    for (pa, pb, pc) in degenerate {
        let mut plc = [SENTINEL; 4];
        let mut plr = [SENTINEL; 4];
        let vc = unsafe { c(plc.as_mut_ptr(), pa.as_ptr(), pb.as_ptr(), pc.as_ptr()) };
        let vr = unsafe { r(plr.as_mut_ptr(), pa.as_ptr(), pb.as_ptr(), pc.as_ptr()) };
        assert_int(
            &format!("PlaneFromPoints({pa:?},{pb:?},{pc:?}) return"),
            vc,
            vr,
        );
        assert_eq!(vc, 0, "degenerate triangle must return qfalse");
        assert_vec(
            &format!("PlaneFromPoints({pa:?},{pb:?},{pc:?}) plane"),
            &plc,
            &plr,
        );
        assert_eq!(plc[3], SENTINEL, "plane[3] must not be written on failure");
    }

    // and the qtrue side, so both possible return values are pinned
    let good = ([0.0f32, 0.0, 0.0], [1.0f32, 0.0, 0.0], [0.0f32, 1.0, 0.0]);
    let mut plc = [SENTINEL; 4];
    let mut plr = [SENTINEL; 4];
    let vc = unsafe { c(plc.as_mut_ptr(), good.0.as_ptr(), good.1.as_ptr(), good.2.as_ptr()) };
    let vr = unsafe { r(plr.as_mut_ptr(), good.0.as_ptr(), good.1.as_ptr(), good.2.as_ptr()) };
    assert_int("PlaneFromPoints(unit triangle) return", vc, vr);
    assert_eq!(vc, 1, "a valid triangle must return qtrue");
    assert_vec("PlaneFromPoints(unit triangle) plane", &plc, &plr);
    assert_ne!(plc[3], SENTINEL, "plane[3] is written on success");
}

// ---------------------------------------------------------------------------
// rows 14, 15, 16 -- the two normalizers when length == 0 / NaN
// ---------------------------------------------------------------------------

#[test]
fn vector_normalize_zero() {
    type F = unsafe extern "C" fn(*mut f32) -> f32;
    let (c, r): (F, F) = both("VectorNormalize");

    // {1e-30,..} squares to 0 in f32, so `length` becomes exactly 0 and the
    // vector is left ALONE (unlike VectorNormalize2, which clears it).
    let cases: &[[f32; 3]] = &[
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [0.0, -0.0, 0.0],
        [1e-30, 0.0, 0.0],
        [1e-30, -1e-30, 1e-30],
        [1e-45, 1e-45, 1e-45],
        [-1e-23, 0.0, 0.0],
    ];
    for v in cases {
        let mut vc = *v;
        let mut vr = *v;
        let lc = unsafe { c(vc.as_mut_ptr()) };
        let lr = unsafe { r(vr.as_mut_ptr()) };
        assert_f32(&format!("VectorNormalize({v:?}) length"), lc, lr);
        assert_vec(&format!("VectorNormalize({v:?}) vector"), &vc, &vr);
        if lc == 0.0 {
            assert_eq!(vc, *v, "v must be left untouched when length == 0");
        }
    }
}

#[test]
fn vector_normalize2_zero() {
    type F = unsafe extern "C" fn(*const f32, *mut f32) -> f32;
    let (c, r): (F, F) = both("VectorNormalize2");

    let cases: &[[f32; 3]] = &[
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [1e-30, 0.0, 0.0],
        [1e-30, -1e-30, 1e-30],
        [1e-45, 1e-45, 1e-45],
    ];
    for v in cases {
        let mut oc = [SENTINEL; 3];
        let mut or_ = [SENTINEL; 3];
        let lc = unsafe { c(v.as_ptr(), oc.as_mut_ptr()) };
        let lr = unsafe { r(v.as_ptr(), or_.as_mut_ptr()) };
        assert_f32(&format!("VectorNormalize2({v:?}) length"), lc, lr);
        assert_vec(&format!("VectorNormalize2({v:?}) out"), &oc, &or_);
        assert_eq!(lc, 0.0);
        assert_eq!(oc, [0.0, 0.0, 0.0], "out is cleared when length == 0");
    }
}

/// `if ( length )` compiles to `ucomiss` + `jp`, so a NaN length takes the
/// *true* branch and the division is performed.
#[test]
fn vector_normalize_nan_length() {
    type F1 = unsafe extern "C" fn(*mut f32) -> f32;
    type F2 = unsafe extern "C" fn(*const f32, *mut f32) -> f32;
    let (c1, r1): (F1, F1) = both("VectorNormalize");
    let (c2, r2): (F2, F2) = both("VectorNormalize2");

    let cases: &[[f32; 3]] = &[
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::NAN, 0.0],
        [0.0, 0.0, f32::NAN],
        [f32::INFINITY, 0.0, 0.0],
        [f32::NEG_INFINITY, 1.0, 2.0],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0], // inf*inf + (-inf*-inf) = inf
        [1e30, 1e30, 1e30],                      // squares overflow to inf
    ];
    for v in cases {
        let mut vc = *v;
        let mut vr = *v;
        let lc = unsafe { c1(vc.as_mut_ptr()) };
        let lr = unsafe { r1(vr.as_mut_ptr()) };
        check_f32(&format!("VectorNormalize({v:?}) length"), v, lc, lr);
        check_vec(&format!("VectorNormalize({v:?}) vector"), v, &vc, &vr);

        let mut oc = [SENTINEL; 3];
        let mut or_ = [SENTINEL; 3];
        let lc = unsafe { c2(v.as_ptr(), oc.as_mut_ptr()) };
        let lr = unsafe { r2(v.as_ptr(), or_.as_mut_ptr()) };
        check_f32(&format!("VectorNormalize2({v:?}) length"), v, lc, lr);
        check_vec(&format!("VectorNormalize2({v:?}) out"), v, &oc, &or_);
        // `if (length)` is ucomiss+jp, so a NaN length takes the TRUE branch:
        // the components get multiplied by 1/NaN = NaN instead of being cleared.
        // An infinite length also takes the true branch, but there 1/inf == 0, so
        // the result is a zero vector that looks like the cleared one.
        if lc.is_nan() {
            assert!(
                oc.iter().all(|x| x.is_nan()),
                "a NaN length must divide, not clear: {oc:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 23 -- SetPlaneSignbits with -0.0 / NaN
// ---------------------------------------------------------------------------

/// `if (out->normal[j] < 0)` -- neither `-0.0` nor NaN is `< 0`.
#[test]
fn set_plane_signbits_edge() {
    type F = unsafe extern "C" fn(*mut CPlane) -> ();
    let (c, r): (F, F) = both("SetPlaneSignbits");

    let normals: &[[f32; 3]] = &[
        [-0.0, -0.0, -0.0],
        [0.0, 0.0, 0.0],
        [-0.0, -1.0, 0.0],
        [f32::NAN, f32::NAN, f32::NAN],
        [f32::NAN, -1.0, -0.0],
        [-1.0, -1.0, -1.0],
        [f32::NEG_INFINITY, 0.0, f32::INFINITY],
        [-1e-45, 0.0, 0.0], // negative denormal IS < 0
    ];
    for n in normals {
        let mk = || CPlane {
            normal: *n,
            dist: 7.5,
            type_: 3,
            signbits: 0xff, // must be overwritten
            pad: [0xaa, 0xbb],
        };
        let mut pc = mk();
        let mut pr = mk();
        unsafe { c(&mut pc) };
        unsafe { r(&mut pr) };
        // raw-byte comparison: a NaN normal would make `==` false by itself
        let bytes = |p: &CPlane| -> [u8; 20] {
            unsafe { std::mem::transmute_copy::<CPlane, [u8; 20]>(p) }
        };
        assert_eq!(
            bytes(&pc),
            bytes(&pr),
            "SetPlaneSignbits(normal={n:?}) struct bytes: C {pc:?} vs Rust {pr:?}"
        );
        let expect = (if n[0] < 0.0 { 1 } else { 0 })
            | (if n[1] < 0.0 { 2 } else { 0 })
            | (if n[2] < 0.0 { 4 } else { 0 });
        assert_eq!(pc.signbits, expect, "signbits for {n:?}");
    }
}

// ---------------------------------------------------------------------------
// rows 24, 25 -- the (int) overflow inside AngleMod / AngleNormalize360/180
// ---------------------------------------------------------------------------

/// `(int)(angle * (65536/360.0))` is a `cvttsd2si`: NaN and every value outside
/// `INT_MIN..INT_MAX` become `0x80000000`, and `& 65535` then yields `0`.
#[test]
fn angle_normalize_int_overflow() {
    type F = unsafe extern "C" fn(f32) -> f32;
    let (mc, mr): (F, F) = both("AngleMod");
    let (c360, r360): (F, F) = both("AngleNormalize360");
    let (c180, r180): (F, F) = both("AngleNormalize180");

    // 2^31 / (65536/360) = 11796480.0 exactly
    let cases: &[f32] = &[
        11796480.0,
        11796481.0,
        11796482.0,
        -11796480.0,
        -11796481.0,
        11796479.0,
        1e30,
        -1e30,
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        1e8,
        -1e8,
    ];
    for &v in cases {
        assert_f32(&format!("AngleMod({v:?})"), unsafe { mc(v) }, unsafe { mr(v) });
        assert_f32(
            &format!("AngleNormalize360({v:?})"),
            unsafe { c360(v) },
            unsafe { r360(v) },
        );
        assert_f32(
            &format!("AngleNormalize180({v:?})"),
            unsafe { c180(v) },
            unsafe { r180(v) },
        );
    }
    // the documented sentinel: the indefinite integer masks down to 0
    for &v in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e30, -1e30, 11796480.0] {
        assert_eq!(unsafe { c360(v) }, 0.0, "AngleNormalize360({v:?})");
        assert_eq!(unsafe { r360(v) }, 0.0, "AngleNormalize360({v:?})");
    }
    // ... and the >180 branch of AngleNormalize180 really subtracts 360
    let v = 270.0f32;
    assert!(unsafe { c180(v) } < 0.0);
    assert_f32("AngleNormalize180(270)", unsafe { c180(v) }, unsafe { r180(v) });
}

// ---------------------------------------------------------------------------
// rows 28, 29 -- AngleSubtract
// ---------------------------------------------------------------------------

/// `AngleSubtract(a1,a2)` runs `while (a > 180) a -= 360;`.  For |a| >= 2^32 the
/// subtraction cannot change `a` in `f32`, and for ±inf it never can either, so
/// the C function never returns.  The Rust translation contains the same loop, so
/// it hangs identically -- there is nothing to compare, and calling either one
/// would hang the test runner.  This test documents the boundary and verifies the
/// largest magnitude that still terminates.
#[test]
fn angle_subtract_hangs_doc() {
    type F = unsafe extern "C" fn(f32, f32) -> f32;
    let (c, r): (F, F) = both("AngleSubtract");

    // 2^25 = 33554432: ulp is 4, so `a -= 360` still makes progress (~93k
    // iterations).  From 2^33 on, half an ulp (512) exceeds 360, so `a - 360`
    // rounds back to `a` and BOTH implementations spin forever.
    for &v in &[33554432.0f32, -33554432.0, 1e7, -1e7, 181.0, -181.0] {
        assert_f32(
            &format!("AngleSubtract({v:?},0)"),
            unsafe { c(v, 0.0) },
            unsafe { r(v, 0.0) },
        );
    }
    // proof that the loop condition is what stops progress: from 2^33 upwards
    // the subtraction is a no-op in f32, which is why neither side is called
    // with such a value.
    let a: f32 = 17179869184.0; // 2^34, ulp = 2048 > 2*360
    assert_eq!(a - 360.0, a, "a -= 360 is a no-op at 2^34 -> infinite loop");
    let b: f32 = 4294967296.0; // 2^32, ulp = 512 -> still makes progress
    assert_ne!(b - 360.0, b);
    let c: f32 = 8589935616.0; // 2^33 + 1 ulp, ulp = 1024 -> already stuck
    assert_eq!(c - 360.0, c);
    assert_eq!(
        f32::INFINITY - 360.0,
        f32::INFINITY,
        "inf never leaves the loop"
    );
}

/// A NaN difference fails both `while` conditions, so the NaN is returned as is.
#[test]
fn angle_subtract_nan() {
    type F = unsafe extern "C" fn(f32, f32) -> f32;
    let (c, r): (F, F) = both("AngleSubtract");
    for (a1, a2) in [
        (f32::NAN, 0.0f32),
        (0.0, f32::NAN),
        (f32::NAN, f32::NAN),
        (f32::INFINITY, f32::INFINITY),
        (f32::NEG_INFINITY, f32::NEG_INFINITY),
    ] {
        let (vc, vr) = (unsafe { c(a1, a2) }, unsafe { r(a1, a2) });
        check_f32(&format!("AngleSubtract({a1:?},{a2:?})"), &[a1, a2], vc, vr);
        assert!(vc.is_nan(), "the NaN is returned unchanged");
    }
}

// ---------------------------------------------------------------------------
// rows 30, 31 -- Q_log2
// ---------------------------------------------------------------------------

/// `while ( ( val>>=1 ) != 0 )` with a negative `val`: `>>` is an arithmetic
/// shift, so `val` reaches `-1` and stays there -- the C loop never exits, and
/// the Rust translation uses the same arithmetic shift, so it hangs identically.
/// Neither is called here; the shift behaviour is asserted instead.
#[test]
fn q_log2_negative_hangs_doc() {
    assert_eq!(-1i32 >> 1, -1, "arithmetic shift keeps -1 forever");
    assert_eq!(i32::MIN >> 1, -1073741824);
    let mut v = i32::MIN;
    for _ in 0..64 {
        v >>= 1;
    }
    assert_eq!(v, -1, "any negative input converges to -1, never to 0");
}

#[test]
fn q_log2_zero() {
    type F = unsafe extern "C" fn(c_int) -> c_int;
    let (c, r): (F, F) = both("Q_log2");
    for v in [0, 1, 2, 3, i32::MAX] {
        let (vc, vr) = (unsafe { c(v) }, unsafe { r(v) });
        assert_int(&format!("Q_log2({v})"), vc, vr);
    }
    assert_eq!(unsafe { c(0) }, 0, "Q_log2(0) == 0");
    assert_eq!(unsafe { c(1) }, 0, "Q_log2(1) == 0");
    assert_eq!(unsafe { c(i32::MAX) }, 30);
}

// ---------------------------------------------------------------------------
// row 32 -- Q_rand signed overflow
// ---------------------------------------------------------------------------

/// `*seed = (69069 * *seed + 1)` overflows a signed int (UB in C, wraps with
/// gcc).  Seeds are chosen so that the very first multiplication overflows.
#[test]
fn q_rand_overflow() {
    type F = unsafe extern "C" fn(*mut c_int) -> c_int;
    let (c, r): (F, F) = both("Q_rand");
    for s in [
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x7fff_ffff,
        1 << 30,
        -(1 << 30),
        123456789,
        -123456789,
    ] {
        let (mut sc, mut sr) = (s, s);
        for step in 0..50 {
            let vc = unsafe { c(&mut sc) };
            let vr = unsafe { r(&mut sr) };
            assert_int(&format!("Q_rand({s}) step {step}"), vc, vr);
            assert_int(&format!("Q_rand({s}) step {step} seed"), sc, sr);
        }
        // the wrap really is modulo 2^32
        let mut probe = s;
        let got = unsafe { c(&mut probe) };
        assert_eq!(got, 69069i32.wrapping_mul(s).wrapping_add(1));
    }
}

// ---------------------------------------------------------------------------
// row 33 -- Q_rsqrt has no guards at all
// ---------------------------------------------------------------------------

#[test]
fn q_rsqrt_special() {
    type F = unsafe extern "C" fn(f32) -> f32;
    let (c, r): (F, F) = both("Q_rsqrt");
    for &v in &[
        0.0f32,
        -0.0,
        -1.0,
        -1e30,
        -f32::MAX,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-45,
        -1e-45,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::MAX,
    ] {
        assert_f32(&format!("Q_rsqrt({v:?})"), unsafe { c(v) }, unsafe { r(v) });
    }
    // 0x5f3759df - (0 >> 1) = 0x5f3759df -> 1.32e19, then one Newton step
    assert_f32(
        "Q_rsqrt(0.0) is the raw magic value, not an error",
        unsafe { c(0.0) },
        unsafe { r(0.0) },
    );
    assert!(unsafe { c(0.0) } > 1.9e19, "no zero guard exists");
}

// ---------------------------------------------------------------------------
// row 35 -- division by a zero denominator
// ---------------------------------------------------------------------------

#[test]
fn project_point_on_plane_zero_normal() {
    type F = unsafe extern "C" fn(*mut f32, *const f32, *const f32) -> ();
    let (c, r): (F, F) = both("ProjectPointOnPlane");

    let normals: &[[f32; 3]] = &[
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [1e-30, 0.0, 0.0], // squares underflow -> DotProduct == 0
        [f32::NAN, 0.0, 0.0],
        [f32::INFINITY, 0.0, 0.0],
        [1e30, 1e30, 1e30], // squares overflow -> inf -> 1/inf == 0
    ];
    let points: &[[f32; 3]] = &[[0.0, 0.0, 0.0], [1.0, 2.0, 3.0], [-1e30, 1.0, 0.0]];
    for n in normals {
        for p in points {
            let mut oc = [SENTINEL; 3];
            let mut or_ = [SENTINEL; 3];
            unsafe { c(oc.as_mut_ptr(), p.as_ptr(), n.as_ptr()) };
            unsafe { r(or_.as_mut_ptr(), p.as_ptr(), n.as_ptr()) };
            let mut inputs = p.to_vec();
            inputs.extend_from_slice(n);
            check_vec(
                &format!("ProjectPointOnPlane(p={p:?}, normal={n:?})"),
                &inputs,
                &oc,
                &or_,
            );
        }
    }
}

#[test]
fn perpendicular_vector_zero() {
    type F = unsafe extern "C" fn(*mut f32, *const f32) -> ();
    let (c, r): (F, F) = both("PerpendicularVector");

    let srcs: &[[f32; 3]] = &[
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [1e-30, 1e-30, 1e-30],
        [f32::NAN, f32::NAN, f32::NAN],
        [f32::INFINITY, f32::INFINITY, f32::INFINITY],
        [1.0, 1.0, 1.0], // all |x| >= minelem -> pos stays 0
        [2.0, 3.0, 4.0],
        [1.0, 0.5, 0.25], // min at index 2
        [0.25, 1.0, 0.5], // min at index 0
    ];
    for s in srcs {
        let mut oc = [SENTINEL; 3];
        let mut or_ = [SENTINEL; 3];
        unsafe { c(oc.as_mut_ptr(), s.as_ptr()) };
        unsafe { r(or_.as_mut_ptr(), s.as_ptr()) };
        check_vec(&format!("PerpendicularVector(src={s:?})"), s, &oc, &or_);
    }
}

// ---------------------------------------------------------------------------
// rows 36..39 -- AngleVectors' three optional outputs
// ---------------------------------------------------------------------------

#[test]
fn angle_vectors_null_outputs() {
    // the C AngleVectors keeps its sines/cosines in `static float`s, so only one
    // thread at a time may be inside it (see harness::angle_vectors_guard)
    let _guard = angle_vectors_guard();
    type F = unsafe extern "C" fn(*const f32, *mut f32, *mut f32, *mut f32) -> ();
    let (c, r): (F, F) = both("AngleVectors");

    let angle_sets: &[[f32; 3]] = &[
        [0.0, 0.0, 0.0],
        [30.0, 40.0, 50.0],
        [-90.0, 180.0, 270.0],
        [f32::NAN, 1.0, 2.0],
        [1e30, -1e30, 0.0],
    ];
    for angles in angle_sets {
        for mask in 0..8u32 {
            let mut fc = [SENTINEL; 3];
            let mut rc = [SENTINEL; 3];
            let mut uc = [SENTINEL; 3];
            let mut fr = [SENTINEL; 3];
            let mut rr = [SENTINEL; 3];
            let mut ur = [SENTINEL; 3];
            let p = |on: bool, a: &mut [f32; 3]| -> *mut f32 {
                if on {
                    a.as_mut_ptr()
                } else {
                    std::ptr::null_mut()
                }
            };
            unsafe {
                c(
                    angles.as_ptr(),
                    p(mask & 1 != 0, &mut fc),
                    p(mask & 2 != 0, &mut rc),
                    p(mask & 4 != 0, &mut uc),
                );
                r(
                    angles.as_ptr(),
                    p(mask & 1 != 0, &mut fr),
                    p(mask & 2 != 0, &mut rr),
                    p(mask & 4 != 0, &mut ur),
                );
            }
            let ctx = format!("AngleVectors({angles:?}, mask={mask:03b})");
            check_vec(&format!("{ctx} forward"), angles, &fc, &fr);
            check_vec(&format!("{ctx} right"), angles, &rc, &rr);
            check_vec(&format!("{ctx} up"), angles, &uc, &ur);
            // a NULL output must be left completely untouched
            if mask & 1 == 0 {
                assert_eq!(fc, [SENTINEL; 3], "{ctx}: forward must not be written");
            }
            if mask & 2 == 0 {
                assert_eq!(rc, [SENTINEL; 3], "{ctx}: right must not be written");
            }
            if mask & 4 == 0 {
                assert_eq!(uc, [SENTINEL; 3], "{ctx}: up must not be written");
            }
            if mask != 0 {
                // ... and a non-NULL one must be
                if mask & 1 != 0 && !angles.iter().any(|v| v.is_nan()) {
                    assert_ne!(fc, [SENTINEL; 3], "{ctx}: forward must be written");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 40 -- RotateAroundDirection with yaw == 0
// ---------------------------------------------------------------------------

#[test]
fn rotate_around_direction_zero_yaw() {
    type F = unsafe extern "C" fn(*mut [f32; 3], f32) -> ();
    let (c, r): (F, F) = both("RotateAroundDirection");

    let dirs: &[[f32; 3]] = &[
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.577, 0.577, 0.577],
    ];
    for d in dirs {
        for &yaw in &[0.0f32, -0.0, 1e-45, 90.0] {
            let mk = || [*d, [SENTINEL; 3], [SENTINEL; 3]];
            let mut ac = mk();
            let mut ar = mk();
            unsafe { c(ac.as_mut_ptr(), yaw) };
            unsafe { r(ar.as_mut_ptr(), yaw) };
            for i in 0..3 {
                check_vec(
                    &format!("RotateAroundDirection({d:?}, yaw={yaw:?}) axis[{i}]"),
                    d,
                    &ac[i],
                    &ar[i],
                );
            }
        }
        // yaw == 0 and yaw == -0 must give the identical (un-rotated) result
        let mut a0 = [*d, [SENTINEL; 3], [SENTINEL; 3]];
        let mut am0 = [*d, [SENTINEL; 3], [SENTINEL; 3]];
        unsafe { c(a0.as_mut_ptr(), 0.0) };
        unsafe { c(am0.as_mut_ptr(), -0.0) };
        for i in 0..3 {
            check_vec(&format!("yaw 0 vs -0 axis[{i}]"), d, &a0[i], &am0[i]);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 41..44 -- vectoangles' degenerate branches
// ---------------------------------------------------------------------------

#[test]
fn vectoangles_degenerate() {
    type F = unsafe extern "C" fn(*const f32, *mut f32) -> ();
    let (c, r): (F, F) = both("vectoangles");

    let cases: &[[f32; 3]] = &[
        // value1[0] == 0 && value1[1] == 0  ->  yaw = 0, pitch = 90 or 270
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, 0.0],   // z == +0 is NOT > 0 -> pitch = 270
        [0.0, 0.0, -0.0],  // ditto
        [-0.0, 0.0, 1.0],  // -0.0 == 0 is true
        [0.0, -0.0, -1.0],
        [-0.0, -0.0, 1e-45],
        // value1[0] == 0, value1[1] != 0  ->  yaw = 90 or 270
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [-0.0, 1e-45, 5.0],
        [0.0, -1e-45, -5.0],
        [0.0, f32::INFINITY, 1.0],
        [0.0, f32::NEG_INFINITY, 1.0],
        // the atan2 path, all four quadrants, plus the yaw<0 / pitch<0 fix-ups
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, -1.0, -1.0],
        [-1.0, -1.0, -1.0],
        [1e30, 1e30, 1e30],
        [1e-30, 1e-30, 1e-30],
        [f32::INFINITY, 1.0, 1.0],
        [f32::NEG_INFINITY, -1.0, -1.0],
    ];
    for v in cases {
        let mut oc = [SENTINEL; 3];
        let mut or_ = [SENTINEL; 3];
        unsafe { c(v.as_ptr(), oc.as_mut_ptr()) };
        unsafe { r(v.as_ptr(), or_.as_mut_ptr()) };
        check_vec(&format!("vectoangles({v:?})"), v, &oc, &or_);
        assert_eq!(oc[2], 0.0, "angles[ROLL] is always 0");
    }
    // the documented sentinels of the degenerate branch
    let mut o = [SENTINEL; 3];
    unsafe { c([0.0f32, 0.0, 1.0].as_ptr(), o.as_mut_ptr()) };
    assert_eq!(o, [-90.0, 0.0, 0.0], "z > 0 -> pitch 90 -> angles = -90");
    unsafe { c([0.0f32, 0.0, 0.0].as_ptr(), o.as_mut_ptr()) };
    assert_eq!(o, [-270.0, 0.0, 0.0], "z == +0 -> pitch 270");
}

#[test]
fn vectoangles_nan() {
    type F = unsafe extern "C" fn(*const f32, *mut f32) -> ();
    let (c, r): (F, F) = both("vectoangles");
    let cases: &[[f32; 3]] = &[
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::NAN, 0.0],
        [0.0, 0.0, f32::NAN],
        [f32::NAN, f32::NAN, f32::NAN],
        [f32::NAN, 1.0, 2.0],
        [1.0, f32::NAN, 2.0],
        [1.0, 2.0, f32::NAN],
    ];
    for v in cases {
        let mut oc = [SENTINEL; 3];
        let mut or_ = [SENTINEL; 3];
        unsafe { c(v.as_ptr(), oc.as_mut_ptr()) };
        unsafe { r(v.as_ptr(), or_.as_mut_ptr()) };
        check_vec(&format!("vectoangles({v:?})"), v, &oc, &or_);
    }
}

// ---------------------------------------------------------------------------
// rows 46, 47 -- NaN in the bounds helpers
// ---------------------------------------------------------------------------

/// `corner[i] = a > b ? a : b` with `a = fabs(mins[i])`: a NaN `a` makes the
/// comparison false, so `b` wins; a NaN `b` wins by being selected.
#[test]
fn radius_from_bounds_nan() {
    type F = unsafe extern "C" fn(*const f32, *const f32) -> f32;
    let (c, r): (F, F) = both("RadiusFromBounds");
    let cases: &[([f32; 3], [f32; 3])] = &[
        ([f32::NAN, 0.0, 0.0], [1.0, 2.0, 3.0]),
        ([1.0, 2.0, 3.0], [f32::NAN, 0.0, 0.0]),
        ([f32::NAN; 3], [f32::NAN; 3]),
        ([f32::INFINITY, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([-0.0, -0.0, -0.0], [0.0, 0.0, 0.0]),
        ([1e30, 1e30, 1e30], [-1e30, -1e30, -1e30]),
    ];
    for (mins, maxs) in cases {
        let mut inputs = mins.to_vec();
        inputs.extend_from_slice(maxs);
        let vc = unsafe { c(mins.as_ptr(), maxs.as_ptr()) };
        let vr = unsafe { r(mins.as_ptr(), maxs.as_ptr()) };
        check_f32(
            &format!("RadiusFromBounds({mins:?},{maxs:?})"),
            &inputs,
            vc,
            vr,
        );
    }
}

/// Both `<` and `>` are false for a NaN point, so the bounds stay put.
#[test]
fn add_point_to_bounds_nan() {
    type F = unsafe extern "C" fn(*const f32, *mut f32, *mut f32) -> ();
    let (c, r): (F, F) = both("AddPointToBounds");

    let pts: &[[f32; 3]] = &[
        [f32::NAN, f32::NAN, f32::NAN],
        [f32::NAN, 1.0, -1.0],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [-0.0, 0.0, -0.0],
    ];
    for p in pts {
        let mut minc = [99999.0f32; 3];
        let mut maxc = [-99999.0f32; 3];
        let mut minr = [99999.0f32; 3];
        let mut maxr = [-99999.0f32; 3];
        unsafe { c(p.as_ptr(), minc.as_mut_ptr(), maxc.as_mut_ptr()) };
        unsafe { r(p.as_ptr(), minr.as_mut_ptr(), maxr.as_mut_ptr()) };
        check_vec(&format!("AddPointToBounds({p:?}) mins"), p, &minc, &minr);
        check_vec(&format!("AddPointToBounds({p:?}) maxs"), p, &maxc, &maxr);
        if p.iter().all(|v| v.is_nan()) {
            assert_eq!(minc, [99999.0; 3], "NaN point must not move mins");
            assert_eq!(maxc, [-99999.0; 3], "NaN point must not move maxs");
        }
    }
}

// ---------------------------------------------------------------------------
// row 50 -- unchecked NULL dereferences must fail the same way
// ---------------------------------------------------------------------------

/// Every pointer parameter except `DirToByte`'s and `AngleVectors`' outputs is
/// dereferenced without a check, so passing NULL is a segmentation fault in both
/// implementations.  That cannot be observed in-process, so each case runs in a
/// re-executed child process and the fatal signal is compared.
///
/// The C library always dies with `SIGSEGV`.  The Rust library does too when it
/// is built with `-Cdebug-assertions=off` (`cargo test --release`), which is the
/// apples-to-apples comparison, because gcc emits no pointer checks either.  In
/// the dev/test profile rustc's `ub_checks` notice the null read *before* the
/// load in some of these functions and turn it into a non-unwinding panic
/// (`SIGABRT`); both signals are accepted in that profile only.
#[test]
fn null_pointer_crashes_match() {
    // child mode: perform one NULL call and die
    if let Ok(spec) = std::env::var("NULL_CRASH_CASE") {
        let (case, which) = spec.split_once(':').expect("case:which");
        run_null_case(case, which);
        // if we get here the call did NOT crash
        std::process::exit(77);
    }

    const SIGSEGV: i32 = 11;
    const SIGABRT: i32 = 6;
    let cases = [
        "VectorNormalize",
        "_DotProduct",
        "ByteToDir",
        "SetPlaneSignbits",
        "BoxOnPlaneSide",
        "MatrixMultiply",
        "vectoangles",
        "AxisClear",
    ];
    for case in cases {
        let cs = spawn_null_case(case, "c");
        let rs = spawn_null_case(case, "rust");
        assert_eq!(
            cs,
            (None, Some(SIGSEGV)),
            "{case}(NULL) must raise SIGSEGV in the C library"
        );
        // A Rust build with -Cdebug-assertions=on (the dev/test profile) checks
        // the pointer *before* the load and turns the same UB into a
        // non-unwinding panic, i.e. SIGABRT; with debug assertions off it is the
        // very same SIGSEGV as C.  Both are "dies from a fatal signal at the
        // same call", which is all the C standard leaves defined here.
        if cfg!(debug_assertions) {
            assert!(
                rs == (None, Some(SIGSEGV)) || rs == (None, Some(SIGABRT)),
                "{case}(NULL): Rust exited with {rs:?}, C with {cs:?}"
            );
        } else {
            // with debug assertions off there is no pointer check left and the
            // Rust library faults exactly like the C one
            assert_eq!(
                rs,
                (None, Some(SIGSEGV)),
                "{case}(NULL): Rust exited with {rs:?}, C with {cs:?}"
            );
        }
    }
}

/// `(exit code, signal)` of a child that performs one NULL call.
fn spawn_null_case(case: &str, which: &str) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", "null_pointer_crashes_match"])
        .env("NULL_CRASH_CASE", format!("{case}:{which}"))
        .output()
        .expect("spawn child");
    (out.status.code(), out.status.signal())
}

fn run_null_case(case: &str, which: &str) {
    let n = std::ptr::null_mut::<f32>();
    let pick = |name: &str| -> usize {
        if which == "c" {
            cfn::<usize>(name)
        } else {
            rfn::<usize>(name)
        }
    };
    unsafe {
        match case {
            "VectorNormalize" => {
                let f: unsafe extern "C" fn(*mut f32) -> f32 =
                    std::mem::transmute(pick("VectorNormalize"));
                std::hint::black_box(f(n));
            }
            "_DotProduct" => {
                let f: unsafe extern "C" fn(*const f32, *const f32) -> f32 =
                    std::mem::transmute(pick("_DotProduct"));
                std::hint::black_box(f(n, n));
            }
            "ByteToDir" => {
                let f: unsafe extern "C" fn(c_int, *mut f32) = std::mem::transmute(pick("ByteToDir"));
                f(0, n);
            }
            "SetPlaneSignbits" => {
                let f: unsafe extern "C" fn(*mut CPlane) =
                    std::mem::transmute(pick("SetPlaneSignbits"));
                f(std::ptr::null_mut());
            }
            "BoxOnPlaneSide" => {
                let f: unsafe extern "C" fn(*mut f32, *mut f32, *mut CPlane) -> c_int =
                    std::mem::transmute(pick("BoxOnPlaneSide"));
                std::hint::black_box(f(n, n, std::ptr::null_mut()));
            }
            "MatrixMultiply" => {
                let f: unsafe extern "C" fn(*mut [f32; 3], *mut [f32; 3], *mut [f32; 3]) =
                    std::mem::transmute(pick("MatrixMultiply"));
                f(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            "vectoangles" => {
                let f: unsafe extern "C" fn(*const f32, *mut f32) =
                    std::mem::transmute(pick("vectoangles"));
                f(n, n);
            }
            "AxisClear" => {
                let f: unsafe extern "C" fn(*mut [f32; 3]) = std::mem::transmute(pick("AxisClear"));
                f(std::ptr::null_mut());
            }
            other => panic!("unknown case {other}"),
        }
    }
}
