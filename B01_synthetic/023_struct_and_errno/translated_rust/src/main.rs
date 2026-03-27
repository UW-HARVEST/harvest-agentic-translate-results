use std::io::{self, BufRead};
use driver::{House, run, parse_val};

fn main() {
    let mut input = String::new();
    let stdin = io::stdin();
    let _ = stdin.lock().read_line(&mut input);
    if let Some(x) = parse_val(&input) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        print!("An error occurred\n");
    }
}
