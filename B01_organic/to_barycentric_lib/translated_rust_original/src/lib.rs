#[repr(C)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

#[unsafe(no_mangle)]
pub extern "C" fn to_barycentric(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0x = p3.x - p1.x;
    let v0y = p3.y - p1.y;
    let v1x = p2.x - p1.x;
    let v1y = p2.y - p1.y;
    let v2x = p.x - p1.x;
    let v2y = p.y - p1.y;
    let dot00 = v0x * v0x + v0y * v0y;
    let dot01 = v0x * v1x + v0y * v1y;
    let dot02 = v0x * v2x + v0y * v2y;
    let dot11 = v1x * v1x + v1y * v1y;
    let dot12 = v1x * v2x + v1y * v2y;
    let inv_denom = 1.0f32 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    lm_vec2 { x: u, y: v }
}
