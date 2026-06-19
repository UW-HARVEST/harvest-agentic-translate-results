use std::ffi::c_char;
use std::ffi::c_int;

unsafe fn ignore_contains(ignore: *const c_char, needle: u8) -> bool {
    let mut pos = 0usize;

    loop {
        let current = *ignore.add(pos) as u8;
        if current == needle {
            return true;
        }
        if current == 0 {
            return false;
        }
        pos += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hex2bin(
    bin: *mut u8,
    bin_maxlen: usize,
    hex: *const c_char,
    hex_len: usize,
    ignore: *const c_char,
    hex_end_p: *mut *const c_char,
) -> c_int {
    let mut bin_pos = 0usize;
    let mut hex_pos = 0usize;
    let mut ret: c_int = 0;
    let mut c_acc = 0u8;
    let mut state = 0u8;

    while hex_pos < hex_len {
        let c = *hex.add(hex_pos) as u8;
        let c_num = (u32::from(c) ^ 48u32) as u8;
        let c_num0 = (u32::from(c_num).wrapping_sub(10u32) >> 8) as u8;
        let c_alpha = ((u32::from(c) & !32u32).wrapping_sub(55u32)) as u8;
        let c_alpha0 =
            ((u32::from(c_alpha).wrapping_sub(10u32) ^ u32::from(c_alpha).wrapping_sub(16u32))
                >> 8) as u8;

        if (c_num0 | c_alpha0) == 0 {
            if !ignore.is_null() && state == 0 && ignore_contains(ignore, c) {
                hex_pos += 1;
                continue;
            }
            break;
        }

        let c_val = ((c_num0 & c_num) | (c_alpha0 & c_alpha)) as u8;
        if bin_pos >= bin_maxlen {
            ret = -1;
            break;
        }

        if state == 0 {
            c_acc = c_val.wrapping_mul(16);
        } else {
            *bin.add(bin_pos) = c_acc | c_val;
            bin_pos += 1;
        }

        state = !state;
        hex_pos += 1;
    }

    if state != 0 {
        hex_pos = hex_pos.wrapping_sub(1);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0;
    }
    if !hex_end_p.is_null() {
        *hex_end_p = hex.wrapping_add(hex_pos);
    } else if hex_pos != hex_len {
        ret = -1;
    }
    if ret != 0 {
        return ret;
    }
    bin_pos as c_int
}
