//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) whose only
//! public symbol is `pow43` (declared in `include/lib.h` as
//! `float pow43(int x);`). There are no namespace/renaming macros in the public
//! header, so the exported linker symbol is plainly `pow43`.
//!
//! The original C:
//!
//! ```c
//! static const float g_pow43[129 + 16] = { ... };
//!
//! float pow43(int x) {
//!     float frac;
//!     int sign, mult = 256;
//!     if (x < 129) {
//!         return g_pow43[16 + x];
//!     }
//!     if (x < 1024) {
//!         mult = 16;
//!         x <<= 3;
//!     }
//!     sign = 2 * x & 64;
//!     frac = (float)((x & 63) - sign) / ((x & ~63) + sign);
//!     return g_pow43[16 + ((x + sign) >> 6)] *
//!            (1.f + frac * ((4.f / 3) + frac * (2.f / 9))) * mult;
//! }
//! ```
//!
//! Bug-for-bug notes (behaviour deliberately preserved, not "fixed"):
//!
//! * The table lookup is *not* range checked. `x` in `-16..=128` indexes the
//!   table in bounds, but any `x < -16` (and, on the second path, any `x`
//!   large enough that `16 + ((x + sign) >> 6)` exceeds 144) performs an
//!   out-of-bounds read exactly as the C does. This is reproduced with raw
//!   pointer arithmetic so that no bounds-check panic is introduced where the
//!   C would simply read adjacent memory.
//! * `2 * x`, `x << 3` and `x + sign` are signed `int` operations that can
//!   overflow for extreme inputs; the C relies on the usual two's-complement
//!   wrap-around of the target, which is reproduced with wrapping operators
//!   instead of Rust's panicking/UB-free-but-different arithmetic.
//! * All arithmetic is performed in single precision (`f32`), matching C's
//!   `float` semantics on x86-64 (`FLT_EVAL_METHOD == 0`), including the
//!   left-to-right association of the final multiplication chain.

#![allow(non_upper_case_globals)]

use std::ffi::c_int;

/// `static const float g_pow43[129 + 16]` from `c_src/src/lib.c`.
///
/// 145 entries: the first 16 hold the (negative) values used when `pow43` is
/// called with a small negative argument, entry 16 onwards hold `x^(4/3)` for
/// `x = 0 ..= 128`.
static g_pow43: [f32; 129 + 16] = [
    0.0f32, -1.0f32, -2.519842f32, -4.326749f32, -6.349604f32,
    -8.549880f32, -10.902724f32, -13.390518f32, -16.000000f32, -18.720754f32,
    -21.544347f32, -24.463781f32, -27.473142f32, -30.567351f32, -33.741992f32,
    -36.993181f32, 0.0f32, 1.0f32, 2.519842f32, 4.326749f32,
    6.349604f32, 8.549880f32, 10.902724f32, 13.390518f32, 16.000000f32,
    18.720754f32, 21.544347f32, 24.463781f32, 27.473142f32, 30.567351f32,
    33.741992f32, 36.993181f32, 40.317474f32, 43.711787f32, 47.173345f32,
    50.699631f32, 54.288352f32, 57.937408f32, 61.644865f32, 65.408941f32,
    69.227979f32, 73.100443f32, 77.024898f32, 81.000000f32, 85.024491f32,
    89.097188f32, 93.216975f32, 97.382800f32, 101.593667f32, 105.848633f32,
    110.146801f32, 114.487321f32, 118.869381f32, 123.292209f32, 127.755065f32,
    132.257246f32, 136.798076f32, 141.376907f32, 145.993119f32, 150.646117f32,
    155.335327f32, 160.060199f32, 164.820202f32, 169.614826f32, 174.443577f32,
    179.305980f32, 184.201575f32, 189.129918f32, 194.090580f32, 199.083145f32,
    204.107210f32, 209.162385f32, 214.248292f32, 219.364564f32, 224.510845f32,
    229.686789f32, 234.892058f32, 240.126328f32, 245.389280f32, 250.680604f32,
    256.000000f32, 261.347174f32, 266.721841f32, 272.123723f32, 277.552547f32,
    283.008049f32, 288.489971f32, 293.998060f32, 299.532071f32, 305.091761f32,
    310.676898f32, 316.287249f32, 321.922592f32, 327.582707f32, 333.267377f32,
    338.976394f32, 344.709550f32, 350.466646f32, 356.247482f32, 362.051866f32,
    367.879608f32, 373.730522f32, 379.604427f32, 385.501143f32, 391.420496f32,
    397.362314f32, 403.326427f32, 409.312672f32, 415.320884f32, 421.350905f32,
    427.402579f32, 433.475750f32, 439.570269f32, 445.685987f32, 451.822757f32,
    457.980436f32, 464.158883f32, 470.357960f32, 476.577530f32, 482.817459f32,
    489.077615f32, 495.357868f32, 501.658090f32, 507.978156f32, 514.317941f32,
    520.677324f32, 527.056184f32, 533.454404f32, 539.871867f32, 546.308458f32,
    552.764065f32, 559.238575f32, 565.731879f32, 572.243870f32, 578.774440f32,
    585.323483f32, 591.890898f32, 598.476581f32, 605.080431f32, 611.702349f32,
    618.342238f32, 625.000000f32, 631.675540f32, 638.368763f32, 645.079578f32,
];

