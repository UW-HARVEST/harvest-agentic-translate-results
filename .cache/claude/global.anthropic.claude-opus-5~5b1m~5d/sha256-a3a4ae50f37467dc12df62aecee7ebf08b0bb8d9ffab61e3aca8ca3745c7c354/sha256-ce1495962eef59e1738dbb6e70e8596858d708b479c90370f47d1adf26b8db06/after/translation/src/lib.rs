//! Rust translation of the `colourblind` C library (c_src/src/lib.c, c_src/include/lib.h).
//!
//! Public ABI, as reported by `nm -D` on the C shared object: a single exported
//! function, `colourblind`. The three per-impairment transform helpers are
//! `static` in the C source and therefore intentionally NOT exported here.
//!
//! # Behaviour preserved exactly from the C
//!
//! * Each helper reads all three components into locals *before* writing any of
//!   them back, then stores in the order Red, Green, Blue. This is observable
//!   when the caller passes aliasing pointers, so raw pointer reads/writes in
//!   that exact order are used rather than a safe by-value abstraction.
//! * The `switch` in `colourblind` has no `default` arm, so an impairment value
//!   outside {0, 1, 2} is a silent no-op. The C compiler compares the enum as an
//!   unsigned int, so out-of-range and "negative" values are no-ops too.
//! * No NULL checks: the C dereferences unconditionally, so this does too.
//! * Every access goes through `sse::load` / `sse::store`, i.e. a literal
//!   `movss`, which is what GCC emits for `*Red`. `movss` places no requirement
//!   on the address, so the C library transforms a deliberately unaligned
//!   `float*` and faults with `SIGSEGV` on a null one. A plain `*red` in Rust
//!   promises alignment and non-nullness, and any build with
//!   `-C debug-assertions` (every `cargo test` without `--release`) enforces
//!   those promises by aborting with `SIGABRT` — a divergence from the C on
//!   inputs the C accepts or fails differently on. Using the raw instruction
//!   makes both Rust profiles behave exactly like the C. The read/write ORDER is
//!   unchanged: all three components are read before any is written.
//! * All arithmetic is IEEE-754 single precision evaluated left-to-right, with
//!   no fused multiply-add contraction and no promotion to double.
//!
//! # Why the arithmetic goes through `sse`
//!
//! For finite inputs, plain `f32` operators already reproduce the C bit-for-bit.
//! They differ only in the *sign of a NaN result* when two NaN operands with
//! different signs meet in one expression: x86's `ADDSS`/`SUBSS`/`MULSS` return
//! the **destination** operand in that case, so which NaN survives depends on
//! the operand order the compiler happened to pick. LLVM canonicalises those
//! operands differently from GCC (and rewrites `a - c*b` into an add of a
//! negated constant), which flips some NaN sign bits.
//!
//! To stay byte-identical the operand order of every add/sub/mul below is
//! transcribed from the reference build's assembly (`gcc -S -O0`, which is what
//! the project's CMakeLists produces since it sets no `CMAKE_BUILD_TYPE`). The
//! `sse` helpers take their arguments in `(dst, src)` order, mirroring Intel
//! syntax, so each call site reads the same way as the instruction it stands for.

// C-style identifiers are kept verbatim so the Rust surface mirrors lib.h.
#![allow(non_camel_case_types, non_upper_case_globals)]
// The coefficients are transcribed digit-for-digit from lib.c. Several carry more
// decimal digits than an f32 can represent (which is exactly why seven of them
// collapse into shared constants, as GCC's .rodata shows) — but shortening any of
// them would break the audit trail against the C source, so the lint is off.
#![allow(clippy::excessive_precision)]

use std::ffi::c_uint;

