//! Exported test hooks for the parts of `c_src/inc/q_shared.h` that have no
//! external linkage in C: the function-like macros (`DotProduct`,
//! `VectorSubtract`, `SnapVector`, `ANGLE2SHORT`, ...) and the
//! `static ID_INLINE` functions (`VectorLength`, `CrossProduct`, ...).
//!
//! `tests/csupport/wrappers.c` defines byte-for-byte the same set of `w_*`
//! entry points on top of the real header, which lets the differential test
//! suite compare this translation of the header against the C preprocessor's
//! own expansion of it.  Nothing in the library itself calls these.

#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use crate::q_shared::{self as qs, cplane_t, vec_t};
use core::ffi::c_int;

// --- macros -----------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn w_DotProduct(x: *const vec_t, y: *const vec_t) -> vec_t {
    qs::DotProduct(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorSubtract(a: *const vec_t, b: *const vec_t, c: *mut vec_t) {
    qs::VectorSubtract(a, b, c)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorAdd(a: *const vec_t, b: *const vec_t, c: *mut vec_t) {
    qs::VectorAdd(a, b, c)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorCopy(a: *const vec_t, b: *mut vec_t) {
    qs::VectorCopy(a, b)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorScale(v: *const vec_t, s: vec_t, o: *mut vec_t) {
    qs::VectorScale(v, s, o)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorMA(v: *const vec_t, s: vec_t, b: *const vec_t, o: *mut vec_t) {
    qs::VectorMA(v, s, b, o)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorClear(a: *mut vec_t) {
    qs::VectorClear(a)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorNegate(a: *const vec_t, b: *mut vec_t) {
    qs::VectorNegate(a, b)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorSet(v: *mut vec_t, x: vec_t, y: vec_t, z: vec_t) {
    qs::VectorSet(v, x, y, z)
}

#[no_mangle]
pub unsafe extern "C" fn w_Vector4Copy(a: *const vec_t, b: *mut vec_t) {
    qs::Vector4Copy(a, b)
}

#[no_mangle]
pub unsafe extern "C" fn w_SnapVector(v: *mut vec_t) {
    qs::SnapVector(v)
}

#[no_mangle]
pub extern "C" fn w_IS_NAN(x: vec_t) -> c_int {
    // The C macro evaluates to an `int` (0 or 1) inside the wrapper's `return`.
    qs::IS_NAN(x) as c_int
}

#[no_mangle]
pub extern "C" fn w_SQRTFAST(x: vec_t) -> vec_t {
    qs::SQRTFAST(x)
}

#[no_mangle]
pub extern "C" fn w_DEG2RAD(a: f32) -> f64 {
    qs::DEG2RAD(a)
}

#[no_mangle]
pub extern "C" fn w_RAD2DEG(a: f32) -> f64 {
    qs::RAD2DEG(a)
}

#[no_mangle]
pub extern "C" fn w_ANGLE2SHORT(x: f32) -> c_int {
    qs::ANGLE2SHORT(x)
}

#[no_mangle]
pub extern "C" fn w_SHORT2ANGLE(x: c_int) -> f64 {
    qs::SHORT2ANGLE(x)
}

#[no_mangle]
pub extern "C" fn w_ColorIndex(c: c_int) -> c_int {
    qs::ColorIndex(c)
}

#[no_mangle]
pub extern "C" fn w_Square(x: f32) -> f32 {
    qs::Square(x)
}

#[no_mangle]
pub unsafe extern "C" fn w_PlaneTypeForNormal(x: *const vec_t) -> c_int {
    qs::PlaneTypeForNormal(x)
}

#[no_mangle]
pub unsafe extern "C" fn w_Q_IsColorString(p: *const core::ffi::c_char) -> c_int {
    qs::Q_IsColorString(p)
}

#[no_mangle]
pub extern "C" fn w_random() -> f32 {
    qs::random_()
}

#[no_mangle]
pub extern "C" fn w_crandom() -> f32 {
    qs::crandom_()
}

#[no_mangle]
pub unsafe extern "C" fn w_MAKERGB(v: *mut vec_t, r: vec_t, g: vec_t, b: vec_t) {
    qs::MAKERGB(v, r, g, b)
}

#[no_mangle]
pub unsafe extern "C" fn w_MAKERGBA(v: *mut vec_t, r: vec_t, g: vec_t, b: vec_t, a: vec_t) {
    qs::MAKERGBA(v, r, g, b, a)
}

// --- static ID_INLINE functions --------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn w_VectorCompare(v1: *const vec_t, v2: *const vec_t) -> c_int {
    qs::VectorCompare(v1, v2)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorLength(v: *const vec_t) -> vec_t {
    qs::VectorLength(v)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorLengthSquared(v: *const vec_t) -> vec_t {
    qs::VectorLengthSquared(v)
}

#[no_mangle]
pub unsafe extern "C" fn w_Distance(p1: *const vec_t, p2: *const vec_t) -> vec_t {
    qs::Distance(p1, p2)
}

#[no_mangle]
pub unsafe extern "C" fn w_DistanceSquared(p1: *const vec_t, p2: *const vec_t) -> vec_t {
    qs::DistanceSquared(p1, p2)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorNormalizeFast(v: *mut vec_t) {
    qs::VectorNormalizeFast(v)
}

#[no_mangle]
pub unsafe extern "C" fn w_VectorInverse(v: *mut vec_t) {
    qs::VectorInverse(v)
}

#[no_mangle]
pub unsafe extern "C" fn w_CrossProduct(v1: *const vec_t, v2: *const vec_t, cross: *mut vec_t) {
    qs::CrossProduct(v1, v2, cross)
}

// --- constants and layout --------------------------------------------------

/// Fills `out[0..9]` with, in order: `sizeof(cplane_t)`, `offsetof(normal)`,
/// `offsetof(dist)`, `offsetof(type)`, `offsetof(signbits)`, `offsetof(pad)`,
/// `NUMVERTEXNORMALS`, `nanmask`, `sizeof(vec_t)`.
#[no_mangle]
pub unsafe extern "C" fn w_layout(out: *mut c_int) {
    *out.add(0) = core::mem::size_of::<cplane_t>() as c_int;
    *out.add(1) = core::mem::offset_of!(cplane_t, normal) as c_int;
    *out.add(2) = core::mem::offset_of!(cplane_t, dist) as c_int;
    *out.add(3) = core::mem::offset_of!(cplane_t, type_) as c_int;
    *out.add(4) = core::mem::offset_of!(cplane_t, signbits) as c_int;
    *out.add(5) = core::mem::offset_of!(cplane_t, pad) as c_int;
    *out.add(6) = qs::NUMVERTEXNORMALS as c_int;
    *out.add(7) = qs::nanmask;
    *out.add(8) = core::mem::size_of::<vec_t>() as c_int;
}

/// `PITCH`, `YAW`, `ROLL`, `PLANE_X`, `PLANE_Y`, `PLANE_Z`, `PLANE_NON_AXIAL`,
/// `qfalse`, `qtrue`, `sizeof(qboolean)`.
#[no_mangle]
pub unsafe extern "C" fn w_angle_indexes(out: *mut c_int) {
    *out.add(0) = qs::PITCH as c_int;
    *out.add(1) = qs::YAW as c_int;
    *out.add(2) = qs::ROLL as c_int;
    *out.add(3) = qs::PLANE_X;
    *out.add(4) = qs::PLANE_Y;
    *out.add(5) = qs::PLANE_Z;
    *out.add(6) = qs::PLANE_NON_AXIAL;
    *out.add(7) = qs::qfalse;
    *out.add(8) = qs::qtrue;
    *out.add(9) = core::mem::size_of::<c_int>() as c_int;
}

/// The value of `M_PI` as seen by the compiled code.
#[no_mangle]
pub extern "C" fn w_M_PI() -> f64 {
    qs::M_PI
}
