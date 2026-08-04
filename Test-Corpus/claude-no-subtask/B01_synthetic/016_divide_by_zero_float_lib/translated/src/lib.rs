// Translation of c_src/src/driver.c to Rust.
// Preserves byte-identical output for the same inputs.

use std::ffi::{c_float, c_int};
use std::io::Write;

fn print_line_str(line: &str) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(line.as_bytes());
    let _ = handle.write_all(b"\n");
}

fn print_int_line(int_number: c_int) {
    // Match C's printf("%d\n", intNumber).
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{}\n", int_number);
}

fn bad(data: c_float) {
    // (int)(100.0 / data) — 100.0 is a C double literal, data is float promoted to double,
    // so the division is performed in double precision, then truncated toward zero on cast to int.
    let result: c_int = (100.0_f64 / (data as f64)) as c_int;
    print_int_line(result);
}

fn good_g2b() {
    let data: c_float;
    data = 2.0_f32;
    {
        let result: c_int = (100.0_f64 / (data as f64)) as c_int;
        print_int_line(result);
    }
}

fn good_b2g(data: c_float) {
    // fabs takes a double in C; data is promoted from float to double.
    if (data as f64).abs() > 0.000001_f64 {
        let result: c_int = (100.0_f64 / (data as f64)) as c_int;
        print_int_line(result);
    } else {
        print_line_str("This would result in a divide by zero");
    }
}

fn good(data: c_float) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_float, bad_data: c_float) {
    print_line_str("Calling good()...");
    good(good_data);
    print_line_str("Finished good()");
    print_line_str("Calling bad()...");
    bad(bad_data);
    print_line_str("Finished bad()");
}