/// Scalar single-precision ops with a pinned operand order.
///
/// `dst` is the operand that wins when both operands are NaN, matching
/// `MULSS`/`ADDSS`/`SUBSS`. On x86 these compile to exactly that instruction;
/// elsewhere they fall back to plain operators, which are identical for every
/// input except same-expression NaN-sign ties.
mod sse {
    /// Emits one scalar SSE instruction with `dst` and `src` in exactly the
    /// written order.
    ///
    /// The `_mm_*_ss` intrinsics are not usable here: LLVM folds them back into
    /// scalar `fadd`/`fmul`, then commutes and re-associates the operands (and
    /// rewrites `x - k*y` into an add of a negated constant), which is precisely
    /// the operand order this module exists to pin down. An `asm!` block is
    /// opaque to those rewrites, so the instruction survives verbatim.
    ///
    /// `pure` + `nomem` still let the compiler CSE and drop redundant copies,
    /// which is harmless: equal inputs always give equal results.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    macro_rules! scalar_op {
        ($name:ident, $mnemonic:literal) => {
            #[doc = concat!("`", $mnemonic, " dst, src`")]
            #[inline(always)]
            pub fn $name(dst: f32, src: f32) -> f32 {
                let mut d = dst;
                // SAFETY: SSE is part of the x86-64 baseline, so this
                // instruction is always available. It only reads/writes the two
                // named registers, touches no memory and clobbers no flags,
                // matching the declared options.
                unsafe {
                    core::arch::asm!(
                        concat!($mnemonic, " {d}, {s}"),
                        d = inout(xmm_reg) d,
                        s = in(xmm_reg) src,
                        options(pure, nomem, nostack, preserves_flags),
                    );
                }
                d
            }
        };
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    scalar_op!(mul, "mulss");
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    scalar_op!(add, "addss");
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    scalar_op!(sub, "subss");

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    #[inline(always)]
    pub fn mul(dst: f32, src: f32) -> f32 {
        dst * src
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    #[inline(always)]
    pub fn add(dst: f32, src: f32) -> f32 {
        dst + src
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    #[inline(always)]
    pub fn sub(dst: f32, src: f32) -> f32 {
        dst - src
    }

    /// `movss dst_reg, [p]` — one scalar load, exactly as GCC emits for `*Red`.
    ///
    /// # Why not `*p` or `p.read_unaligned()`
    ///
    /// `movss` places no requirement on the address: it happily loads from an
    /// unaligned or invalid pointer, faulting with `SIGSEGV` only if the page is
    /// unmapped. Rust's pointer reads promise more than that, and a build with
    /// `-C debug-assertions` (i.e. every `cargo test` without `--release`)
    /// enforces the promise, aborting with `SIGABRT` on a misaligned or null
    /// address. That is a visible divergence from the C on inputs the C library
    /// accepts (unaligned) or faults on differently (null), so the raw
    /// instruction is used instead and both profiles behave like the C.
    ///
    /// Deliberately NOT `pure`/`nomem`: the caller may pass aliasing pointers, so
    /// the compiler must not cache or reorder these accesses around each other.
    ///
    /// # Safety
    /// Same contract as the C: `p` is dereferenced unconditionally.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[inline(always)]
    pub unsafe fn load(p: *const f32) -> f32 {
        let v: f32;
        core::arch::asm!(
            "movss {v}, [{p}]",
            v = out(xmm_reg) v,
            p = in(reg) p,
            options(readonly, nostack, preserves_flags),
        );
        v
    }

    /// `movss [p], src_reg` — one scalar store, as GCC emits for `*Red = …`.
    ///
    /// # Safety
    /// As [`load`].
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[inline(always)]
    pub unsafe fn store(p: *mut f32, v: f32) {
        core::arch::asm!(
            "movss [{p}], {v}",
            p = in(reg) p,
            v = in(xmm_reg) v,
            options(nostack, preserves_flags),
        );
    }

    /// # Safety
    /// As the x86 [`load`].
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    #[inline(always)]
    pub unsafe fn load(p: *const f32) -> f32 {
        p.read_unaligned()
    }

    /// # Safety
    /// As the x86 [`store`].
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    #[inline(always)]
    pub unsafe fn store(p: *mut f32, v: f32) {
        p.write_unaligned(v)
    }
}

use sse::{add, load, mul, store, sub};

/// `typedef enum cb_impairment { cbProtanopia, cbDeuteranopia, cbTritanopia }`
///
/// The C enum's values are 0, 1 and 2; the compiler picks `unsigned int` as its
/// compatible type, which is what the ABI passes in the first integer register.
pub type cb_impairment = c_uint;

pub const cbProtanopia: cb_impairment = 0;
pub const cbDeuteranopia: cb_impairment = 1;
pub const cbTritanopia: cb_impairment = 2;

/// ```c
/// static void Protanopia(float *Red, float *Green, float *Blue) {
///     float R = *Red, G = *Green, B = *Blue;
///     *Red = 0.17055699213417f * R + 0.82944301379913f * G + 2.91188E-9f * B;
///     *Green = 0.17055699092998f * R + 0.82944300785005f * G - 5.98679E-10f * B;
///     *Blue = -0.00451714424166f * R + 0.00451714427397f * G + B;
/// }
/// ```
unsafe fn protanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = (load(red), load(green), load(blue));

    // t3 + (t1 + t2)
    let t1 = mul(r, 0.17055699213417f32);
    let t2 = mul(0.82944301379913f32, g);
    let t3 = mul(2.91188E-9f32, b);
    store(red, add(t3, add(t1, t2)));

    // (t2 + t1) - t3
    let t1 = mul(r, 0.17055699092998f32);
    let t2 = mul(0.82944300785005f32, g);
    let t3 = mul(5.98679E-10f32, b);
    store(green, sub(add(t2, t1), t3));

    // (t2 + t1) + B
    let t1 = mul(r, -0.00451714424166f32);
    let t2 = mul(0.00451714427397f32, g);
    store(blue, add(add(t2, t1), b));
}

/// ```c
/// static void Deuteranopia(float *Red, float *Green, float *Blue) {
///     float R = *Red, G = *Green, B = *Blue;
///     *Red = 0.33066007266046f * R + 0.66933992517563f * G + 3.559314E-9f * B;
///     *Green = 0.33066007387760f * R + 0.66933992719147f * G - 1.758327E-9f * B;
///     *Blue = -0.02785538261323f * R + 0.02785538252318f * G + B;
/// }
/// ```
unsafe fn deuteranopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = (load(red), load(green), load(blue));

