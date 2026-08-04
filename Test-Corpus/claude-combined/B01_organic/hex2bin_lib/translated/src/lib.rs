use std::ffi::c_char;
use std::ffi::c_int;

unsafe fn c_strchr(s: *const c_char, c: u8) -> *const c_char {
    if s.is_null() {
        return std::ptr::null();
    }
    let mut p = s;
    loop {
        let ch = unsafe { *p } as u8;
        if ch == c {
            return p;
        }
        if ch == 0 {
            // strchr also matches the null terminator if c == 0, but we already
            // checked above.
            return std::ptr::null();
        }
        p = unsafe { p.add(1) };
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
    let mut bin_pos: usize = 0;
    let mut hex_pos: usize = 0;
    let mut ret: c_int = 0;
    let mut c_acc: u8 = 0;
    let mut state: u8 = 0;

    while hex_pos < hex_len {
        let c: u8 = unsafe { *hex.add(hex_pos) } as u8;

        // c_num = c ^ 48U  (truncated to unsigned char)
        let c_num: u8 = c ^ 48u8;
        // c_num0 = (c_num - 10U) >> 8  -- compute in u32 to mimic C int promotion
        let c_num0: u8 =
            (((c_num as u32).wrapping_sub(10u32)) >> 8) as u8;
        // c_alpha = (c & ~32U) - 55U  (truncated to unsigned char)
        let c_alpha: u8 =
            ((c as u32) & !32u32).wrapping_sub(55u32) as u8;
        // c_alpha0 = ((c_alpha - 10U) ^ (c_alpha - 16U)) >> 8
        let c_alpha0: u8 = ((((c_alpha as u32).wrapping_sub(10u32))
            ^ ((c_alpha as u32).wrapping_sub(16u32)))
            >> 8) as u8;

        if (c_num0 | c_alpha0) == 0 {
            if !ignore.is_null()
                && state == 0
                && !unsafe { c_strchr(ignore, c) }.is_null()
            {
                hex_pos += 1;
                continue;
            }
            break;
        }

        let c_val: u8 = (c_num0 & c_num) | (c_alpha0 & c_alpha);

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
        hex_pos = hex_pos.wrapping_sub(1);
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
