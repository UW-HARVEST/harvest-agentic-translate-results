extern "C" {
    fn strchr(__s: *const libc::c_char, __c: libc::c_int)
        -> *mut libc::c_char;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type uint8_t = u8;
#[no_mangle]
pub unsafe extern "C" fn hex2bin<'a1>(
    mut bin: * mut u8,
    mut bin_maxlen: usize,
    mut hex: * const libc::c_char,
    mut hex_len: usize,
    mut ignore: * const libc::c_char,
    mut hex_end_p: Option<&'a1 mut * const libc::c_char>,
) -> libc::c_int {
    let mut bin_pos: size_t = 0 as libc::c_uint as size_t;
    let mut hex_pos: size_t = 0 as libc::c_uint as size_t;
    let mut ret: libc::c_int = 0 as libc::c_int;
    let mut c: libc::c_uchar = 0;
    let mut c_alpha0: libc::c_uchar = 0;
    let mut c_alpha: libc::c_uchar = 0;
    let mut c_num0: libc::c_uchar = 0;
    let mut c_num: libc::c_uchar = 0;
    let mut c_acc: uint8_t = 0 as uint8_t;
    let mut c_val: uint8_t = 0;
    let mut state: libc::c_uchar = 0 as libc::c_uchar;
    while hex_pos < hex_len {
        c = *hex.offset(hex_pos as isize) as libc::c_uchar;
        c_num = (c as libc::c_uint ^ 48 as libc::c_uint) as libc::c_uchar;
        c_num0 = ((c_num as libc::c_uint).wrapping_sub(10 as libc::c_uint)
            >> 8 as libc::c_int) as libc::c_uchar;
        c_alpha = (c as libc::c_uint & !(32 as libc::c_uint))
            .wrapping_sub(55 as libc::c_uint) as libc::c_uchar;
        c_alpha0 = (((c_alpha as libc::c_uint).wrapping_sub(10 as libc::c_uint)
            ^ (c_alpha as libc::c_uint).wrapping_sub(16 as libc::c_uint))
            >> 8 as libc::c_int) as libc::c_uchar;
        if (c_num0 as libc::c_int | c_alpha0 as libc::c_int) as libc::c_uint
            == 0 as libc::c_uint
        {
            if !(!ignore.is_null()
                && state as libc::c_uint == 0 as libc::c_uint
                && !strchr(ignore, c as libc::c_int).is_null())
            {
                break;
            }
            hex_pos = hex_pos.wrapping_add(1);
        } else {
            c_val = (c_num0 as libc::c_int & c_num as libc::c_int
                | c_alpha0 as libc::c_int & c_alpha as libc::c_int)
                as uint8_t;
            if bin_pos >= bin_maxlen {
                ret = -(1 as libc::c_int);
                break;
            } else {
                if state as libc::c_uint == 0 as libc::c_uint {
                    c_acc = (c_val as libc::c_uint).wrapping_mul(16 as libc::c_uint)
                        as uint8_t;
                } else {
                    let fresh0 = bin_pos;
                    bin_pos = bin_pos.wrapping_add(1);
                    *bin.offset(fresh0 as isize) =
                        (c_acc as libc::c_int | c_val as libc::c_int) as uint8_t;
                }
                state = !(state as libc::c_int) as libc::c_uchar;
                hex_pos = hex_pos.wrapping_add(1);
            }
        }
    }
    if state as libc::c_uint != 0 as libc::c_uint {
        hex_pos = hex_pos.wrapping_sub(1);
        ret = -(1 as libc::c_int);
    }
    if ret != 0 as libc::c_int {
        bin_pos = 0 as libc::c_uint as size_t;
    }
    if !borrow(& hex_end_p).is_none() {
        *borrow_mut(&mut hex_end_p).unwrap() = hex.offset(hex_pos as isize) as *const libc::c_char;
    } else if hex_pos != hex_len {
        ret = -(1 as libc::c_int);
    }
    if ret != 0 as libc::c_int {
        return ret;
    }
    return bin_pos as libc::c_int;
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

