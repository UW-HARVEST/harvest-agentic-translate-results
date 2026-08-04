use std::os::raw::{c_double, c_int};

const N_SMOOTH: usize = 16;

// In match.c, float_t = double (from match.h)
type FloatT = c_double;

// In spectral_contrast.c, float_t = float (from math.h)
type FloatTSc = f32;

fn total(v: &[FloatT]) -> c_double {
    v.iter().sum()
}

fn smoothen(v: &mut [FloatT]) {
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

fn differentiate(v: &mut [FloatT]) {
    let length = v.len();
    for i in 0..length - 1 {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess(source: &[FloatT], length: usize) -> Vec<FloatT> {
    let mut v = source[..length].to_vec();
    smoothen(&mut v);
    differentiate(&mut v);
    smoothen(&mut v);
    v
}

fn dot_product_sc(a: &[FloatTSc], b: &[FloatTSc]) -> c_double {
    let mut sum: c_double = 0.0;
    for i in 0..a.len() {
        sum += (a[i] * b[i]) as c_double;
    }
    sum
}

fn normalize_sc(v: &mut [FloatTSc]) {
    let magnitude = dot_product_sc(v, v).sqrt();
    for x in v.iter_mut() {
        *x = (*x as c_double / magnitude) as FloatTSc;
    }
}

#[unsafe(export_name = "match")]
pub extern "C" fn match_(test: *mut FloatT, reference: *mut FloatT, bins: c_int, threshold: c_double) -> c_int {
    let bins = bins as usize;
    let test_slice = unsafe { std::slice::from_raw_parts(test, bins) };
    let ref_slice = unsafe { std::slice::from_raw_parts(reference, bins) };
    if total(test_slice) < threshold * total(ref_slice) {
        return 0;
    }
    let mut t = preprocess(test_slice, bins);
    let mut r = preprocess(ref_slice, bins);
    // match.c calls spectral_contrast passing double* to a function expecting float*.
    // Replicate this: pass the raw bytes of the double array as float pointers.
    let sc = spectral_contrast(t.as_mut_ptr() as *mut FloatTSc, r.as_mut_ptr() as *mut FloatTSc, bins as c_int);
    (sc >= threshold) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn spectral_contrast(a: *mut FloatTSc, b: *mut FloatTSc, length: c_int) -> c_double {
    let length = length as usize;
    let a_slice = unsafe { std::slice::from_raw_parts_mut(a, length) };
    let b_slice = unsafe { std::slice::from_raw_parts_mut(b, length) };
    normalize_sc(a_slice);
    normalize_sc(b_slice);
    dot_product_sc(a_slice, b_slice)
}
