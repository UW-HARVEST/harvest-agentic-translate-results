//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `hsv_to_rgb`
//!
//! The translation is intentionally literal: operation order, `f32` arithmetic,
//! the `(int)floorf(h)` truncation and the `switch` fall-through structure are
//! reproduced exactly so that the output is bit-identical to the C version.
//!
//! Three details are not expressible in idiomatic Rust and are therefore pinned
//! explicitly (each one is justified where it is defined, and each one is
//! covered by a differential test that fails if it is removed — see
//! `mutation_check.sh`):
//!
//! 1. `c_float_to_int` — C's `(int)float` is undefined out of range; the C
//!    object's actual behaviour is `cvttss2si`, which yields `INT_MIN`.
//! 2. `subss` / `mulss` / `divss` — when both operands of an SSE arithmetic
//!    instruction are NaN, the *destination* operand's payload survives, so the
//!    operand order gcc chose is observable in the output and must be fixed
//!    rather than left to LLVM's register allocator.
//! 3. `load_f32` / `store_f32` — the C performs three unconditional, ordered,
//!    alignment-agnostic 4-byte accesses per pointer; plain Rust loads get sunk
//!    into the `s != 0` branch, which is observable when `src[0]` alone is
//!    unmapped.

use std::ffi::c_float;
use std::ffi::c_int;

/// Reproduces the target machine's native `float` -> `int` truncating
/// conversion, matching what the C compiler emits for `(int)expr`.
///
/// Rust's `as` cast *saturates* (NaN -> 0, out-of-range -> `i32::MIN`/`i32::MAX`),
/// but C leaves the out-of-range case undefined, so the observable behaviour of
/// the C shared object is whatever the hardware does. On x86-64 the compiler
/// emits `cvttss2si`, which raises #IA and returns the "integer indefinite"
/// value `0x8000_0000` for NaN *and* for any value outside `[-2^31, 2^31)`.
/// Faithfully mirroring that is required for bit-identical output (e.g. a NaN
/// hue must select the `switch` `default` arm, not `case 0`).
#[inline]
#[cfg(target_arch = "x86_64")]
fn c_float_to_int(x: c_float) -> c_int {
    if x.is_nan() || x >= 2_147_483_648.0f32 || x <= -2_147_483_648.0f32 {
        c_int::MIN
    } else {
        x as c_int
    }
}

/// Non-x86-64 fallback: AArch64's `fcvtzs` (and most other ISAs) clamp exactly
/// the way Rust's saturating `as` cast does, so the plain cast is already the
/// faithful translation there.
#[inline]
#[cfg(not(target_arch = "x86_64"))]
fn c_float_to_int(x: c_float) -> c_int {
    x as c_int
}

/// Turn a NaN into its quiet form the way the hardware does when it propagates
/// an operand: the quiet bit is set, every other payload bit is preserved.
#[inline]
fn quiet(x: c_float) -> c_float {
    c_float::from_bits(x.to_bits() | 0x0040_0000)
}

/// `subss dest, src` — i.e. `dest - src` with x86 NaN-operand selection.
///
/// The three arithmetic helpers below exist because a NaN *payload* is
/// observable in this API's output, and which payload survives depends on which
/// operand of the SSE instruction is the destination (Intel SDM "Rules for
/// handling NaNs": the first/destination operand wins if it is a NaN, otherwise
/// the second one does, and an SNaN is quieted on the way out). Plain Rust `-`
/// and `*` leave that choice to LLVM's register allocator, which does *not*
/// always agree with gcc's, so the order is pinned here instead. Every ordering
/// was read off `objdump -d` of the C `.so` and is identical for gcc at -O0,
/// -O1, -O2, -O3 and -Os.
#[inline]
fn subss(dest: c_float, src: c_float) -> c_float {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest - src
    }
}

/// `mulss dest, src` — i.e. `dest * src` with x86 NaN-operand selection.
#[inline]
fn mulss(dest: c_float, src: c_float) -> c_float {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest * src
    }
}

/// `divss dest, src` — i.e. `dest / src` with x86 NaN-operand selection.
#[inline]
fn divss(dest: c_float, src: c_float) -> c_float {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest / src
    }
}

