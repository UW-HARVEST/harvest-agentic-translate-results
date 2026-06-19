use std::arch::x86_64::{__m128, _mm_cvttss_si32, _mm_set_ss};
use std::io::{self, Read, Write};
use std::os::raw::{c_char, c_double};

const CHAR_ARRAY_SIZE: usize = 20;

unsafe extern "C" {
    fn atof(nptr: *const c_char) -> c_double;
}

fn print_line(line: &str) {
    let mut stdout = io::stdout().lock();
    stdout.write_all(line.as_bytes()).unwrap();
    stdout.write_all(b"\n").unwrap();
}

fn print_int_line(int_number: i32) {
    let mut stdout = io::stdout().lock();
    write!(stdout, "{int_number}\n").unwrap();
}

fn fgets_like(stdin: &mut dyn Read, size: usize) -> Option<Vec<u8>> {
    if size == 0 {
        return None;
    }

    let mut buf = Vec::with_capacity(size);
    let mut byte = [0_u8; 1];

    while buf.len() + 1 < size {
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

    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

fn atof_f32(bytes: &[u8]) -> f32 {
    let mut c_buf = Vec::with_capacity(bytes.len() + 1);
    c_buf.extend_from_slice(bytes);
    c_buf.push(0);
    unsafe { atof(c_buf.as_ptr().cast::<c_char>()) as f32 }
}

fn c_float_to_int(value: f32) -> i32 {
    unsafe {
        let packed: __m128 = _mm_set_ss(value);
        _mm_cvttss_si32(packed)
    }
}

fn bad(stdin: &mut dyn Read) {
    let mut data: f32 = 0.0;
    if let Some(input_buffer) = fgets_like(stdin, CHAR_ARRAY_SIZE) {
        data = atof_f32(&input_buffer);
    } else {
        print_line("fgets() failed.");
    }

    let result = c_float_to_int(100.0_f32 / data);
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = c_float_to_int(100.0_f32 / data);
    print_int_line(result);
}

fn good_b2g(stdin: &mut dyn Read) {
    let mut data: f32 = 0.0;
    if let Some(input_buffer) = fgets_like(stdin, CHAR_ARRAY_SIZE) {
        data = atof_f32(&input_buffer);
    } else {
        print_line("fgets() failed.");
    }

    if (data as f64).abs() > 0.000001_f64 {
        let result = c_float_to_int(100.0_f32 / data);
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn good(stdin: &mut dyn Read) {
    good_g2b();
    good_b2g(stdin);
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
