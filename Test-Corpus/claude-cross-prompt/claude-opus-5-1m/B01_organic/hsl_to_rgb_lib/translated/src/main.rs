// Translation of c_src/src/lib.c — preserves all original behavior including
// the apparent bug at the third branch where `h < 120.0 && h < 180.0` is used
// (which simplifies to `h < 120.0`) instead of the likely intended
// `h >= 120.0 && h < 180.0`.
//
// The C source defines only the `hsl_to_rgb` library function — it has no
// `main` and performs no I/O. The byte-identical output for any input is
// therefore empty; this executable mirrors that.

#[allow(dead_code)]
fn hsl_to_rgb(dest: &mut [f32], src: &[f32]) {
    let h: f32 = src[0];
    let s: f32 = src[1];
    let l: f32 = src[2];

    if s == 0.0 {
        dest[0] = l;
        dest[1] = l;
        dest[2] = l;
        return;
    }

    let c: f32 = (1.0f32 - (2.0f32 * l - 1.0f32).abs()) * s;
    let m: f32 = 1.0f32 * (l - 0.5f32 * c);
    // C's fmodf preserves the sign of the dividend; Rust's `%` operator on
    // floats uses the same truncated-division semantics, so it matches fmodf
    // for finite values.
    let x: f32 = c * (1.0f32 - ((h / 60.0f32) % 2.0f32 - 1.0f32).abs());

    if h >= 0.0f32 && h < 60.0f32 {
        dest[0] = c + m;
        dest[1] = x + m;
        dest[2] = m;
    } else if h >= 60.0f32 && h < 120.0f32 {
        dest[0] = x + m;
        dest[1] = c + m;
        dest[2] = m;
    } else if h < 120.0f32 && h < 180.0f32 {
        // Reproducing the original C bug exactly.
        dest[0] = m;
        dest[1] = c + m;
        dest[2] = x + m;
    } else if h >= 180.0f32 && h < 240.0f32 {
        dest[0] = m;
        dest[1] = x + m;
        dest[2] = c + m;
    } else if h >= 240.0f32 && h < 300.0f32 {
        dest[0] = x + m;
        dest[1] = m;
        dest[2] = c + m;
    } else if h >= 300.0f32 && h < 360.0f32 {
        dest[0] = c + m;
        dest[1] = m;
        dest[2] = x + m;
    } else {
        dest[0] = m;
        dest[1] = m;
        dest[2] = m;
    }
}

fn main() {}
