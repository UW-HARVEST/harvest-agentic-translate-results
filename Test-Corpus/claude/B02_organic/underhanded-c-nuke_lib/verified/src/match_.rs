// Translation of c_src/src/match.c
//
//     #include <string.h> /* memcpy */
//     #include "match.h"
//
//     static double total(float_t *v, int length);
//     static void smoothen(float_t *v, int length);
//     static void differentiate(float_t *v, int length);
//     static void preprocess(float_t *v, float_t *source, int length);
//     int match(float_t *test, float_t *reference, int bins, double threshold);
//
// This translation unit includes "match.h", so here
//     #define N_SMOOTH 16
//     typedef double float_t;
// i.e. every `float_t` below is `f64`.

use core::ffi::c_int;

/// `#define N_SMOOTH 16` from c_src/include/match.h (an `int` constant).
const N_SMOOTH: c_int = 16;

/// A stand-in for the C variable-length arrays `float_t t[bins], r[bins];`.
///
/// One extra leading element is reserved so that `differentiate()`'s
/// unconditional `v[length - 1] = 0;` store still lands in owned memory when
/// `length == 0` (in that case the C code writes `v[-1]`, i.e. out of bounds --
/// undefined behaviour that we absorb instead of reproducing as a stack smash).
struct Vla {
    /// `data[0]` backs logical index -1; `data[1 + i]` backs logical index `i`.
    data: Vec<f64>,
}

impl Vla {
    fn new(len: c_int) -> Vla {
        let n = if len > 0 { len as usize } else { 0 };
        Vla {
            data: vec![0.0f64; n + 1],
        }
    }

    /// Read logical element `i` (`v[i]`). Only ever called with `0 <= i < len`.
    #[inline]
    fn get(&self, i: c_int) -> f64 {
        self.data[(i as isize + 1) as usize]
    }

    /// Write logical element `i` (`v[i] = value`).
    ///
    /// Stores that fall outside the owned allocation are dropped; in the C code
    /// those are out-of-bounds writes (only reachable for `bins <= 0`).
    #[inline]
    fn set(&mut self, i: c_int, value: f64) {
        let idx = i as isize + 1;
        if idx >= 0 && (idx as usize) < self.data.len() {
            self.data[idx as usize] = value;
        }
    }

    /// Pointer to logical element 0 -- what the C code would pass along as
    /// `float_t *`.
    #[inline]
    fn base_ptr(&mut self) -> *mut f64 {
        unsafe { self.data.as_mut_ptr().add(1) }
    }
}

/// static double total(float_t *v, int length)
///
/// ```c
/// double sum = 0;
/// int i;
/// for(i = 0; i < length; i++) sum += v[i];
/// return sum;
/// ```
///
/// gcc emits `addsd %sum, %v[i]`, i.e. the loaded element is SRC1, so
/// `fp::addsd` is used to keep the NaN payload selection identical.
#[inline]
unsafe fn total(v: *const f64, length: c_int) -> f64 {
    let mut sum: f64 = 0.0;
    let mut i: c_int = 0;
    while i < length {
        sum = crate::fp::addsd(unsafe { *v.offset(i as isize) }, sum);
        i = i.wrapping_add(1);
    }
    sum
}

/// static void smoothen(float_t *v, int length)
///
/// ```c
/// double sum;
/// int i, j;
/// for(i = 0; i < length; i++) {
///     sum = 0;
///     for(j = 0; j < N_SMOOTH && i + j < length; j++)
///         sum += v[i + j];
///     v[i] = sum / N_SMOOTH;
/// }
/// ```
///
/// The in-place update is safe to mirror directly: iteration `i` only writes
/// index `i` after reading indices `i..i+N_SMOOTH-1`, so every value read is
/// still the pre-smoothing one.
fn smoothen(v: &mut Vla, length: c_int) {
    let mut i: c_int = 0;
    while i < length {
        let mut sum: f64 = 0.0;
        let mut j: c_int = 0;
        while j < N_SMOOTH && i.wrapping_add(j) < length {
            // gcc: `addsd %sum, %v[i+j]` -- the loaded element is SRC1.
            sum = crate::fp::addsd(v.get(i.wrapping_add(j)), sum);
            j = j.wrapping_add(1);
        }
        v.set(i, sum / (N_SMOOTH as f64));
        i = i.wrapping_add(1);
    }
}

/// static void differentiate(float_t *v, int length)
///
/// ```c
/// int i;
/// for(i = 0; i < length - 1; i++) v[i] = v[i + 1] - v[i];
/// v[length - 1] = 0;
/// ```
fn differentiate(v: &mut Vla, length: c_int) {
    let mut i: c_int = 0;
    while i < length.wrapping_sub(1) {
        let next = v.get(i.wrapping_add(1));
        let cur = v.get(i);
        v.set(i, next - cur);
        i = i.wrapping_add(1);
    }
    v.set(length.wrapping_sub(1), 0.0);
}

/// static void preprocess(float_t *v, float_t *source, int length)
///
/// ```c
/// memcpy(v, source, length * sizeof(*v));
/// smoothen(v, length);
/// differentiate(v, length);
/// smoothen(v, length);
/// ```
unsafe fn preprocess(v: &mut Vla, source: *const f64, length: c_int) {
    if length > 0 {
        unsafe {
            core::ptr::copy_nonoverlapping(source, v.base_ptr(), length as usize);
        }
    }
    smoothen(v, length);
    differentiate(v, length);
    smoothen(v, length);
}

/// int match(float_t *test, float_t *reference, int bins, double threshold)
///
/// ```c
/// float_t t[bins], r[bins];
/// if(total(test, bins) < threshold * total(reference, bins)) return 0;
/// preprocess(t, test, bins);
/// preprocess(r, reference, bins);
/// return spectral_contrast(t, r, bins) >= threshold;
/// ```
///
/// Note the deliberate type confusion on the last line: `t` and `r` are arrays
/// of `double` here, but `spectral_contrast()` (a different translation unit,
/// where `float_t` is `float`) reads them as arrays of `float`.  See lib.rs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut f64,
    reference: *mut f64,
    bins: c_int,
    threshold: f64,
) -> c_int {
    let mut t = Vla::new(bins);
    let mut r = Vla::new(bins);

    // gcc: `mulsd -0x70(%rbp), %xmm1` with xmm1 = total(reference, bins), i.e.
    // the reference total is SRC1 and `threshold` is SRC2.
    if unsafe { total(test, bins) }
        < crate::fp::mulsd(unsafe { total(reference, bins) }, threshold)
    {
        return 0;
    }

    unsafe {
        preprocess(&mut t, test, bins);
        preprocess(&mut r, reference, bins);
    }

    let contrast = unsafe {
        crate::spectral_contrast::spectral_contrast(
            t.base_ptr() as *mut f32,
            r.base_ptr() as *mut f32,
            bins,
        )
    };

    (contrast >= threshold) as c_int
}
