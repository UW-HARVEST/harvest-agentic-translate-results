extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const CHAR_MAX: libc::c_int = __SCHAR_MAX__;
#[no_mangle]
pub unsafe extern "C" fn printLine(mut line: *const libc::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const libc::c_char, line);
    }
}
#[no_mangle]
pub unsafe extern "C" fn printHexCharLine(mut charHex: libc::c_char) {
    printf(
        b"%02x\n\0" as *const u8 as *const libc::c_char,
        charHex as libc::c_int,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut data: libc::c_char = 0;
    data = CHAR_MAX as libc::c_char;
    if data as libc::c_int > 0 as libc::c_int {
        let mut result: libc::c_char =
            (data as libc::c_int * 2 as libc::c_int) as libc::c_char;
        printHexCharLine(result);
    }
}
unsafe extern "C" fn goodG2B() {
    let mut data: libc::c_char = 0;
    data = 2 as libc::c_char;
    if data as libc::c_int > 0 as libc::c_int {
        let mut result: libc::c_char =
            (data as libc::c_int * 2 as libc::c_int) as libc::c_char;
        printHexCharLine(result);
    }
}
unsafe extern "C" fn goodB2G() {
    let mut data: libc::c_char = 0;
    data = ' ' as i32 as libc::c_char;
    data = CHAR_MAX as libc::c_char;
    if data as libc::c_int > 0 as libc::c_int {
        if (data as libc::c_int) < CHAR_MAX / 2 as libc::c_int {
            let mut result: libc::c_char =
                (data as libc::c_int * 2 as libc::c_int) as libc::c_char;
            printHexCharLine(result);
        } else {
            printLine(
                b"data value is too large to perform arithmetic safely.\0" as *const u8
                    as *const libc::c_char,
            );
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    goodG2B();
    goodB2G();
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut useGood: libc::c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    };
}
pub const __SCHAR_MAX__: libc::c_int = 127 as libc::c_int;
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

