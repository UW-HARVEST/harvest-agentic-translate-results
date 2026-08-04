// Translated from c_src/src/main.c
// The original C code has undefined behavior in `bad()`: it returns a pointer
// to a local stack array. Empirically on this platform that produces no
// output, so we reproduce that observed behavior exactly.

use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        // Use stdout().write_all to match printf("%s\n", line) byte-exactly.
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(s.as_bytes());
        let _ = handle.write_all(b"\n");
    }
}

// Mirrors the original `helperBad()` in main.c, which returns a pointer to
// a local automatic array. That is undefined behavior in C; on the build
// host the resulting output is empty, so we model it as returning None.
fn helper_bad() -> Option<&'static str> {
    None
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good1() -> Option<&'static str> {
    // Matches `static char charString[] = "helperGood1 string";`
    Some("helperGood1 string")
}

fn good() {
    print_line(helper_good1());
}

// Mimics scanf("%d", &x) reading an integer from stdin.
// scanf with %d skips leading whitespace (including newlines), then parses
// an optional sign and decimal digits. If no integer can be parsed, the
// destination is left unchanged (it was initialized to 0 in main).
fn scanf_d(input: &[u8], x: &mut i32) -> usize {
    let mut i = 0usize;
    // Skip leading whitespace
    while i < input.len() {
        let c = input[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
            || c == 0x0B || c == 0x0C
        {
            i += 1;
        } else {
            break;
        }
    }
    if i >= input.len() {
        return i;
    }
    let start = i;
    let mut negative = false;
    if input[i] == b'+' {
        i += 1;
    } else if input[i] == b'-' {
        negative = true;
        i += 1;
    }
    let digits_start = i;
    let mut value: i64 = 0;
    while i < input.len() && input[i].is_ascii_digit() {
        let d = (input[i] - b'0') as i64;
        // Match C int wrap-around (technically UB on overflow in C, but we
        // emulate with wrapping i32 arithmetic).
        value = value.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    if i == digits_start {
        // No digits found: scanf would not assign and would leave the
        // pointer roughly where parsing began; we return start.
        return start;
    }
    let final_value = if negative {
        value.wrapping_neg() as i32
    } else {
        value as i32
    };
    *x = final_value;
    i
}

fn main() {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);

    let mut x: i32 = 0;
    let _consumed = scanf_d(&input, &mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
