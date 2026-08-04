extern "C" {
    fn fabs(__x: libc::c_double) -> libc::c_double;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn printLine(mut line: *const libc::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const libc::c_char, line);
    }
}
#[no_mangle]
pub unsafe extern "C" fn printIntLine(mut intNumber: libc::c_int) {
    printf(
        b"%d\n\0" as *const u8 as *const libc::c_char,
        intNumber,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad(mut data: libc::c_float) {
    let mut result: libc::c_int =
        (100.0f64 / data as libc::c_double) as libc::c_int;
    printIntLine(result);
}
unsafe extern "C" fn goodG2B() {
    let mut data: libc::c_float = 0.;
    data = 2.0f32;
    let mut result: libc::c_int =
        (100.0f64 / data as libc::c_double) as libc::c_int;
    printIntLine(result);
}
unsafe extern "C" fn goodB2G(mut data: libc::c_float) {
    if fabs(data as libc::c_double) > 0.000001f64 {
        let mut result: libc::c_int =
            (100.0f64 / data as libc::c_double) as libc::c_int;
        printIntLine(result);
    } else {
        printLine(
            b"This would result in a divide by zero\0" as *const u8 as *const libc::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn good(mut data: libc::c_float) {
    goodG2B();
    goodB2G(data);
}
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut goodData: libc::c_float,
    mut badData: libc::c_float,
) {
    printLine(b"Calling good()...\0" as *const u8 as *const libc::c_char);
    good(goodData);
    printLine(b"Finished good()\0" as *const u8 as *const libc::c_char);
    printLine(b"Calling bad()...\0" as *const u8 as *const libc::c_char);
    bad(badData);
    printLine(b"Finished bad()\0" as *const u8 as *const libc::c_char);
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

