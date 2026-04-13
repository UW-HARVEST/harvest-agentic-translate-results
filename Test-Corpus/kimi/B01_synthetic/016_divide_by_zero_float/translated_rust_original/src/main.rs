use std::io::{self, BufRead};

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

const CHAR_ARRAY_SIZE: usize = 20;

fn bad() {
    let mut data: f32 = 0.0;
    {
        let stdin = io::stdin();
        let mut input_buffer = String::new();
        if stdin.lock().read_line(&mut input_buffer).is_ok() {
            if let Ok(parsed) = input_buffer.trim().parse::<f32>() {
                data = parsed;
            }
        } else {
            print_line("fgets() failed.");
        }
    }
    {
        let result = (100.0 / data) as i32;
        print_int_line(result);
    }
}

fn good_g2b() {
    let data: f32 = 2.0;
    {
        let result = (100.0 / data) as i32;
        print_int_line(result);
    }
}

fn good_b2g() {
    let mut data: f32 = 0.0;
    {
        let stdin = io::stdin();
        let mut input_buffer = String::new();
        if stdin.lock().read_line(&mut input_buffer).is_ok() {
            if let Ok(parsed) = input_buffer.trim().parse::<f32>() {
                data = parsed;
            }
        } else {
            print_line("fgets() failed.");
        }
    }
    if data.abs() > 0.000001 {
        let result = (100.0 / data) as i32;
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
