extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type size_t = usize;
#[no_mangle]
pub unsafe extern "C" fn fma_array(
    mut out: *mut ::core::ffi::c_int,
    mut mul1: *const ::core::ffi::c_int,
    mut mul2: *const ::core::ffi::c_int,
    mut add: *const ::core::ffi::c_int,
    mut len: ::core::ffi::c_int,
) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < len {
        *out.offset(i as isize) =
            *mul1.offset(i as isize) * *mul2.offset(i as isize) + *add.offset(i as isize);
        i += 1;
    }
}
unsafe extern "C" fn inner(mut out: *mut ::core::ffi::c_int, mut len: ::core::ffi::c_int) {
    fma_array(out, out, out, out, len);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < len {
        printf(
            b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
            *out.offset(i as isize),
        );
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut data: *const ::core::ffi::c_int, mut len: ::core::ffi::c_int) {
    let vla = len as usize;
    let mut out: Vec<::core::ffi::c_int> = ::std::vec::from_elem(0, vla);
    memcpy(
        out.as_mut_ptr() as *mut ::core::ffi::c_void,
        data as *const ::core::ffi::c_void,
        (len as size_t).wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as size_t),
    );
    inner(out.as_mut_ptr(), len);
}
