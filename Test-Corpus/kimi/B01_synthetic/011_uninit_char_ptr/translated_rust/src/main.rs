use std::io;

fn print_line(line: Option<&str>) {
    if let Some(l) = line {
        println!("{}", l);
    }
}

fn bad() {
    let data: Option<&str> = None;
    print_line(data);
}

fn good() {
    let data: Option<&str> = Some("string");
    print_line(data);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}