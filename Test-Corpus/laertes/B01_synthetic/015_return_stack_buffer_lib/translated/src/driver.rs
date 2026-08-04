extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn printLine(mut line: *const libc::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const libc::c_char, line);
    }
}
unsafe extern "C" fn helperBad() -> *mut libc::c_char {
    let mut charString: [libc::c_char; 17] =
        std::mem::transmute::<[u8; 17], [libc::c_char; 17]>(*b"helperBad string\0");
    return &raw mut charString as *mut libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    printLine(helperBad());
}
unsafe extern "C" fn helperGood1() -> *mut libc::c_char {
    static mut charString: [libc::c_char; 19] = unsafe {
        std::mem::transmute::<[u8; 19], [libc::c_char; 19]>(*b"helperGood1 string\0")
    };
    return &raw mut charString as *mut libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    printLine(helperGood1());
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut useGood: libc::c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    };
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

