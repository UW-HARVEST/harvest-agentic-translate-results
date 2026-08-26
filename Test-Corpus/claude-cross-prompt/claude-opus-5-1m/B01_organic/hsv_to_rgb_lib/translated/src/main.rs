#[allow(dead_code)]
fn hsv_to_rgb(dest: &mut [f32], src: &[f32]) {
    let mut h: f32 = src[0];
    let s: f32 = src[1];
    let v: f32 = src[2];

    if s == 0.0 {
        dest[0] = v;
        dest[1] = v;
        dest[2] = v;
        return;
    }

    h /= 60.0f32;
    let i: i32 = h.floor() as i32;
    let f: f32 = h - (i as f32);
    let p: f32 = v * (1.0 - s);
    let q: f32 = v * (1.0 - s * f);
    let t: f32 = v * (1.0 - s * (1.0 - f));

    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    dest[0] = r;
    dest[1] = g;
    dest[2] = b;
}

fn main() {
    // The original C source defines only the `hsv_to_rgb` library function
    // with no `main` and performs no I/O. Reproduce that behavior exactly:
    // produce no output for any input.
}
