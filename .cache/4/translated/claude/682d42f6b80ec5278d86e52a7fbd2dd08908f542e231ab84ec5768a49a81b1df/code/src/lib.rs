//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (matches `nm -D` of the C shared object):
//!   * `colourblind`
//!
//! Header (`c_src/include/lib.h`):
//! ```c
//! typedef enum cb_impairment {
//!     cbProtanopia,
//!     cbDeuteranopia,
//!     cbTritanopia,
//! } cb_impairment;
//!
//! void colourblind(cb_impairment Impairment, float *R, float *G, float *B);
//! ```
//!
//! All arithmetic is performed in `f32` (single precision), exactly as the C
//! code does (float operands with `f`-suffixed float literals), and with the
//! same left-to-right association `((a*R + b*G) + c*B)` so the results are
//! bit-identical to the C implementation.

#![allow(non_snake_case)]

use std::ffi::c_int;

// The C `cb_impairment` enumerators. The enum is passed as a 32-bit integer.
const CB_PROTANOPIA: c_int = 0;
const CB_DEUTERANOPIA: c_int = 1;
const CB_TRITANOPIA: c_int = 2;

/// `static void Protanopia(float *Red, float *Green, float *Blue)`
fn protanopia(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let (R, G, B) = (*red, *green, *blue);
    *red = 0.17055699213417f32 * R + 0.82944301379913f32 * G + 2.91188E-9f32 * B;
    *green = 0.17055699092998f32 * R + 0.82944300785005f32 * G - 5.98679E-10f32 * B;
    *blue = -0.00451714424166f32 * R + 0.00451714427397f32 * G + B;
}

/// `static void Deuteranopia(float *Red, float *Green, float *Blue)`
fn deuteranopia(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let (R, G, B) = (*red, *green, *blue);
    *red = 0.33066007266046f32 * R + 0.66933992517563f32 * G + 3.559314E-9f32 * B;
    *green = 0.33066007387760f32 * R + 0.66933992719147f32 * G - 1.758327E-9f32 * B;
    *blue = -0.02785538261323f32 * R + 0.02785538252318f32 * G + B;
}

/// `static void Tritanopia(float *Red, float *Green, float *Blue)`
fn tritanopia(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let (R, G, B) = (*red, *green, *blue);
    *red = R + 0.12739886310880f32 * G - 0.12739886341072f32 * B;
    *green = -4.486E-11f32 * R + 0.87390929928361f32 * G + 0.12609070101523f32 * B;
    *blue = 3.1113E-10f32 * R + 0.87390929725848f32 * G + 0.12609070067115f32 * B;
}

/// `void colourblind(cb_impairment Impairment, float *R, float *G, float *B)`
///
/// Note: the C `switch` has no `default` label, so any value other than the
/// three enumerators leaves `*R`, `*G` and `*B` untouched. That behaviour is
/// reproduced verbatim (no "bug fixes").
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colourblind(
    Impairment: c_int,
    R: *mut f32,
    G: *mut f32,
    B: *mut f32,
) {
    // The C code dereferences the pointers unconditionally for the three
    // handled enumerators; it performs no null checks.
    match Impairment {
        CB_PROTANOPIA => protanopia(unsafe { &mut *R }, unsafe { &mut *G }, unsafe { &mut *B }),
        CB_DEUTERANOPIA => {
            deuteranopia(unsafe { &mut *R }, unsafe { &mut *G }, unsafe { &mut *B })
        }
        CB_TRITANOPIA => tritanopia(unsafe { &mut *R }, unsafe { &mut *G }, unsafe { &mut *B }),
        _ => {}
    }
}
