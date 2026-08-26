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

pub fn run(extra_bedrooms: i32) {
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

fn main() {
    let mut x = 0;
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if let Some(token) = input.split_whitespace().next() {
            if let Ok(parsed) = token.parse::<i32>() {
                x = parsed;
            }
        }
    }
    run(x);
    run(x);
}
