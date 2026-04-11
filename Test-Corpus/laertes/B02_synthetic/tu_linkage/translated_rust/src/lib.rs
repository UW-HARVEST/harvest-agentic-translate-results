#[no_mangle]
pub extern "C" fn target(mut code: libc::c_int) -> libc::c_int {
    if code < 0 as libc::c_int {
        return 7 as libc::c_int;
    }
    let mut m: libc::c_int = code % 10 as libc::c_int;
    if m == 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    if m <= 3 as libc::c_int {
        return 1 as libc::c_int;
    }
    if m <= 6 as libc::c_int {
        return 2 as libc::c_int;
    }
    if m == 7 as libc::c_int {
        return 3 as libc::c_int;
    }
    return 4 as libc::c_int;
}
