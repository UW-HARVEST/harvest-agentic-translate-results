use std::io::{self, Read};
use std::mem::MaybeUninit;

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn bad() {
    // Mirrors C: uninitialized char *data; printLine(data);
    // In the compiled C (release), the uninitialized pointer is NULL on zeroed stack,
    // so printLine's NULL check skips the print. Reproduce with MaybeUninit.
    let data: MaybeUninit<Option<&str>> = MaybeUninit::zeroed();
    let data = unsafe { data.assume_init() };
    print_line(data);
}

fn good() {
    let data = "string";
    print_line(Some(data));
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    // scanf("%d", &x) skips whitespace and parses an integer.
    // On failure x stays 0.
    let x: i32 = input.split_whitespace().next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
