// Rust translation of c_src/src/main.c
//
// Original C: Copyright 2025 MIT Lincoln Laboratory (MIT-style license, see
// c_src/src/main.c for the full notice).
//
// This is a faithful, behaviour-preserving translation. Bugs present in the C
// are reproduced, not fixed:
//   * `bad()` validates only `data >= 0` and then performs `buffer[data] = 1`
//     on a 10-element array, so any `data >= 10` is an out-of-bounds write
//     (CWE-129 / CWE-787). See `StackFrame` below for how that is modelled.
//   * The order of reads, validations and prints is preserved exactly.
//   * `fgets` reads at most 13 bytes and never past a newline, so a long input
//     line is split across the two `fgets` call sites, exactly as in C.

use std::io::{BufRead, StdinLock, Write};

/// Size of the C `char inputBuffer[14]`.
const INPUT_BUFFER_LEN: usize = 14;

/// Size of the C `int buffer[10]`.
const BUFFER_LEN: usize = 10;

/// Extra words standing in for the rest of the C function's stack frame.
///
/// In C, `buffer[data] = 1` with `data >= BUFFER_LEN` writes past the array.
/// That is undefined behaviour: with `gcc -O2` the store lands in unused frame
/// space and is unobservable, while with `gcc -O0` a handful of layout-specific
/// indices happen to clobber the return address and crash. Since only
/// `buffer[0..BUFFER_LEN]` is ever printed, the observable, reproducible
/// behaviour is that the stray store is absorbed; we model that with padding
/// rather than replicating one compiler's accidental crash.
const FRAME_PAD_LEN: usize = 4096;

/// Emulates the C stack frame holding `int buffer[10]`.
struct StackFrame {
    buffer: [i32; BUFFER_LEN],
    /// Stands in for the remainder of the frame; written to but never read.
    _pad: [i32; FRAME_PAD_LEN],
}

impl StackFrame {
    fn new() -> Self {
        // `int buffer[10] = { 0 };`
        StackFrame {
            buffer: [0; BUFFER_LEN],
            _pad: [0; FRAME_PAD_LEN],
        }
    }

    /// `buffer[index] = value`, including the out-of-bounds case.
    ///
    /// `index` is the (already known non-negative) C `int`. Indices inside the
    /// array behave normally; indices past it are absorbed by the emulated
    /// frame, and indices beyond even that are dropped (in C they would scribble
    /// on unrelated memory or fault, neither of which is observable in output).
    fn store(&mut self, index: i32, value: i32) {
        let index = index as usize;
        if index < BUFFER_LEN {
            self.buffer[index] = value;
        } else if index < BUFFER_LEN + FRAME_PAD_LEN {
            self._pad[index - BUFFER_LEN] = value;
        }
    }
}

/// `printf("%s\n", line)` guarded by a NULL check.
///
/// The C takes `const char *` and skips NULL; `Option` makes that explicit.
fn print_line(out: &mut impl Write, line: Option<&str>) {
    if let Some(line) = line {
        // Errors are ignored, matching C's unchecked printf.
        let _ = writeln!(out, "{}", line);
    }
}

/// `printf("%d\n", intNumber)`.
fn print_int_line(out: &mut impl Write, int_number: i32) {
    let _ = writeln!(out, "{}", int_number);
}

/// True for the characters glibc's `isspace` accepts in the C locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// glibc `atoi`, which is `(int) strtol(s, NULL, 10)`.
///
/// Skips leading whitespace, accepts one optional sign, consumes ASCII digits
/// and stops at the first non-digit. On `long` overflow `strtol` saturates to
/// `LONG_MAX`/`LONG_MIN`, and the cast to `int` truncates; both are reproduced,
/// so e.g. "9999999999" yields 1410065407 and twenty nines yield -1.
fn atoi(s: &[u8]) -> i32 {
    // The C string ends at the first NUL byte.
    let s = match s.iter().position(|&b| b == 0) {
        Some(nul) => &s[..nul],
        None => s,
    };

    let mut i = 0;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let mut acc: i64 = 0;
    let mut overflowed = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !overflowed {
            match acc.checked_mul(10).and_then(|a| a.checked_add(digit)) {
                Some(next) => acc = next,
                // Keep scanning digits, as strtol does, but remember the overflow.
                None => overflowed = true,
            }
        }
        i += 1;
    }

    if overflowed {
        // (int) LONG_MAX == -1, (int) LONG_MIN == 0 on x86-64.
        return if negative {
            i64::MIN as i32
        } else {
            i64::MAX as i32
        };
    }

    let value = if negative { -acc } else { acc };
    // C's narrowing conversion to `int`: two's-complement truncation.
    value as i32
}

