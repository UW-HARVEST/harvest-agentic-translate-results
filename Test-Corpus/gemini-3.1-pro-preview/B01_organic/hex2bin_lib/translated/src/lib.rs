use std::ffi::{c_char, CStr};
use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn hex2bin(
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

    let hex_slice = if hex_len > 0 {
        if hex.is_null() {
            return -1;
        }
        unsafe { std::slice::from_raw_parts(hex as *const u8, hex_len) }
    } else {
        &[]
    };

    while hex_pos < hex_len {
        let c = hex_slice[hex_pos];

        let val = match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        };

        if let Some(v) = val {
            if bin_pos >= bin_maxlen {
                ret = -1;
                break;
            }
            if state == 0 {
                c_acc = v * 16;
            } else {
                unsafe {
                    *bin.add(bin_pos) = c_acc | v;
                }
                bin_pos += 1;
            }
            state = !state;
            hex_pos += 1;
        } else {
            let mut ignored = false;
            if !ignore.is_null() && state == 0 {
                let ignore_bytes = unsafe { CStr::from_ptr(ignore).to_bytes_with_nul() };
                if ignore_bytes.contains(&c) {
                    ignored = true;
                }
            }
            if ignored {
                hex_pos += 1;
                continue;
            }
            break;
        }
    }

    if state != 0 {
        hex_pos -= 1;
        ret = -1;
    }

    if ret != 0 {
        bin_pos = 0;
    }

    if !hex_end_p.is_null() {
        unsafe {
            *hex_end_p = hex.wrapping_add(hex_pos);
        }
    } else if hex_pos != hex_len {
        ret = -1;
    }

    if ret != 0 {
        return ret;
    }

    bin_pos as c_int
}
