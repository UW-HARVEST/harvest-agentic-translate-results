extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
}
pub type size_t = usize;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn printLine(mut line: *const libc::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const libc::c_char, line);
    }
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut data: libc::c_int) {
    let mut source: [libc::c_char; 100] = [0; 100];
    let mut dest: [libc::c_char; 100] = std::mem::transmute::<
        [u8; 100],
        [libc::c_char; 100],
    >(
        *b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    );
    memset(
        &raw mut source as *mut libc::c_char as *mut libc::c_void,
        'A' as i32,
        (100 as libc::c_int - 1 as libc::c_int) as size_t,
    );
    source[(100 as libc::c_int - 1 as libc::c_int) as usize] =
        '\0' as i32 as libc::c_char;
    if data < 100 as libc::c_int {
        strncpy(
            &raw mut dest as *mut libc::c_char,
            &raw mut source as *mut libc::c_char,
            data as size_t,
        );
        dest[data as usize] = '\0' as i32 as libc::c_char;
    }
    printLine(&raw mut dest as *mut libc::c_char);
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

