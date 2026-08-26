use std::ffi::c_int;

#[inline]
fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    // int p = a + b - c;
    let p: i32 = (a as i32) + (b as i32) - (c as i32);
    // int pa = abs(p - a); int pb = abs(p - b); int pc = abs(p - c);
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

/// Reverses PNG filtering on raw scanlines.
///
/// # Safety
/// `raw` must point to a writable buffer of at least `h * (w * bpp + 1)` bytes
/// (i.e. one filter-type byte plus `w * bpp` data bytes per row).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    let len: i32 = w * bpp;
    let len_us: usize = len as usize;
    let bpp_us: usize = bpp as usize;

    // Track current row pointer as an index offset from the start of the
    // caller-provided buffer. We use raw.add(off) to access.
    let mut row_off: usize = 0;

    if h > 0 {
        // Read filter byte and advance.
        let filter = *raw.add(row_off);
        row_off += 1;
        match filter {
            0 => {}
            1 => {
                let mut x: usize = bpp_us;
                while x < len_us {
                    let v = *raw.add(row_off + x - bpp_us);
                    let cur = *raw.add(row_off + x);
                    *raw.add(row_off + x) = cur.wrapping_add(v);
                    x += 1;
                }
            }
            2 => {}
            3 => {
                let mut x: usize = bpp_us;
                while x < len_us {
                    let v = *raw.add(row_off + x - bpp_us);
                    let cur = *raw.add(row_off + x);
                    *raw.add(row_off + x) = cur.wrapping_add(v / 2);
                    x += 1;
                }
            }
            4 => {
                let mut x: usize = bpp_us;
                while x < len_us {
                    let a = *raw.add(row_off + x - bpp_us);
                    let pae = cp_paeth(a, 0, 0);
                    let cur = *raw.add(row_off + x);
                    *raw.add(row_off + x) = cur.wrapping_add(pae);
                    x += 1;
                }
            }
            _ => return 0,
        }
    }

    // prev = raw (pointer to first row's data)
    // raw += len (advance to filter byte of row 1)
    let mut prev_off: usize = row_off;
    row_off += len_us;

    let mut y: i32 = 1;
    while y < h {
        let filter = *raw.add(row_off);
        row_off += 1;
        match filter {
            0 => {}
            1 => {
                // for (x = 0; x < bpp; x++) raw[x] += 0;
                // (no-op, but preserve loop semantics)
                let mut x: usize = 0;
                while x < bpp_us {
                    let cur = *raw.add(row_off + x);
                    *raw.add(row_off + x) = cur.wrapping_add(0);
                    x += 1;
                }
                while x < len_us {
                    let v = *raw.add(row_off + x - bpp_us);
                    let cur = *raw.add(row_off + x);
                    *raw.add(row_off + x) = cur.wrapping_add(v);
                    x += 1;
                }
            }
            2 => {
                let mut x: usize = 0;
                while x < bpp_us {
                    let p = *raw.add(prev_off + x);
                    let cur = *raw.add(row_off + x);
                    *raw.add(row_off + x) = cur.wrapping_add(p);
                    x += 1;
                }
                while x < len_us {
                    let p = *raw.add(prev_off + x);
                    let cur = *raw.add(row_off + x);
                    *raw.add(row_off + x) = cur.wrapping_add(p);
                    x += 1;
                }
            }
            3 => {
                let mut x: usize = 0;
                while x < bpp_us {
                    let p = *raw.add(prev_off + x);
                    let cur = *raw.add(row_off + x);
                    *raw.add(row_off + x) = cur.wrapping_add(p / 2);
                    x += 1;
                }
                while x < len_us {
                    let a = *raw.add(row_off + x - bpp_us);
                    let p = *raw.add(prev_off + x);
                    // (raw[x - bpp] + prev[x]) / 2 — done in C as int division,
                    // operands promoted to int so no overflow before the divide.
                    let sum = (a as i32) + (p as i32);
                    let cur = *raw.add(row_off + x);
                    *raw.add(row_off + x) = cur.wrapping_add((sum / 2) as u8);
                    x += 1;
                }
            }
            4 => {
                let mut x: usize = 0;
                while x < bpp_us {
                    let p = *raw.add(prev_off + x);
                    let cur = *raw.add(row_off + x);
                    *raw.add(row_off + x) = cur.wrapping_add(p);
                    x += 1;
                }
                while x < len_us {
                    let a = *raw.add(row_off + x - bpp_us);
                    let b = *raw.add(prev_off + x);
                    let c = *raw.add(prev_off + x - bpp_us);
                    let pae = cp_paeth(a, b, c);
                    let cur = *raw.add(row_off + x);
                    *raw.add(row_off + x) = cur.wrapping_add(pae);
                    x += 1;
                }
            }
            _ => return 0,
        }
        // for-iteration step: prev = raw; raw += len;
        prev_off = row_off;
        row_off += len_us;
        y += 1;
    }

    1
}
