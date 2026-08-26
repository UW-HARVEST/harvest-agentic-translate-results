// Rust translation of the C library in c_src/.
//
// The C build (c_src/CMakeLists.txt) compiles src/match.c and
// src/spectral_contrast.c into a single shared library.  `nm -D` on the
// resulting `.so` exports exactly two public symbols:
//
//     T match
//     T spectral_contrast
//
// (everything else in the two translation units is `static`).
//
// IMPORTANT ABI DETAIL, faithfully reproduced here:
//
//   * `c_src/include/match.h` declares `typedef double float_t;` and so, inside
//     match.c, `float_t` is `double` (8 bytes).
//
//   * `c_src/src/spectral_contrast.c` does NOT include "match.h".  It only
//     includes <math.h>, which (C99, glibc, x86-64 where FLT_EVAL_METHOD == 0)
//     provides `typedef float float_t;`.  Therefore inside
//     spectral_contrast.c, `float_t` is `float` (4 bytes).
//
//   Consequently `match()` builds two `double` VLAs and hands them to
//   `spectral_contrast()`, which reinterprets the very same memory as arrays of
//   `float`.  This is a genuine bug in the original C, but it is observable
//   behaviour, so it is reproduced bit-for-bit rather than "fixed".
//
//   Verified against gcc-generated code for the C library:
//     dot_product: movss / movss / mulss / cvtss2sd / addsd
//                  -> the product is formed in *single* precision and only then
//                     widened to double before accumulating.
//     normalize:   movss / cvtss2sd / divsd / cvtsd2ss / movss
//                  -> load float, widen, divide by the double magnitude,
//                     narrow back to float, store.

#![allow(non_snake_case)]

mod match_;
mod spectral_contrast;

pub use match_::r#match;
pub use spectral_contrast::spectral_contrast;
