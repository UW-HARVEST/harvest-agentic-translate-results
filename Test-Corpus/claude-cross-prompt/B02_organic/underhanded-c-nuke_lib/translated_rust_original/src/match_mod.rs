// Translation of c_src/src/match.c
//
// `match` is a Rust keyword, so the module is named `match_mod` and the
// public function is named `match_signal` to avoid conflicting with the
// keyword while preserving the same behavior.

use crate::spectral_contrast::spectral_contrast;

const N_SMOOTH: usize = 16; // Size of smoothing kernel

fn total(v: &[f64], length: usize) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..length {
        sum += v[i];
    }
    sum
}

fn smoothen(v: &mut [f64], length: usize) {
    for i in 0..length {
        let mut sum: f64 = 0.0;
        let mut j: usize = 0;
        while j < N_SMOOTH && i + j < length {
            sum += v[i + j];
            j += 1;
        }
        v[i] = sum / (N_SMOOTH as f64);
    }
}

fn differentiate(v: &mut [f64], length: usize) {
    if length == 0 {
        return;
    }
    for i in 0..(length - 1) {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess(dst: &mut [f64], source: &[f64], length: usize) {
    dst[..length].copy_from_slice(&source[..length]);
    smoothen(dst, length);
    differentiate(dst, length);
    smoothen(dst, length);
}

#[allow(dead_code)]
pub fn match_signal(test: &[f64], reference: &[f64], bins: usize, threshold: f64) -> i32 {
    // Stack-allocated VLAs in the C code; we use heap-allocated Vecs here.
    let mut t: Vec<f64> = vec![0.0; bins];
    let mut r: Vec<f64> = vec![0.0; bins];
    if total(test, bins) < threshold * total(reference, bins) {
        return 0;
    }
    preprocess(&mut t, test, bins);
    preprocess(&mut r, reference, bins);
    if spectral_contrast(&mut t, &mut r, bins) >= threshold {
        1
    } else {
        0
    }
}
