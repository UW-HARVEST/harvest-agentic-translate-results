// stdio.rs
//
// Thin emulation layer for the C standard-library I/O primitives used by the
// original program: buffered `printf` output, `fgets` line input, and the
// `sscanf(buf, "%d", &x)` integer conversion.

use std::cell::RefCell;
use std::fmt;
use std::io::{Read, Write};

const STDIN_CHUNK: usize = 8192;

struct Stdio {
    out: Vec<u8>,
    in_buf: Vec<u8>,
    in_pos: usize,
    in_eof: bool,
}

thread_local! {
    static STDIO: RefCell<Stdio> = RefCell::new(Stdio {
        out: Vec::with_capacity(1 << 16),
        in_buf: Vec::new(),
        in_pos: 0,
        in_eof: false,
    });
}

// ============================================================================
// OUTPUT (printf)
// ============================================================================

/// `printf` with a format string: bytes are accumulated in a buffer, exactly
/// like C's block-buffered `stdout`.
pub fn out_fmt(args: fmt::Arguments<'_>) {
    STDIO.with(|s| {
        let mut s = s.borrow_mut();
        // Writing into a Vec<u8> is infallible.
        let _ = s.out.write_fmt(args);
    });
}

/// `printf("%s", ...)` for a NUL-terminated char buffer: emit the raw bytes so
/// that no transcoding can alter them.
pub fn out_raw(bytes: &[u8]) {
    STDIO.with(|s| s.borrow_mut().out.extend_from_slice(bytes));
}

/// `fflush(stdout)`
pub fn out_flush() {
    STDIO.with(|s| {
        let mut s = s.borrow_mut();
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(&s.out);
        let _ = lock.flush();
        s.out.clear();
    });
}

macro_rules! printf {
    ($($arg:tt)*) => {
        $crate::stdio::out_fmt(format_args!($($arg)*))
    };
}

// ============================================================================
// INPUT (fgets)
// ============================================================================

fn fill(s: &mut Stdio) {
    if s.in_pos < s.in_buf.len() || s.in_eof {
        return;
    }
    s.in_buf.clear();
    s.in_pos = 0;
    let mut chunk = [0u8; STDIN_CHUNK];
    match std::io::stdin().read(&mut chunk) {
        Ok(0) => s.in_eof = true,
        Ok(n) => s.in_buf.extend_from_slice(&chunk[..n]),
        Err(_) => s.in_eof = true,
    }
}

/// `fgets(buf, size, stdin)`.
///
/// Reads at most `size - 1` bytes, stopping just after the first newline (the
/// newline is kept). Anything beyond that stays in the stream for the next
/// call. Returns `None` only when EOF is hit before any byte was read, which is
/// the `NULL` return that terminates the main loop in the C program.
pub fn fgets(size: usize) -> Option<Vec<u8>> {
    // The prompt has to reach the terminal before we block on input.
    out_flush();

    let limit = size - 1;
    let mut line: Vec<u8> = Vec::new();

    STDIO.with(|s| {
        let mut s = s.borrow_mut();
        while line.len() < limit {
            fill(&mut s);
            if s.in_pos >= s.in_buf.len() {
                break; // EOF
            }
            let b = s.in_buf[s.in_pos];
            s.in_pos += 1;
            line.push(b);
            if b == b'\n' {
                break;
            }
        }
    });

    if line.is_empty() {
        None
    } else {
        Some(line)
    }
}

// ============================================================================
// sscanf(buf, "%d", &out)
// ============================================================================

/// `sscanf(buf, "%d", &out)`.
///
/// Returns the number of assigned conversions (1 on success, 0 on a matching
/// failure, and 0 here for the EOF case as well -- the C code only ever
/// compares the result against 1). Conversion mirrors glibc: leading
/// whitespace is skipped, an optional sign is accepted, digits are collected
/// and converted as a `long` (saturating on overflow) which is then truncated
/// to `int`.
pub fn sscanf_int(buf: &[u8]) -> (i32, i32) {
    // C string semantics: stop at the first NUL byte.
    let s = match buf.iter().position(|&b| b == 0) {
        Some(n) => &buf[..n],
        None => buf,
    };

    let mut i = 0usize;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let start = i;
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if i == start {
        // No digits consumed: matching failure (0) or input failure (EOF).
        return (0, 0);
    }

    let as_long: i64 = if overflow {
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

    (1, as_long as i32)
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}
