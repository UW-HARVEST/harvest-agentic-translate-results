use std::io::{self, Read};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn helper_bad() -> &'static str {
    // C original returns pointer to local array (UB).
    // In practice the string content typically survives, so reproduce that.
    "helperBad string"
}

fn bad() {
    print_line(Some(helper_bad()));
}

fn helper_good1() -> &'static str {
    // C original uses a static local.
    "helperGood1 string"
}

fn good() {
    print_line(Some(helper_good1()));
}

fn main() {
    // scanf("%d", &x) — reads an integer, skipping leading whitespace
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let x: i32 = input.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
