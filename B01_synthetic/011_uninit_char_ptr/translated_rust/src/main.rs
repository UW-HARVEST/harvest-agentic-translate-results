use std::io::{self, Read};
use std::mem::MaybeUninit;

fn print_line(line: *const u8) {
    if !line.is_null() {
        unsafe {
            // Walk until null terminator, matching C's printf("%s\n", line)
            let mut len = 0;
            while *line.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(line, len);
            let stdout = io::stdout();
            use std::io::Write;
            let mut out = stdout.lock();
            let _ = out.write_all(slice);
            let _ = out.write_all(b"\n");
        }
    }
}

fn bad() {
    unsafe {
        let data: *const u8 = MaybeUninit::uninit().assume_init();
        print_line(data);
    }
}

fn good() {
    let data: *const u8 = b"string\0".as_ptr();
    print_line(data);
}

fn main() {
    // scanf("%d", &x) — read whitespace-delimited integer token
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x: i32 = input.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
