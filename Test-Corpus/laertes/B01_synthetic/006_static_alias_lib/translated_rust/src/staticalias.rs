extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn static_alias<'a1, 'a2>(
    mut outer: Option<&'a1 mut libc::unix::c_int>,
) -> Option<&'a2 mut libc::unix::c_int> {
    static mut inner: libc::c_int = 1 as libc::c_int;
    if *borrow(& outer).unwrap() >= inner {
        inner += *borrow_mut(&mut outer).unwrap();
        return Some(&raw mut inner);
    } else {
        *borrow_mut(&mut outer).unwrap() += inner;
        return outer;
    };
}
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut initial_value: libc::c_int,
    mut iterations: libc::c_int,
) {
    let mut running_sum: _ = Some(&raw mut initial_value);
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < iterations {
        running_sum = static_alias(borrow_mut(&mut running_sum));
        printf(
            b"%d\n\0" as *const u8 as *const libc::c_char,
            *borrow_mut(&mut running_sum).unwrap(),
        );
        i += 1;
    }
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

