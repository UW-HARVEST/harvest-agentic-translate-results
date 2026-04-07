use std::ffi::c_int;
use std::os::raw::c_char;

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

    while hex_pos < hex_len {
        let c = *hex.add(hex_pos) as u8;
        let c_u32 = c as u32;
        let c_num_32 = c_u32 ^ 48;
        let c_num0_32 = (c_num_32.wrapping_sub(10)) >> 8;
        let c_alpha_32 = (c_u32 & !32u32).wrapping_sub(55);
        let c_alpha0_32 = ((c_alpha_32.wrapping_sub(10)) ^ (c_alpha_32.wrapping_sub(16))) >> 8;

        let c_num0_u8 = c_num0_32 as u8;
        let c_alpha0_u8 = c_alpha0_32 as u8;
        let c_num_u8 = c_num_32 as u8;
        let c_alpha_u8 = c_alpha_32 as u8;

        if (c_num0_u8 | c_alpha0_u8) == 0 {
            if !ignore.is_null() && state == 0 {
                // strchr(ignore, c) — search for c in the ignore string
                let mut p = ignore;
                let mut found = false;
                while *p != 0 {
                    if *p as u8 == c {
                        found = true;
                        break;
                    }
                    p = p.add(1);
                }
                if found {
                    hex_pos += 1;
                    continue;
                }
            }
            break;
        }
        let c_val = (c_num0_u8 & c_num_u8) | (c_alpha0_u8 & c_alpha_u8);
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
