// Rust translation of c_src/src/main.c
//
// Behavior-preserving port of a Juliet-style CWE-129 (improper validation of
// array index) test case. The original C code's bugs are reproduced, not fixed:
//
//   * `bad()` only checks `data >= 0` before `buffer[data] = 1` on a 10-element
//     array, so any value >= 10 stores past the end of the array.
//   * `goodB2G()` performs the full `0 <= data < 10` check.
//   * `goodG2B()` hardcodes a safe index (7).
//
// ---------------------------------------------------------------------------
// Emulating the out-of-bounds store
// ---------------------------------------------------------------------------
// `buffer[data] = 1` with `data >= 10` is undefined behavior in C. The observed
// behavior of the reference build (gcc -O0, x86-64 Linux) follows directly from
// the stack frame layout of `bad()`, taken from its disassembly:
//
//   int buffer[10]      rbp-0x40 .. rbp-0x19    indices  0..9   (the array)
//   char inputBuffer[14] rbp-0x16               indices 10..13  (dead by now)
//   int i               rbp-0x08                index   14      (reset to 0 next)
//   int data            rbp-0x04                index   15      (never re-read)
//   saved rbp           rbp+0x00                indices 16,17
//   return address      rbp+0x08                indices 18,19   -> bad() faults on return
//   main's scratch      rbp+0x10 ..             indices 20..23
//   main's saved rbp    rbp+0x18                indices 24,25   (popped, never used)
//   main's return addr  rbp+0x20                indices 26,27   -> main() faults on return
//   further stack       ...                     indices 28..    (unused frames)
//   top of stack mapping                        far indices     -> the store itself faults
//
// So: indices 10..15 and 20..25 corrupt only dead storage and are invisible;
// indices 16..19 and 26..27 clobber a saved frame pointer or return address and
// fault when the corresponding function returns, i.e. *after* its output has
// been produced; sufficiently large indices run past the end of the stack
// mapping and fault at the store, before anything else is printed. Because
// stdout is fully buffered when redirected, a fault discards everything still
// buffered, so these runs typically emit no output at all and die with SIGSEGV.
//
// Two aspects of the C behavior are not properties of the program and cannot be
// reproduced faithfully:
//   * The distance from the frame to the top of the stack mapping varies with
//     stack ASLR and environment size. Measured over 30 runs each, index 1150
//     never faulted, 1900 faulted about half the time, and 3400 always faulted.
//     FAR_STORE_FAULTS_AT below sits in the middle of that band.
//   * Very large indices land in unrelated mappings and raise SIGSEGV or
//     SIGBUS nondeterministically; SIGSEGV is used for all of them here.

use std::io::{IsTerminal, Read, Write};

/// Number of elements in the C `buffer` array.
const BUFFER_LEN: usize = 10;

/// `char inputBuffer[14]` -> fgets is called with size 14.
const INPUT_BUFFER_SIZE: usize = 14;

/// Smallest index whose store is treated as running off the top of the stack
/// mapping. See the note above: the real boundary is ASLR/environment dependent.
const FAR_STORE_FAULTS_AT: usize = 2048;

/// What the out-of-bounds store in `bad()` corrupted, and hence when the
/// reference build dies.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// Nothing observable was overwritten.
    None,
    /// Saved frame pointer or return address of `bad()`: faults as `bad()`
    /// returns, after its ten values have been printed.
    OnBadReturn,
    /// Return address of `main()`: faults as `main()` returns, after
    /// "Finished bad()" has been printed.
    OnMainReturn,
    /// Store landed outside the stack mapping: faults immediately.
    Immediate,
}

/// Classifies an out-of-bounds index (>= BUFFER_LEN) per the frame layout above.
fn classify_store(index: usize) -> Fault {
    match index {
        16..=19 => Fault::OnBadReturn,
        26..=27 => Fault::OnMainReturn,
        i if i >= FAR_STORE_FAULTS_AT => Fault::Immediate,
        _ => Fault::None,
    }
}

/// Raises SIGSEGV the way the corrupted C control flow does.
///
/// Reproducing a fault is not expressible in safe Rust, so this is the only
/// unsafe block in the program: a volatile store through a null pointer, which
/// LLVM must preserve and which always faults on Linux (address 0 is never
/// mapped). Buffered stdout is deliberately left unflushed, matching the
/// reference build, which loses its buffered output when it dies.
fn raise_fault() -> ! {
    unsafe {
        std::ptr::write_volatile(std::ptr::null_mut::<i32>(), 1);
    }
    // Unreachable on Linux; keeps the `!` return type honest elsewhere.
    std::process::abort();
}

/// stdout with C's buffering semantics: line buffered on a terminal, fully
/// buffered otherwise, so that a fault loses the same output C loses.
struct COut {
    inner: std::io::BufWriter<std::io::Stdout>,
    line_buffered: bool,
}

impl COut {
    fn new() -> Self {
        let stdout = std::io::stdout();
        let line_buffered = stdout.is_terminal();
        COut {
            inner: std::io::BufWriter::new(stdout),
            line_buffered,
        }
    }

    /// Mirrors C's `printLine`. The NULL check in the C source can never fail
    /// for the string literals passed to it, so the string is always printed.
    fn print_line(&mut self, line: &str) {
        let _ = write!(self.inner, "{}\n", line);
        self.maybe_flush();
    }

    /// Mirrors C's `printIntLine`: `printf("%d\n", intNumber)`.
    fn print_int_line(&mut self, int_number: i32) {
        let _ = write!(self.inner, "{}\n", int_number);
        self.maybe_flush();
    }

