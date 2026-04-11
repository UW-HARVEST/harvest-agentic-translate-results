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
#[no_mangle]
pub unsafe extern "C" fn printIntLine(mut intNumber: libc::c_int) {
    printf(
        b"%d\n\0" as *const u8 as *const libc::c_char,
        intNumber,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad(mut data: libc::c_int) {
    let mut i: libc::c_int = 0;
    let mut buffer: [libc::c_int; 10] = [0 as libc::c_int; 10];
    if data >= 0 as libc::c_int {
        buffer[data as usize] = 1 as libc::c_int;
        i = 0 as libc::c_int;
        while i < 10 as libc::c_int {
            printIntLine(buffer[i as usize]);
            i += 1;
        }
    } else {
        printLine(b"ERROR: Array index is negative.\0" as *const u8 as *const libc::c_char);
    };
}
unsafe extern "C" fn goodG2B() {
    let mut data: libc::c_int = 7 as libc::c_int;
    let mut i: libc::c_int = 0;
    let mut buffer: [libc::c_int; 10] = [0 as libc::c_int; 10];
    if data >= 0 as libc::c_int {
        buffer[data as usize] = 1 as libc::c_int;
        i = 0 as libc::c_int;
        while i < 10 as libc::c_int {
            printIntLine(buffer[i as usize]);
            i += 1;
        }
    } else {
        printLine(b"ERROR: Array index is negative.\0" as *const u8 as *const libc::c_char);
    };
}
unsafe extern "C" fn goodB2G(mut data: libc::c_int) {
    let mut i: libc::c_int = 0;
    let mut buffer: [libc::c_int; 10] = [0 as libc::c_int; 10];
    if data >= 0 as libc::c_int && data < 10 as libc::c_int {
        buffer[data as usize] = 1 as libc::c_int;
        i = 0 as libc::c_int;
        while i < 10 as libc::c_int {
            printIntLine(buffer[i as usize]);
            i += 1;
        }
    } else {
        printLine(
            b"ERROR: Array index is out-of-bounds\0" as *const u8 as *const libc::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn good(mut data: libc::c_int) {
    goodG2B();
    goodB2G(data);
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut goodData: libc::c_int, mut badData: libc::c_int) {
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

