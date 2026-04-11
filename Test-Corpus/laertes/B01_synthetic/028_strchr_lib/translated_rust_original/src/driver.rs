extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn strchr(__s: *const libc::c_char, __c: libc::c_int)
        -> *mut libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn foo(
    mut in_0: *const libc::c_char,
    mut c: libc::c_char,
) -> libc::c_int {
    let mut res: libc::c_int = 0 as libc::c_int;
    let mut s: *const libc::c_char = in_0;
    loop {
        s = strchr(s, c as libc::c_int);
        if s.is_null() {
            break;
        }
        res += 1;
        s = s.offset(1);
    }
    return res;
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut in_0: *const libc::c_char) {
    printf(
        b"A: %d\n\0" as *const u8 as *const libc::c_char,
        foo(in_0, 'A' as i32 as libc::c_char),
    );
    printf(
        b"x: %d\n\0" as *const u8 as *const libc::c_char,
        foo(in_0, 'x' as i32 as libc::c_char),
    );
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

