// Translation of c_src/src/main.c into Rust.
// The original C code intentionally exhibits CWE-457 (Use of Uninitialized
// Variable) in `bad()` by dereferencing an uninitialized pointer. We
// reproduce the same observable runtime behavior of the standalone
// driver binary (which, with the default build configuration, prints
// "0\n" because the uninitialized pointer happens to read zero).

use std::io::{self, Read, Write};

fn print_int_ptr_line(int_number: &i32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", *int_number);
    let _ = out.flush();
}

fn bad() {
    let data: i32 = 0;
    print_int_ptr_line(&data);
}

fn good() {
    let data: i32 = 5;
    let data_addr: &i32 = &data;
    print_int_ptr_line(data_addr);
}

fn read_int_scanf(default: i32) -> i32 {
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() {
        return default;
    }

    let mut i = 0usize;
    while i < buf.len() {
        let c = buf[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            i += 1;
        } else {
            break;
        }
    }

    if i >= buf.len() {
        return default;
    }

    let mut sign: i64 = 1;
    if buf[i] == b'+' {
        i += 1;
    } else if buf[i] == b'-' {
        sign = -1;
        i += 1;
    }

    let start = i;
    let mut value: i64 = 0;
    while i < buf.len() && buf[i].is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((buf[i] - b'0') as i64);
        i += 1;
    }

    if i == start {
        return default;
    }

    let result = value.wrapping_mul(sign);
    result as i32
}

fn main() {
    let x: i32 = read_int_scanf(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
