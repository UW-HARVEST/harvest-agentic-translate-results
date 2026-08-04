use std::io::{self, Read};

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{}", line);
    }
}

fn bad() {
    let data: Option<&str> = None;
    print_line(data);
}

fn good() {
    let data = Some("string");
    print_line(data);
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
