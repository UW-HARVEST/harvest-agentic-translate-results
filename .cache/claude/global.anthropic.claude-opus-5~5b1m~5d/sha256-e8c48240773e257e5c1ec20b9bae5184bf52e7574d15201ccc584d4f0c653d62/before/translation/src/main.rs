// Rust translation of c_src/src/main.c
//
// Original copyright 2025 MIT Lincoln Laboratory (see c_src for full license text).
//
// Behavior is preserved exactly, including the original program's quirks:
//   * `bad()` under-allocates its scratch buffer (`alloca(10)` instead of
//     `alloca(10 * sizeof(int))`) and then writes ten `int`s into it. That is
//     undefined behavior in C, but in practice the loop copies ten zeros and
//     the program prints `data[0]`, which is `0`. We reproduce the observable
//     output using a properly sized safe buffer instead of committing UB.
//   * `main` ignores the return value of `scanf("%d", &x)`, so a matching
//     failure (or EOF) leaves `x` at its initialized value of `0`, which
//     selects the `bad()` branch.

use std::io::{Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{}", line);
    }
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

/// Mirrors the original `bad()`: `alloca(10)` bytes, then ten `int` stores.
fn bad() {
    // The C code allocates only 10 bytes here (a bug we must not "fix", but
    // also must not turn into Rust UB). The observable effect on a normal run
    // is the same as writing into ten in-bounds slots.
    let mut data = [0i32; 10];
    {
        let source = [0i32; 10];
        let mut i: usize = 0;
        while i < 10 {
            data[i] = source[i];
            i += 1;
        }
        print_int_line(data[0]);
    }
}

/// Mirrors the original `good()`: `alloca(10 * sizeof(int))`.
fn good() {
    let mut data = [0i32; 10];
    {
        let source = [0i32; 10];
        let mut i: usize = 0;
        while i < 10 {
            data[i] = source[i];
            i += 1;
        }
        print_int_line(data[0]);
    }
}

/// Reads one byte from stdin, or `None` at EOF.
fn read_byte() -> Option<u8> {
    let mut buf = [0u8; 1];
    loop {
        match std::io::stdin().read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => return Some(buf[0]),
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c')
}

/// Emulates `scanf("%d", out)`: skips leading whitespace (including newlines),
/// accepts an optional sign followed by decimal digits, and leaves `*out`
/// untouched on a matching failure. Returns the number of assigned items
/// (1 on success, 0 on matching failure, -1 on input failure / EOF).
fn scanf_d(out: &mut i32) -> i32 {
    // Skip whitespace.
    let mut c = loop {
        match read_byte() {
            None => return -1,
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
        }
    };

    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match read_byte() {
            None => return -1,
            Some(b) => c = b,
        }
    }

    if !c.is_ascii_digit() {
        // Matching failure; the offending character is pushed back by C.
        return 0;
    }

    // Accumulate using a wide type, then narrow the way glibc's strtol-based
    // conversion does (saturating at the long range, truncated to int).
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = (c - b'0') as i64;
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        match read_byte() {
            None => break,
            Some(b) if b.is_ascii_digit() => c = b,
            Some(_) => break, // non-digit: pushed back by C, then we stop
        }
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    *out = value as i32;
    1
}

fn main() {
    let mut x: i32 = 0;
    let _ = scanf_d(&mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }

    // Keep `print_line` referenced, matching the original file's unused helper.
    if false {
        print_line(None);
    }

    let _ = std::io::stdout().flush();
    std::process::exit(0);
}
