

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::collections::HashSet;
use std::io::BufRead;

fn rust_strcspn(s1: &str, s2: &str) -> usize {
    let set: HashSet<char> = s2.chars().collect();
    s1.chars().take_while(|c| !set.contains(c)).count()
}

fn rust_driver(s1: &str, s2: &str) {
    println!("{}", rust_strcspn(s1, s2));
}

fn rust_read_line_capped(reader: &mut impl BufRead, cap: usize) -> String {
    let mut buf = String::new();
    let _ = reader.read_line(&mut buf);
    if buf.len() > cap {
        buf.truncate(cap);
    }
    buf
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();

    // fgets reads at most sizeof(s1)-1 = 99 chars.
    let mut s1 = rust_read_line_capped(&mut handle, 99);
    let mut s2 = rust_read_line_capped(&mut handle, 99);

    // Emulate the C behavior: s1[strlen(s1)-1] = '\0';
    // which strips the trailing newline (or last char if no newline).
    if !s1.is_empty() {
        s1.pop();
    }
    if !s2.is_empty() {
        s2.pop();
    }

    rust_driver(&s1, &s2);
    0
}

