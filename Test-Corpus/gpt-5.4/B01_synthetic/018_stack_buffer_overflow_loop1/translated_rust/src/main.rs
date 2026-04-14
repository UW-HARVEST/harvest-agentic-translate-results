use std::io::{self, Read};

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{}", line);
    }
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn bad() {
    let mut data = [0i32; 2];
    let source = [0i32; 10];
    for i in 0..10 {
        if i < data.len() {
            data[i] = source[i];
        }
    }
    print_int_line(data[0]);
}

fn good() {
    let mut data = [0i32; 10];
    let source = [0i32; 10];
    data.copy_from_slice(&source);
    print_int_line(data[0]);
}

fn main() {
    let _ = print_line as fn(Option<&str>);
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x = input
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
