use std::io::BufRead;

fn driver(s1: &[u8], s2: &[u8]) {
    let n = s1.iter().take_while(|b| !s2.contains(b)).count();
    println!("{}", n);
}

fn main() {
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let s1 = lines.next().unwrap_or(Ok(String::new())).unwrap();
    let s2 = lines.next().unwrap_or(Ok(String::new())).unwrap();
    driver(s1.as_bytes(), s2.as_bytes());
}
