extern "C" {
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut libc::c_void;
    fn strlen(__s: *const libc::c_char) -> size_t;
}
pub type size_t = usize;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
 extern "C" fn encode(mut u: libc::c_uchar) -> libc::c_char {
    if (u as libc::c_int) < 26 as libc::c_int {
        return ('A' as i32 + u as libc::c_int) as libc::c_char;
    }
    if (u as libc::c_int) < 52 as libc::c_int {
        return ('a' as i32 + (u as libc::c_int - 26 as libc::c_int))
            as libc::c_char;
    }
    if (u as libc::c_int) < 62 as libc::c_int {
        return ('0' as i32 + (u as libc::c_int - 52 as libc::c_int))
            as libc::c_char;
    }
    if u as libc::c_int == 62 as libc::c_int {
        return '+' as i32 as libc::c_char;
    }
    return '/' as i32 as libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn encode_base64(
    mut size: libc::c_int,
    mut src: *const libc::c_char,
) -> *mut libc::c_char {
    let mut i: libc::c_int = 0;
    let mut out: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut p: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    if src.is_null() {
        return std::ptr::null_mut::<libc::c_char>();
    }
    if size == 0 {
        size = strlen(src as *mut libc::c_char) as libc::c_int;
    }
    out = calloc(
        std::mem::size_of::<libc::c_char>() as size_t,
        (size * 4 as libc::c_int / 3 as libc::c_int + 4 as libc::c_int)
            as size_t,
    ) as *mut libc::c_char;
    if out.is_null() {
        return std::ptr::null_mut::<libc::c_char>();
    }
    p = out;
    i = 0 as libc::c_int;
    while i < size {
        let mut b1: libc::c_uchar = 0 as libc::c_uchar;
        let mut b2: libc::c_uchar = 0 as libc::c_uchar;
        let mut b3: libc::c_uchar = 0 as libc::c_uchar;
        let mut b4: libc::c_uchar = 0 as libc::c_uchar;
        let mut b5: libc::c_uchar = 0 as libc::c_uchar;
        let mut b6: libc::c_uchar = 0 as libc::c_uchar;
        let mut b7: libc::c_uchar = 0 as libc::c_uchar;
        b1 = *src.offset(i as isize) as libc::c_uchar;
        if (i + 1 as libc::c_int) < size {
            b2 = *src.offset((i + 1 as libc::c_int) as isize) as libc::c_uchar;
        }
        if (i + 2 as libc::c_int) < size {
            b3 = *src.offset((i + 2 as libc::c_int) as isize) as libc::c_uchar;
        }
        b4 = (b1 as libc::c_int >> 2 as libc::c_int) as libc::c_uchar;
        b5 = ((b1 as libc::c_int & 0x3 as libc::c_int) << 4 as libc::c_int
            | b2 as libc::c_int >> 4 as libc::c_int)
            as libc::c_uchar;
        b6 = ((b2 as libc::c_int & 0xf as libc::c_int) << 2 as libc::c_int
            | b3 as libc::c_int >> 6 as libc::c_int)
            as libc::c_uchar;
        b7 = (b3 as libc::c_int & 0x3f as libc::c_int) as libc::c_uchar;
        let fresh0 = p;
        p = p.offset(1);
        *fresh0 = encode(b4);
        let fresh1 = p;
        p = p.offset(1);
        *fresh1 = encode(b5);
        if (i + 1 as libc::c_int) < size {
            let fresh2 = p;
            p = p.offset(1);
            *fresh2 = encode(b6);
        } else {
            let fresh3 = p;
            p = p.offset(1);
            *fresh3 = '=' as i32 as libc::c_char;
        }
        if (i + 2 as libc::c_int) < size {
            let fresh4 = p;
            p = p.offset(1);
            *fresh4 = encode(b7);
        } else {
            let fresh5 = p;
            p = p.offset(1);
            *fresh5 = '=' as i32 as libc::c_char;
        }
        i += 3 as libc::c_int;
    }
    return out;
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

