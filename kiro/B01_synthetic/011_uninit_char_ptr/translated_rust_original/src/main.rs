use std::io::{self, Read};
use std::mem::MaybeUninit;

fn print_line(line: *const u8) {
    if !line.is_null() {
        // Walk the raw pointer until we find a NUL byte, then print as a line.
        // This mirrors: printf("%s\n", line) — UB when line is uninitialized,
        // but we reproduce the C behavior exactly.
        unsafe {
            let mut len = 0usize;
            while *line.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(line, len);
            let s = std::str::from_utf8_unchecked(slice);
            println!("{}", s);
        }
    }
}

fn bad() {
    // Uninitialized pointer — mirrors: char *data; (uninitialized)
    let data: *const u8 = unsafe { MaybeUninit::uninit().assume_init() };
    print_line(data);
}

fn good() {
    let data: *const u8 = b"string\0".as_ptr();
    print_line(data);
}

fn main() {
    // Read all stdin, then parse the first integer — mirrors scanf("%d", &x)
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x: i32 = input.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
