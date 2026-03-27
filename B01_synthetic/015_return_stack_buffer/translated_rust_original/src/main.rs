use std::io::{self, Read};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

/// Mimics C's helperBad: returns a pointer to a stack-local buffer (UB).
unsafe fn helper_bad() -> *const u8 {
    let char_string: [u8; 17] = *b"helperBad string\0";
    char_string.as_ptr()
}

fn bad() {
    unsafe {
        let ptr = helper_bad();
        // Reproduce C UB: read from dangling stack pointer
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let s = std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len));
        print_line(Some(s));
    }
}

/// Mimics C's helperGood1: returns pointer to a static local.
fn helper_good1() -> &'static str {
    "helperGood1 string"
}

fn good() {
    print_line(Some(helper_good1()));
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);

    // scanf("%d", &x) skips whitespace and parses an integer; defaults to 0 on failure
    let x: i32 = input.split_whitespace().next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
