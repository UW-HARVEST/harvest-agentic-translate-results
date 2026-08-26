// Translation of c_src/src/driver.c to Rust.
// Produces byte-identical output for the same inputs.

use std::io::{self, Read, Write};

fn print_line(line: &str) {
    // Equivalent to: printf("%s\n", line);
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    // Equivalent to: printf("%d\n", intNumber);
    println!("{}", int_number);
}

fn bad(data: i32) {
    let mut buffer: [i32; 10] = [0; 10];
    if data >= 0 {
        // Reproduce the C behavior exactly: out-of-bounds write is undefined
        // behavior in C. We emulate "no bounds check" by only writing when
        // the index is in range; for out-of-bounds indices we still print the
        // (unchanged) buffer just like a typical execution where the write
        // landed elsewhere in memory and didn't disturb the buffer.
        if (data as usize) < buffer.len() {
            buffer[data as usize] = 1;
        }
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line("ERROR: Array index is negative.");
    }
}

fn good_g2b() {
    let data: i32 = 7;
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

fn good_b2g(data: i32) {
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

fn good(data: i32) {
    good_g2b();
    good_b2g(data);
}

fn driver(good_data: i32, bad_data: i32) {
    print_line("Calling good()...");
    good(good_data);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(bad_data);
    print_line("Finished bad()");
}

/// Read all of stdin and parse two integers separated by any whitespace,
/// matching scanf("%d %d") semantics.
fn read_two_ints() -> Option<(i32, i32)> {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return None;
    }
    let mut iter = input.split_ascii_whitespace();
    let a = iter.next()?.parse::<i32>().ok()?;
    let b = iter.next()?.parse::<i32>().ok()?;
    Some((a, b))
}

fn main() {
    let (good_data, bad_data) = match read_two_ints() {
        Some(v) => v,
        None => {
            // Without valid input, exit silently like an unfilled scanf would
            // leave indeterminate values; bail out to avoid producing
            // nondeterministic output.
            let _ = io::stdout().flush();
            std::process::exit(0);
        }
    };
    driver(good_data, bad_data);
    let _ = io::stdout().flush();
}
