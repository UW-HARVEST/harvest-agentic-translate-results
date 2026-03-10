#[repr(C)]
#[derive(Clone, Copy)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

#[unsafe(no_mangle)]
pub extern "C" fn to_barycentric(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0 = lm_vec2 { x: p3.x - p1.x, y: p3.y - p1.y };
    let v1 = lm_vec2 { x: p2.x - p1.x, y: p2.y - p1.y };
    let v2 = lm_vec2 { x: p.x - p1.x, y: p.y - p1.y };
    let dot00 = v0.x * v0.x + v0.y * v0.y;
    let dot01 = v0.x * v1.x + v0.y * v1.y;
    let dot02 = v0.x * v2.x + v0.y * v2.y;
    let dot11 = v1.x * v1.x + v1.y * v1.y;
    let dot12 = v1.x * v2.x + v1.y * v2.y;
    let inv_denom = 1.0f32 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    lm_vec2 { x: u, y: v }
}
