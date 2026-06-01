use std::io::{self, Read, Write};
use std::sync::Mutex;

#[derive(Clone, Copy)]
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
    let mut h = THE_HOUSE.lock().unwrap();
    add_floor(&mut h);
}

fn format_one_decimal(value: f64) -> String {
    // Match C printf %.1f formatting (banker's-style isn't used; C uses round-half-away-from-zero
    // semantics depending on libc; for typical glibc it uses round-half-to-even). The values used
    // in this program (2.5, 3.5, etc.) print as "2.5", "3.5", etc. via Rust's default formatting.
    format!("{:.1}", value)
}

fn print_the_house() {
    let h = THE_HOUSE.lock().unwrap();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(
        out,
        "The house has {} floors, {} bedrooms, and {} bathrooms\n",
        h.floors,
        h.bedrooms,
        format_one_decimal(h.bathrooms)
    )
    .unwrap();
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    {
        let mut h = THE_HOUSE.lock().unwrap();
        h.bathrooms += 1.0;
    }
    print_the_house();
    {
        let mut h = THE_HOUSE.lock().unwrap();
        add_bedrooms(&mut h, extra_bedrooms);
    }
    print_the_house();
}

/// Mimic C scanf("%d", &x): skip leading whitespace, then read optional sign and decimal digits.
/// If no integer is parseable, leaves x unchanged (initial 0).
fn scanf_int(input: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip whitespace
    while i < input.len() && (input[i] as char).is_whitespace() {
        i += 1;
    }
    if i >= input.len() {
        return 0;
    }
    let mut negative = false;
    if input[i] == b'-' {
        negative = true;
        i += 1;
    } else if input[i] == b'+' {
        i += 1;
    }
    let start = i;
    let mut value: i64 = 0;
    while i < input.len() && (input[i] as char).is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((input[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return 0;
    }
    if negative {
        value = -value;
    }
    value as i32
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).unwrap();
    let x = scanf_int(&buf);
    run(x);
    run(x);
}
