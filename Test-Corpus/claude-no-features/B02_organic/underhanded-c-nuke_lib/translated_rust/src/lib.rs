//! Rust translation of c_src/ producing byte-identical output.
//!
//! IMPORTANT: There's a subtle bug in the original C code that we must preserve:
//! - `match.h` defines `typedef double float_t;`
//! - `match.c` includes `match.h`, so `float_t` is `double` (8 bytes) there.
//! - `spectral_contrast.c` does NOT include `match.h`; it only includes `<math.h>`.
//!   On x86_64 glibc with FLT_EVAL_METHOD == 0, `<math.h>` defines
//!   `typedef float float_t;` (4 bytes).
//! - Therefore `spectral_contrast` actually reads its input arrays as `f32`
//!   even though `match` passes `f64` arrays to it.
//!
//! We replicate this exactly by reading `f32` values inside `spectral_contrast`
//! while exposing the public signatures from `match.h`.

use std::os::raw::c_int;

const N_SMOOTH: usize = 16; // Size of smoothing kernel

// In match.c, float_t is double (from match.h).
type FloatT = f64;

// ---------------------------------------------------------------------------
// match.c helpers
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
    for i in 0..length - 1 {
        v[i] = v[i + 1] - v[i];
    }
    v[length - 1] = 0.0;
}

fn preprocess(dest: &mut [FloatT], source: &[FloatT]) {
    // memcpy(v, source, length * sizeof(*v));
    dest.copy_from_slice(source);
    smoothen(dest);
    differentiate(dest);
    smoothen(dest);
}

// ---------------------------------------------------------------------------
// spectral_contrast.c helpers
//
// In the C source these operate on `float_t*`, but because that file only
// includes <math.h>, `float_t` resolves to `float` (f32) — NOT `double`.
// We must therefore read 32-bit floats from whatever buffer is passed in.
// ---------------------------------------------------------------------------

fn dot_product_f32(a: &[f32], b: &[f32]) -> f64 {
    let mut sum: f64 = 0.0;
    let length = a.len();
    for i in 0..length {
        // a[i] * b[i] in C is computed in float precision, then promoted to
        // double when added to `sum`.
        sum += (a[i] * b[i]) as f64;
    }
    sum
}

fn normalize_f32(v: &mut [f32]) {
    let magnitude: f64 = dot_product_f32(v, v).sqrt();
    let length = v.len();
    for i in 0..length {
        // In C, `v[i] /= magnitude` where v[i] is float and magnitude is double:
        // v[i] is promoted to double, the division is done in double, then the
        // result is converted back to float for the store.
        v[i] = ((v[i] as f64) / magnitude) as f32;
    }
}

// ---------------------------------------------------------------------------
// Public C ABI
// ---------------------------------------------------------------------------

/// `int match(float_t *test, float_t *reference, int bins, double threshold)`
/// where match.h defines `float_t` = `double`.
///
/// `match` is a Rust keyword, so use a raw identifier; the linker symbol is
/// just `match` thanks to `#[unsafe(no_mangle)]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut FloatT,
    reference: *mut FloatT,
    bins: c_int,
    threshold: f64,
) -> c_int {
    if bins < 0 {
        return 0;
    }
    let n = bins as usize;

    let test_slice: &[FloatT] = if n == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(test, n) }
    };
    let ref_slice: &[FloatT] = if n == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(reference, n) }
    };

    // First check: order of evaluation matches the C source exactly.
    if total(test_slice) < threshold * total(ref_slice) {
        return 0;
    }

    // C uses VLAs: `float_t t[bins], r[bins];`. We allocate on the heap.
    let mut t: Vec<FloatT> = vec![0.0; n];
    let mut r: Vec<FloatT> = vec![0.0; n];

    preprocess(&mut t, test_slice);
    preprocess(&mut r, ref_slice);

    // Now call spectral_contrast. In C, this passes `double*` pointers but
    // spectral_contrast reads them as `float*`. Reproduce this by passing the
    // same memory to the public spectral_contrast entry point.
    let result = unsafe {
        spectral_contrast(
            t.as_mut_ptr(),
            r.as_mut_ptr(),
            bins,
        )
    };

    // C: `return spectral_contrast(t, r, bins) >= threshold;`
    if result >= threshold { 1 } else { 0 }
}

/// `double spectral_contrast(float_t *a, float_t *b, int length)`.
///
/// Per match.h, `float_t` is `double`, but the implementation in
/// spectral_contrast.c sees `float_t` as `float` and reads 4-byte floats from
/// the buffer. The public ABI is just (pointer, pointer, int, returning
/// double); pointer width is the same regardless of element type. Internally
/// we read f32 values to match the original C behavior exactly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut FloatT,
    b: *mut FloatT,
    length: c_int,
) -> f64 {
    if length < 0 {
        // C UB; reproduce by treating as zero-length (avoids panic).
        return 0.0;
    }
    let n = length as usize;

    // Reinterpret the buffers as f32 arrays of `length` elements (NOT
    // `length * 2`). This matches what the C compiler emits when float_t is
    // float.
    let a_f32: &mut [f32] = if n == 0 {
        &mut []
    } else {
        unsafe { std::slice::from_raw_parts_mut(a as *mut f32, n) }
    };
    let b_f32: &mut [f32] = if n == 0 {
        &mut []
    } else {
        unsafe { std::slice::from_raw_parts_mut(b as *mut f32, n) }
    };

    normalize_f32(a_f32);
    normalize_f32(b_f32);
    dot_product_f32(a_f32, b_f32)
}
