use std::io;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

const CHAR_ARRAY_SIZE: usize = 20;

fn read_float_from_stdin() -> Option<f32> {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => {
            print_line("fgets() failed.");
            None
        }
        Ok(_) => {
            let truncated: String = input.chars().take(CHAR_ARRAY_SIZE - 1).collect();
            Some(truncated.trim().parse::<f32>().unwrap_or(0.0))
        }
        Err(_) => {
            print_line("fgets() failed.");
            None
        }
    }
}

fn bad() {
    let mut data: f32 = 0.0;
    if let Some(value) = read_float_from_stdin() {
        data = value;
    }
    let result = (100.0f32 / data) as i32;
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = (100.0f32 / data) as i32;
    print_int_line(result);
}

fn good_b2g() {
    let mut data: f32 = 0.0;
    if let Some(value) = read_float_from_stdin() {
        data = value;
    }
    if data.abs() > 0.000001f32 {
        let result = (100.0f32 / data) as i32;
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
