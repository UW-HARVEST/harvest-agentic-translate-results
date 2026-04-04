extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn cleanup(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut c: ::core::ffi::c_int,
    mut d: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut numbers: [::core::ffi::c_int; 4] = [a, b, c, d];
    let mut dynamic_str: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut expected_str: *const ::core::ffi::c_char =
        b"VALID\0" as *const u8 as *const ::core::ffi::c_char;
    let mut input_str: *const ::core::ffi::c_char =
        b"VALID\0" as *const u8 as *const ::core::ffi::c_char;
    if strncmp(input_str, expected_str, strlen(expected_str)) != 0 as ::core::ffi::c_int {
        printf(b"Input string validation failed.\n\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < 4 as ::core::ffi::c_int {
            let mut current_block_5: u64;
            match numbers[i as usize] {
                10 => {
                    result += 10 as ::core::ffi::c_int;
                    current_block_5 = 8645023485768097611;
                }
                20 => {
                    current_block_5 = 8645023485768097611;
                }
                30 => {
                    result += 30 as ::core::ffi::c_int;
                    current_block_5 = 14909307599308983100;
                }
                40 => {
                    current_block_5 = 14909307599308983100;
                }
                _ => {
                    result += numbers[i as usize];
                    current_block_5 = 1841672684692190573;
                }
            }
            match current_block_5 {
                8645023485768097611 => {
                    result += 20 as ::core::ffi::c_int;
                }
                14909307599308983100 => {
                    result += 40 as ::core::ffi::c_int;
                }
                _ => {}
            }
            i += 1;
        }
        dynamic_str = malloc(
            (50 as size_t).wrapping_mul(::core::mem::size_of::<::core::ffi::c_char>() as size_t),
        ) as *mut ::core::ffi::c_char;
        if dynamic_str.is_null() {
            printf(b"Memory allocation failed.\n\0" as *const u8 as *const ::core::ffi::c_char);
        } else {
            snprintf(
                dynamic_str,
                50 as size_t,
                b"Processed numbers: %s\0" as *const u8 as *const ::core::ffi::c_char,
                b"numbers\0" as *const u8 as *const ::core::ffi::c_char,
            );
            printf(
                b"%s\n\0" as *const u8 as *const ::core::ffi::c_char,
                dynamic_str,
            );
        }
    }
    cleanup_resources(dynamic_str);
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn print_result(
    mut label: *const ::core::ffi::c_char,
    mut result: ::core::ffi::c_int,
) {
    printf(
        b"%s: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        label,
        result,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cleanup_resources(mut dynamic_str: *mut ::core::ffi::c_char) {
    if !dynamic_str.is_null() {
        free(dynamic_str as *mut ::core::ffi::c_void);
        dynamic_str = ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
}
