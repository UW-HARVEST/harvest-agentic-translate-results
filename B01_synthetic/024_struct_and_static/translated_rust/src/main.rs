use std::io::{self, Read};

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

unsafe fn add_floor(house: *mut House) {
    (*house).floors += 1;
}

unsafe fn add_bedrooms(house: *mut House, extra_bedrooms: i32) {
    (*house).bedrooms += extra_bedrooms;
}

unsafe fn add_floor_to_the_house() {
    add_floor(&raw mut THE_HOUSE);
}

unsafe fn print_the_house() {
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        THE_HOUSE.floors, THE_HOUSE.bedrooms, THE_HOUSE.bathrooms
    );
}

unsafe fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    THE_HOUSE.bathrooms += 1.0;
    print_the_house();
    add_bedrooms(&raw mut THE_HOUSE, extra_bedrooms);
    print_the_house();
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap();
    unsafe {
        run(x);
        run(x);
    }
}
