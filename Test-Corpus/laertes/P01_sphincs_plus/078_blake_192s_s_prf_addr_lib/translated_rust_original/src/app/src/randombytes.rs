extern "C" {
    fn open(
        __file: *const libc::c_char,
        __oflag: libc::c_int,
        ...
    ) -> libc::c_int;
    fn read(__fd: libc::c_int, __buf: *mut libc::c_void, __nbytes: size_t)
        -> ssize_t;
    fn sleep(__seconds: libc::c_uint) -> libc::c_uint;
}
pub type __ssize_t = libc::c_long;
pub type ssize_t = __ssize_t;
pub type size_t = usize;
pub const O_RDONLY: libc::c_int = 0 as libc::c_int;
static mut fd: libc::c_int = -(1 as libc::c_int);
#[no_mangle]
pub unsafe extern "C" fn randombytes(
    mut x: *mut libc::c_uchar,
    mut xlen: libc::c_ulonglong,
) {
    let mut i: libc::c_ulonglong = 0;
    if fd == -(1 as libc::c_int) {
        loop {
            fd = open(
                b"/dev/urandom\0" as *const u8 as *const libc::c_char,
                O_RDONLY,
            );
            if fd != -(1 as libc::c_int) {
                break;
            }
            sleep(1 as libc::c_uint);
        }
    }
    while xlen > 0 as libc::c_ulonglong {
        if xlen < 1048576 as libc::c_ulonglong {
            i = xlen;
        } else {
            i = 1048576 as libc::c_ulonglong;
        }
        i = read(fd, x as *mut libc::c_void, i as size_t) as libc::c_ulonglong;
        if i < 1 as libc::c_ulonglong {
            sleep(1 as libc::c_uint);
        } else {
            x = x.offset(i as isize);
            xlen = xlen.wrapping_sub(i);
        }
    }
}
