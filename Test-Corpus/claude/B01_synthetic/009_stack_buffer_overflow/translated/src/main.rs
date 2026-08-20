// Rust translation of c_src/src/main.c
//
// Original copyright notice from the C source:
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::io::{self, BufWriter, Read, Write};

/// Emulation of C `stdio` for the single-threaded, sequential behavior of the
/// original program: byte-at-a-time reads from stdin (so that `fgets` never
/// consumes more input than a C `fgets` would) plus a buffered stdout.
struct Io {
    stdin: io::Stdin,
    stdout: BufWriter<io::Stdout>,
}

impl Io {
    fn new() -> Self {
        Io {
            stdin: io::stdin(),
            stdout: BufWriter::new(io::stdout()),
        }
    }

    /// Reads a single byte from stdin, returning `None` on EOF.
    fn read_byte(&mut self) -> Option<u8> {
        let mut b = [0u8; 1];
        loop {
            match self.stdin.read(&mut b) {
                Ok(0) => return None,
                Ok(_) => return Some(b[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    /// Emulates `fgets(buf, size, stdin)`.
    ///
    /// Reads at most `size - 1` bytes, stopping after a newline (which is kept
    /// in the returned bytes). Returns `None` if EOF is hit before any byte is
    /// read, mirroring `fgets` returning NULL. The implicit NUL terminator that
    /// C writes is not part of the returned data.
    fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size == 0 {
            return None;
        }
        let mut buf: Vec<u8> = Vec::with_capacity(size);
        while buf.len() + 1 < size {
            match self.read_byte() {
                Some(c) => {
                    buf.push(c);
                    if c == b'\n' {
                        break;
                    }
                }
                None => break,
            }
        }
        if buf.is_empty() {
            None
        } else {
            Some(buf)
        }
    }

    fn print_line(&mut self, line: &str) {
        // printf("%s\n", line) -- the NULL check in the C original can never
        // fail for the string literals it is called with.
        let _ = write!(self.stdout, "{}\n", line);
    }

    fn print_int_line(&mut self, int_number: i32) {
        // printf("%d\n", intNumber)
        let _ = write!(self.stdout, "{}\n", int_number);
    }
}

/// Emulates glibc's `atoi()`, which is `(int) strtol(nptr, NULL, 10)`:
/// leading whitespace is skipped, an optional sign is consumed, decimal digits
/// are accumulated, parsing stops at the first non-digit (or the NUL
/// terminator), and out-of-range values saturate to `long` bounds before being
/// truncated to `int`.
fn c_atoi(bytes: &[u8]) -> i32 {
    // A C string ends at the first NUL byte.
    let s = match bytes.iter().position(|&b| b == 0) {
        Some(pos) => &bytes[..pos],
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

    let mut value: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as i64;
        if !overflow {
            match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => value = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    let as_long: i64 = if overflow {
        // strtol clamps to LONG_MAX / LONG_MIN (long is 64-bit here).
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        value.wrapping_neg()
    } else {
        value
    };

    // (int) cast: implementation-defined truncation of the low 32 bits.
    as_long as i32
}

fn bad(io: &mut Io) {
    let mut data: i32;
    /* Initialize data */
    data = -1;
    {
        // char inputBuffer[14] = "";
        match io.fgets(14) {
            Some(input_buffer) => {
                /* Convert to int */
                data = c_atoi(&input_buffer);
            }
            None => {
                io.print_line("fgets() failed.");
            }
        }
    }
    {
        let mut buffer = [0i32; 10];
        if data >= 0 {
            // buffer[data] = 1;
            //
            // This is the CWE-129 sink: the C code performs no upper-bound
            // check, so for data >= 10 it writes past the end of the buffer,
            // which is undefined behavior. In practice (gcc, no optimization)
            // such a write lands in unused stack space or in dead locals of the
            // same frame, leaving all ten printed elements at 0, so the write is
            // simply dropped here. It cannot change any value that gets printed.
            let index = data as usize;
            if index < buffer.len() {
                buffer[index] = 1;
            }
            /* Print the array values */
            for i in 0..10 {
                io.print_int_line(buffer[i]);
            }
        } else {
            io.print_line("ERROR: Array index is negative.");
        }
    }
}

/* goodG2B uses the GoodSource with the BadSink */
#[allow(unused_assignments)] // mirrors the dead store in the C original
fn good_g2b(io: &mut Io) {
    let mut data: i32;
    /* Initialize data */
    data = -1;
    data = 7;
    {
        let mut buffer = [0i32; 10];
        if data >= 0 {
            buffer[data as usize] = 1;
            /* Print the array values */
            for i in 0..10 {
                io.print_int_line(buffer[i]);
            }
        } else {
            io.print_line("ERROR: Array index is negative.");
        }
    }
}

/* goodB2G uses the BadSource with the GoodSink */
fn good_b2g(io: &mut Io) {
    let mut data: i32;
    /* Initialize data */
    data = -1;
    {
        // char inputBuffer[14] = "";
        match io.fgets(14) {
            Some(input_buffer) => {
                /* Convert to int */
                data = c_atoi(&input_buffer);
            }
            None => {
                io.print_line("fgets() failed.");
            }
        }
    }
    {
        let mut buffer = [0i32; 10];
        if data >= 0 && data < 10 {
            buffer[data as usize] = 1;
            /* Print the array values */
            for i in 0..10 {
                io.print_int_line(buffer[i]);
            }
        } else {
            io.print_line("ERROR: Array index is out-of-bounds");
        }
    }
}

fn good(io: &mut Io) {
    good_g2b(io);
    good_b2g(io);
}

fn main() {
    let mut io = Io::new();
    io.print_line("Calling good()...");
    good(&mut io);
    io.print_line("Finished good()");
    io.print_line("Calling bad()...");
    bad(&mut io);
    io.print_line("Finished bad()");
    let _ = io.stdout.flush();
}
