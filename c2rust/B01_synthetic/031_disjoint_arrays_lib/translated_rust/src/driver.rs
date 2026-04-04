extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn sscanf(
        __s: *const ::core::ffi::c_char,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
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
#[no_mangle]
pub unsafe extern "C" fn call_fma(
    mut data: *const ::core::ffi::c_int,
    mut len: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if len == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    let vla = len as usize;
    let mut out: Vec<::core::ffi::c_int> = ::std::vec::from_elem(0, vla);
    let vla_0 = len as usize;
    let mut ones: Vec<::core::ffi::c_int> = ::std::vec::from_elem(0, vla_0);
    let vla_1 = len as usize;
    let mut zeros: Vec<::core::ffi::c_int> = ::std::vec::from_elem(0, vla_1);
    *out.as_mut_ptr().offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < len {
        *ones.as_mut_ptr().offset(i as isize) = 1 as ::core::ffi::c_int;
        *zeros.as_mut_ptr().offset(i as isize) = 0 as ::core::ffi::c_int;
        i += 1;
    }
    fma_array(
        out.as_mut_ptr(),
        ones.as_mut_ptr(),
        data,
        zeros.as_mut_ptr(),
        len,
    );
    return *out
        .as_mut_ptr()
        .offset((len - 1 as ::core::ffi::c_int) as isize);
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut in_0: *const ::core::ffi::c_char) {
    let mut data: [::core::ffi::c_int; 100] = [0; 100];
    let mut i: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < 100 as ::core::ffi::c_int {
        let mut nb: size_t = 0;
        if sscanf(
            in_0,
            b"%d%zn\0" as *const u8 as *const ::core::ffi::c_char,
            (&raw mut data as *mut ::core::ffi::c_int).offset(i as isize)
                as *mut ::core::ffi::c_int,
            &raw mut nb,
        ) != 1 as ::core::ffi::c_int
        {
            break;
        }
        in_0 = in_0.offset(nb as isize);
        i += 1;
    }
    let mut result: ::core::ffi::c_int = call_fma(&raw mut data as *mut ::core::ffi::c_int, i);
    printf(b"%d\n\0" as *const u8 as *const ::core::ffi::c_char, result);
}
