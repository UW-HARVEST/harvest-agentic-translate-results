extern "C" {
    fn atoi(__nptr: *const libc::c_char) -> libc::c_int;
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct test {
    pub a: libc::c_int,
    pub b: libc::c_int,
}
#[no_mangle]
pub unsafe extern "C" fn find_container_of_a(mut i: *mut libc::c_int) -> *mut test {
    return ({
        (i as *mut libc::c_char)
            .offset(-(&raw mut (*std::ptr::null_mut::<test>()).a as size_t as isize))
            as *mut test
    });
}
#[no_mangle]
pub unsafe extern "C" fn find_container_of_b(mut i: *mut libc::c_int) -> *mut test {
    return ({
        (i as *mut libc::c_char)
            .offset(-(&raw mut (*std::ptr::null_mut::<test>()).b as size_t as isize))
            as *mut test
    });
}
unsafe fn main_0(
    mut argc: libc::c_int,
    mut argv: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut a: libc::c_int = atoi(*argv.offset(1 as libc::c_int as isize));
    let mut b: libc::c_int = atoi(*argv.offset(2 as libc::c_int as isize));
    let mut t: test = test { a: 0, b: 0 };
    memset(
        &raw mut t as *mut libc::c_void,
        0 as libc::c_int,
        std::mem::size_of::<test>() as size_t,
    );
    t.a = a;
    t.b = b;
    printf(
        b"%d\n\0" as *const u8 as *const libc::c_char,
        (*find_container_of_a(&raw mut t.a)).a + (*find_container_of_b(&raw mut t.b)).b,
    );
    return 0;
}
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut libc::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut libc::c_char)
        .chain(::core::iter::once(std::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as libc::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut libc::c_char,
        ) as i32)
    }
}
