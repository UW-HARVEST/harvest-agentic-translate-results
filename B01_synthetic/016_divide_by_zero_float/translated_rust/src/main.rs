use std::io::BufRead;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

/// Replicate C's (int)(double) cast: on x86-64, cvttsd2si returns 0x80000000
/// for out-of-range values (inf, nan, too large).
#[cfg(target_arch = "x86_64")]
fn f64_to_i32_c(v: f64) -> i32 {
    unsafe { std::arch::x86_64::_mm_cvttsd_si32(std::arch::x86_64::_mm_set_sd(v)) }
}

#[cfg(not(target_arch = "x86_64"))]
fn f64_to_i32_c(v: f64) -> i32 {
    v as i32
}

fn read_input_line() -> Option<f32> {
    let stdin = std::io::stdin();
    let mut line = String::new();
    let mut reader = stdin.lock();
    // fgets reads up to 19 chars (CHAR_ARRAY_SIZE-1) including newline
    // We replicate by reading a full line then truncating to 19 bytes
    match reader.read_line(&mut line) {
        Ok(0) => None, // EOF
        Ok(_) => {
            // Truncate to 19 bytes like fgets with buffer size 20
            if line.len() > 19 {
                line.truncate(19);
            }
            // atof: parse as f64, then cast to f32
            let val = line.trim_end_matches('\n').trim_end_matches('\r');
            let d: f64 = val.parse().unwrap_or(0.0);
            Some(d as f32)
        }
        Err(_) => None,
    }
}

fn bad() {
    let mut data: f32 = 0.0;
    if let Some(val) = read_input_line() {
        data = val;
    } else {
        print_line("fgets() failed.");
    }
    let result = f64_to_i32_c(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = f64_to_i32_c(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_b2g() {
    let mut data: f32 = 0.0;
    if let Some(val) = read_input_line() {
        data = val;
    } else {
        print_line("fgets() failed.");
    }
    if (data as f64).abs() > 0.000001 {
        let result = f64_to_i32_c(100.0_f64 / data as f64);
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    print_line("Calling good()...");
    good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad();
    print_line("Finished bad()");
}
