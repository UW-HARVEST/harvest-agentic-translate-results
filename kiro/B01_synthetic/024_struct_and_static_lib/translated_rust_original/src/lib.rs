use std::os::raw::c_int;

struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

static mut THE_HOUSE: HouseT = HouseT {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

fn add_floor(house: *mut HouseT) {
    unsafe { (*house).floors += 1 };
}

fn add_bedrooms(house: *mut HouseT, extra_bedrooms: c_int) {
    unsafe { (*house).bedrooms += extra_bedrooms };
}

fn add_floor_to_the_house() {
    add_floor(&raw mut THE_HOUSE);
}

fn print_the_house() {
    unsafe {
        print!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
            (*(&raw const THE_HOUSE)).floors,
            (*(&raw const THE_HOUSE)).bedrooms,
            (*(&raw const THE_HOUSE)).bathrooms
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        (*(&raw mut THE_HOUSE)).bathrooms += 1.0;
    }
    print_the_house();
    add_bedrooms(&raw mut THE_HOUSE, extra_bedrooms);
    print_the_house();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
