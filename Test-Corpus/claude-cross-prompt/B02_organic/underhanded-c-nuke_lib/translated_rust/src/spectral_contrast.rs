// Translation of c_src/src/spectral_contrast.c
//
// In the C code, `float_t` is typedef'd to `double` in match.h, so we use
// `f64` here to match the precision used by the original implementation.

#[allow(dead_code)]
pub fn dot_product(a: &[f64], b: &[f64], length: usize) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..length {
        sum += a[i] * b[i];
    }
    sum
}

#[allow(dead_code)]
pub fn normalize(v: &mut [f64], length: usize) {
    // Compute magnitude using the shared dot_product helper (matching the C).
    let magnitude = dot_product(v, v, length).sqrt();
    for i in 0..length {
        v[i] /= magnitude;
    }
}

#[allow(dead_code)]
pub fn spectral_contrast(a: &mut [f64], b: &mut [f64], length: usize) -> f64 {
    normalize(a, length);
    normalize(b, length);
    dot_product(a, b, length)
}
