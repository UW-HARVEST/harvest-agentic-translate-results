#[derive(Copy, Clone)]
#[repr(C)]
pub struct lm_vec2 {
    pub x: libc::c_float,
    pub y: libc::c_float,
}
 extern "C" fn lm_v2(mut x: libc::c_float, mut y: libc::c_float) -> lm_vec2 {
    let mut v: lm_vec2 = lm_vec2 { x: x, y: y };
    return v;
}
 extern "C" fn lm_sub2(mut a: lm_vec2, mut b: lm_vec2) -> lm_vec2 {
    return lm_v2(a.x - b.x, a.y - b.y);
}
 extern "C" fn lm_dot2(mut a: lm_vec2, mut b: lm_vec2) -> libc::c_float {
    return a.x * b.x + a.y * b.y;
}
#[no_mangle]
pub extern "C" fn to_barycentric(
    mut p1: lm_vec2,
    mut p2: lm_vec2,
    mut p3: lm_vec2,
    mut p: lm_vec2,
) -> lm_vec2 {
    let mut v0: lm_vec2 = lm_sub2(p3, p1);
    let mut v1: lm_vec2 = lm_sub2(p2, p1);
    let mut v2: lm_vec2 = lm_sub2(p, p1);
    let mut dot00: libc::c_float = lm_dot2(v0, v0);
    let mut dot01: libc::c_float = lm_dot2(v0, v1);
    let mut dot02: libc::c_float = lm_dot2(v0, v2);
    let mut dot11: libc::c_float = lm_dot2(v1, v1);
    let mut dot12: libc::c_float = lm_dot2(v1, v2);
    let mut invDenom: libc::c_float = 1.0f32 / (dot00 * dot11 - dot01 * dot01);
    let mut u: libc::c_float = (dot11 * dot02 - dot01 * dot12) * invDenom;
    let mut v: libc::c_float = (dot00 * dot12 - dot01 * dot02) * invDenom;
    return lm_v2(u, v);
}
