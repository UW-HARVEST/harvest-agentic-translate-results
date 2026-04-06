use std::io::BufRead;

fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    for (i, &b) in s1.iter().enumerate() {
        if s2.contains(&b) {
            return i;
        }
    }
    s1.len()
}

fn driver(s1: &[u8], s2: &[u8]) {
    println!("{}", strcspn(s1, s2));
}

fn main() {
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();

    let mut s1 = lines.next().unwrap().unwrap();
    let mut s2 = lines.next().unwrap().unwrap();

    // fgets includes newline, then code does s[strlen(s)-1] = '\0' to strip it.
    // BufRead::lines() already strips the newline, so s1/s2 match the C post-strip state.
    // Truncate to 99 chars to match fgets(s, 100, stdin) behavior.
    s1.truncate(99);
    s2.truncate(99);

    driver(s1.as_bytes(), s2.as_bytes());
}
