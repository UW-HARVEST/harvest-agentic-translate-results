//! Rust translation of `c_src/src/lib.c` (minimp3-style synthesis helper).
//!
//! The C library globs `src/lib.c` into a single shared object whose only
//! exported public symbol is `synth_pair` (verified with `nm -D`). The header
//! `include/lib.h` defines no namespace-renaming macros, so the linker symbol
//! matches the source-level name exactly.
//!
//! Behaviour is reproduced bit-for-bit, including the C code's rounding /
//! clipping quirks; no "bug fixes" are applied.

#![allow(non_snake_case)]

use std::ffi::c_int;

/// `typedef int16_t mp3d_sample_t;` from `include/lib.h`.
pub type Mp3dSampleT = i16;

/// Faithful port of the file-local (`static`) helper:
///
/// ```c
/// static int16_t mp3d_scale_pcm(float sample) {
///     if (sample >= 32766.5)  return (int16_t)32767;
///     if (sample <= -32767.5) return (int16_t)-32768;
///     int16_t s = (int16_t)(sample + .5f);
///     s -= (s < 0);
///     return s;
/// }
/// ```
///
/// Notes on exact-semantics preservation:
/// * The two literals `32766.5` / `-32767.5` are `double` in C, so the standard
///   promotes the `float` operand before comparing. GCC then notices that both
///   constants are exactly representable in `float` and emits a single-precision
///   `comiss` against `float` constants instead (verified by disassembling
///   `lib.c.o`). The two spellings are therefore equivalent; `f64::from` keeps
///   the source-level reading, and `tests/exhaustive.rs` confirms the choice is
///   unobservable across all 2^32 `f32` inputs.
/// * NaN fails both comparisons (every NaN compare is false), so a NaN sample
///   reaches the cast.
/// * `sample + .5f` is a single-precision addition (the literal is `float`).
/// * The C cast truncates toward zero into `int` and then narrows to
///   `int16_t`; `as i32 as i16` reproduces that, including the x86-64 NaN case
///   (`cvttss2si` -> `0x8000_0000`, whose low 16 bits are `0`, matching Rust's
///   saturating `NaN as i32 == 0`).
/// * `s -= (s < 0)` is performed on the promoted `int` and stored back into an
///   `int16_t`; `wrapping_sub` matches GCC's narrowing behaviour. Note this
///   truncates toward zero FIRST, so e.g. `-1.0 -> 0`, not `-1`.
#[inline]
fn mp3d_scale_pcm(sample: f32) -> i16 {
    if f64::from(sample) >= 32766.5 {
        return 32767i16;
    }
    if f64::from(sample) <= -32767.5 {
        return -32768i16;
    }
    let s = (sample + 0.5f32) as i32 as i16;
    s.wrapping_sub(i16::from(s < 0))
}

/// Faithful port of the single public entry point:
///
/// ```c
/// void synth_pair(mp3d_sample_t *pcm, int nch, const float *z);
/// ```
///
/// The accumulation order, the mixed add/subtract pairings and the integer
/// coefficients (all exactly representable in `f32`) are preserved verbatim so
/// that the emitted single-precision rounding sequence is identical.
///
/// # Safety
///
/// Same contract as the C function: `pcm` must be writable at offsets `0` and
/// `16 * nch`, and `z` must be readable for the strided taps used below
/// (`z[0 ..= 14 * 64]` and, after the `z += 2` advance, `z[2 ..= 2 + 14 * 64]`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_pair(pcm: *mut Mp3dSampleT, nch: c_int, z: *const f32) {
    // `z[i * 64]` in the first block.
    //
    // `wrapping_add` + `ptr::read` (rather than `add` + `*p`) is deliberate:
    // * `wrapping_add` emits a plain address computation, matching C's pointer
    //   arithmetic on whatever the caller passes, without asserting LLVM
    //   `inbounds` for offsets the caller may have made nonsensical;
    // * `ptr::read`/`ptr::write` skip the debug-only "null pointer dereference"
    //   UB check that `*p` carries under `-Cdebug-assertions`, so an unchecked
    //   null argument faults with `SIGSEGV` exactly like the C does instead of
    //   aborting with `SIGABRT` in debug builds.
    let tap = |i: usize| -> f32 { unsafe { std::ptr::read(z.wrapping_add(i * 64)) } };

    let mut a: f32;
    a = (tap(14) - tap(0)) * 29.0f32;
    a += (tap(1) + tap(13)) * 213.0f32;
    a += (tap(12) - tap(2)) * 459.0f32;
    a += (tap(3) + tap(11)) * 2037.0f32;
    a += (tap(10) - tap(4)) * 5153.0f32;
    a += (tap(5) + tap(9)) * 6574.0f32;
    a += (tap(8) - tap(6)) * 37489.0f32;
    a += tap(7) * 75038.0f32;
    unsafe { std::ptr::write(pcm, mp3d_scale_pcm(a)) };

    // `z += 2;` -- every subsequent tap is shifted by two floats.
    let tap2 = |i: usize| -> f32 { unsafe { std::ptr::read(z.wrapping_add(2 + i * 64)) } };

    a = tap2(14) * 104.0f32;
    a += tap2(12) * 1567.0f32;
    a += tap2(10) * 9727.0f32;
    a += tap2(8) * 64019.0f32;
    a += tap2(6) * -9975.0f32;
    a += tap2(4) * -45.0f32;
    a += tap2(2) * 146.0f32;
    a += tap2(0) * -5.0f32;
    // `pcm[16 * nch]` -- the subscript `16 * nch` is computed in `int` in C, so
    // the product WRAPS at 32 bits (GCC's two's-complement behaviour for the
    // signed overflow) and only then gets sign-extended for the pointer add.
    // Computing `16 * nch` in `isize` instead would offset by up to ~34e9
    // elements for `nch == INT_MAX` rather than by `-16`.
    let offset = 16i32.wrapping_mul(nch) as isize;
    unsafe { std::ptr::write(pcm.wrapping_offset(offset), mp3d_scale_pcm(a)) };
}
