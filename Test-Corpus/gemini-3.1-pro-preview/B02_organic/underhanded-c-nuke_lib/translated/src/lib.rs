use std::os::raw::{c_double, c_int};

pub type float_t = c_double;
const N_SMOOTH: usize = 16;

fn total(v: &[float_t]) -> c_double {
    v.iter().sum()
}

fn smoothen(v: &mut [float_t]) {
    let length = v.len();
    for i in 0..length {
        let mut sum = 0.0;
        for j in 0..N_SMOOTH {
            if i + j < length {
                sum += v[i + j];
            } else {
                break;
            }
        }
        v[i] = sum / (N_SMOOTH as c_double);
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

#[unsafe(no_mangle)]
#[export_name = "match"]
pub extern "C" fn match_func(
    test: *mut float_t,
    reference: *mut float_t,
    bins: c_int,
    threshold: c_double,
) -> c_int {
    if bins <= 0 {
        return 0;
    }
    let bins_usize = bins as usize;
    let test_slice = unsafe { std::slice::from_raw_parts(test as *const float_t, bins_usize) };
    let ref_slice = unsafe { std::slice::from_raw_parts(reference as *const float_t, bins_usize) };

    if total(test_slice) < threshold * total(ref_slice) {
        return 0;
    }

    let mut t = vec![0.0; bins_usize];
    let mut r = vec![0.0; bins_usize];

    preprocess(&mut t, test_slice);
    preprocess(&mut r, ref_slice);

    if spectral_contrast_internal(&mut t, &mut r) >= threshold {
        1
    } else {
        0
    }
}

fn dot_product(a: &[float_t], b: &[float_t]) -> c_double {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn normalize(v: &mut [float_t]) {
    let magnitude = dot_product(v, v).sqrt();
    for x in v.iter_mut() {
        *x /= magnitude;
    }
}

fn spectral_contrast_internal(a: &mut [float_t], b: &mut [float_t]) -> c_double {
    normalize(a);
    normalize(b);
    dot_product(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn spectral_contrast(
    a: *mut float_t,
    b: *mut float_t,
    length: c_int,
) -> c_double {
    if length <= 0 {
        return 0.0;
    }
    let len_usize = length as usize;
    let a_slice = unsafe { std::slice::from_raw_parts_mut(a, len_usize) };
    let b_slice = unsafe { std::slice::from_raw_parts_mut(b, len_usize) };
    spectral_contrast_internal(a_slice, b_slice)
}
