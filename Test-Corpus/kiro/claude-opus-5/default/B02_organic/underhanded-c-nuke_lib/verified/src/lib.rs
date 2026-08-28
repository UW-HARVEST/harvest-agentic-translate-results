//! Rust translation of `c_src` (MIT Lincoln Laboratory spectral matcher).
//!
//! The C build compiles two translation units, `src/match.c` and
//! `src/spectral_contrast.c`, into one shared library exporting `match` and
//! `spectral_contrast`.
//!
//! # The `float_t` divergence (faithfully reproduced, not fixed)
//!
//! `include/match.h` contains `typedef double float_t;`, so every `float_t` in
//! `match.c` (which includes `match.h`) is a **`double`**.
//!
//! `spectral_contrast.c` includes only `<math.h>` and *never* includes
//! `match.h`. It therefore picks up the C99 `float_t` from `<math.h>`, which on
//! x86-64 Linux (`FLT_EVAL_METHOD == 0`) is a **`float`**. Its functions really
//! do load, multiply and store 4-byte floats -- verified against the emitted
//! object code (`movss` / `mulss` / `cvtss2sd` / `divsd` / `cvtsd2ss`).
//!
//! The two units still link, because the mismatch is only in the pointee type,
//! and the header's declaration makes `match.c` pass `double *` buffers to a
//! function that reinterprets them as `float *`. So `spectral_contrast` chews
//! through only the first `length * 4` bytes of each `length`-element `double`
//! array and fills the leading half with bit-recycled garbage. That is the
//! behaviour the C library has, so it is the behaviour reproduced here: the
//! Rust `spectral_contrast` takes `*mut c_float`, and `match` hands it its
//! `f64` scratch buffers cast to `*mut c_float`.

mod matcher;
mod spectral_contrast;
mod sse;

/// Reconstructs a shared slice from a C pointer, tolerating the null/empty
/// case that `slice::from_raw_parts` forbids but C accepts.
///
/// # Safety
/// For `len > 0`, `p` must be valid for reads of `len` elements.
unsafe fn slice_from_raw<'a, T>(p: *const T, len: usize) -> &'a [T] {
    if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(p, len) }
    }
}

/// C's `int`-typed lengths are used unchecked as array bounds. A non-positive
/// length makes every C loop body run zero times, so clamp to zero.
fn clamp_len(length: core::ffi::c_int) -> usize {
    if length > 0 { length as usize } else { 0 }
}
