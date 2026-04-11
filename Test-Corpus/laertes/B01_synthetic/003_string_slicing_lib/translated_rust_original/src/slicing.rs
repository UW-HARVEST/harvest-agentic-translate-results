extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn strlen(__s: *const libc::c_char) -> size_t;
}
pub type size_t = usize;
#[no_mangle]
pub unsafe extern "C" fn slice<'a1, 'a2>(
    mut mystr: * mut libc::linux_like::linux::gnu::b64::x86_64::c_char,
    mut start_ptr: Option<&'a1 mut libc::c_int>,
    mut stop_ptr: Option<&'a2 mut libc::c_int>,
) -> libc::c_int {
    let mut len: size_t = strlen(mystr);
    let mut end: _ = std::ptr::null_mut::<libc::c_char>();
    let mut start: libc::c_int = 0;
    let mut stop: libc::c_int = 0;
    if !borrow(& start_ptr).is_none() {
        start = *borrow_mut(&mut start_ptr).unwrap();
        if start as size_t > len {
            printf(
                b"Error: start is off the end of the string!\n\0" as *const u8
                    as *const libc::c_char,
            );
            return 1 as libc::c_int;
        }
    } else {
        start = 0 as libc::c_int;
    }
    if !borrow(& stop_ptr).is_none() {
        stop = *borrow_mut(&mut stop_ptr).unwrap();
        if stop as size_t > len {
            printf(
                b"Error: stop is off the end of the string!\n\0" as *const u8
                    as *const libc::c_char,
            );
            return 1 as libc::c_int;
        }
        if stop <= start {
            printf(
                b"Error: stop must come after start!\n\0" as *const u8
                    as *const libc::c_char,
            );
            return 1 as libc::c_int;
        }
    } else {
        stop = len as libc::c_int;
    }
    printf(
        b"%.*s\n\0" as *const u8 as *const libc::c_char,
        stop - start,
        mystr.offset(start as isize),
    );
    return 0 as libc::c_int;
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

