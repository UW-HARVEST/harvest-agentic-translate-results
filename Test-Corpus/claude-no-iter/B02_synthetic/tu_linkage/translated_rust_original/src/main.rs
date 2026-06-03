// Translated from c_src/src/main.c

mod a;
mod b;
mod engine;
mod lib_target;
mod util;

use std::io::{self, Read, Write};
use std::process::ExitCode;

use engine::run_engine;
use util::{vm_print, VM};

fn usage<W: Write>(fp: &mut W, p: &str) {
    write!(
        fp,
        "Usage: {} [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n",
        p
    )
    .ok();
}

/// Mimic C's strtol(p, &e, 10) parsing where success means the parser consumed the
/// entire string (e points to '\0').
///
/// strtol skips leading whitespace, allows optional +/- sign, then parses digits.
/// We must verify the entire input consumed, the result fits in long, and matches
/// what C strtol would return when truncated to int.
fn parse_strtol_full(s: &str) -> Option<i32> {
    if s.is_empty() {
        return None;
    }
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip leading whitespace (C isspace: ' ', '\t', '\n', '\v', '\f', '\r')
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r' => i += 1,
            _ => break,
        }
    }
    let sign_idx = i;
    let mut neg = false;
    if i < bytes.len() {
        if bytes[i] == b'+' {
            i += 1;
        } else if bytes[i] == b'-' {
            neg = true;
            i += 1;
        }
    }
    // Must have at least one digit
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        // strtol fails — *endptr is set to original (or sign_idx-prior) position.
        // For our purposes, this means *e != '\0', so reject.
        let _ = sign_idx;
        return None;
    }
    // Parse digits as i64 (long is 64-bit on Linux x86_64)
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| {
                if neg {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            }) {
                Some(v) => acc = v,
                None => {
                    overflow = true;
                    acc = if neg { i64::MIN } else { i64::MAX };
                }
            }
        }
        i += 1;
    }
    // Check entire string consumed (e points to '\0')
    if i != bytes.len() {
        return None;
    }
    // Truncate to int (C: (int)t)
    Some(acc as i32)
}

fn read_stdin(v: &mut Vec<i32>) -> usize {
    // C behavior: fgets reads up to 4095 bytes per call (stopping at newline if present),
    // then tokenizes that buffer by splitting on space/tab/newline/CR. Each token is
    // passed to strtol with full-string-consumed validation. Reproduce that, including
    // the fgets buffer boundary behavior.
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut count = 0usize;

    // Emulate fgets(buf, 4096, stdin): reads up to 4095 bytes, terminating early at
    // '\n' (which is INCLUDED in the buffer). Returns NULL on EOF/error.
    let mut leftover: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1];

    'outer: loop {
        // Build one fgets-style line (up to 4095 bytes or until newline)
        let mut buf: Vec<u8> = Vec::new();
        // Pull from leftover first (none expected in this design — single-byte reader)
        let _ = &mut leftover;
        loop {
            if buf.len() >= 4095 {
                break;
            }
            match handle.read(&mut chunk) {
                Ok(0) => {
                    if buf.is_empty() {
                        break 'outer;
                    }
                    break;
                }
                Ok(_) => {
                    buf.push(chunk[0]);
                    if chunk[0] == b'\n' {
                        break;
                    }
                }
                Err(_) => {
                    if buf.is_empty() {
                        break 'outer;
                    }
                    break;
                }
            }
        }
        // Tokenize the buffer exactly like the C code
        let mut i = 0;
        while i < buf.len() {
            // Find token start (we don't skip — C just sees if *p is a separator,
            // and would set q=p so the tokenizer enters the token loop and *p='\0' first.
            // Re-read C carefully:
            //   while (*p) {
            //       char *q=p;
            //       while (*q && !sep) ++q;
            //       save=*q; *q=0;
            //       if (*p) { strtol... }   // *p might already be '\0' from prior set
            //       *q=save;
            //       p = (*q ? q+1 : q);
            //   }
            // If p starts on a separator, q==p, save=*p (sep char), *q='\0' makes *p=0,
            // so the if (*p) branch doesn't run. Then *q=save restores it, p moves to q+1.
            // So separators are skipped one at a time.
            let c = buf[i];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                i += 1;
                continue;
            }
            let start = i;
            while i < buf.len() {
                let c = buf[i];
                if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0 {
                    break;
                }
                i += 1;
            }
            // Note C also stops at NUL; we mirror that.
            if start < i {
                if let Ok(s) = std::str::from_utf8(&buf[start..i]) {
                    if let Some(t) = parse_strtol_full(s) {
                        v.push(t);
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    let argc = argv.len();
    let prog_name = argv.get(0).cloned().unwrap_or_default();

    let mut use_stdin = false;
    let mut code: Vec<i32> = Vec::new();

    let mut i = 1;
    while i < argc {
        let arg = &argv[i];
        if arg == "--help" {
            usage(&mut io::stderr(), &prog_name);
            return ExitCode::from(0);
        } else if arg == "--stdin" {
            use_stdin = true;
        } else {
            match parse_strtol_full(arg) {
                Some(t) => code.push(t),
                None => {
                    let _ = writeln!(io::stderr(), "skip '{}'", arg);
                }
            }
        }
        i += 1;
    }

    if use_stdin {
        read_stdin(&mut code);
    }

    if code.is_empty() {
        let _ = writeln!(io::stderr(), "no program");
        return ExitCode::from(2);
    }

    let mut vm_a = VM::new();
    let mut vm_b = VM::new();
    let mut vm_e = VM::new();

    let rc_a = run_engine(0, &code, &mut vm_a);
    let rc_b = run_engine(1, &code, &mut vm_b);
    let rc_e = run_engine(2, &code, &mut vm_e);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "RC:A={} B={} EXT={}\n", rc_a, rc_b, rc_e);
    vm_print(&mut out, "A:", &vm_a);
    vm_print(&mut out, "B:", &vm_b);
    vm_print(&mut out, "EXT:", &vm_e);

    ExitCode::from(0)
}
