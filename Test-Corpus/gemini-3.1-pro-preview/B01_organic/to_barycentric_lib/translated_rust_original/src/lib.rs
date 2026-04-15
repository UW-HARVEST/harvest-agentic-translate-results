#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

impl lm_vec2 {
    #[inline]
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y)
    }

    #[inline]
    fn dot(self, b: Self) -> f32 {
        self.x * b.x + self.y * b.y
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn to_barycentric(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0 = p3.sub(p1);
    let v1 = p2.sub(p1);
    let v2 = p.sub(p1);
    let dot00 = v0.dot(v0);
    let dot01 = v0.dot(v1);
    let dot02 = v0.dot(v2);
    let dot11 = v1.dot(v1);
    let dot12 = v1.dot(v2);
    let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    lm_vec2::new(u, v)
}
