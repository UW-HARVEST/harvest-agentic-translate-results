//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exporting a
//! single public symbol: `premultiply` (declared in `include/lib.h`).
//!
//! The translation reproduces the original semantics exactly, including the
//! integer truncation / wrapping behaviour of the original pointer-arithmetic
//! loop and the f32 arithmetic used for the alpha premultiplication.
//!
//! Reference codegen (the ground truth this file mirrors) is what GCC emits for
//! `c_src/src/lib.c` at `-O0 -fPIC`, which is how `c_src/CMakeLists.txt` builds
//! it (`C_FLAGS = -fPIC`, no optimisation level):
//!
//! * `shl $0x2,%eax`         — `stride = w * sizeof(cp_pixel_t)`, 32-bit wrapping
//! * `imul -0xc(%rbp),%eax`  — `(int)stride * h`, 32-bit wrapping
//! * `add $0x4,%eax`         — `i += sizeof(cp_pixel_t)`, 32-bit wrapping
//! * `cltq` / `lea 0x3(%rax)`— `data[i + k]`: sign-extend `i`, then add `k` in 64 bit
//! * `divss` / `mulss`       — plain IEEE-754 single precision, no FMA, no
//!                             contraction, no reassociation
//! * `cvttss2si %xmm0,%edx` + `mov %dl,(%rax)` — `(uint8_t)(float)`: truncate
//!                             toward zero into a 32-bit int, store the low byte

#![allow(non_camel_case_types)]

use std::ffi::c_int;

/// ```c
/// typedef struct cp_pixel_t {
///     uint8_t r;
///     uint8_t g;
///     uint8_t b;
///     uint8_t a;
/// } cp_pixel_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// ```c
/// typedef struct cp_image_t {
///     int w;
///     int h;
///     cp_pixel_t *pix;
/// } cp_image_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

/// `sizeof(cp_pixel_t)` in the original C code.
const PIXEL_SIZE: c_int = 4;

/// Byte-wise load of `T`, mirroring what a plain C load compiles to.
///
/// A C `-O0` load has no alignment and no null-ness precondition: it either
/// reads the bytes or the hardware faults. `core::ptr::read::<u8>` /
/// `core::ptr::write::<u8>` plus `wrapping_add` have neither an alignment nor a
/// null-pointer debug assertion, so this helper faults with `SIGSEGV` exactly
/// where the C does — in *every* build profile, not only where
/// `-C debug-assertions=off` happens to elide the checks that a plain `(*p).f`
/// dereference would emit.
#[inline]
unsafe fn c_load<T: Copy>(p: *const T) -> T {
    let mut out = core::mem::MaybeUninit::<T>::uninit();
    let src = p as *const u8;
    let dst = out.as_mut_ptr() as *mut u8;
    let n = core::mem::size_of::<T>();
    let mut k = 0usize;
    while k < n {
        core::ptr::write(dst.wrapping_add(k), core::ptr::read(src.wrapping_add(k)));
        k += 1;
    }
    out.assume_init()
}

/// `(uint8_t)f` as GCC implements it: `cvttss2si` into a 32-bit integer
/// (truncation toward zero) followed by storing the low byte.
///
/// Every value that reaches this function is provably in `[0.0, 255.0]`
/// (`a`, `r`, `g`, `b` are all in `[0.0, 1.0]` because the inputs are `u8`
/// divided by `255.0f`), where the x86 instruction and Rust's saturating
/// `as i32` agree bit for bit.
#[inline]
fn c_float_to_u8(f: f32) -> u8 {
    (f as i32) as u8
}

/// ```c
/// void premultiply(cp_image_t *img) {
///     int w = img->w;
///     int h = img->h;
///     int stride = w * sizeof(cp_pixel_t);
///     uint8_t *data = (uint8_t *)img->pix;
///     for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t)) {
///         float a = (float)data[i + 3] / 255.0f;
///         float r = (float)data[i + 0] / 255.0f;
///         float g = (float)data[i + 1] / 255.0f;
///         float b = (float)data[i + 2] / 255.0f;
///         r *= a;
///         g *= a;
///         b *= a;
///         data[i + 0] = (uint8_t)(r * 255.0f);
///         data[i + 1] = (uint8_t)(g * 255.0f);
///         data[i + 2] = (uint8_t)(b * 255.0f);
///     }
/// }
/// ```
///
/// Notes on the faithful reproduction of the original:
///
/// * `stride` is computed with `size_t` arithmetic and then truncated back to
///   `int`, which is equivalent to a wrapping 32-bit multiplication by 4.
/// * The loop bound `(int)stride * h` is a 32-bit `int` multiplication; GCC
///   emits a plain `imul`, i.e. it wraps.
/// * `i += sizeof(cp_pixel_t)` widens `i` to `size_t`, adds 4 and truncates
///   back to `int`: a wrapping 32-bit add.
/// * `data[i + k]` sign-extends `i` to 64 bit and then adds `k`, so the index
///   arithmetic is done in `isize`.
/// * The alpha channel (`data[i + 3]`) is read but never written, exactly as in
///   the original.
/// * `img` is dereferenced unconditionally and `data` is never null-checked,
///   exactly as in the original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn premultiply(img: *mut cp_image_t) {
    // int w = img->w;
    let w: c_int = c_load(core::ptr::addr_of!((*img).w));
    // int h = img->h;
    let h: c_int = c_load(core::ptr::addr_of!((*img).h));
    // int stride = w * sizeof(cp_pixel_t);
    let stride: c_int = w.wrapping_mul(PIXEL_SIZE);
    // uint8_t *data = (uint8_t *)img->pix;
    let data: *mut u8 = c_load(core::ptr::addr_of!((*img).pix)) as *mut u8;

    // (int)stride * h
    let end: c_int = stride.wrapping_mul(h);

    let mut i: c_int = 0;
    while i < end {
        // `data[i + off]`: `cltq` then `lea off(%rax)`.
        let byte = |off: isize| -> *mut u8 { data.wrapping_offset(i as isize + off) };

        // float a = (float)data[i + 3] / 255.0f;
        let a: f32 = f32::from(core::ptr::read(byte(3))) / 255.0f32;
        // float r = (float)data[i + 0] / 255.0f;
        let mut r: f32 = f32::from(core::ptr::read(byte(0))) / 255.0f32;
        // float g = (float)data[i + 1] / 255.0f;
        let mut g: f32 = f32::from(core::ptr::read(byte(1))) / 255.0f32;
        // float b = (float)data[i + 2] / 255.0f;
        let mut b: f32 = f32::from(core::ptr::read(byte(2))) / 255.0f32;

        // r *= a; g *= a; b *= a;
        r *= a;
        g *= a;
        b *= a;

        // data[i + 0] = (uint8_t)(r * 255.0f);
        core::ptr::write(byte(0), c_float_to_u8(r * 255.0f32));
        // data[i + 1] = (uint8_t)(g * 255.0f);
        core::ptr::write(byte(1), c_float_to_u8(g * 255.0f32));
        // data[i + 2] = (uint8_t)(b * 255.0f);
        core::ptr::write(byte(2), c_float_to_u8(b * 255.0f32));

        // i += sizeof(cp_pixel_t)
        i = i.wrapping_add(PIXEL_SIZE);
    }
}
