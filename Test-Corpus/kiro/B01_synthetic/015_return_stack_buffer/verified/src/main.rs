use std::io::{self, Read};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn helper_bad() -> Option<&'static str> {
    // C returns address of local variable — UB. GCC optimizes to return NULL.
    None
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good1() -> Option<&'static str> {
    // C uses static local array
    Some("helperGood1 string")
}

fn good() {
    print_line(helper_good1());
}

fn main() {
    // Mimic scanf("%d", &x): read all stdin, skip leading whitespace, parse int.
    // On failure x remains 0.
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x: i32 = input.trim_start().split_whitespace().next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
