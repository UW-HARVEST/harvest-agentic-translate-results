use std::os::raw::c_float;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn bad(data: f32) {
    let result = (100.0 / data) as i32;
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = (100.0 / data) as i32;
    print_int_line(result);
}

fn good_b2g(data: f32) {
    if data.abs() > 0.000001 {
        let result = (100.0 / data) as i32;
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn good(data: f32) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_float, bad_data: c_float) {
    print_line("Calling good()...");
    good(good_data as f32);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(bad_data as f32);
    print_line("Finished bad()");
}
