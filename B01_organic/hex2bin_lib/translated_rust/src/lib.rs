use std::ffi::{c_char, c_int, CStr};

/// # Safety
/// Caller must ensure all pointers are valid per the C API contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hex2bin(
    bin: *mut u8,
    bin_maxlen: usize,
    hex: *const c_char,
    hex_len: usize,
    ignore: *const c_char,
    hex_end_p: *mut *const c_char,
) -> c_int {
    let mut bin_pos: usize = 0;
    let mut hex_pos: usize = 0;
    let mut ret: c_int = 0;
    let mut c_acc: u8 = 0;
    let mut state: u8 = 0;

    let hex_bytes = unsafe { std::slice::from_raw_parts(hex as *const u8, hex_len) };

    while hex_pos < hex_len {
        let c = hex_bytes[hex_pos];
        // C promotes unsigned char to int for arithmetic; use u32 for shifts > 7
        let c_num = c ^ 48u8;
        let c_num0 = ((c_num.wrapping_sub(10) as u32) >> 8) as u8;
        let c_alpha = (c & !32u8).wrapping_sub(55);
        let c_alpha0 = (((c_alpha.wrapping_sub(10) ^ c_alpha.wrapping_sub(16)) as u32) >> 8) as u8;

        if (c_num0 | c_alpha0) == 0 {
            if !ignore.is_null() && state == 0 {
                let ignore_cstr = unsafe { CStr::from_ptr(ignore) };
                if ignore_cstr.to_bytes().contains(&c) {
                    hex_pos += 1;
                    continue;
                }
            }
            break;
        }

        let c_val = (c_num0 & c_num) | (c_alpha0 & c_alpha);

        if bin_pos >= bin_maxlen {
            ret = -1;
            break;
        }

        if state == 0 {
            c_acc = c_val.wrapping_mul(16);
        } else {
            unsafe { *bin.add(bin_pos) = c_acc | c_val };
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
        unsafe { *hex_end_p = hex.add(hex_pos) };
    } else if hex_pos != hex_len {
        ret = -1;
    }

    if ret != 0 {
        return ret;
    }

    bin_pos as c_int
}
