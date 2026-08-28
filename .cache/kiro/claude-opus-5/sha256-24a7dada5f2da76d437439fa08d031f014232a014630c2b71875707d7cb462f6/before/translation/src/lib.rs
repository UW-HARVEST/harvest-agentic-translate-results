//! Rust translation of `c_src/src/lib.c`.
//!
//! Behaviour is preserved exactly, including the original naming quirk: the
//! function is called `flip_horizontal` but it swaps whole rows, i.e. it
//! performs a vertical flip. That is not "fixed" here.

use std::ffi::c_int;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

/// Swaps row `i` with row `h - i - 1` for the first `h / 2` rows.
///
/// # Safety
///
/// `img` must point to a valid `cp_image_t` whose `pix` buffer holds at least
/// `w * h` pixels, exactly as required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flip_horizontal(img: *mut cp_image_t) {
    unsafe {
        let pix = (*img).pix;
        let w = (*img).w;
        let h = (*img).h;

        // C integer division truncates toward zero, so a negative `h` yields a
        // non-positive count and the loop body never runs.
        let flips = h / 2;

        let mut i: c_int = 0;
        while i < flips {
            // Mirrors the C pointer arithmetic `pix + w * i` and
            // `pix + w * (h - i - 1)`, including wrap-around on overflow.
            let a = pix.offset((w as isize).wrapping_mul(i as isize));
            let b = pix.offset((w as isize).wrapping_mul((h - i - 1) as isize));

            let mut j: c_int = 0;
            while j < w {
                ptr::swap(a.offset(j as isize), b.offset(j as isize));
                j += 1;
            }

            i += 1;
        }
    }
}
