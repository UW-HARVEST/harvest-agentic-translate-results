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
    let len = w * bpp;
    let mut raw = raw;
    let mut x: c_int;

    if h > 0 {
        let filter = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*raw.offset((x - bpp) as isize));
                    }
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add((*raw.offset((x - bpp) as isize)) / 2);
                    }
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add(cp_paeth(*raw.offset((x - bpp) as isize), 0, 0));
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }
    }

    let mut prev = raw;
    raw = unsafe { raw.offset(len as isize) };

    let mut y = 1;
    while y < h {
        let filter = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(0);
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*raw.offset((x - bpp) as isize));
                    }
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                    }
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add((*prev.offset(x as isize)) / 2);
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(
                            ((*raw.offset((x - bpp) as isize) as u16
                                + *prev.offset(x as isize) as u16)
                                / 2) as u8,
                        );
                    }
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(
                            cp_paeth(
                                *raw.offset((x - bpp) as isize),
                                *prev.offset(x as isize),
                                *prev.offset((x - bpp) as isize),
                            ),
                        );
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }

        prev = raw;
        raw = unsafe { raw.offset(len as isize) };
        y += 1;
    }

    1
}
