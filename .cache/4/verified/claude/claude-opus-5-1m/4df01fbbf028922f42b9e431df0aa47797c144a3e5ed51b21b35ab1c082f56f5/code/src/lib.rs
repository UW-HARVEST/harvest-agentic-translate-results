//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `flip_horizontal`
//!
//! Behaviour is reproduced exactly as written in C, including the fact that
//! `flip_horizontal` actually swaps whole *rows* (i.e. it performs a vertical
//! flip) despite its name. That discrepancy is part of the observable
//! behaviour of the original library and is deliberately preserved.

use core::ffi::c_int;

/// `typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// `typedef struct cp_image_t { int w; int h; cp_pixel_t *pix; } cp_image_t;`
#[repr(C)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

/// Byte offset (in elements) for the C expression `pix + w * i`.
///
/// In C, `w * i` is computed in `int` (32-bit) and only then sign-extended for
/// the pointer arithmetic; mirroring that keeps the generated addresses
/// identical to the C version even for pathological inputs.
#[inline]
fn elem_offset(w: c_int, i: c_int) -> isize {
    w.wrapping_mul(i) as isize
}

/// void flip_horizontal(cp_image_t *img);
///
/// Direct translation of `c_src/src/lib.c`:
///
/// ```c
/// void flip_horizontal(cp_image_t *img) {
///     cp_pixel_t *pix = img->pix;
///     int w = img->w;
///     int h = img->h;
///     int flips = h / 2;
///     for (int i = 0; i < flips; ++i) {
///         cp_pixel_t *a = pix + w * i;
///         cp_pixel_t *b = pix + w * (h - i - 1);
///         for (int j = 0; j < w; ++j) {
///             cp_pixel_t t = *a;
///             *a = *b;
///             *b = t;
///             ++a;
///             ++b;
///         }
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flip_horizontal(img: *mut cp_image_t) {
    // The C code unconditionally dereferences `img`; no NULL check is added.
    //
    // These three loads go through `ptr::read` on raw field pointers rather
    // than through plain place expressions (`(*img).pix`). rustc instruments a
    // *place-expression* dereference with a null/alignment check whenever
    // `-C debug-assertions` is on, which turns C's `SIGSEGV` on
    // `flip_horizontal(NULL)` into a Rust panic -> `SIGABRT` (an `extern "C"`
    // fn aborts rather than unwinding). Reading via a raw field pointer
    // reproduces C's plain hardware fault, and matches how the pixel loads in
    // the loop below already behave.
    let pix: *mut cp_pixel_t = unsafe { core::ptr::read(&raw const (*img).pix) };
    let w: c_int = unsafe { core::ptr::read(&raw const (*img).w) };
    let h: c_int = unsafe { core::ptr::read(&raw const (*img).h) };

    // C integer division truncates toward zero, as does Rust's `/`.
    let flips: c_int = h / 2;

    let mut i: c_int = 0;
    while i < flips {
        // `wrapping_offset` / `wrapping_add` are used deliberately instead of
        // `offset` / `add`: C's `pix + w * i` is plain (wrapping) address
        // arithmetic that the surrounding code may legally never dereference,
        // e.g. when `w <= 0` the inner loop below never runs. `offset`/`add`
        // carry a "must not wrap the address space" precondition that is
        // checked at run time whenever `-C debug-assertions` / UB checks are
        // enabled, which would abort for inputs the C accepts silently.
        let mut a: *mut cp_pixel_t = pix.wrapping_offset(elem_offset(w, i));
        let mut b: *mut cp_pixel_t =
            pix.wrapping_offset(elem_offset(w, h.wrapping_sub(i).wrapping_sub(1)));

        let mut j: c_int = 0;
        while j < w {
            // Copy-through swap, matching the C temporary exactly (and
            // therefore also matching its behaviour should `a == b`).
            unsafe {
                let t: cp_pixel_t = a.read();
                a.write(b.read());
                b.write(t);
            }
            a = a.wrapping_add(1);
            b = b.wrapping_add(1);
            j += 1;
        }

        i += 1;
    }
}
