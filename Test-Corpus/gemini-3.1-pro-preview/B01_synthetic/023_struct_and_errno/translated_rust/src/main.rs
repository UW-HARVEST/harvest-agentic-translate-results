use std::io;

pub struct HouseT {
    pub floors: i32,
    pub bedrooms: i32,
    pub bathrooms: f64,
}

fn add_floor(house: &mut HouseT) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut HouseT, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &HouseT) {
    println!("The house has {} floors, {} bedrooms, and {:.1} bathrooms", house.floors, house.bedrooms, house.bathrooms);
}

pub fn run(the_house: &mut HouseT, extra_bedrooms: i32) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    the_house.bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

fn parse_val(s: &str) -> Option<i32> {
    let s = s.trim_start();
    let end = s.find(|c: char| !c.is_ascii_digit() && c != '+' && c != '-').unwrap_or(s.len());
    s[..end].parse::<i32>().ok()
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or(0);
    if let Some(x) = parse_val(&input) {
        let mut the_house = HouseT {
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
