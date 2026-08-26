use std::ffi::{CStr, c_char};
use std::os::raw::c_int;
use std::sync::Mutex;

#[derive(Clone, Copy)]
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
    let mut house = THE_HOUSE.lock().unwrap();
    add_floor(&mut house);
}

fn print_the_house() {
    let house = THE_HOUSE.lock().unwrap();
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(extra_bedrooms: c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    {
        let mut house = THE_HOUSE.lock().unwrap();
        house.bathrooms += 1.0;
    }
    print_the_house();
    {
        let mut house = THE_HOUSE.lock().unwrap();
        add_bedrooms(&mut house, extra_bedrooms);
    }
    print_the_house();
}

fn parse_val(s: &str) -> Option<c_int> {
    s.trim().parse::<c_int>().ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    if input.is_null() {
        println!("An error occurred");
        return;
    }

    let input_str = unsafe { CStr::from_ptr(input) };
    let input_str = match input_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            println!("An error occurred");
            return;
        }
    };

    if let Some(x) = parse_val(input_str) {
        run(x);
        run(x);
    } else {
        println!("An error occurred");
    }
}
