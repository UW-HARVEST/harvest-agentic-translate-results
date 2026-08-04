use std::ffi::{CStr, c_char};
use std::os::raw::c_int;

struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: c_int) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &House) {
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(the_house: &mut House, extra_bedrooms: c_int) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    the_house.bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

fn parse_val(s: &str) -> Option<c_int> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = trimmed.parse::<i64>().ok()?;
    c_int::try_from(parsed).ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    if input.is_null() {
        println!("An error occurred");
        return;
    }

    let c_str = unsafe { CStr::from_ptr(input) };
    let input_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            println!("An error occurred");
            return;
        }
    };

    if let Some(x) = parse_val(input_str) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        println!("An error occurred");
    }
}
