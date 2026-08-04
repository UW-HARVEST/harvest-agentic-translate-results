use std::io::{self, Read, Write};

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
    unsafe {
        add_floor(&mut *std::ptr::addr_of_mut!(THE_HOUSE));
    }
}

fn print_the_house() {
    unsafe {
        let h = &*std::ptr::addr_of!(THE_HOUSE);
        print!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
            h.floors, h.bedrooms, h.bathrooms
        );
    }
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    unsafe {
        let h = &mut *std::ptr::addr_of_mut!(THE_HOUSE);
        h.bathrooms += 1.0;
    }
    print_the_house();
    unsafe {
        add_bedrooms(&mut *std::ptr::addr_of_mut!(THE_HOUSE), extra_bedrooms);
    }
    print_the_house();
}

/// Read an integer from stdin, mimicking C's scanf("%d", &x).
/// Skips leading whitespace, then parses an optional sign followed by digits.
/// Returns 0 if parsing fails (matching the behavior of leaving x unchanged when x was 0).
fn scanf_int() -> i32 {
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() {
        return 0;
    }
    let mut i = 0;
    // Skip leading whitespace (space, tab, newline, carriage return, form feed, vertical tab)
    while i < buf.len() && (buf[i] == b' ' || buf[i] == b'\t' || buf[i] == b'\n'
        || buf[i] == b'\r' || buf[i] == 0x0b || buf[i] == 0x0c)
    {
        i += 1;
    }
    if i >= buf.len() {
        return 0;
    }
    let mut sign: i64 = 1;
    if buf[i] == b'-' {
        sign = -1;
        i += 1;
    } else if buf[i] == b'+' {
        i += 1;
    }
    let start = i;
    let mut value: i64 = 0;
    while i < buf.len() && buf[i].is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((buf[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return 0;
    }
    (value.wrapping_mul(sign)) as i32
}

fn main() {
    let x = scanf_int();
    run(x);
    run(x);
    io::stdout().flush().ok();
}
