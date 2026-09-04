//! Minimal emulation of the C stdio behaviour that the original program relies
//! on: a block-buffered `stdout`, an unbuffered `stderr` and `fgets`.

use std::cell::RefCell;
use std::io::{IsTerminal, Read, Write};

/// glibc uses the block size of the underlying file for `stdout` when it is not
/// a terminal; for pipes and regular files this is 4096 bytes.
const OUT_BUF_SIZE: usize = 4096;

struct OutState {
    buf: Vec<u8>,
    line_buffered: bool,
}

struct InState {
    reader: std::io::BufReader<std::io::Stdin>,
}

thread_local! {
    static OUT: RefCell<OutState> = RefCell::new(OutState {
        buf: Vec::with_capacity(OUT_BUF_SIZE),
        line_buffered: std::io::stdout().is_terminal(),
    });

    static IN: RefCell<InState> = RefCell::new(InState {
        reader: std::io::BufReader::new(std::io::stdin()),
    });
}

/// The Rust runtime ignores `SIGPIPE`, C programs do not.  Restoring the
/// default disposition makes a truncated output pipe terminate the process the
/// same way the original executable does.
pub fn restore_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // The single unsafe operation in this program: an FFI call that only
    // changes the signal disposition of the current process.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn raw_write(data: &[u8]) {
    let mut handle = std::io::stdout();
    let _ = handle.write_all(data);
    let _ = handle.flush();
}

fn drain(state: &mut OutState) {
    if !state.buf.is_empty() {
        let data = std::mem::take(&mut state.buf);
        raw_write(&data);
        state.buf.reserve(OUT_BUF_SIZE);
    }
}

/// `fwrite`-like write to the buffered `stdout`.
pub fn out_bytes(data: &[u8]) {
    if data.is_empty() {
        return;
    }
    OUT.with(|cell| {
        let mut state = cell.borrow_mut();
        state.buf.extend_from_slice(data);
        if state.line_buffered {
            if state.buf.contains(&b'\n') {
                drain(&mut state);
            }
        } else {
            while state.buf.len() >= OUT_BUF_SIZE {
                let rest = state.buf.split_off(OUT_BUF_SIZE);
                let head = std::mem::replace(&mut state.buf, rest);
                raw_write(&head);
            }
        }
    });
}

/// `printf`-style helper for plain text.
pub fn out_str(s: &str) {
    out_bytes(s.as_bytes());
}

/// Flush the buffered `stdout` (called at every program exit point).
pub fn out_flush() {
    OUT.with(|cell| {
        let mut state = cell.borrow_mut();
        drain(&mut state);
    });
}

fn out_is_line_buffered() -> bool {
    OUT.with(|cell| cell.borrow().line_buffered)
}

/// `fprintf(stderr, ...)`: stderr is unbuffered in C.
pub fn err_str(s: &str) {
    err_bytes(s.as_bytes());
}

pub fn err_bytes(data: &[u8]) {
    let mut handle = std::io::stderr();
    let _ = handle.write_all(data);
    let _ = handle.flush();
}

fn read_byte(state: &mut InState) -> Option<u8> {
    let mut b = [0u8; 1];
    loop {
        match state.reader.read(&mut b) {
            Ok(0) => return None,
            Ok(_) => return Some(b[0]),
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

/// `fgets(buf, size, stdin)`.
///
/// Reads at most `size - 1` bytes, stopping after a newline (which is kept).
/// Returns `None` when end-of-file is reached before any byte was read, which
/// mirrors `fgets` returning `NULL`.
pub fn fgets(size: usize) -> Option<Vec<u8>> {
    // glibc flushes all line buffered streams when refilling an input buffer,
    // which is what makes the interactive prompts appear on a terminal.
    if out_is_line_buffered() {
        out_flush();
    }
    if size <= 1 {
        return None;
    }
    IN.with(|cell| {
        let mut state = cell.borrow_mut();
        let mut line: Vec<u8> = Vec::new();
        while line.len() < size - 1 {
            match read_byte(&mut state) {
                None => break,
                Some(b) => {
                    line.push(b);
                    if b == b'\n' {
                        break;
                    }
                }
            }
        }
        if line.is_empty() {
            None
        } else {
            Some(line)
        }
    })
}

/// The contents of a C string stored in a byte buffer: everything up to (but
/// excluding) the first NUL byte.
pub fn c_str(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == 0) {
        Some(i) => &bytes[..i],
        None => bytes,
    }
}

/// `strstr(haystack, needle) != NULL`
pub fn c_strstr(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// `sscanf(s, "%d", &out)`: returns the number of assignments performed (0 or
/// 1) together with the converted value.
pub fn sscanf_int(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }
    if i >= s.len() {
        // Input exhausted before a conversion: `sscanf` returns EOF.
        return None;
    }
    let mut negative = false;
    if s[i] == b'+' || s[i] == b'-' {
        negative = s[i] == b'-';
        i += 1;
    }
    let start = i;
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
            Some(v) => value = v,
            None => overflow = true,
        }
        i += 1;
    }
    if i == start {
        // Matching failure.
        return None;
    }
    if overflow {
        // glibc converts through `strtol`, which saturates at LONG_MIN/LONG_MAX
        // and is then truncated to `int`.
        let saturated: i64 = if negative { i64::MIN } else { i64::MAX };
        return Some(saturated as i32);
    }
    let value = if negative { -value } else { value };
    Some(value as i32)
}

/// `isspace` in the C locale.
pub fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `isalnum` in the C locale.
pub fn is_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

/// `isalpha` in the C locale.
pub fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

/// `isdigit` in the C locale.
pub fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}