/// Unchecked `g_pow43[idx]`, mirroring C's unchecked array subscript.
///
/// `idx` is the value of the C expression `16 + x` (resp.
/// `16 + ((x + sign) >> 6)`), which the C code uses verbatim as a subscript
/// without validating it.
#[inline]
fn table(idx: c_int) -> f32 {
    // SAFETY: this is intentionally as (un)safe as the C it translates. For
    // every input for which the C program is well defined the index lies in
    // `0..145`; for the remaining inputs the C performs the same out-of-bounds
    // load, and reproducing it (rather than panicking) is what keeps the two
    // libraries observationally identical.
    unsafe { *g_pow43.as_ptr().offset(idx as isize) }
}

/// `float pow43(int x)` — see `c_src/include/lib.h`.
#[unsafe(no_mangle)]
pub extern "C" fn pow43(x: c_int) -> f32 {
    let mut x: c_int = x;
    let frac: f32;
    let sign: c_int;
    let mut mult: c_int = 256;

    if x < 129 {
        return table(16i32.wrapping_add(x));
    }
    if x < 1024 {
        mult = 16;
        x = x.wrapping_shl(3);
    }
    // sign = 2 * x & 64;  ->  ((2 * x) & 64)
    sign = x.wrapping_mul(2) & 64;
    // frac = (float)((x & 63) - sign) / ((x & ~63) + sign);
    //
    // The numerator is cast to `float`; the integer denominator is then
    // converted to `float` by the usual arithmetic conversions, so the
    // division itself is a single-precision division.
    frac = ((x & 63).wrapping_sub(sign) as f32) / ((x & !63).wrapping_add(sign) as f32);
    // return g_pow43[16 + ((x + sign) >> 6)] *
    //        (1.f + frac * ((4.f / 3) + frac * (2.f / 9))) * mult;
    //
    // `*` is left associative, so the table entry is multiplied by the
    // polynomial first and that product is then scaled by `mult`.
    let poly: f32 = 1.0f32 + frac * ((4.0f32 / 3.0f32) + frac * (2.0f32 / 9.0f32));
    (table(16i32.wrapping_add((x.wrapping_add(sign)) >> 6)) * poly) * (mult as f32)
}

#[cfg(test)]
mod tests {
    use super::pow43;

    /// Raw IEEE-754 bit patterns captured from the original C library
    /// (`gcc`/`clang`, every optimisation level agrees on these).
    const REFERENCE: &[(i32, u32)] = &[
        (-16, 0x00000000),
        (-15, 0xbf800000),
        (-1, 0xc213f904),
        (0, 0x00000000),
        (1, 0x3f800000),
        (2, 0x40214517),
        (127, 0x441f979a),
        (128, 0x44214518),
        (129, 0x4422f3b5),
        (130, 0x4424a371),
        (255, 0x44ca2138),
        (256, 0x44cb2ff5),
        (1023, 0x46210f58),
        (1024, 0x46214518),
        (1025, 0x46217adb),
        (4096, 0x47800000),
        (8191, 0x48213e60),
        (8223, 0x48221588),
    ];

    #[test]
    fn matches_c_bit_for_bit() {
        for &(x, bits) in REFERENCE {
            assert_eq!(
                pow43(x).to_bits(),
                bits,
                "pow43({x}) = {:#010x}, expected {bits:#010x}",
                pow43(x).to_bits()
            );
        }
    }

    #[test]
    fn table_path_is_monotonic_for_valid_inputs() {
        // Sanity check over the whole well-defined domain (x in -16..=8223):
        // the non-negative part of the curve is strictly increasing.
        let mut prev = pow43(1);
        for x in 2..=8223 {
            let cur = pow43(x);
            assert!(cur > prev, "not increasing at x={x}: {prev} -> {cur}");
            prev = cur;
        }
    }
}
