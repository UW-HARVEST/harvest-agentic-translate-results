use std::ffi::{CStr, c_char};
use std::io::Write;
use std::sync::Mutex;

struct House {
    floors: i32,
    bedrooms: i32,
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

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    let mut house = THE_HOUSE.lock().unwrap();
    add_floor(&mut house);
}

fn print_the_house() {
    let house = THE_HOUSE.lock().unwrap();
    let _ = writeln!(
        std::io::stdout(),
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors,
        house.bedrooms,
        house.bathrooms
    );
}

fn run(extra_bedrooms: i32) {
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

fn parse_val(str: &str) -> Option<i32> {
    str.parse::<i32>().ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_ptr: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(in_ptr) };
    let str_slice = c_str.to_str().unwrap_or("");
    
    if let Some(x) = parse_val(str_slice) {
        run(x);
        run(x);
    } else {
        let _ = writeln!(std::io::stdout(), "An error occurred");
    }
}
