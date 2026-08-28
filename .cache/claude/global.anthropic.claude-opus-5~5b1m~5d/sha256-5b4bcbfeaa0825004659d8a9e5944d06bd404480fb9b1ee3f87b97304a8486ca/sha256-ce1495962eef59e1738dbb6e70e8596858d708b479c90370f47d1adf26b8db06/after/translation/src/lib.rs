// Rust translation of the C library in c_src/.
//
// Public ABI (from `nm -D` on the C shared object):
//     T match
//     T spectral_contrast
//
// `preprocess`, `total`, `smoothen`, `differentiate`, `dot_product` and
// `normalize` are all `static` in the C sources and therefore local symbols
// (`t` in nm output); they are NOT part of the exported ABI.
//
// ---------------------------------------------------------------------------
// CRITICAL DETAIL: the two translation units disagree about `float_t`.
//
//   * match.c            includes "match.h", which does
//                            typedef double float_t;
//                        so in match.c `float_t` is `double` (8 bytes).
//
//   * spectral_contrast.c includes ONLY <math.h> and never includes match.h.
//                        On x86-64 glibc FLT_EVAL_METHOD == 0, so <math.h>
//                        supplies its own `typedef float float_t;`
//                        (verified: sizeof(float_t) == 4).
//                        So in spectral_contrast.c `float_t` is `float`.
//
// The declaration of spectral_contrast that match.c sees says `double *`, but
// the compiled definition indexes its arguments as `float *`. Because the two
// only ever agree on the ABI (a pointer is a pointer), the code compiles and
// links, but `spectral_contrast` reinterprets the bytes of the caller's
// `double` array as an array of `float`.
//
// This is a genuine bug in the original library. Per the translation rules it
// is reproduced exactly rather than fixed. Confirmed against the disassembly
// of the C shared object, which uses `movss` / `mulss` / `cvtss2sd`:
//
//     16d0: movss    (%rax),%xmm0        ; load a[i] as f32
//     16d8: mulss    %xmm0,%xmm0         ; multiply in SINGLE precision
//     16dc: cvtss2sd %xmm0,%xmm0         ; widen the f32 product to f64
//     16e0: addsd    %xmm0,%xmm4         ; accumulate in DOUBLE precision
//
// and, in `normalize`:
//
//     1763: cvtss2sd (%rcx),%xmm0        ; widen v[i] to f64
//     1767: divsd    %xmm4,%xmm0         ; divide in DOUBLE precision
//     176b: cvtsd2ss %xmm0,%xmm0         ; round the quotient back to f32
//     176f: movss    %xmm0,(%rcx)        ; store as f32
//
// Every exported function below therefore operates on raw pointers rather than
// Rust slices: the C API permits the two argument arrays to alias (e.g.
// `spectral_contrast(v, v, n)`), and building two overlapping `&mut [T]`
// would be undefined behaviour in Rust even though it is well-defined in C.
//
// ---------------------------------------------------------------------------
// VERIFICATION
//
// Differentially tested against the C shared object through `libloading` (both
// libraries loaded as `.so`s and called through their exported symbols) over
// every row of CONFIGS.md and ERRORS.md, comparing the raw IEEE-754 bits of
// each return value *and* of every element of every mutated buffer. See
// tests/configs.rs, tests/errors.rs and tests/symbols.rs.
//
// The reference `.so` is the one the task prescribes:
//
//     cd c_src && mkdir -p build && cd build && \
//       cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
//
// which sets no CMAKE_BUILD_TYPE, so GCC compiles at `-O0`.
//
// CAVEAT, inherent to the C rather than to this translation: the NaN *payload*
// that `spectral_contrast` returns is not fixed by the C source. It depends on
// the compiler's choice of SSE destination operand, which differs between
// optimization levels -- `-O0` and `-O2` of these very sources disagree with
// each other (e.g. `spectral_contrast` on a[0]=0x7FC00001, b[0]=0x7FC00002,
// length 1: `-O0` returns 0x7FF8000040000000 but `-O2` returns
// 0x7FF8000020000000). No single translation can match both. This translation
// reproduces the `-O0` build, i.e. the exact `.so` produced by the prescribed
// cmake invocation, with the operand roles read out of its disassembly and
// pinned down by the addsd/subsd/mulsd/divsd/mulss helpers below.
//
// Everything the C language actually specifies is bit-exact under every
// optimization level. In particular `match`'s integer return value is
// unaffected: any NaN reaching `match` makes both of its ordered comparisons
// false regardless of payload.
// ---------------------------------------------------------------------------

