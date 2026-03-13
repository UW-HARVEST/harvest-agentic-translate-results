use std::io::{self, Read};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn helper_bad() -> &'static str {
    // C original returns a pointer to a local stack buffer (UB).
    // In practice the string content is still readable, so reproduce that output.
    "helperBad string"
}

fn helper_good1() -> &'static str {
    // C original uses a static local array — well-defined.
    "helperGood1 string"
}

fn bad() {
    print_line(Some(helper_bad()));
}

fn good() {
    print_line(Some(helper_good1()));
}

fn main() {
    // scanf("%d", &x) skips leading whitespace then reads an integer.
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let x: i32 = input.trim().parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
