//! Rust translation of `c_src/src/lib.c` — colour-blindness (dichromacy)
//! simulation matrices applied in-place to a linear RGB triple.
//!
//! Behaviour is a faithful 1:1 port: the same `f32` arithmetic, in the same
//! order, with the same constants, and the same "unknown impairment values are
//! silently ignored" fallthrough as the original C `switch` (which has no
//! `default` label).

use std::ffi::c_int;

/// Mirrors the C `enum cb_impairment` discriminants from `include/lib.h`.
pub const CB_PROTANOPIA: c_int = 0;
pub const CB_DEUTERANOPIA: c_int = 1;
pub const CB_TRITANOPIA: c_int = 2;

/// `static void Protanopia(float *Red, float *Green, float *Blue)`
///
/// Operates on values, not pointers, so the load-all-then-store-all ordering of
/// the C original is preserved even when the caller passes aliasing pointers.
fn protanopia(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (
        0.17055699213417f32 * r + 0.82944301379913f32 * g + 2.91188E-9f32 * b,
        0.17055699092998f32 * r + 0.82944300785005f32 * g - 5.98679E-10f32 * b,
        -0.00451714424166f32 * r + 0.00451714427397f32 * g + b,
    )
}

/// `static void Deuteranopia(float *Red, float *Green, float *Blue)`
fn deuteranopia(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (
        0.33066007266046f32 * r + 0.66933992517563f32 * g + 3.559314E-9f32 * b,
        0.33066007387760f32 * r + 0.66933992719147f32 * g - 1.758327E-9f32 * b,
        -0.02785538261323f32 * r + 0.02785538252318f32 * g + b,
    )
}

/// `static void Tritanopia(float *Red, float *Green, float *Blue)`
fn tritanopia(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (
        r + 0.12739886310880f32 * g - 0.12739886341072f32 * b,
        -4.486E-11f32 * r + 0.87390929928361f32 * g + 0.12609070101523f32 * b,
        3.1113E-10f32 * r + 0.87390929725848f32 * g + 0.12609070067115f32 * b,
    )
}

/// `void colourblind(cb_impairment Impairment, float *R, float *G, float *B)`
///
/// # Safety
///
/// `r`, `g` and `b` must each be valid, aligned, readable and writable pointers
/// to an `f32`, exactly as required by the C original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colourblind(impairment: c_int, r: *mut f32, g: *mut f32, b: *mut f32) {
    // The C code reads *Red, *Green, *Blue up front in every branch; for an
    // unrecognised impairment it dereferences nothing at all.
    let transform = match impairment {
        CB_PROTANOPIA => protanopia,
        CB_DEUTERANOPIA => deuteranopia,
        CB_TRITANOPIA => tritanopia,
        // No `default:` in the C switch: leave the values untouched.
        _ => return,
    };

    unsafe {
        let (nr, ng, nb) = transform(*r, *g, *b);
        *r = nr;
        *g = ng;
        *b = nb;
    }
}
