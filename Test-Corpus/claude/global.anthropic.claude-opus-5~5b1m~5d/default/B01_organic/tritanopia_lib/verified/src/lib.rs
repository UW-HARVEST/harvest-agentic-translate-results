//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) whose only
//! public (exported) symbol is `tritanopia`, declared in `include/lib.h`:
//!
//! ```c
//! typedef struct cb_rgb_255 {
//!     unsigned char R;
//!     unsigned char G;
//!     unsigned char B;
//! } cb_rgb_255;
//!
//! cb_rgb_255 tritanopia(cb_rgb_255 RGB);
//! ```
//!
//! Everything else in `lib.c` is `static` (internal linkage) and therefore not
//! part of the ABI; those helpers are translated as private Rust functions.
//!
//! The header contains no namespace/renaming macros, so the final linker symbol
//! is plainly `tritanopia`.
//!
//! Numerical fidelity notes (byte-identical output is required):
//!
//! * The C code mixes `float` lvalues with `double` literals. Under the usual
//!   arithmetic conversions each such expression is evaluated in `double` and
//!   only then truncated back to `float` by the explicit `(float)` casts. Every
//!   promotion below is therefore spelled out explicitly (`as f64` / `as f32`)
//!   so the rounding steps match the C exactly.
//! * `pow` is bound directly to the platform libm (the C build links `m`), so
//!   the transcendental results are bit-for-bit the same ones the C library
//!   observes rather than whatever Rust's `f64::powf` might lower to.
//! * The matrix in `Tritanopia` is pure `float` arithmetic and is evaluated in
//!   the same left-to-right association as the C, since IEEE-754 addition is
//!   not associative and Rust (like C without `-ffast-math`) never reassociates.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

// `pow` from the platform math library (the C target links against `m`).
//
// Bound explicitly instead of using `f64::powf` to guarantee the exact same
// implementation - and therefore the exact same bits - as the C library.
#[link(name = "m")]
extern "C" {
    fn pow(x: f64, y: f64) -> f64;
}

/// `typedef struct cb_rgb_255 { unsigned char R, G, B; }` from `include/lib.h`.
///
/// A 3-byte aggregate: under the x86-64 SysV ABI it is classified INTEGER and
/// passed/returned in a single general-purpose register. `repr(C)` plus
/// `extern "C"` makes Rust follow that same platform rule.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct cb_rgb_255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

/// `typedef struct cb_rgb { float R, G, B; }` - file-local in `src/lib.c`.
#[derive(Copy, Clone)]
struct cb_rgb {
    R: f32,
    G: f32,
    B: f32,
}

/// Reproduces C's conversion of a `float` to `unsigned char`.
///
/// `cbDenorm` casts `RGB.R * 255.f + 0.5f` straight to `unsigned char`. For
/// values outside `[0, UCHAR_MAX]` that conversion is undefined behaviour in C,
/// and the tritanopia matrix genuinely produces out-of-range values (e.g. pure
/// blue drives the red channel to about -1.65, i.e. about -419 after scaling),
/// so the observable behaviour of the shared library is whatever the compiler
/// emitted. GCC/Clang on x86-64 lower this to `CVTTSS2SI` into a 32-bit
/// register followed by a read of the low byte (`cvttss2si %xmm,%edx; mov %dl,..`
/// in the reference build's disassembly).
///
/// `CVTTSS2SI` truncates toward zero and, when the result is not representable
/// in the destination (overflow or NaN), yields the "integer indefinite" value
/// `0x8000_0000`. Rust's own `as i32` saturates instead, so the hardware
/// semantics are emulated here. The instruction is then followed by a plain
/// byte truncation, which `as u8` provides.
#[inline]
fn f32_to_u8_c_cast(v: f32) -> u8 {
    // Widen to f64 for the range test: f32 -> f64 is exact, and the bounds
    // +/-2^31 are exactly representable in f64 (unlike in f32, where
    // 2147483647.0 rounds up to 2^31 and would wrongly be accepted).
    let truncated = v.trunc() as f64;
    let as_i32 = if v.is_nan() || truncated < -2147483648.0 || truncated >= 2147483648.0 {
        i32::MIN // 0x8000_0000: the x86 "integer indefinite" result
    } else {
        truncated as i32
    };
    as_i32 as u8
}

/// ```c
/// static cb_rgb cbRemoveGammaRGB(cb_rgb RGB) {
///     cb_rgb Result = {
///         ((float)(RGB.R > 0.04045 ? pow((RGB.R + 0.055) / 1.055, 2.4)
///                                  : RGB.R / 12.92)), ... };
///     return Result;
/// }
/// ```
///
/// The comparison and both arms happen in `double` (the literals are `double`),
/// with a final narrowing to `float`.
fn cbRemoveGammaRGB(RGB: cb_rgb) -> cb_rgb {
    #[inline]
    fn channel(c: f32) -> f32 {
        let c = c as f64;
        // NaN takes the `else` arm, matching C's `>` (and the `comisd`/`ja`
        // pair in the reference build, which falls through when unordered).
        let v = if c > 0.04045 {
            unsafe { pow((c + 0.055) / 1.055, 2.4) }
        } else {
            c / 12.92
        };
        v as f32
    }

    cb_rgb {
        R: channel(RGB.R),
        G: channel(RGB.G),
        B: channel(RGB.B),
    }
}

