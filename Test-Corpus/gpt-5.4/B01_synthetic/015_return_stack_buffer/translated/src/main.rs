use std::io::{self, Read};

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{}", line);
    }
}

fn helper_bad() -> String {
    String::from("helperBad string")
}

fn bad() {
    let s = helper_bad();
    print_line(Some(&s));
}

fn helper_good1() -> &'static str {
    "helperGood1 string"
}

fn good() {
    print_line(Some(helper_good1()));
}

fn main() {
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
