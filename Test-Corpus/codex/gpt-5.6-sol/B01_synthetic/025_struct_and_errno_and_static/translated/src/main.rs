use std::io::{self, BufRead, Write};

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn add_floor_to_the_house(house: &mut House) {
    add_floor(house);
}

fn print_the_house<W: Write>(house: &House, output: &mut W) {
    let _ = writeln!(
        output,
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run<W: Write>(house: &mut House, extra_bedrooms: i32, output: &mut W) {
    print_the_house(house, output);
    add_floor_to_the_house(house);
    print_the_house(house, output);
    house.bathrooms += 1.0;
    print_the_house(house, output);
    add_bedrooms(house, extra_bedrooms);
    print_the_house(house, output);
}

fn read_fgets_100<R: BufRead>(input: &mut R) -> Vec<u8> {
    let mut result = Vec::with_capacity(99);

    while result.len() < 99 {
        let (take, found_newline) = {
            let available = match input.fill_buf() {
                Ok(available) => available,
                Err(_) => break,
            };
            if available.is_empty() {
                break;
            }

            let available = &available[..available.len().min(99 - result.len())];
            let newline = available.iter().position(|byte| *byte == b'\n');
            let take = newline.map_or(available.len(), |position| position + 1);
            result.extend_from_slice(&available[..take]);
            (take, newline.is_some())
        };

        input.consume(take);
        if found_newline {
            break;
        }
    }

    result
}

fn parse_val(input: &[u8]) -> Option<i32> {
    let input = match input.iter().position(|byte| *byte == 0) {
        Some(position) => &input[..position],
        None => input,
    };
    let mut position = 0;

    while position < input.len()
        && matches!(input[position], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        position += 1;
    }

    let negative = if input.get(position) == Some(&b'-') {
        position += 1;
        true
    } else {
        if input.get(position) == Some(&b'+') {
            position += 1;
        }
        false
    };

    let limit = if negative {
        i32::MAX as u64 + 1
    } else {
        i32::MAX as u64
    };
    let mut magnitude = 0_u64;
    let mut found_digit = false;
    let mut overflowed = false;

    while let Some(byte @ b'0'..=b'9') = input.get(position) {
        found_digit = true;
        let digit = u64::from(*byte - b'0');
        if magnitude > (limit - digit) / 10 {
            overflowed = true;
        } else if !overflowed {
            magnitude = magnitude * 10 + digit;
        }
        position += 1;
    }

    if !found_digit || overflowed {
        return None;
    }

    if negative {
        if magnitude == i32::MAX as u64 + 1 {
            Some(i32::MIN)
        } else {
            Some(-(magnitude as i32))
        }
    } else {
        Some(magnitude as i32)
    }
}

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let in_buffer = read_fgets_100(&mut input);

    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    if let Some(extra_bedrooms) = parse_val(&in_buffer) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, extra_bedrooms, &mut output);
        run(&mut the_house, extra_bedrooms, &mut output);
    } else {
        let _ = writeln!(output, "An error occurred");
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_val, read_fgets_100};
    use std::io::Cursor;

    #[test]
    fn parse_val_matches_strtol_prefix_rules() {
        assert_eq!(parse_val(b"  -2147483648junk\n"), Some(i32::MIN));
        assert_eq!(parse_val(b"+2147483647"), Some(i32::MAX));
        assert_eq!(parse_val(b"12x34"), Some(12));
        assert_eq!(parse_val(b"1\0more"), Some(1));
        assert_eq!(parse_val(b"2147483648"), None);
        assert_eq!(parse_val(b"-2147483649"), None);
        assert_eq!(parse_val(b" +x"), None);
    }

    #[test]
    fn fgets_stops_at_newline_or_99_bytes() {
        let mut lines = Cursor::new(b"12\n34\n");
        assert_eq!(read_fgets_100(&mut lines), b"12\n");
        assert_eq!(read_fgets_100(&mut lines), b"34\n");

        let data = vec![b'7'; 120];
        let mut long_line = Cursor::new(data);
        assert_eq!(read_fgets_100(&mut long_line).len(), 99);
        assert_eq!(read_fgets_100(&mut long_line).len(), 21);
    }
}
