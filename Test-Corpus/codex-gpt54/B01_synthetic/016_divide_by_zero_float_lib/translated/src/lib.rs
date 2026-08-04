use std::ffi::{CStr, c_char, c_double, c_float, c_int};
use std::io::{self, Write};

fn write_bytes(bytes: &[u8]) {
    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(bytes);
}

fn write_newline() {
    write_bytes(b"\n");
}

fn print_line_impl(line: *const c_char) {
    if !line.is_null() {
        // Match C's "%s\n" by writing the string bytes directly without UTF-8 validation.
        let bytes = unsafe { CStr::from_ptr(line) }.to_bytes();
        write_bytes(bytes);
        write_newline();
    }
}

fn print_int_line_impl(int_number: c_int) {
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{int_number}");
}

fn bad_impl(data: c_float) {
    let quotient = 100.0f64 / data as c_double;
    let result = unsafe { quotient.to_int_unchecked::<c_int>() };
    print_int_line_impl(result);
}

fn good_g2b() {
    let data: c_float = 2.0;
    let quotient = 100.0f64 / data as c_double;
    let result = unsafe { quotient.to_int_unchecked::<c_int>() };
    print_int_line_impl(result);
}

fn good_b2g(data: c_float) {
    if (data as c_double).abs() > 0.000001f64 {
        let quotient = 100.0f64 / data as c_double;
        let result = unsafe { quotient.to_int_unchecked::<c_int>() };
        print_int_line_impl(result);
    } else {
        print_line_impl(c"This would result in a divide by zero".as_ptr());
    }
}

fn good_impl(data: c_float) {
    good_g2b();
    good_b2g(data);
}

fn driver_impl(good_data: c_float, bad_data: c_float) {
    print_line_impl(c"Calling good()...".as_ptr());
    good_impl(good_data);
    print_line_impl(c"Finished good()".as_ptr());
    print_line_impl(c"Calling bad()...".as_ptr());
    bad_impl(bad_data);
    print_line_impl(c"Finished bad()".as_ptr());
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    print_line_impl(line);
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    print_int_line_impl(int_number);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_float) {
    bad_impl(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_float) {
    good_impl(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_float, bad_data: c_float) {
    driver_impl(good_data, bad_data);
}
