use std::ffi::{c_char, c_double};
use std::io::{self, Read};

const CHAR_ARRAY_SIZE: usize = 20;

unsafe extern "C" {
    fn atof(input: *const c_char) -> c_double;
}

fn print_line(line: &str) {
    println!("{line}");
}

fn print_int_line(number: i32) {
    println!("{number}");
}

fn fgets<R: Read>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut input = Vec::with_capacity(CHAR_ARRAY_SIZE);

    while input.len() < CHAR_ARRAY_SIZE - 1 {
        let mut byte = [0_u8; 1];
        match reader.read(&mut byte)? {
            0 => break,
            _ => {
                input.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
        }
    }

    if input.is_empty() {
        Ok(None)
    } else {
        Ok(Some(input))
    }
}

fn parse_atof(mut input: Vec<u8>) -> f32 {
    input.push(0);

    // The appended NUL gives libc the same byte sequence that fgets provides.
    unsafe { atof(input.as_ptr().cast()) as f32 }
}

fn read_data<R: Read>(reader: &mut R) -> f32 {
    match fgets(reader) {
        Ok(Some(input)) => parse_atof(input),
        Ok(None) | Err(_) => {
            print_line("fgets() failed.");
            0.0
        }
    }
}

fn c_double_to_int(value: f64) -> i32 {
    if value.is_nan() || value >= 2_147_483_648.0 {
        i32::MIN
    } else {
        value as i32
    }
}

fn divide_to_int(data: f32) -> i32 {
    c_double_to_int(100.0 / f64::from(data))
}

fn bad<R: Read>(reader: &mut R) {
    let data = read_data(reader);
    let result = divide_to_int(data);
    print_int_line(result);
}

fn good_g2b() {
    let data = 2.0_f32;
    let result = divide_to_int(data);
    print_int_line(result);
}

fn good_b2g<R: Read>(reader: &mut R) {
    let data = read_data(reader);

    if f64::from(data).abs() > 0.000001 {
        let result = divide_to_int(data);
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn good<R: Read>(reader: &mut R) {
    good_g2b();
    good_b2g(reader);
}

fn main() {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    print_line("Calling good()...");
    good(&mut stdin);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(&mut stdin);
    print_line("Finished bad()");
}
