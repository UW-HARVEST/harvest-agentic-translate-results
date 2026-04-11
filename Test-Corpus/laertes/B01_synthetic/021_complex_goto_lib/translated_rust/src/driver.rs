extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut x: libc::c_int, mut y: libc::c_int) {
    let mut current_block_6: u64;
    while x > 0 as libc::c_int || y > 0 as libc::c_int {
        printf(b"loop\n\0" as *const u8 as *const libc::c_char);
        if x == 1 as libc::c_int && y == 4 as libc::c_int {
            current_block_6 = 13277901459238179029;
        } else {
            current_block_6 = 861247850213060928;
        }
        loop {
            match current_block_6 {
                861247850213060928 => {
                    if x > 0 as libc::c_int {
                        printf(b"x\n\0" as *const u8 as *const libc::c_char);
                        x -= 1;
                    }
                    current_block_6 = 13277901459238179029;
                }
                _ => {
                    if y == 0 as libc::c_int {
                        break;
                    }
                    printf(b"y\n\0" as *const u8 as *const libc::c_char);
                    y -= 1;
                    if x < 3 as libc::c_int {
                        current_block_6 = 861247850213060928;
                    } else {
                        break;
                    }
                }
            }
        }
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

