extern "C" {
    fn __errno_location() -> *mut ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strtol(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct house_t {
    pub floors: ::core::ffi::c_int,
    pub bedrooms: ::core::ffi::c_int,
    pub bathrooms: ::core::ffi::c_double,
}
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const INT_MIN: ::core::ffi::c_int = -__INT_MAX__ - 1 as ::core::ffi::c_int;
unsafe extern "C" fn add_floor(mut house: *mut house_t) {
    (*house).floors += 1;
}
unsafe extern "C" fn add_bedrooms(mut house: *mut house_t, mut extra_bedrooms: ::core::ffi::c_int) {
    (*house).bedrooms += extra_bedrooms;
}
unsafe extern "C" fn print_house(mut house: *mut house_t) {
    printf(
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0" as *const u8
            as *const ::core::ffi::c_char,
        (*house).floors,
        (*house).bedrooms,
        (*house).bathrooms,
    );
}
#[no_mangle]
pub unsafe extern "C" fn run(mut the_house: *mut house_t, mut extra_bedrooms: ::core::ffi::c_int) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    (*the_house).bathrooms += 1.0f64;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}
unsafe extern "C" fn parse_val(
    mut str: *const ::core::ffi::c_char,
    mut val: *mut ::core::ffi::c_int,
) -> bool {
    *__errno_location() = 0 as ::core::ffi::c_int;
    let mut endp: *mut ::core::ffi::c_char = str as *mut ::core::ffi::c_char;
    let mut tmp: ::core::ffi::c_long = strtol(str, &raw mut endp, 10 as ::core::ffi::c_int);
    if endp != str as *mut ::core::ffi::c_char
        && *__errno_location() == 0 as ::core::ffi::c_int
        && tmp >= INT_MIN as ::core::ffi::c_long
        && tmp <= INT_MAX as ::core::ffi::c_long
    {
        *val = tmp as ::core::ffi::c_int;
        return true_0 != 0;
    } else {
        return false_0 != 0;
    };
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut in_0: *const ::core::ffi::c_char) {
    let mut x: ::core::ffi::c_int = 0;
    if parse_val(in_0, &raw mut x) {
        let mut the_house: house_t = house_t {
            floors: 2 as ::core::ffi::c_int,
            bedrooms: 5 as ::core::ffi::c_int,
            bathrooms: 2.5f64,
        };
        run(&raw mut the_house, x);
        run(&raw mut the_house, x);
    } else {
        printf(b"An error occurred\n\0" as *const u8 as *const ::core::ffi::c_char);
    };
}
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
