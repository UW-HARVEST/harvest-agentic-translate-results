use std::ffi::{c_char, c_int};

#[inline]
unsafe fn c_string_contains(mut string: *const c_char, needle: u8) -> bool {
    loop {
        let current = unsafe { string.read() } as u8;
        if current == needle {
            return true;
        }
        if current == 0 {
            return false;
        }
        string = unsafe { string.add(1) };
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
    let mut ret = 0;
    let mut c_acc = 0u8;
    let mut state = 0u8;

    while hex_pos < hex_len {
        let c = unsafe { hex.add(hex_pos).read() } as u8;
        let c_num = c ^ 48;
        let c_num0 = (u32::from(c_num).wrapping_sub(10) >> 8) as u8;
        let c_alpha = ((u32::from(c) & !32u32).wrapping_sub(55)) as u8;
        let c_alpha0 = ((u32::from(c_alpha).wrapping_sub(10) ^ u32::from(c_alpha).wrapping_sub(16))
            >> 8) as u8;

        if c_num0 | c_alpha0 == 0 {
            if !ignore.is_null() && state == 0 && unsafe { c_string_contains(ignore, c) } {
                hex_pos = hex_pos.wrapping_add(1);
                continue;
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
            unsafe { bin.add(bin_pos).write(c_acc | c_val) };
            bin_pos = bin_pos.wrapping_add(1);
        }
        state = !state;
        hex_pos = hex_pos.wrapping_add(1);
    }

    if state != 0 {
        hex_pos = hex_pos.wrapping_sub(1);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0;
    }
    if !hex_end_p.is_null() {
        unsafe { hex_end_p.write(hex.add(hex_pos)) };
    } else if hex_pos != hex_len {
        ret = -1;
    }
    if ret != 0 {
        return ret;
    }
    bin_pos as c_int
}
