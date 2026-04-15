use std::io;

fn driver(s1: &str, s2: &str) {
    let pos = s1.as_bytes().iter().position(|&b| s2.as_bytes().contains(&b)).unwrap_or(s1.len());
    println!("{}", pos);
}

fn main() {
    let mut s1 = String::new();
    let mut s2 = String::new();

    let stdin = io::stdin();
    let _ = stdin.read_line(&mut s1);
    let _ = stdin.read_line(&mut s2);

    let s1 = s1.trim_end_matches('\n').trim_end_matches('\r');
    let s2 = s2.trim_end_matches('\n').trim_end_matches('\r');

    driver(s1, s2);
}
