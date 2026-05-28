use std::ffi::{c_char, c_int};
use std::ptr;

unsafe fn c_strchr(s: *const c_char, c: u8) -> *const c_char {
    let mut p = s;
    loop {
        let v = unsafe { *p } as u8;
        if v == c {
            return p;
        }
        if v == 0 {
            return ptr::null();
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
    let mut bin_pos: usize = 0usize;
    let mut hex_pos: usize = 0usize;
    let mut ret: c_int = 0;
    let mut c_acc: u8 = 0u8;
    let mut state: u8 = 0u8;

    while hex_pos < hex_len {
        let c: u8 = unsafe { *hex.add(hex_pos) } as u8;
        // In C, integer promotion converts unsigned char to int/unsigned int
        // before arithmetic. We replicate that with u32 arithmetic, then
        // cast back to u8 to mirror the implicit narrowing assignment.
        let c_num: u8 = c ^ 48u8;
        let c_num0: u8 = ((c_num as u32).wrapping_sub(10u32) >> 8) as u8;
        let c_alpha: u8 = ((c & !32u8) as u32).wrapping_sub(55u32) as u8;
        let c_alpha0: u8 = ((((c_alpha as u32).wrapping_sub(10u32))
            ^ ((c_alpha as u32).wrapping_sub(16u32)))
            >> 8) as u8;

        if (c_num0 | c_alpha0) == 0u8 {
            if !ignore.is_null()
                && state == 0u8
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
        if state == 0u8 {
            // c_acc = c_val * 16U; truncated to uint8_t
            c_acc = c_val.wrapping_mul(16u8);
        } else {
            unsafe { *bin.add(bin_pos) = c_acc | c_val };
            bin_pos += 1;
        }
        // state = ~state; alternates between 0 and 0xFF
        state = !state;
        hex_pos += 1;
    }
    if state != 0u8 {
        hex_pos = hex_pos.wrapping_sub(1);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0usize;
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