use std::ffi::c_int;

/// `#define N_SMOOTH 16` from match.h -- size of the smoothing kernel.
const N_SMOOTH: c_int = 16;

// ===========================================================================
// Bit-exact scalar SSE arithmetic
// ===========================================================================
//
// For every finite/infinite input these helpers are plain IEEE-754 operations.
// They exist solely to pin down NaN *payload* propagation.
//
// x86 SSE two-operand arithmetic (`addsd dst, src` computing `dst = dst OP
// src`) picks its NaN result as follows: if `dst` is NaN the result is `dst`
// quieted; otherwise if `src` is NaN the result is `src` quieted. So when both
// operands are NaN the *destination* operand's payload survives.
//
// At `-O0` -- which is what the prescribed `cmake` invocation produces, since
// it sets no CMAKE_BUILD_TYPE -- GCC compiles `sum += v[i]` by loading the new
// term into the destination register and the accumulator into the source:
//
//     movsd  (%rax),%xmm0             ; xmm0 = v[i]
//     movsd  -0x8(%rbp),%xmm1         ; xmm1 = sum
//     addsd  %xmm1,%xmm0              ; xmm0 = v[i] + sum   -> dst is v[i]
//
// so the LAST NaN to enter the accumulator wins and is carried to the end.
// (An optimized GCC build keeps the accumulator in the destination instead,
// `addsd (%rdi,%rax,8),%xmm2`, making the FIRST NaN win -- that is the
// -O0/-O2 disagreement documented above.)
//
// Likewise `a[i] * b[i]` at `-O0` puts `b[i]` in the destination:
//
//     movss  (%rax),%xmm1             ; xmm1 = a[i]
//     movss  (%rax),%xmm0             ; xmm0 = b[i]
//     mulss  %xmm1,%xmm0              ; xmm0 = b[i] * a[i]  -> dst is b[i]
//
// LLVM is free to commute floating-point addition and multiplication (it treats
// the NaN payload choice as non-deterministic), so writing `sum + x` in Rust
// does not pin the operand roles down. Routing the arithmetic through these
// helpers makes the roles explicit, so the result no longer depends on LLVM's
// choice and matches the reference `.so` on every input.
//
// Non-commutative operations (`subsd`, `divsd`) cannot be reordered by the
// compiler, but they are modelled here too so that all four agree in style.

/// Quiet a NaN the way SSE does: force the most-significant mantissa bit on.
#[inline(always)]
fn quiet64(x: f64) -> f64 {
    f64::from_bits(x.to_bits() | 0x0008_0000_0000_0000)
}

/// Quiet a NaN the way SSE does: force the most-significant mantissa bit on.
#[inline(always)]
fn quiet32(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `addsd dst, src` -- computes `dst + src`, keeping `dst`'s NaN payload.
#[inline(always)]
fn addsd(dst: f64, src: f64) -> f64 {
    if dst.is_nan() {
        quiet64(dst)
    } else if src.is_nan() {
        quiet64(src)
    } else {
        dst + src
    }
}

/// `subsd dst, src` -- computes `dst - src`, keeping `dst`'s NaN payload.
#[inline(always)]
fn subsd(dst: f64, src: f64) -> f64 {
    if dst.is_nan() {
        quiet64(dst)
    } else if src.is_nan() {
        quiet64(src)
    } else {
        dst - src
    }
}

/// `mulsd dst, src` -- computes `dst * src`, keeping `dst`'s NaN payload.
#[inline(always)]
fn mulsd(dst: f64, src: f64) -> f64 {
    if dst.is_nan() {
        quiet64(dst)
    } else if src.is_nan() {
        quiet64(src)
    } else {
        dst * src
    }
}

/// `divsd dst, src` -- computes `dst / src`, keeping `dst`'s NaN payload.
#[inline(always)]
fn divsd(dst: f64, src: f64) -> f64 {
    if dst.is_nan() {
        quiet64(dst)
    } else if src.is_nan() {
        quiet64(src)
    } else {
        dst / src
    }
}

/// `mulss dst, src` -- computes `dst * src`, keeping `dst`'s NaN payload.
#[inline(always)]
fn mulss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet32(dst)
    } else if src.is_nan() {
        quiet32(src)
    } else {
        dst * src
    }
}

