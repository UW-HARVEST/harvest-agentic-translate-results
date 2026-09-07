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
// is reproduced exactly rather than fixed. Confirmed against the disassembly of
// the canonical C shared object (`objdump -d`), where `dot_product` scales its
// index by **4**, not 8, and multiplies with `mulss`:
//
//     14e6: lea      0x0(,%rax,4),%rdx   ; index * sizeof(float)
//     14f5: movss    (%rax),%xmm1        ; xmm1 = a[i]           (f32)
//     150d: movss    (%rax),%xmm0        ; xmm0 = b[i]           (f32)
//     1511: mulss    %xmm1,%xmm0         ; SINGLE-precision product, dst = b[i]
//     1515: cvtss2sd %xmm0,%xmm0         ; widen the f32 product to f64
//     1519: movsd    -0x8(%rbp),%xmm1    ; xmm1 = sum
//     151e: addsd    %xmm1,%xmm0         ; accumulate in DOUBLE, dst = product
//
// and, in `normalize`:
//
//     1594: movss    (%rax),%xmm0        ; xmm0 = v[i]           (f32)
//     1598: cvtss2sd %xmm0,%xmm0         ; widen to f64
//     159c: divsd    -0x10(%rbp),%xmm0   ; divide in DOUBLE, dst = v[i]
//     15b5: cvtsd2ss %xmm0,%xmm0         ; round the quotient back to f32
//     15b9: movss    %xmm0,(%rax)        ; store as f32
//
// Every exported function below therefore operates on raw pointers rather than
// Rust slices: the C API permits the two argument arrays to alias (e.g.
// `spectral_contrast(v, v, n)`), and building two overlapping `&mut [T]`
// would be undefined behaviour in Rust even though it is well-defined in C.
//
// ---------------------------------------------------------------------------
// VERIFICATION
//
// Differentially tested through `dlopen` against the *canonical* C shared
// object -- the one produced by exactly the documented build,
//
//     cd c_src && mkdir -p build && cd build &&
//     cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
//
// which sets no CMAKE_BUILD_TYPE and therefore compiles at `-O0`. See
// `tests/{configs,errors,soak}.rs`, `CONFIGS.md`, `ERRORS.md`, `SYMBOLS.md`.
// Coverage: 32 configuration rows and 18 error rows, plus a 1,000,000-case
// fixed-seed soak. Every returned `int` / IEEE-754 bit pattern and every byte
// of every in-place-mutated buffer is identical.
//
// SSE OPERAND ROLES ARE TAKEN FROM THE DISASSEMBLY, NOT GUESSED. When both
// operands of an SSE arithmetic instruction are NaN, the *destination*
// operand's payload survives (quieted). At `-O0` GCC 11.5 makes the freshly
// loaded array ELEMENT the destination and the accumulator the source:
//
//     total / smoothen:  movsd (v,i)->xmm0; movsd sum->xmm1; addsd %xmm1,%xmm0
//     dot_product:       movss (a,i)->xmm1; movss (b,i)->xmm0; mulss %xmm1,%xmm0
//                        cvtss2sd %xmm0,%xmm0; movsd sum->xmm1; addsd %xmm1,%xmm0
//
// so the LAST NaN to enter an accumulator wins, and in the product it is `b`
// that wins, not `a`. The helpers below encode exactly that. (An earlier
// revision of this file assumed the opposite -- accumulator-as-destination --
// and diverged from the canonical library on the very first NaN-valued input.)
//
// KNOWN, IRREDUCIBLE COMPILER SENSITIVITY. The NaN *payload* that
// `spectral_contrast` returns is not fixed by the C source; it is whatever the
// compiler's operand ordering makes it. Measured across gcc 11.5
// `-O0/-O1/-O2/-O3/-Os` of the same sources: `-O0` is one group and `-O1`+ are
// another, and the two disagree. No single translation can match both, so this
// one matches the canonical `-O0` artifact the documented build produces.
//
// Everything the C language actually *specifies* is bit-exact at EVERY
// optimization level: re-running the full suite with the C object rebuilt at
// `-O1`, `-O2`, `-O3` and `-Os` leaves all `match` rows (26 of 26 configuration
// rows that call `match`, all error rows, and the 500,000-case `soak_match`)
// passing unchanged -- `match`'s `int` result never differs, because it only
// ever compares the contrast, and a NaN of any payload compares the same way.
// The only divergence at `-O1`+ is the payload bits of a NaN returned directly
// out of `spectral_contrast`.
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
// The canonical library (gcc 11.5, `-O0`, i.e. `cmake ..` with no build type)
// compiles `sum += v[i]` by loading the ELEMENT into the destination register
// and the accumulator into the source register:
//
//     movsd  (%rax),%xmm0             ; xmm0 = v[i]
//     movsd  -0x8(%rbp),%xmm1         ; xmm1 = sum
//     addsd  %xmm1,%xmm0              ; xmm0 = v[i] + sum      <-- dst = v[i]
//     movsd  %xmm0,-0x8(%rbp)         ; sum  = xmm0
//
// so the LAST NaN to enter the accumulator wins and is carried to the end.
// `dot_product` does the same, and additionally makes `b[i]` the destination of
// the `mulss`, so `b`'s payload beats `a`'s.
//
// Rust/LLVM is otherwise free to commute floating-point addition (it treats the
// NaN payload choice as non-deterministic). Routing every arithmetic operation
// through these helpers makes the operand roles explicit at the source level, so
// the result no longer depends on LLVM's choice of operand order and reproduces
// the C object's payloads on every input.
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
unsafe fn dot_product(a: *const f32, b: *const f32, length: c_int) -> f64 {
    let mut sum: f64 = 0.0;
    let mut i: c_int = 0;
    while i < length {
        let av = unsafe { *a.offset(i as isize) };
        let bv = unsafe { *b.offset(i as isize) };
        // f32 multiply, then widen to f64: mulss + cvtss2sd + addsd.
        //
        // Operand roles taken verbatim from the shipped object code (see the
        // module comment): `movss (a)->xmm1; movss (b)->xmm0; mulss %xmm1,%xmm0`
        // makes *b* the destination, and `movsd sum->xmm1; addsd %xmm1,%xmm0`
        // makes the *product* the destination of the accumulation.
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
        // `movsd (v,i)->xmm0; movsd sum->xmm1; addsd %xmm1,%xmm0` -- the freshly
        // loaded ELEMENT is the destination operand, so the LAST NaN seen wins
        // the payload, not the first.
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
unsafe fn smoothen(v: *mut f64, length: c_int) {
    let mut i: c_int = 0;
    while i < length {
        let mut sum: f64 = 0.0;
        let mut j: c_int = 0;
        // `i + j` / `length - 1` use wrapping arithmetic to mirror the C's
        // `add %edx,%eax` / `sub $0x1,%eax` exactly, and so that a debug build
        // of this crate cannot panic on `int` inputs (`INT_MAX`, `INT_MIN`) on
        // which the release build, and the C, simply wrap.
        while j < N_SMOOTH && i.wrapping_add(j) < length {
            // Same operand roles as `total`: the element is the `addsd`
            // destination (`addsd %xmm1,%xmm0` with xmm0 = v[i+j]).
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
/// The trailing store is guarded by `length >= 1` here. In C, `length == 0`
/// makes it `v[-1] = 0`, an out-of-bounds write.
///
/// MEASURED (do not take the earlier claim that this is harmless -- it is not):
/// with `bins == 0`, `match`'s two zero-length VLAs both land at `%rsp`, so
/// `v[-1]` is `%rsp - 8`, which is precisely the slot holding `preprocess`'s
/// return address into `match`. Zeroing it makes `preprocess` return to address
/// 0 and the process dies with SIGSEGV. Verified standalone against the C `.so`:
/// exit status 139. So C `match(_, _, 0, _)` is *always* fatal, for every
/// `threshold` (the energy gate can never fire first: it needs
/// `0.0 < threshold * 0.0`, and that product is `0.0` or a QNaN).
///
/// There is therefore no C return value to reproduce for `bins <= 0`; the
/// behaviour is undefined and observably fatal. Corrupting this crate's own
/// caller's stack to imitate it would be gratuitous, so the store is skipped
/// for `length <= 0`. Covered by `tests/errors.rs::e5_match_bins_zero_crashes_in_c`
/// and `::e8_match_negative_bins_crashes_in_c`, which assert the C crash in a
/// child process rather than pretend the input is valid.
unsafe fn differentiate(v: *mut f64, length: c_int) {
    let mut i: c_int = 0;
    while i < length.wrapping_sub(1) {
        let cur = unsafe { *v.offset(i as isize) };
        let next = unsafe { *v.offset((i + 1) as isize) };
        unsafe { *v.offset(i as isize) = subsd(next, cur) };
        i += 1;
    }
    if length >= 1 {
        unsafe { *v.offset((length - 1) as isize) = 0.0 };
    }
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
