use std::os::raw::c_int;

pub const N_SMOOTH: usize = 16;
pub type float_t = f64;

fn total(v: &[float_t]) -> f64 {
    v.iter().copied().sum()
}

fn smoothen(v: &mut [float_t]) {
    let length = v.len();
    for i in 0..length {
        let mut sum = 0.0;
        let mut j = 0;
        while j < N_SMOOTH && i + j < length {
            sum += v[i + j];
            j += 1;
        }
        v[i] = sum / N_SMOOTH as f64;
    }
}

fn differentiate(v: &mut [float_t]) {
    let length = v.len();
    if length == 0 {
        return;
    }
    for i in 0..length - 1 {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess(v: &mut [float_t], source: &[float_t]) {
    v.copy_from_slice(source);
    smoothen(v);
    differentiate(v);
    smoothen(v);
}

fn dot_product(a: &[float_t], b: &[float_t]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn normalize(v: &mut [float_t]) {
    let magnitude = dot_product(v, v).sqrt();
    for x in v.iter_mut() {
        *x /= magnitude;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn spectral_contrast(a: *mut float_t, b: *mut float_t, length: c_int) -> f64 {
    if a.is_null() || b.is_null() || length < 0 {
        return 0.0;
    }
    let length = length as usize;
    let a = unsafe { std::slice::from_raw_parts_mut(a, length) };
    let b = unsafe { std::slice::from_raw_parts_mut(b, length) };
    normalize(a);
    normalize(b);
    dot_product(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn r#match(test: *mut float_t, reference: *mut float_t, bins: c_int, threshold: f64) -> c_int {
    if test.is_null() || reference.is_null() || bins < 0 {
        return 0;
    }
    let bins = bins as usize;
    let test = unsafe { std::slice::from_raw_parts(test, bins) };
    let reference = unsafe { std::slice::from_raw_parts(reference, bins) };
    if total(test) < threshold * total(reference) {
        return 0;
    }
    let mut t = vec![0.0; bins];
    let mut r = vec![0.0; bins];
    preprocess(&mut t, test);
    preprocess(&mut r, reference);
    if spectral_contrast(t.as_mut_ptr(), r.as_mut_ptr(), bins as c_int) >= threshold {
        1
    } else {
        0
    }
}