// ===========================================================================
// spectral_contrast.c   --   here `float_t` == `f32`
// ===========================================================================

/// ```c
/// static double dot_product(float_t *a, float_t *b, int length) {
///     double sum = 0;
///     int i;
///     for(i = 0; i < length; i++) sum += a[i] * b[i];
///     return sum;
/// }
/// ```
///
/// `a[i] * b[i]` has type `float`, so with FLT_EVAL_METHOD == 0 the product is
/// rounded to single precision *before* being widened and added to the
/// double-precision accumulator. The accumulation is strictly sequential (GCC
/// cannot reassociate floating-point additions without `-ffast-math`, and the
/// disassembly confirms a single `addsd` dependency chain).
///
/// Note that the loop guard is `i < length` with `i` starting at zero, so any
/// `length <= 0` -- including negative and `INT_MIN` -- runs zero iterations and
/// returns `+0.0` without dereferencing either pointer.
unsafe fn dot_product(a: *const f32, b: *const f32, length: c_int) -> f64 {
    let mut sum: f64 = 0.0;
    let mut i: c_int = 0;
    while i < length {
        let av = unsafe { *a.offset(i as isize) };
        let bv = unsafe { *b.offset(i as isize) };
        // f32 multiply, then widen to f64: mulss + cvtss2sd + addsd.
        //
        //     movss (%rax),%xmm1        ; xmm1 = a[i]
        //     movss (%rax),%xmm0        ; xmm0 = b[i]
        //     mulss %xmm1,%xmm0         ; xmm0 = b[i] * a[i]   -> dst is b[i]
        //     cvtss2sd %xmm0,%xmm0
        //     movsd -0x8(%rbp),%xmm1    ; xmm1 = sum
        //     addsd %xmm1,%xmm0         ; xmm0 = product + sum  -> dst is product
        sum = addsd(mulss(bv, av) as f64, sum);
        i += 1;
    }
    sum
}

/// ```c
/// static void normalize(float_t *v, int length) {
///     double magnitude = sqrt(dot_product(v, v, length));
///     int i;
///     for(i = 0; i < length; i++) v[i] /= magnitude;
/// }
/// ```
///
/// `v[i] /= magnitude` is `v[i] = (float)((double)v[i] / magnitude)`: the usual
/// arithmetic conversions promote the `float` element to `double`, the division
/// happens in double precision, and the result is rounded back to `float` on
/// store.
///
/// No division-by-zero guard exists in the C, so a zero magnitude yields
/// +/-inf or NaN elements. That is reproduced verbatim.
unsafe fn normalize(v: *mut f32, length: c_int) {
    let magnitude: f64 = unsafe { dot_product(v, v, length) }.sqrt();
    let mut i: c_int = 0;
    while i < length {
        let p = unsafe { v.offset(i as isize) };
        let x = unsafe { *p };
        unsafe { *p = divsd(x as f64, magnitude) as f32 };
        i += 1;
    }
}

/// Shared body of `spectral_contrast`, so that `match` reproduces the exact
/// same computation the C `match` gets from its call through the PLT.
///
/// ```c
/// double spectral_contrast(float_t *a, float_t *b, int length) {
///     normalize(a, length);
///     normalize(b, length);
///     return dot_product(a, b, length);
/// }
/// ```
unsafe fn spectral_contrast_impl(a: *mut f32, b: *mut f32, length: c_int) -> f64 {
    unsafe { normalize(a, length) };
    unsafe { normalize(b, length) };
    unsafe { dot_product(a, b, length) }
}

