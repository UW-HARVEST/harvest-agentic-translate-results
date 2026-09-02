//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) with a
//! single public header (`include/lib.h`) that exports exactly one public
//! symbol: `flip_horizontal`. There are no namespace / renaming macros in the
//! header, so the linker symbol matches the source-level name verbatim.

use core::ffi::c_int;

/// Mirror of the C `cp_pixel_t`:
///
/// ```c
/// typedef struct cp_pixel_t {
///     uint8_t r;
///     uint8_t g;
///     uint8_t b;
///     uint8_t a;
/// } cp_pixel_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Mirror of the C `cp_image_t`:
///
/// ```c
/// typedef struct cp_image_t {
///     int w;
///     int h;
///     cp_pixel_t *pix;
/// } cp_image_t;
/// ```
#[repr(C)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

/// Translation of:
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
///
/// Note: despite its name, the C function swaps whole rows (a *vertical* flip).
/// That behavior is reproduced exactly and deliberately not "fixed".
///
/// # Safety
///
/// `img` must be a valid pointer to a `cp_image_t` whose `pix` buffer holds at
/// least `w * h` pixels, exactly as the C implementation requires. A null or
/// invalid `img` faults here just as it does in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flip_horizontal(img: *mut cp_image_t) {
    // `img->pix`, `img->w`, `img->h` are read unconditionally by the C code,
    // before any bounds or sanity checking (of which there is none). A NULL
    // `img` therefore faults here, exactly as it does in C.
    let pix: *mut cp_pixel_t = (*img).pix;
    let w: c_int = (*img).w;
    let h: c_int = (*img).h;

    // C integer division truncates toward zero, so a negative `h` yields a
    // non-positive `flips` and the outer loop body never runs. `INT_MIN / 2`
    // is well-defined (only `INT_MIN / -1` traps), so a plain division is the
    // faithful translation.
    let flips: c_int = h / 2;

    let mut i: c_int = 0;
    while i < flips {
        // `w * i` and `w * (h - i - 1)` are `int` expressions in C; use
        // wrapping arithmetic to mirror the two's-complement result the C
        // compiler emits, then widen for the pointer offset.
        let off_a = w.wrapping_mul(i) as isize;
        let off_b = w.wrapping_mul(h.wrapping_sub(i).wrapping_sub(1)) as isize;

        // The inner loop runs `w` times; for `w <= 0` it does nothing.
        //
        // `wrapping_offset` / `wrapping_add` are used instead of
        // `offset` / `add` on purpose: for the degenerate inputs the C
        // tolerates (`w < 0`, `w == INT_MIN`, `pix == NULL` with `w <= 0`) the
        // C code *forms* these addresses without ever dereferencing them.
        // The wrapping variants reproduce the same two's-complement address
        // arithmetic without imposing Rust's stricter in-bounds requirement,
        // so those inputs stay well-defined here exactly as they are in C.
        let mut a: *mut cp_pixel_t = pix.wrapping_offset(off_a);
        let mut b: *mut cp_pixel_t = pix.wrapping_offset(off_b);
        let mut j: c_int = 0;
        while j < w {
            // Element-wise temp-copy swap, matching the C loop exactly. Using
            // `ptr::swap` keeps the semantics correct even for the degenerate
            // case where the two rows would overlap. (They cannot: `a == b`
            // would require `i == h - i - 1`, i.e. `i == flips`, which the
            // loop guard excludes.)
            core::ptr::swap(a, b);
            a = a.wrapping_add(1);
            b = b.wrapping_add(1);
            j = j.wrapping_add(1);
        }

        i = i.wrapping_add(1);
    }
}
