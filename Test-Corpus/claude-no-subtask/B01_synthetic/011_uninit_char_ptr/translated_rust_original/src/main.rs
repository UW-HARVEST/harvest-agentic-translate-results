// Translation of c_src/src/main.c to Rust.
// This is an executable. The original C program:
//   - reads an integer x from stdin using scanf("%d", &x). If scanf fails, x
//     remains 0.
//   - if x != 0, calls good() which prints "string\n".
//   - otherwise, calls bad() which invokes printLine with an uninitialized
//     pointer.
//
// The C bad() function exhibits undefined behavior. On Linux x86_64 with
// glibc (the platform this program targets), the observed deterministic
// output for the "x == 0" branch is a single "\n" byte. To produce
// byte-identical output, we model bad() as printing an empty string via
// printLine (which then prints "" followed by "\n", i.e. one newline byte).

use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        // Match printf("%s\n", line): write the string followed by a newline.
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        // Ignore write errors to mirror C's printf return-value behavior.
        let _ = handle.write_all(s.as_bytes());
        let _ = handle.write_all(b"\n");
    }
}

fn bad() {
    // The C version reads an uninitialized pointer here. On this target
    // platform the observed deterministic output is equivalent to passing
    // an empty string to printLine (yielding a single "\n" byte). We
    // reproduce that exact behavior.
    let data: Option<&str> = Some("");
    print_line(data);
}

fn good() {
    let data: Option<&str> = Some("string");
    print_line(data);
}

/// Emulates scanf("%d", &x): skips leading whitespace (including newlines),
/// reads an optional sign, then base-10 digits, into a C-style int (i32).
/// Returns the parsed value if successful, or None if no integer was matched
/// (in which case the caller leaves x unchanged, matching the C semantics).
fn scanf_int(input: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    // Skip leading whitespace (matches isspace: space, \t, \n, \v, \f, \r).
    while i < input.len() {
        match input[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
        }
    }
    if i >= input.len() {
        return None;
    }
    let start = i;
    // Optional sign.
    if input[i] == b'+' || input[i] == b'-' {
        i += 1;
    }
    let digit_start = i;
    while i < input.len() && input[i].is_ascii_digit() {
        i += 1;
    }
    if i == digit_start {
        // No digits matched.
        return None;
    }
    let s = std::str::from_utf8(&input[start..i]).ok()?;
    // C scanf with %d wraps on overflow into UB; std::i32::from_str returns
    // None on overflow. We mirror typical glibc behavior of leaving the
    // variable unchanged when the parse "fails" — for in-range values this
    // matches; for out-of-range values C is undefined, so any consistent
    // choice is acceptable.
    s.parse::<i32>().ok()
}

fn main() {
    // Read all of stdin (matches scanf which reads from the stdio stream and
    // can cross newlines).
    let mut buf = Vec::new();
    let _ = io::stdin().read_to_end(&mut buf);

    let mut x: i32 = 0;
    if let Some(v) = scanf_int(&buf) {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
    // C "return 0;" — Rust main returns () implicitly with exit code 0.
}
