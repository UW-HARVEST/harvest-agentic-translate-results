#[repr(C)]
#[derive(Clone, Copy)]
pub struct LmVec2 {
    pub x: f32,
    pub y: f32,
}

#[inline(never)]
fn lm_v2(x: f32, y: f32) -> LmVec2 {
    LmVec2 { x, y }
}

#[inline(never)]
fn lm_sub2(a: LmVec2, b: LmVec2) -> LmVec2 {
    lm_v2(sub(a.x, b.x), sub(a.y, b.y))
}

#[inline(never)]
fn add(a: f32, b: f32) -> f32 {
    a + b
}

#[inline(never)]
fn sub(a: f32, b: f32) -> f32 {
    a - b
}

#[inline(never)]
fn mul(a: f32, b: f32) -> f32 {
    a * b
}

#[inline(never)]
fn div(a: f32, b: f32) -> f32 {
    a / b
}

#[unsafe(no_mangle)]
pub extern "C" fn to_barycentric(p1: LmVec2, p2: LmVec2, p3: LmVec2, p: LmVec2) -> LmVec2 {
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);
    // Match the reference instruction operands to preserve NaN payloads and signs.
    let dot00 = add(mul(v0.x, v0.x), mul(v0.y, v0.y));
    let dot01 = add(mul(v1.x, v0.x), mul(v1.y, v0.y));
    let dot02 = add(mul(v0.y, v2.y), mul(v0.x, v2.x));
    let dot11 = add(mul(v1.x, v1.x), mul(v1.y, v1.y));
    let dot12 = add(mul(v1.x, v2.x), mul(v2.y, v1.y));
    let inv_denom = div(1.0_f32, sub(mul(dot11, dot00), mul(dot01, dot01)));
    let u = mul(sub(mul(dot11, dot02), mul(dot12, dot01)), inv_denom);
    let v = mul(sub(mul(dot12, dot00), mul(dot02, dot01)), inv_denom);
    lm_v2(u, v)
}
