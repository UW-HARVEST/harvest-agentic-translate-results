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
//! # Fidelity notes
//!
//! 1. **Precision / association.** All arithmetic is performed in `f32` (single
//!    precision), exactly as the C code does (float operands with `f`-suffixed
//!    float literals), and with the same left-to-right association
//!    `((a*R + b*G) + c*B)`. The C `.so` is built with `-fPIC` and no
//!    optimisation or `-ffast-math`, so GCC emits plain `mulss`/`addss`/`subss`
//!    with no FMA contraction and no excess precision; `rustc` likewise never
//!    contracts float expressions by default.
//!
//! 2. **Aliasing.** The C parameters are plain `float *`, **not `restrict`**,
//!    and each kernel reads all three inputs into locals *before* performing
//!    any store: `float R = *Red, G = *Green, B = *Blue;`. A caller may
//!    therefore legally pass the same pointer for two or three arguments, and
//!    the C behaviour is well defined (every read observes the original value;
//!    the last store to a given address wins). To reproduce that exactly, the
//!    kernels below operate on raw `*mut f32` and never form a `&mut f32`: two
//!    `&mut` to the same place would be undefined behaviour in Rust and would
//!    let LLVM's `noalias` reorder the loads past the stores, silently
//!    diverging from C.
//!
//! 3. **Alignment.** Reads and writes use `read_unaligned`/`write_unaligned`.
//!    GCC's `movss` has no alignment requirement, so the C build honours a
//!    byte-offset `float *`; a plain `*ptr` in Rust would instead assume 4-byte
//!    alignment (and abort with "misaligned pointer dereference" in debug
//!    builds). On x86-64 both lower to the same `movss`, so this costs nothing.
//!
//! 4. **NaN payload propagation.** This is the subtle one. When an operand is
//!    already a NaN, an x86 SSE arithmetic instruction does not compute
//!    anything — it forwards a NaN operand to the result, and *which* one it
//!    forwards depends on the operand's register role. Per Intel SDM
//!    "Rules for Handling NaNs": for a two-operand instruction the result is
//!    `src1` (the destination register) if that is a NaN, otherwise `src2`,
//!    quieted if it was a signalling NaN. Because GCC's `-O0` register
//!    allocation puts a *different* term in the destination register for
//!    different sub-expressions, the NaN that survives differs per output
//!    channel, e.g. for `Protanopia` the final instruction for `*Red` is
//!    `addss %xmm1,%xmm0` with `%xmm0 = c*B`, so `B`'s NaN payload wins,
//!    whereas for `*Green` it is `subss %xmm1,%xmm0` with `%xmm0 = b*G + a*R`,
//!    so `G`'s payload wins. The resulting priority orders are:
//!
//!    | kernel         | `*Red`  | `*Green` | `*Blue` |
//!    |----------------|---------|----------|---------|
//!    | `Protanopia`   | B, R, G | G, R, B  | G, R, B |
//!    | `Deuteranopia` | B, R, G | G, R, B  | G, R, B |
//!    | `Tritanopia`   | G, R, B | B, R, G  | B, R, G |
//!
//!    Rather than hope that LLVM happens to pick the same register roles, the
//!    kernels below are written as an explicit transcription of the C object
//!    code's instruction sequence, using the [`mulss`], [`addss`] and [`subss`]
//!    helpers which take their arguments in `(dest, src)` order and implement
//!    the hardware's NaN-selection rule in software. For operands that are not
//!    NaN the helpers reduce to plain `*`, `+` and `-`, so every finite,
//!    infinite, zero and subnormal input is unaffected.
//!
//! Some C literals differ only far below `f32` precision and therefore round to
//! the same `f32`; GCC merges them into one constant-pool entry (for instance
//! `0.17055699213417f` and `0.17055699092998f`, or `Tritanopia`'s
//! `0.12739886310880f` and `0.12739886341072f`). Each literal is nevertheless
//! written out here exactly as it appears in the C source; Rust rounds it to
//! the identical `f32`.

#![allow(non_snake_case)]

use std::ffi::c_int;

// The C `cb_impairment` enumerators. The enum is passed as a 32-bit integer.
//
// GCC selects `unsigned int` as the underlying type (all enumerators are
// non-negative), which is observable in the generated `switch`: it compares
// with `cmpl $0x2` followed by an *unsigned* `ja` to the fall-through path.
// Matching on the three values below and ignoring everything else therefore
// reproduces the C dispatch for every 32-bit input, negative values included.
const CB_PROTANOPIA: c_int = 0;
const CB_DEUTERANOPIA: c_int = 1;
const CB_TRITANOPIA: c_int = 2;

