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
static mut the_house: house_t = house_t {
    floors: 2 as ::core::ffi::c_int,
    bedrooms: 5 as ::core::ffi::c_int,
    bathrooms: 2.5f64,
};
unsafe extern "C" fn add_floor(mut house: *mut house_t) {
    (*house).floors += 1;
}
unsafe extern "C" fn add_bedrooms(mut house: *mut house_t, mut extra_bedrooms: ::core::ffi::c_int) {
    (*house).bedrooms += extra_bedrooms;
}
unsafe extern "C" fn add_floor_to_the_house() {
    add_floor(&raw mut the_house);
}
unsafe extern "C" fn print_the_house() {
    printf(
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0" as *const u8
            as *const ::core::ffi::c_char,
        the_house.floors,
        the_house.bedrooms,
        the_house.bathrooms,
    );
}
#[no_mangle]
pub unsafe extern "C" fn run(mut extra_bedrooms: ::core::ffi::c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    the_house.bathrooms += 1.0f64;
    print_the_house();
    add_bedrooms(&raw mut the_house, extra_bedrooms);
    print_the_house();
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut x: ::core::ffi::c_int) {
    run(x);
    run(x);
}
