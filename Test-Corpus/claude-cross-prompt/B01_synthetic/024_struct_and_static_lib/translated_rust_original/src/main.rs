use std::io::{self, Read, Write};

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

static mut THE_HOUSE: House = House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    // Safety: single-threaded program mirroring C global state
    unsafe {
        add_floor(&mut *std::ptr::addr_of_mut!(THE_HOUSE));
    }
}

fn print_the_house() {
    // Safety: single-threaded program mirroring C global state
    let (floors, bedrooms, bathrooms) = unsafe {
        let h = &*std::ptr::addr_of!(THE_HOUSE);
        (h.floors, h.bedrooms, h.bathrooms)
    };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(
        out,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        floors, bedrooms, bathrooms
    );
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    // Safety: single-threaded program mirroring C global state
    unsafe {
        let h = &mut *std::ptr::addr_of_mut!(THE_HOUSE);
        h.bathrooms += 1.0;
    }
    print_the_house();
    // Safety: single-threaded program mirroring C global state
    unsafe {
        add_bedrooms(&mut *std::ptr::addr_of_mut!(THE_HOUSE), extra_bedrooms);
    }
    print_the_house();
}

fn driver(x: i32) {
    run(x);
    run(x);
}

/// Read a single integer from stdin emulating C's `scanf("%d", &x)` behavior:
/// skip leading whitespace (including newlines), then parse an optional sign
/// followed by digits, stopping at the first non-digit.
fn scanf_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace
    while *pos < input.len() && (input[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= input.len() {
        return None;
    }
    let start = *pos;
    if input[*pos] == b'-' || input[*pos] == b'+' {
        *pos += 1;
    }
    let digits_start = *pos;
    while *pos < input.len() && (input[*pos] as char).is_ascii_digit() {
        *pos += 1;
    }
    if *pos == digits_start {
        return None;
    }
    let s = std::str::from_utf8(&input[start..*pos]).ok()?;
    s.parse::<i32>().ok()
}

fn main() {
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() {
        return;
    }
    let mut pos = 0usize;
    let x = scanf_int(&buf, &mut pos).unwrap_or(0);
    driver(x);
}
