use std::io::{self, Read, Write};

fn print_line(line: &str) {
    // Mirrors `printLine` in C: printf("%s\n", line)
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(line.as_bytes());
    let _ = out.write_all(b"\n");
}

/// Mimic C `fgets(buf, size, stdin)`.
///
/// Reads at most `size - 1` bytes from stdin. Stops after a `\n` is read
/// (the `\n` is kept in the buffer) or on EOF. Returns the bytes read, or
/// `None` if no bytes were read before EOF (matching C's NULL return).
fn fgets_stdin(size: usize) -> Option<Vec<u8>> {
    if size == 0 {
        return Some(Vec::new());
    }
    let max = size - 1;
    let mut buf: Vec<u8> = Vec::with_capacity(max);
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while buf.len() < max {
        match handle.read(&mut byte) {
            Ok(0) => break,            // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        // C's fgets returns NULL when no chars were read before EOF/error.
        None
    } else {
        Some(buf)
    }
}

/// Mimic C `atoi`.
///
/// Skips leading ASCII whitespace, optional sign, then parses base-10 digits.
/// Stops at the first non-digit. Returns 0 if no digits are seen. Behaves
/// like libc atoi for valid inputs; on overflow we saturate (atoi's overflow
/// behavior is UB in C, but most libc implementations also produce some
/// value -- we choose saturation to avoid panicking).
fn c_atoi(bytes: &[u8]) -> i32 {
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        // C's isspace for ASCII includes ' ', '\t', '\n', '\v', '\f', '\r'
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0B || c == 0x0C || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }

    let mut sign: i32 = 1;
    if i < bytes.len() {
        match bytes[i] {
            b'+' => {
                i += 1;
            }
            b'-' => {
                sign = -1;
                i += 1;
            }
            _ => {}
        }
    }

    let mut result: i64 = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if !c.is_ascii_digit() {
            break;
        }
        let digit = (c - b'0') as i64;
        result = result.saturating_mul(10).saturating_add(digit);
        if result > i32::MAX as i64 + 1 {
            // saturate to avoid overflow.
            result = i32::MAX as i64 + 1;
        }
        i += 1;
    }

    let signed = (sign as i64) * result;
    if signed > i32::MAX as i64 {
        i32::MAX
    } else if signed < i32::MIN as i64 {
        i32::MIN
    } else {
        signed as i32
    }
}

fn main() {
    let mut data: i32 = -1;
    // Block 1: fgets into 14-byte buffer
    {
        match fgets_stdin(14) {
            Some(input_buffer) => {
                // Convert to int (C atoi over the C string up to first NUL).
                // The buffer never contains an embedded NUL from fgets, so
                // we can pass it directly.
                data = c_atoi(&input_buffer);
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }

    // Block 2: build source[100] of 'A' (null-terminated at index 99),
    // dest[100] zero-initialized; if data < 100, copy `data` chars from
    // source to dest then null-terminate at dest[data].
    {
        // C: char source[100]; memset(source, 'A', 99); source[99] = '\0';
        //    char dest[100] = ""; (zero-initialised)
        let source: [u8; 100] = {
            let mut s = [b'A'; 100];
            s[99] = 0;
            s
        };
        let mut dest: [u8; 100] = [0u8; 100];

        if data < 100 {
            // C: strncpy(dest, source, data); dest[data] = '\0';
            //
            // For data >= 0 and data <= 99 this copies `data` 'A's into
            // dest (strncpy null-pads if source is shorter, but source
            // has 99 'A's before NUL). Then dest[data] = '\0'.
            //
            // For data < 0 the original C invokes UB by passing a negative
            // value as size_t to strncpy. We avoid replicating UB; in the
            // realistic case where data < 0 only happens after a fgets()
            // failure, the printed message before this block is the
            // observable output that tests will check.
            if data >= 0 {
                let n = data as usize;
                // strncpy copies min(n, strlen(source)) bytes then null pads.
                // source has 99 'A's, so for n <= 99 we copy n 'A's.
                let copy_len = n.min(99); // strlen(source) == 99
                for i in 0..copy_len {
                    dest[i] = source[i];
                }
                // dest[data] = '\0';  (n is in 0..=99 since data < 100)
                if n < dest.len() {
                    dest[n] = 0;
                }
            }
            // else: skip — original C would invoke UB here.
        }

        // printLine(dest) — prints up to first NUL.
        let nul = dest.iter().position(|&b| b == 0).unwrap_or(dest.len());
        let s = std::str::from_utf8(&dest[..nul]).unwrap_or("");
        print_line(s);
    }

    // Ensure stdout is flushed before exit.
    let _ = io::stdout().flush();
}
