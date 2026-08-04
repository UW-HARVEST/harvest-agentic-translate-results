use std::ffi::{c_char, c_int, c_void};
use std::os::raw::c_uchar;
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn hex2bin(
    bin: *mut c_uchar,
    bin_maxlen: usize,
    hex: *const c_char,
    hex_len: usize,
    ignore: *const c_char,
    hex_end_p: *mut *const c_char,
) -> c_int {
    let mut bin_pos: usize = 0;
    let mut hex_pos: usize = 0;
    let mut ret: c_int = 0;
    let mut c: c_uchar;
    let mut c_alpha0: c_uchar;
    let mut c_alpha: c_uchar;
    let mut c_num0: c_uchar;
    let mut c_num: c_uchar;
    let mut c_acc: c_uchar = 0;
    let mut c_val: c_uchar;
    let mut state: c_uchar = 0;

    unsafe {
        let hex_slice = std::slice::from_raw_parts(hex as *const c_uchar, hex_len);
        let bin_slice = std::slice::from_raw_parts_mut(bin, bin_maxlen);

        while hex_pos < hex_len {
            c = hex_slice[hex_pos];
            c_num = c ^ 48;
            c_num0 = ((c_num as i16 - 10) >> 8) as c_uchar;
            c_alpha = (c & !32) - 55;
            c_alpha0 = (((c_alpha as i16 - 10) ^ (c_alpha as i16 - 16)) >> 8) as c_uchar;

            if (c_num0 | c_alpha0) == 0 {
                if !ignore.is_null() && state == 0 {
                    let ignore_str = std::ffi::CStr::from_ptr(ignore);
                    let ignore_bytes = ignore_str.to_bytes();
                    if ignore_bytes.contains(&c) {
                        hex_pos += 1;
                        continue;
                    }
                }
                break;
            }

            c_val = ((c_num0 & c_num) | (c_alpha0 & c_alpha)) as c_uchar;

            if bin_pos >= bin_maxlen {
                ret = -1;
                break;
            }

            if state == 0 {
                c_acc = c_val.wrapping_mul(16);
            } else {
                bin_slice[bin_pos] = c_acc | c_val;
                bin_pos += 1;
            }

            state = !state;
            hex_pos += 1;
        }

        if state != 0 {
            hex_pos -= 1;
            ret = -1;
        }

        if ret != 0 {
            bin_pos = 0;
        }

        if !hex_end_p.is_null() {
            *hex_end_p = hex.add(hex_pos);
        } else if hex_pos != hex_len {
            ret = -1;
        }

        if ret != 0 {
            return ret;
        }

        bin_pos as c_int
    }
}
