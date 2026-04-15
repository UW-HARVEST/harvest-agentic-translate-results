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
    let mut data = [0i32; 10 / std::mem::size_of::<i32>()];
    let source = [0i32; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn good() {
    let mut data = [0i32; (10 * std::mem::size_of::<i32>()) / std::mem::size_of::<i32>()];
    let source = [0i32; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn main() {
    let mut x = 0;
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if let Some(token) = input.split_whitespace().next() {
            x = token.parse().unwrap_or(0);
        }
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
