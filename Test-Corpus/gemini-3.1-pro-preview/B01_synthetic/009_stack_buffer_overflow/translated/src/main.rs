use std::io::{self, BufRead};

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn bad() {
    let mut data: i32 = -1;
    let mut input_buffer = String::new();
    if io::stdin().lock().read_line(&mut input_buffer).is_ok() && !input_buffer.is_empty() {
        data = input_buffer.trim().parse::<i32>().unwrap_or(0);
    } else {
        print_line("fgets() failed.");
    }
    let mut buffer = [0; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line("ERROR: Array index is negative.");
    }
}

fn good_g2b() {
    let data: i32 = 7;
    let mut buffer = [0; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line("ERROR: Array index is negative.");
    }
}

fn good_b2g() {
    let mut data: i32 = -1;
    let mut input_buffer = String::new();
    if io::stdin().lock().read_line(&mut input_buffer).is_ok() && !input_buffer.is_empty() {
        data = input_buffer.trim().parse::<i32>().unwrap_or(0);
    } else {
        print_line("fgets() failed.");
    }
    let mut buffer = [0; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line("ERROR: Array index is out-of-bounds");
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
