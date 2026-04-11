extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn printIntPtrLine<'a1>(mut intNumber: Option<&'a1 libc::c_int>) {
    printf(
        b"%d\n\0" as *const u8 as *const libc::c_char,
        *intNumber.unwrap(),
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut data: _ = std::ptr::null_mut::<libc::c_int>();
    printIntPtrLine(borrow(& data));
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    let mut data: libc::c_int = 0;
    data = 5 as libc::c_int;
    let mut data_addr: _ = std::ptr::null_mut::<libc::c_int>();
    data_addr = Some(&raw mut data);
    printIntPtrLine(borrow(& data_addr));
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

