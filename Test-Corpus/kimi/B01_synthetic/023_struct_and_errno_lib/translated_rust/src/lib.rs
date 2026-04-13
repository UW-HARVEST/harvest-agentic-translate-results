use std::ffi::{CStr, c_char};
use std::io::{self, Write};

#[repr(C)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &House) {
    let _ = writeln!(
        io::stdout(),
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors,
        house.bedrooms,
        house.bathrooms
    );
}

fn run(the_house: &mut House, extra_bedrooms: i32) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    the_house.bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

fn parse_val(s: &str) -> Option<i32> {
    s.parse::<i32>().ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    if input.is_null() {
        let _ = writeln!(io::stdout(), "An error occurred");
        return;
    }
    let c_str = unsafe { CStr::from_ptr(input) };
    let s = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            let _ = writeln!(io::stdout(), "An error occurred");
            return;
        }
    };
    if let Some(x) = parse_val(s) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        let _ = writeln!(io::stdout(), "An error occurred");
    }
}