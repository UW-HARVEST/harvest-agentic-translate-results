extern "C" {
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn strlen(__s: *const libc::c_char) -> size_t;
}
pub type size_t = usize;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const TRUE: libc::c_int = 1 as libc::c_int;
pub const FALSE: libc::c_int = 0 as libc::c_int;
 extern "C" fn decode(mut c: libc::c_char) -> libc::c_uchar {
    if c as libc::c_int >= 'A' as i32 && c as libc::c_int <= 'Z' as i32 {
        return (c as libc::c_int - 'A' as i32) as libc::c_uchar;
    }
    if c as libc::c_int >= 'a' as i32 && c as libc::c_int <= 'z' as i32 {
        return (c as libc::c_int - 'a' as i32 + 26 as libc::c_int)
            as libc::c_uchar;
    }
    if c as libc::c_int >= '0' as i32 && c as libc::c_int <= '9' as i32 {
        return (c as libc::c_int - '0' as i32 + 52 as libc::c_int)
            as libc::c_uchar;
    }
    if c as libc::c_int == '+' as i32 {
        return 62 as libc::c_uchar;
    }
    return 63 as libc::c_uchar;
}
 extern "C" fn is_base64(mut c: libc::c_char) -> libc::c_int {
    if c as libc::c_int >= 'A' as i32 && c as libc::c_int <= 'Z' as i32
        || c as libc::c_int >= 'a' as i32 && c as libc::c_int <= 'z' as i32
        || c as libc::c_int >= '0' as i32 && c as libc::c_int <= '9' as i32
        || c as libc::c_int == '+' as i32
        || c as libc::c_int == '/' as i32
        || c as libc::c_int == '=' as i32
    {
        return TRUE;
    }
    return FALSE;
}
#[no_mangle]
pub unsafe extern "C" fn decode_base64(
    mut src: *const libc::c_char,
) -> *mut libc::c_char {
    if !src.is_null() && *src as libc::c_int != 0 {
        let mut dest: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
        let mut p: *mut libc::c_uchar = std::ptr::null_mut::<libc::c_uchar>();
        let mut k: libc::c_int = 0;
        let mut l: libc::c_int = strlen(src).wrapping_add(1 as size_t) as libc::c_int;
        let mut buf: *mut libc::c_uchar = std::ptr::null_mut::<libc::c_uchar>();
        dest = calloc(
            std::mem::size_of::<libc::c_char>() as size_t,
            (l + 13 as libc::c_int) as size_t,
        ) as *mut libc::c_char;
        if dest.is_null() {
            return std::ptr::null_mut::<libc::c_char>();
        }
        p = dest as *mut libc::c_uchar;
        buf = malloc(l as size_t) as *mut libc::c_uchar;
        if buf.is_null() {
            free(dest as *mut libc::c_void);
            return std::ptr::null_mut::<libc::c_char>();
        }
        k = 0 as libc::c_int;
        l = 0 as libc::c_int;
        while *src.offset(k as isize) != 0 {
            if is_base64(*src.offset(k as isize)) != 0 {
                let fresh0 = l;
                l = l + 1;
                *buf.offset(fresh0 as isize) = *src.offset(k as isize) as libc::c_uchar;
            }
            k += 1;
        }
        k = 0 as libc::c_int;
        while k < l {
            let mut c1: libc::c_char = 'A' as i32 as libc::c_char;
            let mut c2: libc::c_char = 'A' as i32 as libc::c_char;
            let mut c3: libc::c_char = 'A' as i32 as libc::c_char;
            let mut c4: libc::c_char = 'A' as i32 as libc::c_char;
            let mut b1: libc::c_uchar = 0 as libc::c_uchar;
            let mut b2: libc::c_uchar = 0 as libc::c_uchar;
            let mut b3: libc::c_uchar = 0 as libc::c_uchar;
            let mut b4: libc::c_uchar = 0 as libc::c_uchar;
            c1 = *buf.offset(k as isize) as libc::c_char;
            if (k + 1 as libc::c_int) < l {
                c2 = *buf.offset((k + 1 as libc::c_int) as isize) as libc::c_char;
            }
            if (k + 2 as libc::c_int) < l {
                c3 = *buf.offset((k + 2 as libc::c_int) as isize) as libc::c_char;
            }
            if (k + 3 as libc::c_int) < l {
                c4 = *buf.offset((k + 3 as libc::c_int) as isize) as libc::c_char;
            }
            b1 = decode(c1);
            b2 = decode(c2);
            b3 = decode(c3);
            b4 = decode(c4);
            let fresh1 = p;
            p = p.offset(1);
            *fresh1 = ((b1 as libc::c_int) << 2 as libc::c_int
                | b2 as libc::c_int >> 4 as libc::c_int)
                as libc::c_uchar;
            if c3 as libc::c_int != '=' as i32 {
                let fresh2 = p;
                p = p.offset(1);
                *fresh2 = ((b2 as libc::c_int & 0xf as libc::c_int)
                    << 4 as libc::c_int
                    | b3 as libc::c_int >> 2 as libc::c_int)
                    as libc::c_uchar;
            }
            if c4 as libc::c_int != '=' as i32 {
                let fresh3 = p;
                p = p.offset(1);
                *fresh3 = ((b3 as libc::c_int & 0x3 as libc::c_int)
                    << 6 as libc::c_int
                    | b4 as libc::c_int) as libc::c_uchar;
            }
            k += 4 as libc::c_int;
        }
        free(buf as *mut libc::c_void);
        return dest;
    }
    return std::ptr::null_mut::<libc::c_char>();
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

