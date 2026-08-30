use std::io::{self, BufRead};

fn print_int_line(int_number: i32) {
    println!("{int_number}");
}

fn bad() {
    let source = [0_i32; 10];
    let mut data = [0_i32; 10];

    for (destination, value) in data.iter_mut().zip(source) {
        *destination = value;
    }
    print_int_line(data[0]);
}

fn good() {
    let source = [0_i32; 10];
    let mut data = [0_i32; 10];

    for (destination, value) in data.iter_mut().zip(source) {
        *destination = value;
    }
    print_int_line(data[0]);
}

fn peek_byte<R: BufRead>(reader: &mut R) -> io::Result<Option<u8>> {
    Ok(reader.fill_buf()?.first().copied())
}

fn scan_decimal_i32<R: BufRead>(reader: &mut R) -> io::Result<Option<i32>> {
    while matches!(
        peek_byte(reader)?,
        Some(b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
    ) {
        reader.consume(1);
    }

    let negative = match peek_byte(reader)? {
        Some(b'-') => {
            reader.consume(1);
            true
        }
        Some(b'+') => {
            reader.consume(1);
            false
        }
        _ => false,
    };

    let mut found_digit = false;
    let mut magnitude = 0_u32;
    while let Some(byte @ b'0'..=b'9') = peek_byte(reader)? {
        found_digit = true;
        magnitude = magnitude
            .wrapping_mul(10)
            .wrapping_add(u32::from(byte - b'0'));
        reader.consume(1);
    }

    if !found_digit {
        return Ok(None);
    }

    let value = if negative {
        0_u32.wrapping_sub(magnitude)
    } else {
        magnitude
    };
    Ok(Some(value as i32))
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut x = 0;

    if let Some(value) = scan_decimal_i32(&mut input)? {
        x = value;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
    Ok(())
}
