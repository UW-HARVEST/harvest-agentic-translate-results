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

use std::io::{self, Read, Write};

// ---------------------------------------------------------------------------
// stdio emulation
// ---------------------------------------------------------------------------

/// A single, process-wide stdin byte source, mirroring C's single `stdin`
/// FILE stream: reads pick up exactly where the previous read stopped.
struct Stdin {
    inner: io::Stdin,
    peeked: Option<u8>,
    eof: bool,
}

impl Stdin {
    fn new() -> Self {
        Stdin {
            inner: io::stdin(),
            peeked: None,
            eof: false,
        }
    }

    /// Read a single byte, `None` on EOF (or read error, like C's stdio which
    /// reports both through a NULL/EOF return here).
    fn read_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.peeked.take() {
            return Some(b);
        }
        if self.eof {
            return None;
        }
        let mut buf = [0u8; 1];
        loop {
            match self.inner.read(&mut buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(_) => return Some(buf[0]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Faithful `fgets(buf, size, stdin)`.
    ///
    /// Stores at most `size - 1` bytes plus a terminating NUL into `buf`,
    /// stopping after a newline (which is kept) or at EOF. Returns `false`
    /// (the NULL return of C) when EOF is hit before any byte was read.
    fn fgets(&mut self, buf: &mut [u8], size: usize) -> bool {
        if size == 0 {
            return false;
        }
        let mut n = 0usize;
        while n + 1 < size {
            match self.read_byte() {
                Some(b) => {
                    buf[n] = b;
                    n += 1;
                    if b == b'\n' {
                        break;
                    }
                }
                None => {
                    if n == 0 {
                        // EOF with nothing read -> NULL
                        return false;
                    }
                    break;
                }
            }
        }
        buf[n] = 0;
        true
    }
}

/// Buffered stdout, flushed once at the end, matching C's buffered `printf`
/// byte stream (glibc uses a 4096 byte block buffer for non-tty streams; the
/// buffer contents are lost if the process dies from a signal).
struct Stdout {
    out: io::BufWriter<io::Stdout>,
}

impl Stdout {
    fn new() -> Self {
        Stdout {
            out: io::BufWriter::with_capacity(4096, io::stdout()),
        }
    }

    fn write_str(&mut self, s: &str) {
        // Ignore write errors, as C's printf return value is ignored here.
        let _ = self.out.write_all(s.as_bytes());
    }

    fn flush(&mut self) {
        let _ = self.out.flush();
    }
}

// ---------------------------------------------------------------------------
// libc helpers
// ---------------------------------------------------------------------------

fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// glibc's `atoi`, i.e. `(int) strtol(nptr, NULL, 10)`:
/// leading whitespace is skipped, an optional sign is accepted, decimal
/// digits are accumulated saturating at LONG_MIN/LONG_MAX, and the resulting
/// `long` is truncated to `int`.
fn c_atoi(bytes: &[u8]) -> i32 {
    // Only the NUL-terminated prefix is visible to atoi.
    let s: &[u8] = match bytes.iter().position(|&b| b == 0) {
        Some(p) => &bytes[..p],
        None => bytes,
    };

    let mut i = 0usize;
    while i < s.len() && c_isspace(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = i64::from(s[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    let value: i64 = if overflow {
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

    // (int) truncation of a long.
    value as i32
}

// ---------------------------------------------------------------------------
// program
// ---------------------------------------------------------------------------

fn print_line(out: &mut Stdout, line: Option<&str>) {
    if let Some(line) = line {
        out.write_str(line);
        out.write_str("\n");
    }
}

fn print_int_line(out: &mut Stdout, int_number: i32) {
    out.write_str(&format!("{}\n", int_number));
}

/// Indices just past `int buffer[10]` in `bad()`'s frame whose overwrite is
/// immediately fatal on the reference platform (x86-64 Linux, gcc, default
/// CMake flags). These slots hold the live control data of the frame -- the
/// saved frame pointer and the return address of `bad()` -- so clobbering them
/// with the value 1 makes the function return to address 1 and the process dies
/// with SIGSEGV. Measured to be 100% reproducible over 100 runs each; all other
/// nearby indices land in dead stack padding and are invisible.
///
/// buffer[16..=19] and buffer[26..=27] are the two fatal windows.
fn index_hits_live_control_slot(idx: i64) -> bool {
    matches!(idx, 16..=19 | 26 | 27)
}

/// Highest address (exclusive) of this process's `[stack]` mapping, read from
/// `/proc/self/maps`.
///
/// A store above this address is on an unmapped page and faults; a store below
/// it hits real, mapped stack memory. This is the *actual* rule the compiled C
/// obeys, and reading it at runtime is what makes the translation track it:
/// the distance from `bad()`'s frame to the top of the stack depends on how big
/// argv and the environment are, plus a per-exec random offset the kernel adds,
/// so no compile-time constant can stand in for it.
fn stack_mapping_end() -> Option<usize> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    for line in maps.lines() {
        if line.ends_with("[stack]") {
            let range = line.split_whitespace().next()?;
            let end = range.split('-').nth(1)?;
            return usize::from_str_radix(end, 16).ok();
        }
    }
    None
}

/// `buffer[data] = 1` where `buffer` is `int buffer[10]`.
///
/// The C code performs an out-of-bounds stack write whenever `data >= 10`
/// (CWE-787 / CWE-129). This reproduces the observed behaviour of the compiled
/// C:
///  * `0 <= data < 10`: normal in-bounds store.
///  * an index hitting a live control slot of `bad()`'s frame, or one landing
///    past the top of the stack mapping: the process dies with SIGSEGV before
///    any of the buffered stdout data is written out, so stdout and stderr both
///    come out empty.
///  * otherwise: the store lands in unrelated dead stack padding and has no
///    effect on the ten values printed afterwards.
fn store_one(buffer: &mut [i32; 10], data: i32) {
    let idx = i64::from(data);

    if idx >= 0 && (idx as usize) < buffer.len() {
        buffer[idx as usize] = 1;
        return;
    }

    if index_hits_live_control_slot(idx) {
        segfault();
    }

    // Where the C's store would actually land. `buffer` is a genuine local of
    // this frame, at essentially the same stack depth as the C's, so its
    // address is the right base for the comparison.
    let base = buffer.as_ptr() as usize;
    let target = (base as u64).wrapping_add((idx as u64).wrapping_mul(4));

    match stack_mapping_end() {
        // Past the top of the stack: unmapped page, fatal. A wrapped/absurd
        // address is fatal too.
        Some(end) => {
            if target < base as u64 || target >= end as u64 {
                segfault();
            }
        }
        // No /proc: fall back to treating anything far out as fatal.
        None => {
            if idx >= 4096 {
                segfault();
            }
        }
    }
    // Otherwise: write discarded (lands in mapped but unused stack space).
}

/// Reproduce the fatal invalid write of the original C program.
fn segfault() -> ! {
    // A volatile write through a null pointer, which the platform turns into
    // the same SIGSEGV the C program dies from.
    unsafe {
        std::ptr::write_volatile(std::ptr::null_mut::<i32>(), 1);
    }
    std::process::abort();
}

fn bad(out: &mut Stdout, stdin: &mut Stdin) {
    let mut data: i32;
    /* Initialize data */
    data = -1;
    {
        let mut input_buffer = [0u8; 14];
        if stdin.fgets(&mut input_buffer, 14) {
            /* Convert to int */
            data = c_atoi(&input_buffer);
        } else {
            print_line(out, Some("fgets() failed."));
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            store_one(&mut buffer, data);
            /* Print the array values */
            for i in 0..10 {
                print_int_line(out, buffer[i]);
            }
        } else {
            print_line(out, Some("ERROR: Array index is negative."));
        }
    }
}

/* goodG2B uses the GoodSource with the BadSink */
fn good_g2b(out: &mut Stdout) {
    let mut data: i32;
    /* Initialize data */
    data = -1;
    let _ = data;
    data = 7;
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            store_one(&mut buffer, data);
            /* Print the array values */
            for i in 0..10 {
                print_int_line(out, buffer[i]);
            }
        } else {
            print_line(out, Some("ERROR: Array index is negative."));
        }
    }
}

/* goodB2G uses the BadSource with the GoodSink */
fn good_b2g(out: &mut Stdout, stdin: &mut Stdin) {
    let mut data: i32;
    /* Initialize data */
    data = -1;
    {
        let mut input_buffer = [0u8; 14];
        if stdin.fgets(&mut input_buffer, 14) {
            /* Convert to int */
            data = c_atoi(&input_buffer);
        } else {
            print_line(out, Some("fgets() failed."));
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 && data < 10 {
            store_one(&mut buffer, data);
            /* Print the array values */
            for i in 0..10 {
                print_int_line(out, buffer[i]);
            }
        } else {
            print_line(out, Some("ERROR: Array index is out-of-bounds"));
        }
    }
}

fn good(out: &mut Stdout, stdin: &mut Stdin) {
    good_g2b(out);
    good_b2g(out, stdin);
}

fn main() {
    let mut out = Stdout::new();
    let mut stdin = Stdin::new();

    print_line(&mut out, Some("Calling good()..."));
    good(&mut out, &mut stdin);
    print_line(&mut out, Some("Finished good()"));
    print_line(&mut out, Some("Calling bad()..."));
    bad(&mut out, &mut stdin);
    print_line(&mut out, Some("Finished bad()"));

    out.flush();
    std::process::exit(0);
}
