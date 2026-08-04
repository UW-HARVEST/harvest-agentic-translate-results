extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
static mut y: libc::c_int = 123 as libc::c_int;
unsafe extern "C" fn multi_stage(
    mut x: libc::c_int,
    mut z: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    if x != 1 as libc::c_int {
        printf(b"Error: x != 1\n\0" as *const u8 as *const libc::c_char);
        result = 1 as libc::c_int;
    } else if y != 2 as libc::c_int {
        printf(b"Error: x == 1 but y != 2\n\0" as *const u8 as *const libc::c_char);
        result = 2 as libc::c_int;
    } else if z != 3 as libc::c_int {
        printf(
            b"Error: x == 1 and y == 2, but z != 3\n\0" as *const u8 as *const libc::c_char,
        );
        result = 3 as libc::c_int;
    } else {
        printf(b"Ok!\n\0" as *const u8 as *const libc::c_char);
        return result;
    }
    printf(b"Operation failed\n\0" as *const u8 as *const libc::c_char);
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut x: libc::c_int,
    mut local_y: libc::c_int,
    mut z: libc::c_int,
) {
    y = local_y;
    let mut result: libc::c_int = multi_stage(x, z);
    printf(
        b"Result: %d\n\0" as *const u8 as *const libc::c_char,
        result,
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

