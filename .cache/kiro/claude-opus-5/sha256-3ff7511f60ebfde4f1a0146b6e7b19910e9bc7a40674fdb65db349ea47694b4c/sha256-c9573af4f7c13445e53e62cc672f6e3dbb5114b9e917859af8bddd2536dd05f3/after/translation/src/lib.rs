//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `pow43`
//!
//! Source-level declaration (`c_src/include/lib.h`):
//!   `float pow43(int x);`
//!
//! There are no namespace/renaming macros in the header, so the linker symbol
//! is plainly `pow43`.

use std::ffi::c_int;

/// Direct transcription of `static const float g_pow43[129 + 16]` from
/// `c_src/src/lib.c`. Static in C, so it is *not* part of the exported ABI.
static G_POW43: [f32; 129 + 16] = [
    0.0,
    -1.0,
    -2.519842,
    -4.326749,
    -6.349604,
    -8.549880,
    -10.902724,
    -13.390518,
    -16.000000,
    -18.720754,
    -21.544347,
    -24.463781,
    -27.473142,
    -30.567351,
    -33.741992,
    -36.993181,
    0.0,
    1.0,
    2.519842,
    4.326749,
    6.349604,
    8.549880,
    10.902724,
    13.390518,
    16.000000,
    18.720754,
    21.544347,
    24.463781,
    27.473142,
    30.567351,
    33.741992,
    36.993181,
    40.317474,
    43.711787,
    47.173345,
    50.699631,
    54.288352,
    57.937408,
    61.644865,
    65.408941,
    69.227979,
    73.100443,
    77.024898,
    81.000000,
    85.024491,
    89.097188,
    93.216975,
    97.382800,
    101.593667,
    105.848633,
    110.146801,
    114.487321,
    118.869381,
    123.292209,
    127.755065,
    132.257246,
    136.798076,
    141.376907,
    145.993119,
    150.646117,
    155.335327,
    160.060199,
    164.820202,
    169.614826,
    174.443577,
    179.305980,
    184.201575,
    189.129918,
    194.090580,
    199.083145,
    204.107210,
    209.162385,
    214.248292,
    219.364564,
    224.510845,
    229.686789,
    234.892058,
    240.126328,
    245.389280,
    250.680604,
    256.000000,
    261.347174,
    266.721841,
    272.123723,
    277.552547,
    283.008049,
    288.489971,
    293.998060,
    299.532071,
    305.091761,
    310.676898,
    316.287249,
    321.922592,
    327.582707,
    333.267377,
    338.976394,
    344.709550,
    350.466646,
    356.247482,
    362.051866,
    367.879608,
    373.730522,
    379.604427,
    385.501143,
    391.420496,
    397.362314,
    403.326427,
    409.312672,
    415.320884,
    421.350905,
    427.402579,
    433.475750,
    439.570269,
    445.685987,
    451.822757,
    457.980436,
    464.158883,
    470.357960,
    476.577530,
    482.817459,
    489.077615,
    495.357868,
    501.658090,
    507.978156,
    514.317941,
    520.677324,
    527.056184,
    533.454404,
    539.871867,
    546.308458,
    552.764065,
    559.238575,
    565.731879,
    572.243870,
    578.774440,
    585.323483,
    591.890898,
    598.476581,
    605.080431,
    611.702349,
    618.342238,
    625.000000,
    631.675540,
    638.368763,
    645.079578,
];

/// Reproduces the C table access `g_pow43[i]` verbatim, *including* the
/// out-of-bounds accesses the C code performs for inputs outside the range it
/// implicitly assumes (e.g. `x < -16`, or `x` large enough that
/// `16 + ((x + sign) >> 6)` exceeds the table). In C those reads are undefined
/// behaviour but in practice just load adjacent memory; a bounds-checked Rust
/// index would panic instead, which would be a behavioural divergence. So the
/// raw pointer read is used to mirror the original as closely as possible.
#[inline(always)]
unsafe fn g_pow43_at(i: c_int) -> f32 {
    unsafe { *G_POW43.as_ptr().offset(i as isize) }
}

/// `float pow43(int x);`
///
/// ```c
/// float pow43(int x) {
///     float frac;
///     int sign, mult = 256;
///     if (x < 129) {
///         return g_pow43[16 + x];
///     }
///     if (x < 1024) {
///         mult = 16;
///         x <<= 3;
///     }
///     sign = 2 * x & 64;
///     frac = (float)((x & 63) - sign) / ((x & ~63) + sign);
///     return g_pow43[16 + ((x + sign) >> 6)] *
///            (1.f + frac * ((4.f / 3) + frac * (2.f / 9))) * mult;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn pow43(x: c_int) -> f32 {
    let mut x = x;
    let mut mult: c_int = 256;

    if x < 129 {
        // g_pow43[16 + x]
        return unsafe { g_pow43_at(16i32.wrapping_add(x)) };
    }
    if x < 1024 {
        mult = 16;
        // x <<= 3  (bits shifted out of an int are simply discarded)
        x = x.wrapping_shl(3);
    }

    // sign = 2 * x & 64;   -- `*` binds tighter than `&`
    let sign: c_int = x.wrapping_mul(2) & 64;

    // frac = (float)((x & 63) - sign) / ((x & ~63) + sign);
    // The cast applies to the numerator only; the integer denominator is then
    // converted to float, so this is a single-precision division. A zero
    // denominator yields +/-inf or NaN exactly as it does in C.
    let numerator = ((x & 63).wrapping_sub(sign)) as f32;
    let denominator = ((x & !63i32).wrapping_add(sign)) as f32;
    let frac: f32 = numerator / denominator;

    // g_pow43[16 + ((x + sign) >> 6)] * (1.f + frac * ((4.f/3) + frac*(2.f/9))) * mult
    // `>>` on a signed int is an arithmetic shift with gcc/clang.
    let idx = 16i32.wrapping_add(x.wrapping_add(sign) >> 6);
    let table = unsafe { g_pow43_at(idx) };

    // `4.f / 3` and `2.f / 9` are float-typed constant expressions in C.
    const C43: f32 = 4.0f32 / 3.0f32;
    const C29: f32 = 2.0f32 / 9.0f32;

    table * (1.0f32 + frac * (C43 + frac * C29)) * (mult as f32)
}
