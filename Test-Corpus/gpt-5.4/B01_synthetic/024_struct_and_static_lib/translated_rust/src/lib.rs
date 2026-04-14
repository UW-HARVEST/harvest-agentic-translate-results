use std::os::raw::c_int;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

static THE_HOUSE: OnceLock<Mutex<House>> = OnceLock::new();

fn the_house() -> &'static Mutex<House> {
    THE_HOUSE.get_or_init(|| {
        Mutex::new(House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        })
    })
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    let mut house = the_house().lock().unwrap();
    add_floor(&mut house);
}

fn print_the_house() {
    let house = the_house().lock().unwrap();
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    {
        let mut house = the_house().lock().unwrap();
        house.bathrooms += 1.0;
    }
    print_the_house();
    {
        let mut house = the_house().lock().unwrap();
        add_bedrooms(&mut house, extra_bedrooms);
    }
    print_the_house();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    run(x);
    run(x);
}
