use std::ffi::{c_char, c_int};
use std::ptr;
use std::slice;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hex2bin(
    bin: *mut u8,
    bin_maxlen: usize,
    hex: *const c_char,
    hex_len: usize,
    ignore: *const c_char,
    hex_end_p: *mut *const c_char,
) -> c_int {
    let hex_bytes = if hex_len == 0 {
        &[][..]
    } else {
        unsafe { slice::from_raw_parts(hex as *const u8, hex_len) }
    };

    let mut bin_pos: usize = 0;
    let mut hex_pos: usize = 0;
    let mut ret: c_int = 0;
    let mut c_acc: u8 = 0;
    let mut state = false;

    while hex_pos < hex_len {
        let c = hex_bytes[hex_pos];
        let c_num = c ^ 48u8;
        let c_num0 = (((c_num as u16).wrapping_sub(10)) >> 8) as u8;
        let c_alpha = (c & !32u8).wrapping_sub(55u8);
        let c_alpha0 = ((((c_alpha as u16).wrapping_sub(10)) ^ ((c_alpha as u16).wrapping_sub(16))) >> 8) as u8;

        if (c_num0 | c_alpha0) == 0 {
            if !ignore.is_null() && !state {
                let mut p = ignore as *const u8;
                let mut found = false;
                loop {
                    let ch = unsafe { ptr::read(p) };
                    if ch == 0 {
                        break;
                    }
                    if ch == c {
                        found = true;
                        break;
                    }
                    p = unsafe { p.add(1) };
                }
                if found {
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

        if !state {
            c_acc = c_val.wrapping_mul(16);
        } else {
            unsafe {
                ptr::write(bin.add(bin_pos), c_acc | c_val);
            }
            bin_pos += 1;
        }
        state = !state;
        hex_pos += 1;
    }

    if state {
        hex_pos = hex_pos.wrapping_sub(1);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0;
    }
    if !hex_end_p.is_null() {
        unsafe {
            ptr::write(hex_end_p, hex.add(hex_pos));
        }
    } else if hex_pos != hex_len {
        ret = -1;
    }
    if ret != 0 {
        ret
    } else {
        bin_pos as c_int
    }
}
