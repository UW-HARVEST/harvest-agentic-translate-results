use std::io;
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
    if let Ok(mut house) = THE_HOUSE.lock() {
        add_floor(&mut house);
    }
}

fn print_the_house() {
    if let Ok(house) = THE_HOUSE.lock() {
        println!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
            house.floors, house.bedrooms, house.bathrooms
        );
    }
}

pub fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    if let Ok(mut house) = THE_HOUSE.lock() {
        house.bathrooms += 1.0;
    }
    print_the_house();
    if let Ok(mut house) = THE_HOUSE.lock() {
        add_bedrooms(&mut house, extra_bedrooms);
    }
    print_the_house();
}

fn parse_val(s: &str) -> Option<i32> {
    s.trim().parse::<i32>().ok()
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or(0);
    if let Some(x) = parse_val(&input) {
        run(x);
        run(x);
    } else {
        println!("An error occurred");
    }
}