/// Reads one byte, or `None` at EOF / on error.
fn read_byte(reader: &mut StdinLock<'_>) -> Option<u8> {
    loop {
        let byte = match reader.fill_buf() {
            Ok(available) => {
                if available.is_empty() {
                    return None; // EOF
                }
                available[0]
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        };
        reader.consume(1);
        return Some(byte);
    }
}

/// `fgets(dst, dst.len(), stdin)`; returns false where C returns NULL.
///
/// Copies at most `dst.len() - 1` bytes, stops after a newline (which is kept),
/// NUL-terminates, and leaves anything beyond that in the stream for the next
/// call. Returns false only when EOF/error is hit before any byte is read.
fn fgets(dst: &mut [u8], reader: &mut StdinLock<'_>) -> bool {
    let capacity = dst.len() - 1;
    let mut written = 0;
    while written < capacity {
        match read_byte(reader) {
            Some(byte) => {
                dst[written] = byte;
                written += 1;
                if byte == b'\n' {
                    break;
                }
            }
            None => break,
        }
    }
    if written == 0 {
        return false;
    }
    dst[written] = 0;
    true
}

fn bad(out: &mut impl Write, reader: &mut StdinLock<'_>) {
    let mut data: i32;
    // Initialize data
    data = -1;
    {
        let mut input_buffer = [0u8; INPUT_BUFFER_LEN];
        if fgets(&mut input_buffer, reader) {
            // Convert to int
            data = atoi(&input_buffer);
        } else {
            print_line(out, Some("fgets() failed."));
        }
    }
    {
        let mut frame = StackFrame::new();
        if data >= 0 {
            // BUG (reproduced): no upper bound check, so data >= 10 writes
            // out of bounds.
            frame.store(data, 1);
            // Print the array values
            for i in 0..BUFFER_LEN {
                print_int_line(out, frame.buffer[i]);
            }
        } else {
            print_line(out, Some("ERROR: Array index is negative."));
        }
    }
}

/// goodG2B uses the GoodSource with the BadSink
// The C has a dead store (`data = -1;` then `data = 7;`); it is kept verbatim.
#[allow(unused_assignments)]
fn good_g2b(out: &mut impl Write) {
    let mut data: i32;
    // Initialize data
    data = -1;
    data = 7;
    {
        let mut frame = StackFrame::new();
        if data >= 0 {
            frame.store(data, 1);
            // Print the array values
            for i in 0..BUFFER_LEN {
                print_int_line(out, frame.buffer[i]);
            }
        } else {
            print_line(out, Some("ERROR: Array index is negative."));
        }
    }
}

/// goodB2G uses the BadSource with the GoodSink
fn good_b2g(out: &mut impl Write, reader: &mut StdinLock<'_>) {
    let mut data: i32;
    // Initialize data
    data = -1;
    {
        let mut input_buffer = [0u8; INPUT_BUFFER_LEN];
        if fgets(&mut input_buffer, reader) {
            // Convert to int
            data = atoi(&input_buffer);
        } else {
            print_line(out, Some("fgets() failed."));
        }
    }
    {
        let mut frame = StackFrame::new();
        if data >= 0 && data < (BUFFER_LEN as i32) {
            frame.store(data, 1);
            // Print the array values
            for i in 0..BUFFER_LEN {
                print_int_line(out, frame.buffer[i]);
            }
        } else {
            print_line(out, Some("ERROR: Array index is out-of-bounds"));
        }
    }
}

fn good(out: &mut impl Write, reader: &mut StdinLock<'_>) {
    good_g2b(out);
    good_b2g(out, reader);
}

fn main() {
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();

    let stdout = std::io::stdout();
    // Buffered like C's stdout, so the byte stream is emitted in the same order.
    let mut out = std::io::BufWriter::new(stdout.lock());

    print_line(&mut out, Some("Calling good()..."));
    good(&mut out, &mut reader);
    print_line(&mut out, Some("Finished good()"));
    print_line(&mut out, Some("Calling bad()..."));
    bad(&mut out, &mut reader);
    print_line(&mut out, Some("Finished bad()"));

    let _ = out.flush();
    // C returns 0 from main.
    std::process::exit(0);
}
