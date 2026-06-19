use std::ffi::c_int;
use std::ptr;

#[derive(Copy, Clone)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

fn add_floor(house: *mut House) {
    unsafe {
        (*house).floors += 1;
    }
}

fn add_bedrooms(house: *mut House, extra_bedrooms: c_int) {
    unsafe {
        (*house).bedrooms += extra_bedrooms;
    }
}

fn add_floor_to_the_house() {
    add_floor(ptr::addr_of_mut!(THE_HOUSE));
}

fn print_the_house() {
    unsafe {
        let house = ptr::read_volatile(ptr::addr_of!(THE_HOUSE));
        println!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
            house.floors, house.bedrooms, house.bathrooms
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        (*ptr::addr_of_mut!(THE_HOUSE)).bathrooms += 1.0;
    }
    print_the_house();
    add_bedrooms(ptr::addr_of_mut!(THE_HOUSE), extra_bedrooms);
    print_the_house();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
