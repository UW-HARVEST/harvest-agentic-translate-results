use std::io::{self, Read};

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn bad() {
    let source = [0_i32; 10];
    let mut data = [0_i32; 10];

    for i in 0..10 {
        data[i] = source[i];
    }

    print_int_line(data[0]);
}

fn good() {
    let source = [0_i32; 10];
    let mut data = [0_i32; 10];

    for i in 0..10 {
        data[i] = source[i];
    }

    print_int_line(data[0]);
}

fn scanf_decimal_int(input: &[u8]) -> Option<i32> {
    let mut index = 0;

    while index < input.len() && input[index].is_ascii_whitespace() {
        index += 1;
    }

    let negative = if index < input.len() {
        match input[index] {
            b'-' => {
                index += 1;
                true
            }
            b'+' => {
                index += 1;
                false
            }
            _ => false,
        }
    } else {
        false
    };

    let first_digit = index;
    let mut value: i64 = 0;

    while index < input.len() && input[index].is_ascii_digit() {
        value = value * 10 + i64::from(input[index] - b'0');
        index += 1;
    }

    if index == first_digit {
        return None;
    }

    if negative {
        value = -value;
    }

    Some(value as i32)
}

fn main() {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);

    let mut x = 0_i32;
    if let Some(parsed) = scanf_decimal_int(&input) {
        x = parsed;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