/// `double spectral_contrast(float_t *a, float_t *b, int length);`
///
/// Declared in match.h as taking `double *`, but compiled from
/// spectral_contrast.c where `float_t` is `float`, so the arguments are indexed
/// as `f32`. See the module comment.
///
/// For `length <= 0` every loop is empty and the function returns `0.0`; the C
/// compiler folds this into an early `pxor %xmm0,%xmm0; ret`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(a: *mut f32, b: *mut f32, length: c_int) -> f64 {
    unsafe { spectral_contrast_impl(a, b, length) }
}

// ===========================================================================
// match.c   --   here `float_t` == `f64` (typedef from match.h)
// ===========================================================================

/// ```c
/// static double total(float_t *v, int length) {
///     double sum = 0;
///     int i;
///     for(i = 0; i < length; i++) sum += v[i];
///     return sum;
/// }
/// ```
unsafe fn total(v: *const f64, length: c_int) -> f64 {
    let mut sum: f64 = 0.0;
    let mut i: c_int = 0;
    while i < length {
        //     movsd (%rax),%xmm0        ; xmm0 = v[i]
        //     movsd -0x8(%rbp),%xmm1    ; xmm1 = sum
        //     addsd %xmm1,%xmm0         ; xmm0 = v[i] + sum  -> dst is v[i]
        sum = addsd(unsafe { *v.offset(i as isize) }, sum);
        i += 1;
    }
    sum
}

/// ```c
/// static void smoothen(float_t *v, int length) {
///     double sum;
///     int i, j;
///     for(i = 0; i < length; i++) {
///         sum = 0;
///         for(j = 0; j < N_SMOOTH && i + j < length; j++)
///             sum += v[i + j];
///         v[i] = sum / N_SMOOTH;
///     }
/// }
/// ```
///
/// An in-place *forward-looking* box filter. Iteration `i` reads `v[i..i+16]`,
/// i.e. only indices `>= i`, none of which have been overwritten yet (only
/// indices `< i` have), so there is no read-after-write aliasing.
///
/// Note the window is clamped at the end of the array but the divisor stays
/// N_SMOOTH, so the last 15 outputs are progressively attenuated. That is the
/// C behaviour and is preserved. `sum / N_SMOOTH` divides by the integer 16
/// converted to 16.0; GCC emits a multiply by the exactly-representable
/// constant 0.0625, which gives bit-identical results.
///
/// `i + j` is spelled `wrapping_add` so that the two implementations agree even
/// in the degenerate `length > INT_MAX - 16` case, where the C's `int` addition
/// wraps to a negative index (both then read out of bounds and fault) and a
/// Rust debug build would otherwise panic on overflow instead.
unsafe fn smoothen(v: *mut f64, length: c_int) {
    let mut i: c_int = 0;
    while i < length {
        let mut sum: f64 = 0.0;
        let mut j: c_int = 0;
        while j < N_SMOOTH && i.wrapping_add(j) < length {
            //     movsd (%rax),%xmm0        ; xmm0 = v[i + j]
            //     movsd -0x8(%rbp),%xmm1    ; xmm1 = sum
            //     addsd %xmm1,%xmm0         ; dst is v[i + j]
            sum = addsd(unsafe { *v.offset(i.wrapping_add(j) as isize) }, sum);
            j += 1;
        }
        unsafe { *v.offset(i as isize) = divsd(sum, N_SMOOTH as f64) };
        i += 1;
    }
}

