#[derive(Copy, Clone)]
#[repr(C)]
pub struct lm_vec2 {
    pub x: ::core::ffi::c_float,
    pub y: ::core::ffi::c_float,
}
unsafe extern "C" fn lm_v2(mut x: ::core::ffi::c_float, mut y: ::core::ffi::c_float) -> lm_vec2 {
    let mut v: lm_vec2 = lm_vec2 { x: x, y: y };
    return v;
}
unsafe extern "C" fn lm_sub2(mut a: lm_vec2, mut b: lm_vec2) -> lm_vec2 {
    return lm_v2(a.x - b.x, a.y - b.y);
}
unsafe extern "C" fn lm_dot2(mut a: lm_vec2, mut b: lm_vec2) -> ::core::ffi::c_float {
    return a.x * b.x + a.y * b.y;
}
#[no_mangle]
pub unsafe extern "C" fn to_barycentric(
    mut p1: lm_vec2,
    mut p2: lm_vec2,
    mut p3: lm_vec2,
    mut p: lm_vec2,
) -> lm_vec2 {
    let mut v0: lm_vec2 = lm_sub2(p3, p1);
    let mut v1: lm_vec2 = lm_sub2(p2, p1);
    let mut v2: lm_vec2 = lm_sub2(p, p1);
    let mut dot00: ::core::ffi::c_float = lm_dot2(v0, v0);
    let mut dot01: ::core::ffi::c_float = lm_dot2(v0, v1);
    let mut dot02: ::core::ffi::c_float = lm_dot2(v0, v2);
    let mut dot11: ::core::ffi::c_float = lm_dot2(v1, v1);
    let mut dot12: ::core::ffi::c_float = lm_dot2(v1, v2);
    let mut invDenom: ::core::ffi::c_float = 1.0f32 / (dot00 * dot11 - dot01 * dot01);
    let mut u: ::core::ffi::c_float = (dot11 * dot02 - dot01 * dot12) * invDenom;
    let mut v: ::core::ffi::c_float = (dot00 * dot12 - dot01 * dot02) * invDenom;
    return lm_v2(u, v);
}
