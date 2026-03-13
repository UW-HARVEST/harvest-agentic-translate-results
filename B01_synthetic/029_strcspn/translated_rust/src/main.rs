use std::io::BufRead;

fn driver(s1: &[u8], s2: &[u8]) {
    let n = s1.iter().take_while(|c| !s2.contains(c)).count();
    println!("{}", n);
}

fn main() {
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let s1 = lines.next().unwrap_or(Ok(String::new())).unwrap_or_default();
    let s2 = lines.next().unwrap_or(Ok(String::new())).unwrap_or_default();
    driver(s1.as_bytes(), s2.as_bytes());
}