/// ```c
/// static cb_rgb cbNorm(cb_rgb_255 RGB) {
///     cb_rgb Result = {((float)(RGB.R) / 255.f), ...};
///     return Result;
/// }
/// ```
///
/// `unsigned char` -> `float` is exact, then a single `float` division.
fn cbNorm(RGB: cb_rgb_255) -> cb_rgb {
    cb_rgb {
        R: (RGB.R as f32) / 255.0f32,
        G: (RGB.G as f32) / 255.0f32,
        B: (RGB.B as f32) / 255.0f32,
    }
}

/// ```c
/// static cb_rgb_255 cbDenorm(cb_rgb RGB) {
///     cb_rgb_255 Result = {((unsigned char)((RGB.R) * 255.f + 0.5f)), ...};
///     return Result;
/// }
/// ```
///
/// `float` multiply and add, then the C `float` -> `unsigned char` conversion.
fn cbDenorm(RGB: cb_rgb) -> cb_rgb_255 {
    cb_rgb_255 {
        R: f32_to_u8_c_cast(RGB.R * 255.0f32 + 0.5f32),
        G: f32_to_u8_c_cast(RGB.G * 255.0f32 + 0.5f32),
        B: f32_to_u8_c_cast(RGB.B * 255.0f32 + 0.5f32),
    }
}

/// ```c
/// static cb_rgb cbApplyGammaRGB(cb_rgb RGB) {
///     cb_rgb Result = {((float)(RGB.R > 0.00313080495356037151702786377709
///                                   ? 1.055 * pow((RGB.R), 0.4166666666) - 0.055
///                                   : RGB.R * 12.92)), ... };
///     return Result;
/// }
/// ```
///
/// Note the exponent is the truncated literal `0.4166666666`, not `1.0 / 2.4`;
/// it is reproduced verbatim. Again everything is `double` until the cast.
fn cbApplyGammaRGB(RGB: cb_rgb) -> cb_rgb {
    #[inline]
    fn channel(c: f32) -> f32 {
        let c = c as f64;
        // Threshold written out verbatim as in the C source.
        let v = if c > 0.00313080495356037151702786377709 {
            1.055 * unsafe { pow(c, 0.4166666666) } - 0.055
        } else {
            c * 12.92
        };
        v as f32
    }

    cb_rgb {
        R: channel(RGB.R),
        G: channel(RGB.G),
        B: channel(RGB.B),
    }
}

/// ```c
/// static void Tritanopia(float *Red, float *Green, float *Blue) {
///     float R = *Red, G = *Green, B = *Blue;
///     *Red = R + 0.12739886310880f * G - 0.12739886341072f * B;
///     *Green = -4.486E-11f * R + 0.87390929928361f * G + 0.12609070101523f * B;
///     *Blue = 3.1113E-10f * R + 0.87390929725848f * G + 0.12609070067115f * B;
/// }
/// ```
///
/// All coefficients are `f32` literals, so the whole matrix is single
/// precision. The three inputs are snapshotted first, exactly as the C does,
/// so later rows see the original values rather than the just-written ones.
fn Tritanopia(Red: &mut f32, Green: &mut f32, Blue: &mut f32) {
    let R: f32 = *Red;
    let G: f32 = *Green;
    let B: f32 = *Blue;
    *Red = R + 0.12739886310880f32 * G - 0.12739886341072f32 * B;
    *Green = -4.486E-11f32 * R + 0.87390929928361f32 * G + 0.12609070101523f32 * B;
    *Blue = 3.1113E-10f32 * R + 0.87390929725848f32 * G + 0.12609070067115f32 * B;
}

/// ```c
/// cb_rgb_255 tritanopia(cb_rgb_255 RGB) {
///     cb_rgb RGBNorm = cbRemoveGammaRGB(cbNorm(RGB));
///     Tritanopia(&RGBNorm.R, &RGBNorm.G, &RGBNorm.B);
///     cb_rgb_255 Result = cbDenorm(cbApplyGammaRGB(RGBNorm));
///     return Result;
/// }
/// ```
///
/// The one and only exported symbol of the library.
#[unsafe(no_mangle)]
pub extern "C" fn tritanopia(RGB: cb_rgb_255) -> cb_rgb_255 {
    let mut RGBNorm = cbRemoveGammaRGB(cbNorm(RGB));
    Tritanopia(&mut RGBNorm.R, &mut RGBNorm.G, &mut RGBNorm.B);
    cbDenorm(cbApplyGammaRGB(RGBNorm))
}
