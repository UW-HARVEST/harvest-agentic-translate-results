extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
static mut y: ::core::ffi::c_int = 123 as ::core::ffi::c_int;
unsafe extern "C" fn multi_stage(
    mut x: ::core::ffi::c_int,
    mut z: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if x != 1 as ::core::ffi::c_int {
        printf(b"Error: x != 1\n\0" as *const u8 as *const ::core::ffi::c_char);
        result = 1 as ::core::ffi::c_int;
    } else if y != 2 as ::core::ffi::c_int {
        printf(b"Error: x == 1 but y != 2\n\0" as *const u8 as *const ::core::ffi::c_char);
        result = 2 as ::core::ffi::c_int;
    } else if z != 3 as ::core::ffi::c_int {
        printf(
            b"Error: x == 1 and y == 2, but z != 3\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        result = 3 as ::core::ffi::c_int;
    } else {
        printf(b"Ok!\n\0" as *const u8 as *const ::core::ffi::c_char);
        return result;
    }
    printf(b"Operation failed\n\0" as *const u8 as *const ::core::ffi::c_char);
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut x: ::core::ffi::c_int,
    mut local_y: ::core::ffi::c_int,
    mut z: ::core::ffi::c_int,
) {
    y = local_y;
    let mut result: ::core::ffi::c_int = multi_stage(x, z);
    printf(
        b"Result: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        result,
    );
}
