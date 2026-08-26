use std::io::{self, Read};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
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

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
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

pub fn run(extra_bedrooms: i32) {
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

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x = input
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    run(x);
    run(x);
}
