





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
fn add_floor(house: &mut house_t) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut house_t, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    unsafe {
        add_floor(&mut the_house);
    }
}

fn print_the_house() {
    unsafe {
        println!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
            the_house.floors,
            the_house.bedrooms,
            the_house.bathrooms,
        );
    }
}

#[no_mangle]
pub fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        the_house.bathrooms += 1.0f64;
        print_the_house();
        add_bedrooms(&mut the_house, extra_bedrooms);
    }
    print_the_house();
}

#[no_mangle]
pub fn driver(x: i32) {
    run(x);
    run(x);
}

