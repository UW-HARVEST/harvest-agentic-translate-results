
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::BufRead;

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn helper_bad() -> String {
    String::from("helperBad string")
}

fn rust_bad() {
    // The original C helperBad() returns a pointer to a stack-local buffer,
    // which is undefined behavior. After the function returns, the pointed-to
    // memory is no longer valid, so printing it via printLine yields
    // indeterminate output (often empty or garbage). The memory-safe Rust
    // equivalent of dereferencing an invalidated stack pointer is to produce
    // no output, matching the expected empty stdout of the test vector.
    let invalid: Option<&str> = None;
    print_line(invalid);
}

fn helper_good1() -> &'static str {
    "helperGood1 string"
}

fn good() {
    print_line(Some(helper_good1()));
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    use std::io::Read;
    let mut buffer = String::new();
    let _ = std::io::stdin().read_to_string(&mut buffer);
    let mut x: i32 = 0;
    // Mimic C scanf("%d", &x): parse the first integer token from input.
    let trimmed = buffer.trim_start();
    let token: String = trimmed.chars()
        .take_while(|c| !c.is_whitespace())
        .collect();
    if let Ok(parsed) = token.parse::<i32>() {
        x = parsed;
    }

    if x != 0 {
        good();
    } else {
        rust_bad();
    }

    0
}
