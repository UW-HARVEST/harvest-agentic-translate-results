use std::cell::RefCell;
use std::io::{self, Read, Write};

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

thread_local! {
    static THE_HOUSE: RefCell<House> = const { RefCell::new(House {
        floors: 2,
        bedrooms: 5,
        bathrooms: 2.5,
    }) };
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    THE_HOUSE.with(|h| {
        add_floor(&mut h.borrow_mut());
    });
}

fn print_the_house() {
    THE_HOUSE.with(|h| {
        let h = h.borrow();
        print!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
            h.floors, h.bedrooms, h.bathrooms
        );
    });
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    THE_HOUSE.with(|h| {
        h.borrow_mut().bathrooms += 1.0;
    });
    print_the_house();
    THE_HOUSE.with(|h| {
        add_bedrooms(&mut h.borrow_mut(), extra_bedrooms);
    });
    print_the_house();
}

/// Mimics C `strtol(str, &endp, 10)` followed by checks:
///   endp != str && errno == 0 && tmp in [INT_MIN, INT_MAX]
/// Returns Some(value as i32) on success, None otherwise.
fn parse_val(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    // Skip leading whitespace as defined by C `isspace` in the "C" locale:
    // space, \t, \n, \v, \f, \r
    while i < s.len() {
        let b = s[i];
        if b == b' ' || b == b'\t' || b == b'\n' || b == 0x0b || b == 0x0c || b == b'\r' {
            i += 1;
        } else {
            break;
        }
    }

    // Optional sign
    let mut negative = false;
    if i < s.len() {
        if s[i] == b'-' {
            negative = true;
            i += 1;
        } else if s[i] == b'+' {
            i += 1;
        }
    }

    let digit_start = i;
    // Parse digits as long (i64). Track range overflow which would set errno=ERANGE.
    let mut val: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !overflow {
            if negative {
                match val.checked_mul(10).and_then(|v| v.checked_sub(d)) {
                    Some(v) => val = v,
                    _ => {
                        overflow = true;
                    }
                }
            } else {
                match val.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => val = v,
                    _ => {
                        overflow = true;
                    }
                }
            }
        }
        i += 1;
    }

    // No digits consumed -> strtol leaves endp == str (the original nptr).
    // C check `endp != str` -> false -> return false.
    if i == digit_start {
        return None;
    }

    // errno != 0 in C means overflow during strtol -> reject.
    if overflow {
        return None;
    }

    // tmp must fit in int.
    if val < i32::MIN as i64 || val > i32::MAX as i64 {
        return None;
    }

    Some(val as i32)
}

/// Replicates `fgets(in, 100, stdin)` semantics:
/// reads up to (n-1) = 99 bytes into the buffer, stopping after a newline
/// (which is included in the buffer) or on EOF. Does not cross newlines.
fn fgets_99(stdin: &mut impl Read) -> Vec<u8> {
    let mut buf = Vec::with_capacity(100);
    let mut byte = [0u8; 1];
    while buf.len() < 99 {
        match stdin.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    buf
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let in_buf = fgets_99(&mut handle);

    // C only inspects the buffer up to the first NUL. Since fgets writes a NUL
    // terminator after the data, parse_val sees only the read bytes.
    match parse_val(&in_buf) {
        Some(x) => {
            run(x);
            run(x);
        }
        _ => {
            print!("An error occurred\n");
        }
    }

    // Ensure all buffered output is flushed before exit.
    let _ = io::stdout().flush();
}
