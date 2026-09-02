//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` on the C shared object exactly):
//!   * `match`             -- `int match(double *, double *, int, double)`
//!   * `spectral_contrast` -- `double spectral_contrast(float *, float *, int)`
//!
//! `include/match.h` declares both with `float_t *` parameters and
//! `typedef double float_t`, but `src/spectral_contrast.c` never includes that
//! header, so its `float_t` resolves to `<math.h>`'s `float_t` (== `float` on
//! x86-64 glibc). The two translation units therefore disagree about the
//! element type, and the compiled library really does behave that way. See
//! `spectral_contrast.rs` for details.

mod fp;
#[path = "match.rs"]
mod matching;
mod spectral_contrast;
