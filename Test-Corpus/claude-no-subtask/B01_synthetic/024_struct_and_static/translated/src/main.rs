use std::io::{self, Read, Write};
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

fn format_bathrooms_one_decimal(value: f64) -> String {
    // Match C printf("%.1f") behavior using Rust's default rounding which
    // is round-half-to-even (matches glibc printf default rounding mode).
    format!("{:.1}", value)
}

fn print_the_house() {
    let house = THE_HOUSE.lock().unwrap();
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(
        handle,
        "The house has {} floors, {} bedrooms, and {} bathrooms",
        house.floors,
        house.bedrooms,
        format_bathrooms_one_decimal(house.bathrooms),
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

/// Read an integer from stdin, mimicking C's `scanf("%d", &x)`.
/// scanf skips leading whitespace (including newlines), then reads
/// an optional sign and decimal digits. If no valid integer is found,
/// the destination is unchanged (x stays as its initial value of 0).
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace.
    while *pos < input.len() && (input[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= input.len() {
        return None;
    }
    let start = *pos;
    let mut i = *pos;
    if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < input.len() && (input[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        // No digits found; nothing matched.
        *pos = start;
        return None;
    }
    let s = std::str::from_utf8(&input[start..i]).ok()?;
    *pos = i;
    // C's scanf with %d performs wrapping on overflow in practice is
    // undefined behavior; use wrapping parse via i64 then cast.
    match s.parse::<i64>() {
        Ok(v) => Some(v as i32),
        Err(_) => None,
    }
}

fn main() {
    let mut buf = Vec::new();
    let _ = io::stdin().read_to_end(&mut buf);
    let mut pos = 0usize;
    let mut x: i32 = 0;
    if let Some(v) = scanf_int(&buf, &mut pos) {
        x = v;
    }
    run(x);
    run(x);
}
