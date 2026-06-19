use std::io::{self, Read};
use std::os::raw::{c_char, c_double};

const CHAR_ARRAY_SIZE: usize = 20;

extern "C" {
    fn atof(nptr: *const c_char) -> c_double;
}

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

struct FgetsInput<R> {
    reader: R,
}

impl<R: Read> FgetsInput<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }

    fn fgets(&mut self) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(CHAR_ARRAY_SIZE);
        while out.len() < CHAR_ARRAY_SIZE - 1 {
            let mut byte = [0_u8; 1];
            match self.reader.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {}
                Err(_) => return None,
            }

            let byte = byte[0];
            out.push(byte);
            if byte == b'\n' {
                break;
            }
        }

        if out.is_empty() {
            return None;
        }

        out.push(0);
        Some(out)
    }
}

fn c_atof(buffer: &[u8]) -> f32 {
    unsafe { atof(buffer.as_ptr() as *const c_char) as f32 }
}

fn c_double_to_int(value: f64) -> i32 {
    if value.is_nan() || value >= i32::MAX as f64 + 1.0 || value < i32::MIN as f64 {
        i32::MIN
    } else {
        value.trunc() as i32
    }
}

fn bad<R: Read>(input: &mut FgetsInput<R>) {
    let mut data = 0.0_f32;
    if let Some(input_buffer) = input.fgets() {
        data = c_atof(&input_buffer);
    } else {
        print_line("fgets() failed.");
    }

    let result = c_double_to_int(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_g2b() {
    let data = 2.0_f32;
    let result = c_double_to_int(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_b2g<R: Read>(input: &mut FgetsInput<R>) {
    let mut data = 0.0_f32;
    if let Some(input_buffer) = input.fgets() {
        data = c_atof(&input_buffer);
    } else {
        print_line("fgets() failed.");
    }

    if (data as f64).abs() > 0.000001_f64 {
        let result = c_double_to_int(100.0_f64 / data as f64);
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn good<R: Read>(input: &mut FgetsInput<R>) {
    good_g2b();
    good_b2g(input);
}

fn main() {
    let stdin = io::stdin();
    let mut input = FgetsInput::new(stdin.lock());

    print_line("Calling good()...");
    good(&mut input);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(&mut input);
    print_line("Finished bad()");
}
