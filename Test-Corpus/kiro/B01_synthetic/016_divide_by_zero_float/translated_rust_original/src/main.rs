use std::io::{self, BufRead};

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

/// Matches C's (int)(double_val) which is UB for out-of-range values.
/// On x86 GCC, cvttsd2si returns 0x80000000 for inf/nan/out-of-range.
fn double_to_int_c(v: f64) -> i32 {
    if v.is_nan() || v.is_infinite() || v > i32::MAX as f64 || v < i32::MIN as f64 {
        i32::MIN
    } else {
        v as i32
    }
}

const CHAR_ARRAY_SIZE: usize = 20;

fn read_input_float(reader: &mut impl BufRead) -> Option<f32> {
    let mut buf = String::new();
    match reader.read_line(&mut buf) {
        Ok(0) => None,
        Ok(_) => {
            // Truncate to CHAR_ARRAY_SIZE-1 chars to match fgets behavior
            if buf.len() > CHAR_ARRAY_SIZE - 1 {
                buf.truncate(CHAR_ARRAY_SIZE - 1);
            }
            // atof: parse as f64, cast to f32
            let val = buf.trim_end_matches('\n').parse::<f64>().unwrap_or(0.0);
            Some(val as f32)
        }
        Err(_) => None,
    }
}

fn bad(reader: &mut impl BufRead) {
    let data: f32;
    if let Some(val) = read_input_float(reader) {
        data = val;
    } else {
        print_line("fgets() failed.");
        data = 0.0_f32;
    }
    let result = double_to_int_c(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = double_to_int_c(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_b2g(reader: &mut impl BufRead) {
    let data: f32;
    if let Some(val) = read_input_float(reader) {
        data = val;
    } else {
        print_line("fgets() failed.");
        data = 0.0_f32;
    }
    if (data as f64).abs() > 0.000001 {
        let result = double_to_int_c(100.0_f64 / data as f64);
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn good(reader: &mut impl BufRead) {
    good_g2b();
    good_b2g(reader);
}

fn main() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    print_line("Calling good()...");
    good(&mut reader);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(&mut reader);
    print_line("Finished bad()");
}
