//! Translation of the PNG scanline de-filtering part of `lib.c`
//! (`cp_paeth` and the public `unfilter`).
//!
//! The C code's quirks are reproduced verbatim, in particular:
//!   * row 0 filter type 2 (Up) is a no-op,
//!   * row 0 filter type 1/3/4 start at `x = bpp` (leaving the first pixel
//!     untouched) and type 4 calls `cp_paeth(raw[x - bpp], 0, 0)`,
//!   * in the row loop, filter type 1's first loop adds 0 and filter type 2's
//!     two loops are identical,
//!   * all arithmetic wraps like C's `uint8_t +=`.

use core::ffi::c_int;

/// ```c
/// static uint8_t cp_paeth(uint8_t a, uint8_t b, uint8_t c) {
///   int p = a + b - c;
///   int pa = abs(p - a);
///   int pb = abs(p - b);
///   int pc = abs(p - c);
///   return (pa <= pb && pa <= pc) ? a : (pb <= pc) ? b : c;
/// }
/// ```
fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p: c_int = (a as c_int).wrapping_add(b as c_int).wrapping_sub(c as c_int);
    let pa: c_int = p.wrapping_sub(a as c_int).wrapping_abs();
    let pb: c_int = p.wrapping_sub(b as c_int).wrapping_abs();
    let pc: c_int = p.wrapping_sub(c as c_int).wrapping_abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// ```c
/// int unfilter(int w, int h, int bpp, uint8_t *raw);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unfilter(
    w: c_int,
    h: c_int,
    bpp: c_int,
    raw: *mut u8,
) -> c_int {
    let len: c_int = w.wrapping_mul(bpp);
    let mut prev: *mut u8;
    let mut x: c_int;
    let mut raw = raw;

    #[inline(always)]
    unsafe fn get(p: *mut u8, i: c_int) -> u8 {
        *p.wrapping_offset(i as isize)
    }
    #[inline(always)]
    unsafe fn add(p: *mut u8, i: c_int, v: c_int) {
        let q = p.wrapping_offset(i as isize);
        *q = (*q as c_int).wrapping_add(v) as u8;
    }

    if h > 0 {
        let filter = *raw;
        raw = raw.wrapping_offset(1);
        match filter {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    add(raw, x, get(raw, x.wrapping_sub(bpp)) as c_int);
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    add(raw, x, get(raw, x.wrapping_sub(bpp)) as c_int / 2);
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    let v = cp_paeth(get(raw, x.wrapping_sub(bpp)), 0, 0);
                    add(raw, x, v as c_int);
                    x += 1;
                }
            }
            _ => return 0,
        }
    }

    prev = raw;
    raw = raw.wrapping_offset(len as isize);

    let mut y: c_int = 1;
    while y < h {
        let filter = *raw;
        raw = raw.wrapping_offset(1);
        match filter {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    add(raw, x, 0);
                    x += 1;
                }
                while x < len {
                    add(raw, x, get(raw, x.wrapping_sub(bpp)) as c_int);
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    add(raw, x, get(prev, x) as c_int);
                    x += 1;
                }
                while x < len {
                    add(raw, x, get(prev, x) as c_int);
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    add(raw, x, get(prev, x) as c_int / 2);
                    x += 1;
                }
                while x < len {
                    let v = (get(raw, x.wrapping_sub(bpp)) as c_int)
                        .wrapping_add(get(prev, x) as c_int)
                        / 2;
                    add(raw, x, v);
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    add(raw, x, get(prev, x) as c_int);
                    x += 1;
                }
                while x < len {
                    let v = cp_paeth(
                        get(raw, x.wrapping_sub(bpp)),
                        get(prev, x),
                        get(prev, x.wrapping_sub(bpp)),
                    );
                    add(raw, x, v as c_int);
                    x += 1;
                }
            }
            _ => return 0,
        }
        // y++, prev = raw, raw += len
        y += 1;
        prev = raw;
        raw = raw.wrapping_offset(len as isize);
    }
    1
}
