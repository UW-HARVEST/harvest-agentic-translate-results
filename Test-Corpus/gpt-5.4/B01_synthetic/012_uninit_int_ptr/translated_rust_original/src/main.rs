use std::io::{self, Read};

fn print_int_ptr_line(int_number: &i32) {
    println!("{}", *int_number);
}

fn bad() {
    let data: Option<&i32> = None;
    if let Some(value) = data {
        print_int_ptr_line(value);
    }
}

fn good() {
    let data = 5;
    let data_addr = &data;
    print_int_ptr_line(data_addr);
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x = input.split_whitespace().next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
