extern "C" {
    fn __errno_location() -> *mut libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn strtol(
        __nptr: *const libc::c_char,
        __endptr: *mut *mut libc::c_char,
        __base: libc::c_int,
    ) -> libc::c_long;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct house_t {
    pub floors: libc::c_int,
    pub bedrooms: libc::c_int,
    pub bathrooms: libc::c_double,
}
pub const INT_MAX: libc::c_int = __INT_MAX__;
pub const INT_MIN: libc::c_int = -__INT_MAX__ - 1 as libc::c_int;
static mut the_house: house_t = house_t {
    floors: 2 as libc::c_int,
    bedrooms: 5 as libc::c_int,
    bathrooms: 2.5f64,
};
unsafe extern "C" fn add_floor(mut house: *mut house_t) {
    (*house).floors += 1;
}
unsafe extern "C" fn add_bedrooms(mut house: *mut house_t, mut extra_bedrooms: libc::c_int) {
    (*house).bedrooms += extra_bedrooms;
}
unsafe extern "C" fn add_floor_to_the_house() {
    add_floor(&raw mut the_house);
}
unsafe extern "C" fn print_the_house() {
    printf(
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0" as *const u8
            as *const libc::c_char,
        the_house.floors,
        the_house.bedrooms,
        the_house.bathrooms,
    );
}
#[no_mangle]
pub unsafe extern "C" fn run(mut extra_bedrooms: libc::c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    the_house.bathrooms += 1.0f64;
    print_the_house();
    add_bedrooms(&raw mut the_house, extra_bedrooms);
    print_the_house();
}
unsafe extern "C" fn parse_val(
    mut str: *const libc::c_char,
    mut val: *mut libc::c_int,
) -> bool {
    *__errno_location() = 0 as libc::c_int;
    let mut endp: *mut libc::c_char = str as *mut libc::c_char;
    let mut tmp: libc::c_long = strtol(str, &raw mut endp, 10 as libc::c_int);
    if endp != str as *mut libc::c_char
        && *__errno_location() == 0 as libc::c_int
        && tmp >= INT_MIN as libc::c_long
        && tmp <= INT_MAX as libc::c_long
    {
        *val = tmp as libc::c_int;
        return true_0 != 0;
    } else {
        return false_0 != 0;
    };
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut in_0: *const libc::c_char) {
    let mut x: libc::c_int = 0;
    if parse_val(in_0, &raw mut x) {
        run(x);
        run(x);
    } else {
        printf(b"An error occurred\n\0" as *const u8 as *const libc::c_char);
    };
}
pub const __INT_MAX__: libc::c_int = 2147483647 as libc::c_int;
pub const true_0: libc::c_int = 1 as libc::c_int;
pub const false_0: libc::c_int = 0 as libc::c_int;
