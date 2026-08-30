// Rust translation of c_src/src/main.c
//
// Behavior-preserving port of a CWE-129 style test driver
// (unvalidated array index used as a write offset).
//
// The original C is reproduced exactly, including its defects:
//   * `bad()` only checks `data >= 0` before `buffer[data] = 1`, so an index
//     >= 10 is an out-of-bounds stack write in C.  C's behavior there is
//     undefined; this port models it as a discarded write, which reproduces
//     the observable stdout of the C program (the 10 in-bounds slots stay 0).
//   * `goodG2B()` hard-codes data = 7 and never touches stdin, so exactly two
//     `fgets()` reads happen over the program's life: `goodB2G()` consumes the
//     first line, `bad()` the second.

use std::io::{self, BufRead, BufWriter, Write};

const BUFFER_LEN: usize = 10;
const INPUT_BUFFER_LEN: usize = 14;

/// `printf("%s\n", line)` -- the C version skips NULL, which cannot happen
/// here because every call site passes a string literal.
fn print_line(out: &mut impl Write, line: &str) {
    let _ = writeln!(out, "{}", line);
}

/// `printf("%d\n", intNumber)`
fn print_int_line(out: &mut impl Write, int_number: i32) {
    let _ = writeln!(out, "{}", int_number);
}

/// `fgets(buf, size, stdin)`: reads at most `size - 1` bytes, stopping after a
/// newline (which is retained).  Returns `None` for the NULL case, i.e. EOF
/// with nothing read, or a read error.  Anything past the limit stays in the
/// stream for the next call, matching C's buffered stdin.
fn fgets<R: BufRead>(reader: &mut R, size: usize) -> Option<Vec<u8>> {
    if size == 0 {
        return None;
    }
    let max = size - 1;
    let mut out: Vec<u8> = Vec::with_capacity(max);

    while out.len() < max {
        let available = match reader.fill_buf() {
            Ok(buf) => buf,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        };
        if available.is_empty() {
            // EOF
            return if out.is_empty() { None } else { Some(out) };
        }
        let take = available.len().min(max - out.len());
        match available[..take].iter().position(|&c| c == b'\n') {
            Some(pos) => {
                out.extend_from_slice(&available[..=pos]);
                reader.consume(pos + 1);
                return Some(out);
            }
            None => {
                out.extend_from_slice(&available[..take]);
                reader.consume(take);
            }
        }
    }
    Some(out)
}

/// `atoi()` on a NUL-terminated buffer: skip leading whitespace, accept an
/// optional sign, consume digits.  glibc implements it as `(int)strtol(...)`,
/// so out-of-range values saturate at the `long` boundary and are then
/// truncated to `int`.  Parsing stops at the first non-digit, NUL included.
fn atoi(bytes: &[u8]) -> i32 {
    let s: &[u8] = match bytes.iter().position(|&c| c == 0) {
        Some(nul) => &bytes[..nul],
        None => bytes,
    };

    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let mut acc: i64 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        i += 1;
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

    value as i32 // truncating conversion, as in (int)strtol(...)
}

/// Reads a line via `fgets(inputBuffer, 14, stdin)` and converts it with
/// `atoi()`; leaves `data` at -1 and reports failure when `fgets` returns NULL.
fn bad_source<R: BufRead>(out: &mut impl Write, reader: &mut R) -> i32 {
    let mut data: i32 = -1;
    match fgets(reader, INPUT_BUFFER_LEN) {
        Some(input_buffer) => {
            data = atoi(&input_buffer);
        }
        None => {
            print_line(out, "fgets() failed.");
        }
    }
    data
}

fn print_buffer(out: &mut impl Write, buffer: &[i32; BUFFER_LEN]) {
    for i in 0..BUFFER_LEN {
        print_int_line(out, buffer[i]);
    }
}

fn bad<R: BufRead>(out: &mut impl Write, reader: &mut R) {
    let data = bad_source(out, reader);

    let mut buffer: [i32; BUFFER_LEN] = [0; BUFFER_LEN];
    if data >= 0 {
        // The missing upper-bound check is the defect being demonstrated.
        // In C this writes past `buffer` for data >= 10; the write lands in
        // unrelated stack storage and leaves all printed slots at 0.
        if (data as usize) < BUFFER_LEN {
            buffer[data as usize] = 1;
        }
        print_buffer(out, &buffer);
    } else {
        print_line(out, "ERROR: Array index is negative.");
    }
}

/// goodG2B uses the GoodSource with the BadSink
fn good_g2b(out: &mut impl Write) {
    let data: i32 = 7;

    let mut buffer: [i32; BUFFER_LEN] = [0; BUFFER_LEN];
    if data >= 0 {
        if (data as usize) < BUFFER_LEN {
            buffer[data as usize] = 1;
        }
        print_buffer(out, &buffer);
    } else {
        print_line(out, "ERROR: Array index is negative.");
    }
}

/// goodB2G uses the BadSource with the GoodSink
fn good_b2g<R: BufRead>(out: &mut impl Write, reader: &mut R) {
    let data = bad_source(out, reader);

    let mut buffer: [i32; BUFFER_LEN] = [0; BUFFER_LEN];
    if data >= 0 && data < BUFFER_LEN as i32 {
        buffer[data as usize] = 1;
        print_buffer(out, &buffer);
    } else {
        print_line(out, "ERROR: Array index is out-of-bounds");
    }
}

fn good<R: BufRead>(out: &mut impl Write, reader: &mut R) {
    good_g2b(out);
    good_b2g(out, reader);
}

fn main() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    print_line(&mut out, "Calling good()...");
    good(&mut out, &mut reader);
    print_line(&mut out, "Finished good()");
    print_line(&mut out, "Calling bad()...");
    bad(&mut out, &mut reader);
    print_line(&mut out, "Finished bad()");

    let _ = out.flush();
}
