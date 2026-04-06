extern "C" {
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
unsafe extern "C" fn encode(mut u: ::core::ffi::c_uchar) -> ::core::ffi::c_char {
    if (u as ::core::ffi::c_int) < 26 as ::core::ffi::c_int {
        return ('A' as i32 + u as ::core::ffi::c_int) as ::core::ffi::c_char;
    }
    if (u as ::core::ffi::c_int) < 52 as ::core::ffi::c_int {
        return ('a' as i32 + (u as ::core::ffi::c_int - 26 as ::core::ffi::c_int))
            as ::core::ffi::c_char;
    }
    if (u as ::core::ffi::c_int) < 62 as ::core::ffi::c_int {
        return ('0' as i32 + (u as ::core::ffi::c_int - 52 as ::core::ffi::c_int))
            as ::core::ffi::c_char;
    }
    if u as ::core::ffi::c_int == 62 as ::core::ffi::c_int {
        return '+' as i32 as ::core::ffi::c_char;
    }
    return '/' as i32 as ::core::ffi::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn encode_base64(
    mut size: ::core::ffi::c_int,
    mut src: *const ::core::ffi::c_char,
) -> *mut ::core::ffi::c_char {
    let mut i: ::core::ffi::c_int = 0;
    let mut out: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut p: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    if src.is_null() {
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    if size == 0 {
        size = strlen(src as *mut ::core::ffi::c_char) as ::core::ffi::c_int;
    }
    out = calloc(
        ::core::mem::size_of::<::core::ffi::c_char>() as size_t,
        (size * 4 as ::core::ffi::c_int / 3 as ::core::ffi::c_int + 4 as ::core::ffi::c_int)
            as size_t,
    ) as *mut ::core::ffi::c_char;
    if out.is_null() {
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    p = out;
    i = 0 as ::core::ffi::c_int;
    while i < size {
        let mut b1: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
        let mut b2: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
        let mut b3: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
        let mut b4: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
        let mut b5: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
        let mut b6: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
        let mut b7: ::core::ffi::c_uchar = 0 as ::core::ffi::c_uchar;
        b1 = *src.offset(i as isize) as ::core::ffi::c_uchar;
        if (i + 1 as ::core::ffi::c_int) < size {
            b2 = *src.offset((i + 1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_uchar;
        }
        if (i + 2 as ::core::ffi::c_int) < size {
            b3 = *src.offset((i + 2 as ::core::ffi::c_int) as isize) as ::core::ffi::c_uchar;
        }
        b4 = (b1 as ::core::ffi::c_int >> 2 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
        b5 = ((b1 as ::core::ffi::c_int & 0x3 as ::core::ffi::c_int) << 4 as ::core::ffi::c_int
            | b2 as ::core::ffi::c_int >> 4 as ::core::ffi::c_int)
            as ::core::ffi::c_uchar;
        b6 = ((b2 as ::core::ffi::c_int & 0xf as ::core::ffi::c_int) << 2 as ::core::ffi::c_int
            | b3 as ::core::ffi::c_int >> 6 as ::core::ffi::c_int)
            as ::core::ffi::c_uchar;
        b7 = (b3 as ::core::ffi::c_int & 0x3f as ::core::ffi::c_int) as ::core::ffi::c_uchar;
        let fresh0 = p;
        p = p.offset(1);
        *fresh0 = encode(b4);
        let fresh1 = p;
        p = p.offset(1);
        *fresh1 = encode(b5);
        if (i + 1 as ::core::ffi::c_int) < size {
            let fresh2 = p;
            p = p.offset(1);
            *fresh2 = encode(b6);
        } else {
            let fresh3 = p;
            p = p.offset(1);
            *fresh3 = '=' as i32 as ::core::ffi::c_char;
        }
        if (i + 2 as ::core::ffi::c_int) < size {
            let fresh4 = p;
            p = p.offset(1);
            *fresh4 = encode(b7);
        } else {
            let fresh5 = p;
            p = p.offset(1);
            *fresh5 = '=' as i32 as ::core::ffi::c_char;
        }
        i += 3 as ::core::ffi::c_int;
    }
    return out;
}
