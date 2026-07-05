
extern "C" {
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type uint8_t = __uint8_t;
#[no_mangle]
pub fn hex2bin(
    bin: &mut [uint8_t],
    hex: &[u8],
    ignore: Option<&[u8]>,
    hex_end_p: Option<&mut usize>,
) -> ::core::ffi::c_int {
    let mut bin_pos: usize = 0;
    let mut hex_pos: usize = 0;
    let mut ret: ::core::ffi::c_int = 0;
    let mut c_acc: uint8_t = 0;
    let mut state = false;

    while hex_pos < hex.len() {
        let c = hex[hex_pos];
        let is_num = c.is_ascii_digit();
        let is_alpha = matches!(c, b'a'..=b'f' | b'A'..=b'F');

        if !is_num && !is_alpha {
            if !(ignore.is_some()
                && !state
                && ignore.unwrap().contains(&c))
            {
                break;
            }
            hex_pos += 1;
            continue;
        }

        let c_val: uint8_t = if is_num {
            c - b'0'
        } else {
            (c.to_ascii_uppercase() - b'A' + 10) as uint8_t
        };

        if bin_pos >= bin.len() {
            ret = -1;
            break;
        }

        if !state {
            c_acc = c_val << 4;
        } else {
            bin[bin_pos] = c_acc | c_val;
            bin_pos += 1;
        }
        state = !state;
        hex_pos += 1;
    }

    if state {
        hex_pos = hex_pos.saturating_sub(1);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0;
    }
    if let Some(end_pos) = hex_end_p {
        *end_pos = hex_pos;
    } else if hex_pos != hex.len() {
        ret = -1;
    }
    if ret != 0 {
        ret
    } else {
        bin_pos as ::core::ffi::c_int
    }
}

