extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct house_t {
    pub floors: ::core::ffi::c_int,
    pub bedrooms: ::core::ffi::c_int,
    pub bathrooms: ::core::ffi::c_double,
}
unsafe extern "C" fn print_hex(mut p: *mut ::core::ffi::c_uchar, mut len: ::core::ffi::c_int) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < len {
        printf(
            b"%02x\0" as *const u8 as *const ::core::ffi::c_char,
            *p.offset(i as isize) as ::core::ffi::c_int,
        );
        i += 1;
    }
    printf(b"\n\0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut floors: ::core::ffi::c_int) {
    let mut house: house_t = house_t {
        floors: 0 as ::core::ffi::c_int,
        bedrooms: 0,
        bathrooms: 0.,
    };
    house.floors = floors;
    house.bedrooms = 3 as ::core::ffi::c_int;
    house.bathrooms = 2.0f64;
    print_hex(
        &raw mut house as *mut ::core::ffi::c_uchar,
        ::core::mem::size_of::<house_t>() as ::core::ffi::c_int,
    );
}
