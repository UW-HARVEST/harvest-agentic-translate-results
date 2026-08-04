use std::io;

fn print_line(line: Option<&str>) {
    if let Some(l) = line {
        println!("{}", l);
    }
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn bad() {
    let mut data: f32 = 0.0;
    let mut input_buffer = String::new();
    if io::stdin().read_line(&mut input_buffer).is_ok() && !input_buffer.is_empty() {
        data = input_buffer.trim().parse::<f32>().unwrap_or(0.0);
    } else {
        print_line(Some("fgets() failed."));
    }
    let result = (100.0 / data as f64) as i32;
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = (100.0 / data as f64) as i32;
    print_int_line(result);
}

fn good_b2g() {
    let mut data: f32 = 0.0;
    let mut input_buffer = String::new();
    if io::stdin().read_line(&mut input_buffer).is_ok() && !input_buffer.is_empty() {
        data = input_buffer.trim().parse::<f32>().unwrap_or(0.0);
    } else {
        print_line(Some("fgets() failed."));
    }
    if data.abs() > 0.000001 {
        let result = (100.0 / data as f64) as i32;
        print_int_line(result);
    } else {
        print_line(Some("This would result in a divide by zero"));
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));
}
