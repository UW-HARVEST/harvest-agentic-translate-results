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

pub fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

pub fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

pub fn bad(data: f32) {
    let result = (100.0_f32 / data) as i32;
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0_f32;
    let result = (100.0_f32 / data) as i32;
    print_int_line(result);
}

fn good_b2g(data: f32) {
    if (data as f64).abs() > 0.000001 {
        let result = (100.0_f32 / data) as i32;
        print_int_line(result);
    } else {
        print_line(Some("This would result in a divide by zero"));
    }
}

pub fn good(data: f32) {
    good_g2b();
    good_b2g(data);
}

pub fn driver(good_data: f32, bad_data: f32) {
    print_line(Some("Calling good()..."));
    good(good_data);
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad(bad_data);
    print_line(Some("Finished bad()"));
}
