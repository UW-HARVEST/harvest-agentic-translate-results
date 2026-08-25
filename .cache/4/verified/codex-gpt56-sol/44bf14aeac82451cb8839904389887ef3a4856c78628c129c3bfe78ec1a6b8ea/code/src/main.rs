use std::ffi::{c_char, c_double};
use std::io::{self, BufWriter, Read, Write};

const CHAR_ARRAY_SIZE: usize = 20;

extern "C" {
    fn atof(input: *const c_char) -> c_double;
}

fn print_line(output: &mut impl Write, line: &str) {
    let _ = writeln!(output, "{line}");
}

fn print_int_line(output: &mut impl Write, number: i32) {
    let _ = writeln!(output, "{number}");
}

fn fgets(reader: &mut impl Read) -> Option<Vec<u8>> {
    let mut input = Vec::with_capacity(CHAR_ARRAY_SIZE);

    while input.len() < CHAR_ARRAY_SIZE - 1 {
        let mut byte = [0_u8; 1];
        match reader.read(&mut byte) {
            Ok(0) if input.is_empty() => return None,
            Ok(0) => break,
            Ok(_) => {
                input.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return None,
        }
    }

    Some(input)
}

fn parse_float(mut input: Vec<u8>) -> f32 {
    input.push(0);
    // The buffer has a trailing NUL and remains alive for the duration of the call.
    unsafe { atof(input.as_ptr().cast()) as f32 }
}

fn read_data(reader: &mut impl Read, output: &mut impl Write) -> f32 {
    match fgets(reader) {
        Some(input) => parse_float(input),
        None => {
            print_line(output, "fgets() failed.");
            0.0
        }
    }
}

fn c_double_to_int(value: f64) -> i32 {
    if value.is_nan() || !(-2147483648.0..2147483648.0).contains(&value) {
        i32::MIN
    } else {
        value.trunc() as i32
    }
}

fn divide_and_convert(data: f32) -> i32 {
    c_double_to_int(100.0_f64 / f64::from(data))
}

fn bad(reader: &mut impl Read, output: &mut impl Write) {
    let data = read_data(reader, output);
    print_int_line(output, divide_and_convert(data));
}

fn good_g2b(output: &mut impl Write) {
    let data = 2.0_f32;
    print_int_line(output, divide_and_convert(data));
}

fn good_b2g(reader: &mut impl Read, output: &mut impl Write) {
    let data = read_data(reader, output);
    if f64::from(data).abs() > 0.000001_f64 {
        print_int_line(output, divide_and_convert(data));
    } else {
        print_line(output, "This would result in a divide by zero");
    }
}

fn good(reader: &mut impl Read, output: &mut impl Write) {
    good_g2b(output);
    good_b2g(reader, output);
}

fn run(reader: &mut impl Read, output: &mut impl Write) {
    print_line(output, "Calling good()...");
    good(reader, output);
    print_line(output, "Finished good()");
    print_line(output, "Calling bad()...");
    bad(reader, output);
    print_line(output, "Finished bad()");
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = BufWriter::new(stdout.lock());
    run(&mut input, &mut output);
}
