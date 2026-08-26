use std::ffi::c_int;

#[inline]
fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    // C: int p = a + b - c; with uint8_t a,b,c promoted to int
    let p: i32 = a as i32 + b as i32 - c as i32;
    let pa: i32 = (p - a as i32).abs();
    let pb: i32 = (p - b as i32).abs();
    let pc: i32 = (p - c as i32).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// PNG row unfilter. Mirrors the original C `unfilter` exactly, including its
/// (slightly odd) behavior for the very first row.
///
/// Signature must remain: `int unfilter(int w, int h, int bpp, uint8_t *raw)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unfilter(
    w: c_int,
    h: c_int,
    bpp: c_int,
    raw: *mut u8,
) -> c_int {
    let len: c_int = w * bpp;
    let mut raw: *mut u8 = raw;
    let mut prev: *mut u8;
    let mut x: c_int;

    if h > 0 {
        // Read filter byte and advance: equivalent to C `*raw++`
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    let v = (*raw.offset(x as isize))
                        .wrapping_add(*raw.offset((x - bpp) as isize));
                    *raw.offset(x as isize) = v;
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    let v = (*raw.offset(x as isize))
                        .wrapping_add(*raw.offset((x - bpp) as isize) / 2);
                    *raw.offset(x as isize) = v;
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    let v = (*raw.offset(x as isize)).wrapping_add(cp_paeth(
                        *raw.offset((x - bpp) as isize),
                        0,
                        0,
                    ));
                    *raw.offset(x as isize) = v;
                    x += 1;
                }
            }
            _ => return 0,
        }
    }

    prev = raw;
    raw = raw.add(len as usize);

    let mut y: c_int = 1;
    while y < h {
        // *raw++
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                // for (x = 0; x < bpp; x++) raw[x] += 0;  (no-op, just runs x to bpp)
                x = 0;
                while x < bpp {
                    // raw[x] += 0 — no-op, kept for fidelity
                    let v = (*raw.offset(x as isize)).wrapping_add(0);
                    *raw.offset(x as isize) = v;
                    x += 1;
                }
                while x < len {
                    let v = (*raw.offset(x as isize))
                        .wrapping_add(*raw.offset((x - bpp) as isize));
                    *raw.offset(x as isize) = v;
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    let v = (*raw.offset(x as isize))
                        .wrapping_add(*prev.offset(x as isize));
                    *raw.offset(x as isize) = v;
                    x += 1;
                }
                while x < len {
                    let v = (*raw.offset(x as isize))
                        .wrapping_add(*prev.offset(x as isize));
                    *raw.offset(x as isize) = v;
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    let v = (*raw.offset(x as isize))
                        .wrapping_add(*prev.offset(x as isize) / 2);
                    *raw.offset(x as isize) = v;
                    x += 1;
                }
                while x < len {
                    // (raw[x-bpp] + prev[x]) / 2  with both promoted to int
                    let sum = *raw.offset((x - bpp) as isize) as i32
                        + *prev.offset(x as isize) as i32;
                    let half = (sum / 2) as u8;
                    let v = (*raw.offset(x as isize)).wrapping_add(half);
                    *raw.offset(x as isize) = v;
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    let v = (*raw.offset(x as isize))
                        .wrapping_add(*prev.offset(x as isize));
                    *raw.offset(x as isize) = v;
                    x += 1;
                }
                while x < len {
                    let v = (*raw.offset(x as isize)).wrapping_add(cp_paeth(
                        *raw.offset((x - bpp) as isize),
                        *prev.offset(x as isize),
                        *prev.offset((x - bpp) as isize),
                    ));
                    *raw.offset(x as isize) = v;
                    x += 1;
                }
            }
            _ => return 0,
        }
        // Loop step: y++, prev = raw, raw += len
        y += 1;
        prev = raw;
        raw = raw.add(len as usize);
    }

    // suppress unused warning for `prev` after final iteration
    let _ = prev;
    1
}