    // t3 + (t1 + t2)
    let t1 = mul(r, 0.33066007266046f32);
    let t2 = mul(0.66933992517563f32, g);
    let t3 = mul(3.559314E-9f32, b);
    store(red, add(t3, add(t1, t2)));

    // (t2 + t1) - t3
    let t1 = mul(r, 0.33066007387760f32);
    let t2 = mul(0.66933992719147f32, g);
    let t3 = mul(1.758327E-9f32, b);
    store(green, sub(add(t2, t1), t3));

    // (t2 + t1) + B
    let t1 = mul(r, -0.02785538261323f32);
    let t2 = mul(0.02785538252318f32, g);
    store(blue, add(add(t2, t1), b));
}

/// ```c
/// static void Tritanopia(float *Red, float *Green, float *Blue) {
///     float R = *Red, G = *Green, B = *Blue;
///     *Red = R + 0.12739886310880f * G - 0.12739886341072f * B;
///     *Green = -4.486E-11f * R + 0.87390929928361f * G + 0.12609070101523f * B;
///     *Blue = 3.1113E-10f * R + 0.87390929725848f * G + 0.12609070067115f * B;
/// }
/// ```
unsafe fn tritanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = (load(red), load(green), load(blue));

    // (tG + R) - tB
    let t_g = mul(0.12739886310880f32, g);
    let t_b = mul(0.12739886341072f32, b);
    store(red, sub(add(t_g, r), t_b));

    // t3 + (t1 + t2)
    let t1 = mul(r, -4.486E-11f32);
    let t2 = mul(0.87390929928361f32, g);
    let t3 = mul(0.12609070101523f32, b);
    store(green, add(t3, add(t1, t2)));

    // t3 + (t1 + t2)
    let t1 = mul(r, 3.1113E-10f32);
    let t2 = mul(0.87390929725848f32, g);
    let t3 = mul(0.12609070067115f32, b);
    store(blue, add(t3, add(t1, t2)));
}

/// `void colourblind(cb_impairment Impairment, float *R, float *G, float *B);`
///
/// # Safety
///
/// Exactly the C's contract, which imposes no checks of its own:
///
/// * If `impairment` is one of `0`, `1`, `2`, then `r`, `g` and `b` must each be
///   valid for a 4-byte read and a 4-byte write. They need **not** be aligned or
///   distinct — the C dereferences through `movss` and reads all three before
///   writing any, so aliasing is well defined and reproduced here.
/// * For any other `impairment` the pointers are never touched, so they may be
///   null or dangling: like the C, this is then a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colourblind(
    impairment: cb_impairment,
    r: *mut f32,
    g: *mut f32,
    b: *mut f32,
) {
    match impairment {
        cbProtanopia => protanopia(r, g, b),
        cbDeuteranopia => deuteranopia(r, g, b),
        cbTritanopia => tritanopia(r, g, b),
        // No `default` label in the C switch: every other value does nothing.
        _ => {}
    }
}
