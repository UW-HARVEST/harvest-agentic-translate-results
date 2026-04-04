extern "C" {
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type uint8_t = __uint8_t;
#[no_mangle]
pub unsafe extern "C" fn hex2bin(
    mut bin: *mut uint8_t,
    mut bin_maxlen: size_t,
    mut hex: *const ::core::ffi::c_char,
    mut hex_len: size_t,
    mut ignore: *const ::core::ffi::c_char,
    mut hex_end_p: *mut *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut bin_pos: size_t = 0 as ::core::ffi::c_uint as size_t;
    let mut hex_pos: size_t = 0 as ::core::ffi::c_uint as size_t;
    let mut ret: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut c: ::core::ffi::c_uchar = 0;
    let mut c_alpha0: ::core::ffi::c_uchar = 0;
    let mut c_alpha: ::core::ffi::c_uchar = 0;
    let mut c_num0: ::core::ffi::c_uchar = 0;
    let mut c_num: ::core::ffi::c_uchar = 0;
    let mut c_acc: uint8_t = 0 as uint8_t;
    let mut c_val: uint8_t = 0;
    let mut state: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
    while hex_pos < hex_len {
        c = *hex.offset(hex_pos as isize) as ::core::ffi::c_uchar;
        c_num = (c as ::core::ffi::c_uint ^ 48 as ::core::ffi::c_uint) as ::core::ffi::c_uchar;
        c_num0 = ((c_num as ::core::ffi::c_uint).wrapping_sub(10 as ::core::ffi::c_uint)
            >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
        c_alpha = (c as ::core::ffi::c_uint & !(32 as ::core::ffi::c_uint))
            .wrapping_sub(55 as ::core::ffi::c_uint) as ::core::ffi::c_uchar;
        c_alpha0 = (((c_alpha as ::core::ffi::c_uint).wrapping_sub(10 as ::core::ffi::c_uint)
            ^ (c_alpha as ::core::ffi::c_uint).wrapping_sub(16 as ::core::ffi::c_uint))
            >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
        if (c_num0 as ::core::ffi::c_int | c_alpha0 as ::core::ffi::c_int) as ::core::ffi::c_uint
            == 0 as ::core::ffi::c_uint
        {
            if !(!ignore.is_null()
                && state as ::core::ffi::c_uint == 0 as ::core::ffi::c_uint
                && !strchr(ignore, c as ::core::ffi::c_int).is_null())
            {
                break;
            }
            hex_pos = hex_pos.wrapping_add(1);
        } else {
            c_val = (c_num0 as ::core::ffi::c_int & c_num as ::core::ffi::c_int
                | c_alpha0 as ::core::ffi::c_int & c_alpha as ::core::ffi::c_int)
                as uint8_t;
            if bin_pos >= bin_maxlen {
                ret = -(1 as ::core::ffi::c_int);
                break;
            } else {
                if state as ::core::ffi::c_uint == 0 as ::core::ffi::c_uint {
                    c_acc = (c_val as ::core::ffi::c_uint).wrapping_mul(16 as ::core::ffi::c_uint)
                        as uint8_t;
                } else {
                    let fresh0 = bin_pos;
                    bin_pos = bin_pos.wrapping_add(1);
                    *bin.offset(fresh0 as isize) =
                        (c_acc as ::core::ffi::c_int | c_val as ::core::ffi::c_int) as uint8_t;
                }
                state = !(state as ::core::ffi::c_int) as ::core::ffi::c_uchar;
                hex_pos = hex_pos.wrapping_add(1);
            }
        }
    }
    if state as ::core::ffi::c_uint != 0 as ::core::ffi::c_uint {
        hex_pos = hex_pos.wrapping_sub(1);
        ret = -(1 as ::core::ffi::c_int);
    }
    if ret != 0 as ::core::ffi::c_int {
        bin_pos = 0 as ::core::ffi::c_uint as size_t;
    }
    if !hex_end_p.is_null() {
        *hex_end_p = hex.offset(hex_pos as isize) as *const ::core::ffi::c_char;
    } else if hex_pos != hex_len {
        ret = -(1 as ::core::ffi::c_int);
    }
    if ret != 0 as ::core::ffi::c_int {
        return ret;
    }
    return bin_pos as ::core::ffi::c_int;
}
