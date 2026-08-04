// Translation of c_src/src/main.c

use std::io::{Read, Write};

use driver::engine::run_engine;
use driver::util::{vm_print, IntVec, VM};

fn usage(p: &str) {
    let stderr = std::io::stderr();
    let mut h = stderr.lock();
    write!(
        h,
        "Usage: {} [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n",
        p
    )
    .unwrap();
}

/// Reads from stdin into the IntVec, mimicking the C reader using fgets with a 4096-byte buffer.
/// IMPORTANT: The C version uses fgets with a fixed-size buffer; tokens that span buffer boundaries
/// will be split. We must reproduce this behavior to be byte-identical.
fn read_stdin(v: &mut IntVec) -> usize {
    let mut count: usize = 0;
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    // Reproduce C's fgets behavior: read up to 4095 bytes or until newline, whichever first.
    // Then process the line.
    let mut buf: Vec<u8> = Vec::new();
    loop {
        buf.clear();
        // fgets: reads until either newline (inclusive) or 4095 bytes. Stops at EOF.
        let mut got_any = false;
        let mut newline_found = false;
        while buf.len() < 4095 {
            let mut byte = [0u8; 1];
            match handle.read(&mut byte) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    got_any = true;
                    buf.push(byte[0]);
                    if byte[0] == b'\n' {
                        newline_found = true;
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        if !got_any {
            break;
        }
        let _ = newline_found;
        // Process buf: tokenize on space, tab, newline, carriage return.
        let mut i = 0usize;
        while i < buf.len() {
            // Skip past separators? No — C's algorithm: it scans chars using *p,
            // separates by NUL termination. It doesn't skip leading separators —
            // it splits, and on empty token (immediately *p==0), nothing is pushed.
            // Replicate exactly:
            let p_start = i;
            // q advances while not separator and not nul
            let mut q = p_start;
            while q < buf.len() {
                let c = buf[q];
                if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0 {
                    break;
                }
                q += 1;
            }
            // If p_start..q is non-empty, parse it
            if q > p_start {
                let token = &buf[p_start..q];
                if let Ok(s) = std::str::from_utf8(token) {
                    if let Some(parsed) = parse_strtol(s) {
                        v.push(parsed);
                        count = count.saturating_add(1);
                    }
                }
            }
            // Advance past separator (or stay if at end)
            if q < buf.len() {
                i = q + 1;
            } else {
                i = q;
            }
        }
    }
    count
}

/// Parse like C's strtol, requiring the *entire* string to be consumed (after parsing).
/// Returns Some(value) only if parse succeeded and reached end-of-string.
/// strtol skips leading whitespace, then parses optional sign + digits.
/// We must match the C behavior: `if (e && *e=='\0')`.
fn parse_strtol(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip leading whitespace (C strtol: " \t\n\v\f\r")
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0b || c == 0x0c || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }
    let start = i;
    let neg;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    } else {
        neg = false;
    }
    let digit_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // strtol: if no digits parsed, endptr is set to original start, and 0 is returned.
    // The C code's check `*e=='\0'` — e points to where parsing stopped.
    // If no digits, e == original input start (or after sign? No, e is set to nptr per spec
    // when no conversion was performed). Then *e=='\0' only if nptr was empty.
    // For our tokens, they're never empty, so no_digits → return None.
    if digit_start == i {
        // no digits parsed — endptr set to original nptr.
        // *e=='\0' iff input was empty. Since we tokenized non-empty, return None.
        // (Actually, strtol's endptr behavior: if no conversion, endptr = nptr).
        // So *e=='\0' would require the input itself to be empty.
        return None;
    }
    // After digits, must reach end for *e=='\0'
    if i != bytes.len() {
        return None;
    }
    // Parse the number
    let num_str = std::str::from_utf8(&bytes[start..i]).ok()?;
    // strtol returns long. We cast to int (i32). Out-of-range long → strtol clamps to LONG_MIN/MAX
    // and sets errno. The C code doesn't check errno; it just casts to int.
    // For our purposes, parse as i64 and cast. If it overflows i64 we mimic clamping.
    let parsed: i64 = match num_str.parse::<i64>() {
        Ok(n) => n,
        Err(_) => {
            // overflow — strtol would clamp to LONG_MAX or LONG_MIN.
            if neg {
                i64::MIN
            } else {
                i64::MAX
            }
        }
    };
    let _ = neg; // already handled in parse if sign present
    Some(parsed as i32)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prog_name = args.get(0).map(|s| s.as_str()).unwrap_or("driver");
    let mut use_stdin = false;
    let mut code = IntVec::new();

    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if a == "--help" {
            usage(prog_name);
            std::process::exit(0);
        } else if a == "--stdin" {
            use_stdin = true;
        } else {
            if let Some(parsed) = parse_strtol(a) {
                code.push(parsed);
            } else {
                eprintln!("skip '{}'", a);
            }
        }
        i += 1;
    }
    if use_stdin {
        read_stdin(&mut code);
    }
    if code.len() == 0 {
        eprintln!("no program");
        std::process::exit(2);
    }

    let mut vm_a = VM::new();
    let mut vm_b = VM::new();
    let mut vm_e = VM::new();

    let rc_a = run_engine(0, code.as_slice(), &mut vm_a);
    let rc_b = run_engine(1, code.as_slice(), &mut vm_b);
    let rc_e = run_engine(2, code.as_slice(), &mut vm_e);

    let stdout = std::io::stdout();
    let mut h = stdout.lock();
    write!(h, "RC:A={} B={} EXT={}\n", rc_a, rc_b, rc_e).unwrap();
    vm_print(&mut h, "A:", &vm_a);
    vm_print(&mut h, "B:", &vm_b);
    vm_print(&mut h, "EXT:", &vm_e);
    h.flush().unwrap();
}