// ---------------------------------------------------------------------------
// SSE scalar-single primitives with x86 NaN-selection semantics
// ---------------------------------------------------------------------------

/// Set the quiet bit of a NaN, as x86 does when it forwards a signalling NaN.
/// Idempotent for a NaN that is already quiet.
#[inline(always)]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `MULSS dest, src` — `dest = dest * src`.
///
/// If either operand is a NaN the instruction forwards one of them, preferring
/// `dest` (`src1`); otherwise it multiplies. Arguments are in machine order.
#[inline(always)]
fn mulss(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest * src
    }
}

/// `ADDSS dest, src` — `dest = dest + src`. See [`mulss`] for NaN handling.
#[inline(always)]
fn addss(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest + src
    }
}

/// `SUBSS dest, src` — `dest = dest - src`.
///
/// Note that a forwarded NaN is *not* negated: the instruction returns the NaN
/// operand as-is (quieted), sign bit included. See [`mulss`].
#[inline(always)]
fn subss(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest - src
    }
}

// ---------------------------------------------------------------------------
// The three transform kernels (`static` in C; not exported from the .so)
// ---------------------------------------------------------------------------

/// `static void Protanopia(float *Red, float *Green, float *Blue)`
///
/// ```c
/// float R = *Red, G = *Green, B = *Blue;
/// *Red   =  0.17055699213417f * R + 0.82944301379913f * G + 2.91188E-9f  * B;
/// *Green =  0.17055699092998f * R + 0.82944300785005f * G - 5.98679E-10f * B;
/// *Blue  = -0.00451714424166f * R + 0.00451714427397f * G + B;
/// ```
///
/// # Safety
/// `Red`, `Green` and `Blue` must be valid for unaligned reads and writes of an
/// `f32`. They are allowed to alias one another, exactly as in C.
unsafe fn Protanopia(Red: *mut f32, Green: *mut f32, Blue: *mut f32) {
    // `float R = *Red, G = *Green, B = *Blue;` — all three loads happen before
    // any store, which is what makes aliased arguments well defined.
    let (R, G, B) = unsafe {
        (
            Red.read_unaligned(),
            Green.read_unaligned(),
            Blue.read_unaligned(),
        )
    };

    // *Red: mulss(R,a) ; mulss(b,G) ; addss(t_r,t_g) ; mulss(c,B) ; addss(t_b,s1)
    let red = {
        let t_r = mulss(R, 0.17055699213417f32);
        let t_g = mulss(0.82944301379913f32, G);
        let s1 = addss(t_r, t_g);
        let t_b = mulss(2.91188E-9f32, B);
        addss(t_b, s1)
    };

    // *Green: mulss(R,a) ; mulss(b,G) ; addss(t_g,t_r) ; mulss(c,B) ; subss(s1,t_b)
    let green = {
        let t_r = mulss(R, 0.17055699092998f32);
        let t_g = mulss(0.82944300785005f32, G);
        let s1 = addss(t_g, t_r);
        let t_b = mulss(5.98679E-10f32, B);
        subss(s1, t_b)
    };

    // *Blue: mulss(R,-a) ; mulss(b,G) ; addss(t_g,t_r) ; addss(s1,B)
    let blue = {
        let t_r = mulss(R, -0.00451714424166f32);
        let t_g = mulss(0.00451714427397f32, G);
        let s1 = addss(t_g, t_r);
        addss(s1, B)
    };

    unsafe {
        Red.write_unaligned(red);
        Green.write_unaligned(green);
        Blue.write_unaligned(blue);
    }
}

