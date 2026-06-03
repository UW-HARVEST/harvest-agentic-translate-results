// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::os::raw::c_int;

pub fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

pub fn print_int_line(int_number: c_int) {
    println!("{}", int_number);
}

pub fn bad(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        // Faithfully translates the C semantics: this is an intentional
        // out-of-bounds write when `data >= 10` (the original C code's
        // CWE-style vulnerability). In Rust this is achieved via unsafe
        // pointer arithmetic to mirror the C behavior.
        unsafe {
            let ptr = buffer.as_mut_ptr();
            *ptr.offset(data as isize) = 1;
        }
        // Print the array values
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line(Some("ERROR: Array index is negative."));
    }
}

fn good_g2b() {
    let data: c_int = 7;
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        // Print the array values
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line(Some("ERROR: Array index is negative."));
    }
}

fn good_b2g(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        // Print the array values
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line(Some("ERROR: Array index is out-of-bounds"));
    }
}

pub fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

pub fn driver_rs(good_data: c_int, bad_data: c_int) {
    print_line(Some("Calling good()..."));
    good(good_data);
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad(bad_data);
    print_line(Some("Finished bad()"));
}

/// C-compatible entry point matching the original `driver` symbol.
#[no_mangle]
pub extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    driver_rs(good_data, bad_data);
}
