use std::io::{self, BufRead};

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

/// Mimics C's atoi: skip leading whitespace, optional sign, parse digits.
fn c_atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let neg = match chars.peek() {
        Some('-') => { chars.next(); true }
        Some('+') => { chars.next(); false }
        _ => false,
    };
    let mut result: i32 = 0;
    for c in chars {
        if let Some(d) = c.to_digit(10) {
            result = result.wrapping_mul(10).wrapping_add(d as i32);
        } else {
            break;
        }
    }
    if neg { result.wrapping_neg() } else { result }
}

/// Mimics fgets(buf, 14, stdin). Returns None on EOF with no data read.
fn read_input() -> Option<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => {
            if line.len() > 13 {
                line.truncate(13);
            }
            Some(line)
        }
        Err(_) => None,
    }
}

fn bad() {
    let mut data: i32 = -1;
    {
        match read_input() {
            Some(line) => {
                data = c_atoi(&line);
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            // Reproduce the C bug: unchecked array write (stack buffer overflow)
            unsafe {
                *buffer.as_mut_ptr().offset(data as isize) = 1;
            }
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is negative.");
        }
    }
}

fn good_g2b() {
    #[allow(unused_assignments)]
    let mut data: i32 = -1;
    data = 7;
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            buffer[data as usize] = 1;
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is negative.");
        }
    }
}

fn good_b2g() {
    let mut data: i32 = -1;
    {
        match read_input() {
            Some(line) => {
                data = c_atoi(&line);
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 && data < 10 {
            buffer[data as usize] = 1;
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is out-of-bounds");
        }
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    print_line("Calling good()...");
    good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad();
    print_line("Finished bad()");
}
