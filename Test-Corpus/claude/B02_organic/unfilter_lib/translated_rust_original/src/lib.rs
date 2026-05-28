use std::ffi::c_int;

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    let len: isize = (w as isize) * (bpp as isize);
    let bpp_isize: isize = bpp as isize;

    // Track positions via offsets into the raw pointer (mirroring the C
    // pointer arithmetic exactly).
    let mut raw_ofs: isize = 0;

    if h > 0 {
        let filter = unsafe { *raw.offset(raw_ofs) };
        raw_ofs += 1;
        match filter {
            0 => {}
            1 => {
                let mut x = bpp_isize;
                while x < len {
                    unsafe {
                        let v = *raw.offset(raw_ofs + x - bpp_isize);
                        let cur = *raw.offset(raw_ofs + x);
                        *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    }
                    x += 1;
                }
            }
            2 => {}
            3 => {
                let mut x = bpp_isize;
                while x < len {
                    unsafe {
                        let v = *raw.offset(raw_ofs + x - bpp_isize) / 2;
                        let cur = *raw.offset(raw_ofs + x);
                        *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    }
                    x += 1;
                }
            }
            4 => {
                let mut x = bpp_isize;
                while x < len {
                    unsafe {
                        let a = *raw.offset(raw_ofs + x - bpp_isize);
                        let v = cp_paeth(a, 0, 0);
                        let cur = *raw.offset(raw_ofs + x);
                        *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }
    }

    let mut prev_ofs: isize = raw_ofs;
    raw_ofs += len;

    let mut y: c_int = 1;
    while y < h {
        let filter = unsafe { *raw.offset(raw_ofs) };
        raw_ofs += 1;
        match filter {
            0 => {}
            1 => {
                let mut x: isize = 0;
                while x < bpp_isize {
                    // raw[x] += 0 — no-op
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let v = *raw.offset(raw_ofs + x - bpp_isize);
                        let cur = *raw.offset(raw_ofs + x);
                        *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    }
                    x += 1;
                }
            }
            2 => {
                let mut x: isize = 0;
                while x < bpp_isize {
                    unsafe {
                        let v = *raw.offset(prev_ofs + x);
                        let cur = *raw.offset(raw_ofs + x);
                        *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let v = *raw.offset(prev_ofs + x);
                        let cur = *raw.offset(raw_ofs + x);
                        *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    }
                    x += 1;
                }
            }
            3 => {
                let mut x: isize = 0;
                while x < bpp_isize {
                    unsafe {
                        let v = *raw.offset(prev_ofs + x) / 2;
                        let cur = *raw.offset(raw_ofs + x);
                        *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let a = *raw.offset(raw_ofs + x - bpp_isize) as u32;
                        let b = *raw.offset(prev_ofs + x) as u32;
                        let v = ((a + b) / 2) as u8;
                        let cur = *raw.offset(raw_ofs + x);
                        *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    }
                    x += 1;
                }
            }
            4 => {
                let mut x: isize = 0;
                while x < bpp_isize {
                    unsafe {
                        let v = *raw.offset(prev_ofs + x);
                        let cur = *raw.offset(raw_ofs + x);
                        *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let a = *raw.offset(raw_ofs + x - bpp_isize);
                        let b = *raw.offset(prev_ofs + x);
                        let c = *raw.offset(prev_ofs + x - bpp_isize);
                        let v = cp_paeth(a, b, c);
                        let cur = *raw.offset(raw_ofs + x);
                        *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }
        // y++, prev = raw, raw += len
        y += 1;
        prev_ofs = raw_ofs;
        raw_ofs += len;
    }

    1
}