/// `static void Deuteranopia(float *Red, float *Green, float *Blue)`
///
/// ```c
/// float R = *Red, G = *Green, B = *Blue;
/// *Red   =  0.33066007266046f * R + 0.66933992517563f * G + 3.559314E-9f * B;
/// *Green =  0.33066007387760f * R + 0.66933992719147f * G - 1.758327E-9f * B;
/// *Blue  = -0.02785538261323f * R + 0.02785538252318f * G + B;
/// ```
///
/// The object code has the same instruction shape as [`Protanopia`], so the
/// same `(dest, src)` roles apply.
///
/// # Safety
/// See [`Protanopia`].
unsafe fn Deuteranopia(Red: *mut f32, Green: *mut f32, Blue: *mut f32) {
    let (R, G, B) = unsafe {
        (
            Red.read_unaligned(),
            Green.read_unaligned(),
            Blue.read_unaligned(),
        )
    };

    let red = {
        let t_r = mulss(R, 0.33066007266046f32);
        let t_g = mulss(0.66933992517563f32, G);
        let s1 = addss(t_r, t_g);
        let t_b = mulss(3.559314E-9f32, B);
        addss(t_b, s1)
    };

    let green = {
        let t_r = mulss(R, 0.33066007387760f32);
        let t_g = mulss(0.66933992719147f32, G);
        let s1 = addss(t_g, t_r);
        let t_b = mulss(1.758327E-9f32, B);
        subss(s1, t_b)
    };

    let blue = {
        let t_r = mulss(R, -0.02785538261323f32);
        let t_g = mulss(0.02785538252318f32, G);
        let s1 = addss(t_g, t_r);
        addss(s1, B)
    };

    unsafe {
        Red.write_unaligned(red);
        Green.write_unaligned(green);
        Blue.write_unaligned(blue);
    }
}

/// `static void Tritanopia(float *Red, float *Green, float *Blue)`
///
/// ```c
/// float R = *Red, G = *Green, B = *Blue;
/// *Red   =  R              + 0.12739886310880f * G - 0.12739886341072f * B;
/// *Green = -4.486E-11f * R + 0.87390929928361f * G + 0.12609070101523f * B;
/// *Blue  =  3.1113E-10f * R + 0.87390929725848f * G + 0.12609070067115f * B;
/// ```
///
/// # Safety
/// See [`Protanopia`].
unsafe fn Tritanopia(Red: *mut f32, Green: *mut f32, Blue: *mut f32) {
    let (R, G, B) = unsafe {
        (
            Red.read_unaligned(),
            Green.read_unaligned(),
            Blue.read_unaligned(),
        )
    };

    // *Red: mulss(b,G) ; addss(t_g,R) ; mulss(c,B) ; subss(s1,t_b)
    // (`R` has no coefficient, so GCC folds it straight into the first addss.)
    let red = {
        let t_g = mulss(0.12739886310880f32, G);
        let s1 = addss(t_g, R);
        let t_b = mulss(0.12739886341072f32, B);
        subss(s1, t_b)
    };

    // *Green: mulss(R,-a) ; mulss(b,G) ; addss(t_r,t_g) ; mulss(c,B) ; addss(t_b,s1)
    let green = {
        let t_r = mulss(R, -4.486E-11f32);
        let t_g = mulss(0.87390929928361f32, G);
        let s1 = addss(t_r, t_g);
        let t_b = mulss(0.12609070101523f32, B);
        addss(t_b, s1)
    };

    // *Blue: mulss(R,a) ; mulss(b,G) ; addss(t_r,t_g) ; mulss(c,B) ; addss(t_b,s1)
    let blue = {
        let t_r = mulss(R, 3.1113E-10f32);
        let t_g = mulss(0.87390929725848f32, G);
        let s1 = addss(t_r, t_g);
        let t_b = mulss(0.12609070067115f32, B);
        addss(t_b, s1)
    };

    unsafe {
        Red.write_unaligned(red);
        Green.write_unaligned(green);
        Blue.write_unaligned(blue);
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// `void colourblind(cb_impairment Impairment, float *R, float *G, float *B)`
///
/// Note: the C `switch` has no `default` label, so any value other than the
/// three enumerators leaves `*R`, `*G` and `*B` untouched — and, crucially,
/// never dereferences the pointers at all, so passing null with an invalid
/// impairment is harmless in C. That behaviour is reproduced verbatim (no
/// "bug fixes").
///
/// # Safety
/// For `Impairment` in `0..=2`, all three pointers must be valid for unaligned
/// reads and writes of an `f32`; they may alias. For any other `Impairment` the
/// pointers are never accessed and may be arbitrary, including null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colourblind(Impairment: c_int, R: *mut f32, G: *mut f32, B: *mut f32) {
    // The C code dereferences the pointers unconditionally for the three
    // handled enumerators; it performs no null checks.
    match Impairment {
        CB_PROTANOPIA => unsafe { Protanopia(R, G, B) },
        CB_DEUTERANOPIA => unsafe { Deuteranopia(R, G, B) },
        CB_TRITANOPIA => unsafe { Tritanopia(R, G, B) },
        _ => {}
    }
}
