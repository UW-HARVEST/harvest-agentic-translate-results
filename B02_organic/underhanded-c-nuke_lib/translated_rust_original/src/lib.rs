use std::os::raw::c_int;

const N_SMOOTH: usize = 16;

fn total(v: &[f64]) -> f64 {
    v.iter().sum()
}

fn smoothen(v: &mut [f64]) {
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

fn differentiate(v: &mut [f64]) {
    let length = v.len();
    for i in 0..length - 1 {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess(source: &[f64]) -> Vec<f64> {
    let mut v = source.to_vec();
    smoothen(&mut v);
    differentiate(&mut v);
    smoothen(&mut v);
    v
}

fn dot_product(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn normalize(v: &mut [f64]) {
    let magnitude = dot_product(v, v).sqrt();
    for x in v.iter_mut() {
        *x /= magnitude;
    }
}

/// # Safety
/// `a` and `b` must point to at least `length` valid f64 elements.
/// This function mutates the data pointed to by `a` and `b` (matching C behavior).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(a: *mut f64, b: *mut f64, length: c_int) -> f64 {
    let length = length as usize;
    let a = unsafe { std::slice::from_raw_parts_mut(a, length) };
    let b = unsafe { std::slice::from_raw_parts_mut(b, length) };
    normalize(a);
    normalize(b);
    dot_product(a, b)
}

/// # Safety
/// `test` and `reference` must point to at least `bins` valid f64 elements.
#[export_name = "match"]
pub unsafe extern "C" fn match_(
    test: *mut f64,
    reference: *mut f64,
    bins: c_int,
    threshold: f64,
) -> c_int {
    let bins = bins as usize;
    let test_slice = unsafe { std::slice::from_raw_parts(test, bins) };
    let ref_slice = unsafe { std::slice::from_raw_parts(reference, bins) };
    if total(test_slice) < threshold * total(ref_slice) {
        return 0;
    }
    let mut t = preprocess(test_slice);
    let mut r = preprocess(ref_slice);
    let sc = spectral_contrast(t.as_mut_ptr(), r.as_mut_ptr(), bins as c_int);
    (sc >= threshold) as c_int
}
