use std::ffi::c_int;
use std::sync::Mutex;

struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

static THE_HOUSE: Mutex<House> = Mutex::new(House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
});

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    add_floor(&mut THE_HOUSE.lock().unwrap());
}

fn print_the_house() {
    let h = THE_HOUSE.lock().unwrap();
    print!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        h.floors, h.bedrooms, h.bathrooms
    );
}

fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    THE_HOUSE.lock().unwrap().bathrooms += 1.0;
    print_the_house();
    add_bedrooms(&mut THE_HOUSE.lock().unwrap(), extra_bedrooms);
    print_the_house();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