/// One `float` load, exactly like the `movss` gcc emits for `src[k]`.
///
/// This is deliberately *not* `*ptr` and *not* `ptr::read_volatile`:
///   * a plain load is sinkable, and LLVM does sink the `src[0]` load into the
///     `s != 0` branch (verified in the generated code). The C loads all three
///     elements unconditionally, in index order, *before* the `s == 0` test, so
///     with a plain load the two objects disagree when `src[0]` is unreadable
///     while `src[1..2]` are and `s == 0`;
///   * `ptr::read_volatile` fixes the order but adds an alignment/non-null
///     precondition (a hard `abort` in debug builds), whereas `movss` — and
///     therefore the C — happily accepts a misaligned pointer and faults with
///     `SIGSEGV`, not `SIGABRT`, on a null one.
///
/// The `asm!` has side effects (no `pure`), so it can be neither removed nor
/// sunk past the branch. No value is transformed, only the access itself is
/// pinned.
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn load_f32(p: *const c_float) -> c_float {
    let out: c_float;
    core::arch::asm!(
        "movss {out}, dword ptr [{p}]",
        p = in(reg) p,
        out = out(xmm_reg) out,
        options(nostack, preserves_flags, readonly),
    );
    out
}

/// One `float` store, exactly like the `movss` gcc emits for `dest[k]`.
/// Keeping the three stores separate and ordered matters for the case where a
/// later element is unwritable: the C commits `dest[0]`/`dest[1]` before it
/// faults on `dest[2]`.
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn store_f32(p: *mut c_float, x: c_float) {
    core::arch::asm!(
        "movss dword ptr [{p}], {x}",
        p = in(reg) p,
        x = in(xmm_reg) x,
        options(nostack, preserves_flags),
    );
}

/// Portable fallback. `[u8; 4]` has alignment 1, so — unlike a volatile `f32`
/// access — this tolerates the misaligned pointers the C tolerates.
#[inline]
#[cfg(not(target_arch = "x86_64"))]
unsafe fn load_f32(p: *const c_float) -> c_float {
    c_float::from_ne_bytes(core::ptr::read_volatile(p.cast::<[u8; 4]>()))
}

#[inline]
#[cfg(not(target_arch = "x86_64"))]
unsafe fn store_f32(p: *mut c_float, x: c_float) {
    core::ptr::write_volatile(p.cast::<[u8; 4]>(), x.to_ne_bytes())
}

/// ```c
/// void hsv_to_rgb(float *dest, const float *src);
/// ```
///
/// `src` is read as `[h, s, v]`, `dest` is written as `[r, g, b]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    // float r, g, b;
    // float f, p, q, t;
    let r: c_float;
    let g: c_float;
    let b: c_float;
    let f: c_float;
    let p: c_float;
    let q: c_float;
    let t: c_float;

    // float h = src[0]; float s = src[1]; float v = src[2];
    // (all three loaded unconditionally and in index order, see `load_f32`)
    let mut h: c_float = load_f32(src.add(0));
    let s: c_float = load_f32(src.add(1));
    let v: c_float = load_f32(src.add(2));

    // int i;
    let i: c_int;

    // if (s == 0) { dest[0] = dest[1] = dest[2] = v; return; }
    if s == 0.0 {
        store_f32(dest.add(0), v);
        store_f32(dest.add(1), v);
        store_f32(dest.add(2), v);
        return;
    }

    // h /= 60.0f;                       divss  %xmm1,%xmm0   ; dest = h
    h = divss(h, 60.0f32);
    // i = (int)floorf(h);               call floorf ; cvttss2si %xmm0,%eax
    i = c_float_to_int(h.floor());
    // f = h - i;                        subss  %xmm1,%xmm0   ; dest = h
    f = subss(h, i as c_float);
    // p = v * (1 - s);                  subss  s,%xmm0(=1.0) ; mulss %xmm1(v),%xmm0(1-s)
    p = mulss(subss(1.0f32, s), v);
    // q = v * (1 - s * f);              mulss  f,%xmm1(=s)   ; subss %xmm1(s*f),%xmm0(=1.0)
    //                                   mulss  %xmm1(v),%xmm0(1-s*f)
    q = mulss(subss(1.0f32, mulss(s, f)), v);
    // t = v * (1 - s * (1 - f));        subss  f,%xmm0(=1.0) ; mulss s,%xmm1(=1-f)
    //                                   subss  %xmm1,%xmm0(=1.0) ; mulss %xmm1(v),%xmm0
    t = mulss(subss(1.0f32, mulss(subss(1.0f32, f), s)), v);

    match i {
        0 => {
            r = v;
            g = t;
            b = p;
        }
        1 => {
            r = q;
            g = v;
            b = p;
        }
        2 => {
            r = p;
            g = v;
            b = t;
        }
        3 => {
            r = p;
            g = q;
            b = v;
        }
        4 => {
            r = t;
            g = p;
            b = v;
        }
        _ => {
            r = v;
            g = p;
            b = q;
        }
    }

    // dest[0] = r; dest[1] = g; dest[2] = b;
    store_f32(dest.add(0), r);
    store_f32(dest.add(1), g);
    store_f32(dest.add(2), b);
}