    fn maybe_flush(&mut self) {
        if self.line_buffered {
            let _ = self.inner.flush();
        }
    }

    fn flush(&mut self) {
        let _ = self.inner.flush();
    }
}

/// Byte-oriented stdin reader providing C `fgets` semantics.
struct CStdin {
    inner: std::io::Stdin,
}

impl CStdin {
    fn new() -> Self {
        CStdin {
            inner: std::io::stdin(),
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        let mut byte = [0u8; 1];
        loop {
            match self.inner.read(&mut byte) {
                Ok(0) => return None,
                Ok(_) => return Some(byte[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    /// `fgets(buf, size, stdin)`: reads at most `size - 1` bytes, stopping after
    /// a newline (which is kept in the buffer). Returns `None` (NULL) when EOF
    /// is reached before any byte is read. Unlike `scanf`, it never reads past
    /// the newline, so the tail of an over-long line is left for the next call.
    fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if size == 0 {
            return None;
        }
        let mut buf: Vec<u8> = Vec::new();
        while buf.len() + 1 < size {
            match self.read_byte() {
                Some(b) => {
                    buf.push(b);
                    if b == b'\n' {
                        break;
                    }
                }
                None => {
                    if buf.is_empty() {
                        return None;
                    }
                    break;
                }
            }
        }
        Some(buf)
    }
}

/// glibc `atoi`: `(int) strtol(nptr, NULL, 10)`. Leading whitespace is skipped,
/// an optional sign is accepted, digits are consumed until a non-digit, and
/// overflow saturates at LONG_MIN/LONG_MAX before truncation to `int`. Parsing
/// also stops at the terminating NUL byte of the C string.
fn c_atoi(bytes: &[u8]) -> i32 {
    // Respect C string termination.
    let s: &[u8] = match bytes.iter().position(|&b| b == 0) {
        Some(nul) => &bytes[..nul],
        None => bytes,
    };

    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Accumulate as i64 (64-bit `long`) with saturation, as strtol does.
    let mut value: i64 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as i64;
        if !saturated {
            match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => value = v,
                None => saturated = true,
            }
        }
        i += 1;
    }

    let as_long: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -value
    } else {
        value
    };

    // Truncation from `long` to `int`.
    as_long as i32
}

/// BadSource: reads a line with `fgets` into a 14-byte buffer and converts it
/// with `atoi`. Returns -1 (the initial value) if `fgets` fails.
fn bad_source(out: &mut COut, stdin: &mut CStdin) -> i32 {
    let mut data: i32 = -1;
    // char inputBuffer[14] = "";
    let mut input_buffer = [0u8; INPUT_BUFFER_SIZE];
    match stdin.fgets(INPUT_BUFFER_SIZE) {
        Some(line) => {
            input_buffer[..line.len()].copy_from_slice(&line);
            input_buffer[line.len()] = 0;
            /* Convert to int */
            data = c_atoi(&input_buffer);
        }
        None => {
            out.print_line("fgets() failed.");
        }
    }
    data
}

/// BadSink: only rejects negative indices, so `data >= 10` overruns the array.
/// Returns the fault the C build defers until it returns, if any.
fn bad_sink(out: &mut COut, data: i32) -> Fault {
    let mut buffer = [0i32; BUFFER_LEN];
    let mut deferred = Fault::None;
    if data >= 0 {
        let index = data as usize;
        if index < BUFFER_LEN {
            buffer[index] = 1;
        } else {
            match classify_store(index) {
                Fault::Immediate => raise_fault(),
                other => deferred = other,
            }
        }
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            out.print_int_line(buffer[i]);
        }
    } else {
        out.print_line("ERROR: Array index is negative.");
    }
    deferred
}

fn bad(out: &mut COut, stdin: &mut CStdin) -> Fault {
    let data = bad_source(out, stdin);
    let deferred = bad_sink(out, data);
    if deferred == Fault::OnBadReturn {
        // The return address / saved rbp of bad() was overwritten.
        raise_fault();
    }
    deferred
}

/// goodG2B uses the GoodSource with the BadSink.
fn good_g2b(out: &mut COut) {
    /* Initialize data */
    // data = -1; data = 7;
    let data: i32 = 7;
    let mut buffer = [0i32; BUFFER_LEN];
    if data >= 0 {
        buffer[data as usize] = 1;
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            out.print_int_line(buffer[i]);
        }
    } else {
        out.print_line("ERROR: Array index is negative.");
    }
}

/// goodB2G uses the BadSource with the GoodSink.
fn good_b2g(out: &mut COut, stdin: &mut CStdin) {
    let data = bad_source(out, stdin);
    let mut buffer = [0i32; BUFFER_LEN];
    if data >= 0 && data < BUFFER_LEN as i32 {
        buffer[data as usize] = 1;
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            out.print_int_line(buffer[i]);
        }
    } else {
        out.print_line("ERROR: Array index is out-of-bounds");
    }
}

fn good(out: &mut COut, stdin: &mut CStdin) {
    good_g2b(out);
    good_b2g(out, stdin);
}

fn main() {
    let mut out = COut::new();
    let mut stdin = CStdin::new();

    out.print_line("Calling good()...");
    good(&mut out, &mut stdin);
    out.print_line("Finished good()");
    out.print_line("Calling bad()...");
    let deferred = bad(&mut out, &mut stdin);
    out.print_line("Finished bad()");

    if deferred == Fault::OnMainReturn {
        // main()'s return address was overwritten by the out-of-bounds store.
        raise_fault();
    }

    out.flush();
}
