extern "C" {
    fn open(
        __file: *const ::core::ffi::c_char,
        __oflag: ::core::ffi::c_int,
        ...
    ) -> ::core::ffi::c_int;
    fn read(__fd: ::core::ffi::c_int, __buf: *mut ::core::ffi::c_void, __nbytes: size_t)
        -> ssize_t;
    fn sleep(__seconds: ::core::ffi::c_uint) -> ::core::ffi::c_uint;
}
pub type __ssize_t = ::core::ffi::c_long;
pub type ssize_t = __ssize_t;
pub type size_t = usize;
pub const O_RDONLY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
static mut fd: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
#[no_mangle]
pub unsafe extern "C" fn randombytes(
    mut x: *mut ::core::ffi::c_uchar,
    mut xlen: ::core::ffi::c_ulonglong,
) {
    let mut i: ::core::ffi::c_ulonglong = 0;
    if fd == -(1 as ::core::ffi::c_int) {
        loop {
            fd = open(
                b"/dev/urandom\0" as *const u8 as *const ::core::ffi::c_char,
                O_RDONLY,
            );
            if fd != -(1 as ::core::ffi::c_int) {
                break;
            }
            sleep(1 as ::core::ffi::c_uint);
        }
    }
    while xlen > 0 as ::core::ffi::c_ulonglong {
        if xlen < 1048576 as ::core::ffi::c_ulonglong {
            i = xlen;
        } else {
            i = 1048576 as ::core::ffi::c_ulonglong;
        }
        i = read(fd, x as *mut ::core::ffi::c_void, i as size_t) as ::core::ffi::c_ulonglong;
        if i < 1 as ::core::ffi::c_ulonglong {
            sleep(1 as ::core::ffi::c_uint);
        } else {
            x = x.offset(i as isize);
            xlen = xlen.wrapping_sub(i);
        }
    }
}
