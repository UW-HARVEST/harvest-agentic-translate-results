// Minimal executable wrapper around the translated `findrep` library.
// The original C source ships only a library (no `main`); this driver reads
// four integers from stdin (whitespace-separated, like C `scanf("%d ...")`)
// and prints the result of `findrep`.

use std::io::{self, Read};

mod findrep;
use findrep::findrep;

fn read_all_stdin() -> String {
    let mut buf = String::new();
    let _ = io::stdin().read_to_string(&mut buf);
    buf
}

fn parse_four_ints(input: &str) -> (i32, i32, i32, i32) {
    let mut it = input.split_ascii_whitespace();
    let parse = |s: Option<&str>| -> i32 {
        s.and_then(|t| t.parse::<i32>().ok()).unwrap_or(0)
    };
    let a = parse(it.next());
    let b = parse(it.next());
    let c = parse(it.next());
    let d = parse(it.next());
    (a, b, c, d)
}

fn main() {
    let input = read_all_stdin();
    let (a, b, c, d) = parse_four_ints(&input);
    let result = findrep(a, b, c, d);
    println!("{}", result);
}
