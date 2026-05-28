use std::ffi::c_int;
use std::slice;

const N_SMOOTH: usize = 16;

// In `match.h`, `float_t` is defined as a typedef for `double` (8 bytes).
// However, `spectral_contrast.c` only includes <math.h>, which on this
// platform (where __FLT_EVAL_METHOD__ == 0) defines `float_t` as `float`
// (4 bytes). This is an underhanded C "bug" baked into the ground-truth
// C source — the same memory is reinterpreted at different element widths
// depending on which translation unit is acting on it. We must replicate
// this behavior precisely.

// Type used by `match` and its helpers (smoothen, differentiate, total,
// preprocess) — matches `match.h`.
type FloatT = f64;

// Type used inside the spectral_contrast translation unit — matches the
// `<math.h>` definition of `float_t` on this platform.
type FloatTSpectral = f32;

// ---------------------------------------------------------------------------
// match.c helpers (operate on f64)
// ---------------------------------------------------------------------------

fn total(v: &[FloatT]) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..v.len() {
        sum += v[i];
    }
    sum
}

fn smoothen(v: &mut [FloatT]) {
    let length = v.len();
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

fn differentiate(v: &mut [FloatT]) {
    let length = v.len();
    if length == 0 {
        return;
    }
    for i in 0..(length - 1) {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess(dest: &mut [FloatT], source: &[FloatT]) {
    dest.copy_from_slice(source);
    smoothen(dest);
    differentiate(dest);
    smoothen(dest);
}

// ---------------------------------------------------------------------------
// spectral_contrast.c helpers (operate on f32 — the <math.h> float_t)
// ---------------------------------------------------------------------------

fn dot_product_f32(a: &[FloatTSpectral], b: &[FloatTSpectral]) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..a.len() {
        // C: sum += a[i] * b[i]; where a[i], b[i] are floats. The
        // multiplication is done at float precision (f32), then promoted
        // to double for the accumulation.
        sum += (a[i] * b[i]) as f64;
    }
    sum
}

fn normalize_f32(v: &mut [FloatTSpectral]) {
    let magnitude = dot_product_f32(v, v).sqrt();
    let length = v.len();
    for i in 0..length {
        // C: v[i] /= magnitude; where v[i] is float and magnitude is double.
        // The division is performed at double precision after promoting v[i],
        // then converted back to float on store.
        v[i] = ((v[i] as f64) / magnitude) as f32;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut FloatT, // matches match.h signature, but interpreted as f32* inside
    b: *mut FloatT,
    length: c_int,
) -> f64 {
    let len = length as usize;
    // Reinterpret as f32 slices — this is the exact C behavior of
    // spectral_contrast.c, which sees `float_t` as `float`.
    let a_slice = unsafe { slice::from_raw_parts_mut(a as *mut FloatTSpectral, len) };
    let b_slice = unsafe { slice::from_raw_parts_mut(b as *mut FloatTSpectral, len) };
    normalize_f32(a_slice);
    normalize_f32(b_slice);
    dot_product_f32(a_slice, b_slice)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut FloatT,
    reference: *mut FloatT,
    bins: c_int,
    threshold: f64,
) -> c_int {
    let bins_usize = bins as usize;
    let test_slice = unsafe { slice::from_raw_parts(test, bins_usize) };
    let reference_slice = unsafe { slice::from_raw_parts(reference, bins_usize) };

    if total(test_slice) < threshold * total(reference_slice) {
        return 0;
    }

    // C uses VLAs: float_t t[bins], r[bins]; (i.e., double[bins]).
    let mut t: Vec<FloatT> = vec![0.0; bins_usize];
    let mut r: Vec<FloatT> = vec![0.0; bins_usize];

    preprocess(&mut t, test_slice);
    preprocess(&mut r, reference_slice);

    let sc = unsafe { spectral_contrast(t.as_mut_ptr(), r.as_mut_ptr(), bins) };
    if sc >= threshold { 1 } else { 0 }
}
