use std::os::raw::c_float;

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{}", line);
    }
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn bad(data: f32) {
    let result = (100.0f32 / data) as i32;
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = (100.0f32 / data) as i32;
    print_int_line(result);
}

fn good_b2g(data: f32) {
    if data.abs() > 0.000001f32 {
        let result = (100.0f32 / data) as i32;
        print_int_line(result);
    } else {
        print_line(Some("This would result in a divide by zero"));
    }
}

fn good(data: f32) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(goodData: c_float, badData: c_float) {
    print_line(Some("Calling good()..."));
    good(goodData);
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad(badData);
    print_line(Some("Finished bad()"));
}
