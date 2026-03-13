const N_SMOOTH: usize = 16;

fn total(v: &[f64]) -> f64 {
    v.iter().sum()
}

fn smoothen(v: &mut [f64]) {
    let length = v.len();
    for i in 0..length {
        let mut sum = 0.0;
        for j in 0..N_SMOOTH {
            if i + j >= length {
                break;
            }
            sum += v[i + j];
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
/// `test` and `reference` must point to at least `bins` valid f64 values.
#[export_name = "match"]
pub unsafe extern "C" fn match_(
    test: *const f64,
    reference: *const f64,
    bins: std::ffi::c_int,
    threshold: f64,
) -> std::ffi::c_int {
    let bins = bins as usize;
    let test = unsafe { std::slice::from_raw_parts(test, bins) };
    let reference = unsafe { std::slice::from_raw_parts(reference, bins) };

    if total(test) < threshold * total(reference) {
        return 0;
    }

    let mut t = preprocess(test);
    let mut r = preprocess(reference);

    normalize(&mut t);
    normalize(&mut r);
    (dot_product(&t, &r) >= threshold) as std::ffi::c_int
}

/// # Safety
/// `a` and `b` must point to at least `length` valid f64 values.
/// The pointed-to data will be mutated (normalized in place).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut f64,
    b: *mut f64,
    length: std::ffi::c_int,
) -> f64 {
    let length = length as usize;
    let a = unsafe { std::slice::from_raw_parts_mut(a, length) };
    let b = unsafe { std::slice::from_raw_parts_mut(b, length) };
    normalize(a);
    normalize(b);
    dot_product(a, b)
}