/// ```c
/// static void differentiate(float_t *v, int length) {
///     int i;
///     for(i = 0; i < length - 1; i++) v[i] = v[i + 1] - v[i];
///     v[length - 1] = 0;
/// }
/// ```
///
/// Identical to the C for every `length >= 1`, which is the only range in which
/// the C is defined at all:
///
///   * `length == 0` makes the trailing store `v[-1] = 0`. In the real library
///     `v` is a zero-length VLA sitting exactly at `match`'s stack pointer, so
///     `v[-1]` is `preprocess`'s **saved return address** -- the built `.so`
///     therefore returns to `0x0` and dies with SIGSEGV (verified at `-O0` and
///     `-O2`; see ERRORS.md row E8). Reproducing that is impossible and
///     pointless, so the function returns early instead.
///   * `length < 0` never reaches here in the C: `preprocess`'s
///     `memcpy(v, source, length * sizeof(*v))` converts the negative byte count
///     to a huge `size_t` and faults first (row E9).
///
/// The early return also keeps `length - 1` from overflowing for
/// `length == INT_MIN`, which would be UB in C and a debug-build panic in Rust.
unsafe fn differentiate(v: *mut f64, length: c_int) {
    if length <= 0 {
        return;
    }
    let mut i: c_int = 0;
    while i < length - 1 {
        let cur = unsafe { *v.offset(i as isize) };
        let next = unsafe { *v.offset((i + 1) as isize) };
        unsafe { *v.offset(i as isize) = subsd(next, cur) };
        i += 1;
    }
    unsafe { *v.offset((length - 1) as isize) = 0.0 };
}

/// ```c
/// static void preprocess(float_t *v, float_t *source, int length) {
///     memcpy(v, source, length * sizeof(*v));
///     smoothen(v, length);
///     differentiate(v, length);
///     smoothen(v, length);
/// }
/// ```
///
/// `v` is always one of `match`'s own scratch buffers, so the copy never
/// overlaps `source`.
unsafe fn preprocess(v: *mut f64, source: *const f64, length: c_int) {
    if length > 0 {
        unsafe { std::ptr::copy_nonoverlapping(source, v, length as usize) };
    }
    unsafe { smoothen(v, length) };
    unsafe { differentiate(v, length) };
    unsafe { smoothen(v, length) };
}

/// ```c
/// int match(float_t *test, float_t *reference, int bins, double threshold) {
///     float_t t[bins], r[bins];
///     if(total(test, bins) < threshold * total(reference, bins)) return 0;
///     preprocess(t, test, bins);
///     preprocess(r, reference, bins);
///     return spectral_contrast(t, r, bins) >= threshold;
/// }
/// ```
///
/// `t` and `r` are VLAs of `bins` *doubles*; they are replaced here by heap
/// buffers, which is not observable. `threshold` is reused for two unrelated
/// purposes (an energy ratio gate and a correlation cut-off) exactly as in the
/// C.
///
/// The energy gate is evaluated first and short-circuits before any
/// preprocessing, preserving the C's order of checks. Both comparisons are
/// ordered comparisons, so a NaN operand makes them false: a NaN energy total
/// falls through the gate rather than returning 0, and a NaN spectral contrast
/// yields 0. This matches the `comisd`/`ja` and `comisd`/`setae` pair emitted
/// by GCC.
///
/// `spectral_contrast` is then handed pointers to the `double` buffers but
/// treats them as `float` arrays, reading `bins` f32 elements out of the
/// `2 * bins` f32-sized slots the buffers occupy -- always in bounds. See the
/// module comment.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r#match(
    test: *mut f64,
    reference: *mut f64,
    bins: c_int,
    threshold: f64,
) -> c_int {
    // `total(test, bins) < threshold * total(reference, bins)`
    //
    // GCC emits `mulsd %xmm4,%xmm1` for the product, i.e. the reference total is
    // the destination operand and therefore wins the NaN payload, even though
    // `threshold` is written first in the source.
    let total_test = unsafe { total(test, bins) };
    let total_ref = unsafe { total(reference, bins) };
    if total_test < mulsd(total_ref, threshold) {
        return 0;
    }

    // `float_t t[bins], r[bins];`
    let n = if bins > 0 { bins as usize } else { 0 };
    let mut t: Vec<f64> = vec![0.0; n];
    let mut r: Vec<f64> = vec![0.0; n];

    unsafe { preprocess(t.as_mut_ptr(), test, bins) };
    unsafe { preprocess(r.as_mut_ptr(), reference, bins) };

    // The type-confused call: `double *` in, indexed as `float *` inside.
    let contrast = unsafe {
        spectral_contrast_impl(
            t.as_mut_ptr() as *mut f32,
            r.as_mut_ptr() as *mut f32,
            bins,
        )
    };

    (contrast >= threshold) as c_int
}
