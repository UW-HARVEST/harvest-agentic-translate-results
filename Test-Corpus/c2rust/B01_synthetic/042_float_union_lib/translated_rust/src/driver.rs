extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub type uint64_t = __uint64_t;
pub type __uint64_t = u64;
#[derive(Copy, Clone)]
#[repr(C)]
pub union raw_double_t {
    pub x: uint64_t,
    pub f: ::core::ffi::c_double,
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut f: ::core::ffi::c_double) {
    let mut u: raw_double_t = raw_double_t { f: f };
    printf(
        b"%llx %a %.4f\n\0" as *const u8 as *const ::core::ffi::c_char,
        u.x,
        f,
        f,
    );
}
